// ABOUTME: Unified SSE route handlers for both OAuth notifications and MCP protocol streaming
// ABOUTME: Provides HTTP endpoints for establishing SSE connections with proper session management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::manager::SseManager;
use crate::config::environment::SseBufferStrategy;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::mcp::resources::ServerResources;
use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    Router,
};
use futures_util::stream::Stream;
use std::{convert::Infallible, sync::Arc, time::Duration};
use uuid::Uuid;

/// SSE routes implementation
pub struct SseRoutes;

impl SseRoutes {
    /// Create all SSE routes
    pub fn routes(manager: Arc<SseManager>, resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route(
                "/notifications/sse/:user_id",
                axum::routing::get(Self::handle_notification_sse),
            )
            .route(
                "/mcp/sse/:session_id",
                axum::routing::get(Self::handle_protocol_sse),
            )
            .route(
                "/a2a/tasks/:task_id/stream",
                axum::routing::get(Self::handle_a2a_task_sse),
            )
            .with_state((manager, resources))
    }

    /// Handle OAuth notification SSE connection
    ///
    /// REQUIRES: JWT authentication (Bearer token in Authorization header)
    ///
    /// Security: Only authenticated users can subscribe to their own notification streams
    /// to prevent unauthorized access to OAuth tokens and personal notifications.
    async fn handle_notification_sse(
        Path(user_id): Path<String>,
        headers: axum::http::HeaderMap,
        State((manager, resources)): State<(Arc<SseManager>, Arc<ServerResources>)>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
        tracing::info!("New notification SSE connection for user: {}", user_id);

        let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
            tracing::warn!(user_id = %user_id, error = %e, "Invalid user ID format for SSE connection");
            AppError::invalid_input(format!("Invalid user ID format: {e}"))
        })?;

        // Extract and validate JWT token
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!(user_id = %user_uuid, "Missing Authorization header for SSE notification stream");
                AppError::auth_invalid("Missing Authorization header - JWT token required for SSE notifications")
            })?;

        let token = crate::utils::auth::extract_bearer_token_owned(auth_header).map_err(|_| {
            tracing::warn!(user_id = %user_uuid, "Invalid Authorization header format for SSE");
            AppError::auth_invalid("Invalid Authorization header format")
        })?;

        // Authenticate user
        let auth_result = resources
            .auth_middleware
            .authenticate_request(Some(&format!("Bearer {token}")))
            .await
            .map_err(|e| {
                tracing::warn!(user_id = %user_uuid, error = %e, "Failed to authenticate JWT token for SSE");
                AppError::auth_invalid(format!("Authentication failed: {e}"))
            })?;

        // Verify authenticated user matches requested user_id
        if auth_result.user_id != user_uuid {
            tracing::warn!(
                authenticated_user = %auth_result.user_id,
                requested_user = %user_uuid,
                "User attempting to access another user's SSE notification stream"
            );
            return Err(AppError::auth_invalid(
                "Cannot access notification stream for another user",
            ));
        }

        let mut receiver = manager.register_notification_stream(user_uuid).await;
        let manager_clone = manager.clone();
        let user_id_clone = user_uuid;
        let overflow_strategy = resources.config.sse.buffer_overflow_strategy;

        let stream = async_stream::stream! {
            // Send initial connection established event with sequential event IDs
            let mut event_id: u64 = 0;
            event_id += 1;
            yield Ok::<_, Infallible>(
                Event::default()
                    .id(event_id.to_string())
                    .data("connected")
                    .event("connection")
            );

            // Listen for notifications
            loop {
                match receiver.recv().await {
                    Ok(message) => {
                        event_id += 1;
                        yield Ok(
                            Event::default()
                                .id(event_id.to_string())
                                .data(message)
                                .event("notification")
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "SSE buffer overflow for user {}: {} messages dropped (strategy: {:?})",
                            user_id_clone, skipped, overflow_strategy
                        );

                        // Handle overflow based on configured strategy
                        match overflow_strategy {
                            SseBufferStrategy::DropOldest => {
                                // Continue operation - this is the default broadcast behavior
                                tracing::debug!("Continuing with DropOldest strategy for user {}", user_id_clone);
                            }
                            SseBufferStrategy::DropNew => {
                                // Note: broadcast channels inherently drop oldest, not newest
                                // For true DropNew behavior, would need mpsc bounded channel
                                tracing::warn!(
                                    "DropNew strategy configured but broadcast channels drop oldest. \
                                    Consider using bounded mpsc for true DropNew behavior."
                                );
                            }
                            SseBufferStrategy::CloseConnection => {
                                tracing::info!(
                                    "Closing SSE connection for user {} due to buffer overflow",
                                    user_id_clone
                                );
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("SSE channel closed for user: {}", user_id_clone);
                        break;
                    }
                }
            }

            // Clean up connection
            manager_clone.unregister_notification_stream(user_id_clone).await;
        };

        // Configure keepalive with 15-second interval
        Ok(Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keepalive"),
        ))
    }

    /// Handle MCP protocol SSE connection
    ///
    /// REQUIRES: JWT authentication (Bearer token in Authorization header or Mcp-Session-Id)
    ///
    /// Security: Only authenticated users can establish SSE streams for MCP protocol
    /// to prevent unauthorized access to protocol messages and session hijacking.
    async fn handle_protocol_sse(
        Path(session_id): Path<String>,
        headers: axum::http::HeaderMap,
        State((manager, resources)): State<(Arc<SseManager>, Arc<ServerResources>)>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
        tracing::info!(
            "New MCP protocol SSE connection for session: {}",
            session_id
        );

        // Extract authorization header for session validation
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        // Validate authentication if provided
        if let Some(ref auth) = auth_header {
            let token = crate::utils::auth::extract_bearer_token_owned(auth).map_err(|_| {
                tracing::warn!(session_id = %session_id, "Invalid Authorization header format for MCP SSE");
                AppError::auth_invalid("Invalid Authorization header format")
            })?;

            // Authenticate user to ensure valid JWT
            resources
                .auth_middleware
                .authenticate_request(Some(&format!("Bearer {token}")))
                .await
                .map_err(|e| {
                    tracing::warn!(session_id = %session_id, error = %e, "Failed to authenticate JWT token for MCP SSE");
                    AppError::auth_invalid(format!("Authentication failed: {e}"))
                })?;
        } else {
            // MCP SSE requires authentication
            tracing::warn!(session_id = %session_id, "Missing Authorization header for MCP SSE connection");
            return Err(AppError::auth_invalid(
                "Missing Authorization header - JWT token required for MCP SSE",
            ));
        }

        let mut receiver = manager
            .register_protocol_stream(session_id.clone(), auth_header, resources.clone())
            .await;
        let manager_clone = manager.clone();
        let session_id_clone = session_id.clone();

        let stream = async_stream::stream! {
            // Send initial connection established event
            let mut event_id: u64 = 0;
            event_id += 1;
            yield Ok::<_, Infallible>(
                Event::default()
                    .id(event_id.to_string())
                    .data("connected")
                    .event("connection")
            );

            // Listen for MCP protocol messages
            loop {
                match receiver.recv().await {
                    Ok(message) => {
                        event_id += 1;
                        yield Ok(
                            Event::default()
                                .id(event_id.to_string())
                                .data(message)
                                .event("message")
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "SSE buffer overflow for session {}: {} messages dropped",
                            session_id_clone, skipped
                        );
                        // Continue operation for protocol streams
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("SSE channel closed for session: {}", session_id_clone);
                        break;
                    }
                }
            }

            // Clean up connection
            manager_clone.unregister_protocol_stream(&session_id_clone).await;
        };

        // Configure keepalive with 15-second interval
        Ok(Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keepalive"),
        ))
    }

    /// Handle A2A task SSE connection for task progress streaming
    ///
    /// REQUIRES: JWT authentication (Bearer token in Authorization header)
    ///
    /// Security: Only authenticated users can subscribe to A2A task streams
    /// to prevent unauthorized monitoring of agent-to-agent task progress.
    async fn handle_a2a_task_sse(
        Path(task_id): Path<String>,
        headers: axum::http::HeaderMap,
        State((manager, resources)): State<(Arc<SseManager>, Arc<ServerResources>)>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
        tracing::info!("New A2A task SSE connection for task: {}", task_id);

        // Extract and validate JWT token
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!(task_id = %task_id, "Missing Authorization header for A2A task SSE");
                AppError::auth_invalid(
                    "Missing Authorization header - JWT token required for A2A task streams",
                )
            })?;

        let token = crate::utils::auth::extract_bearer_token_owned(auth_header).map_err(|_| {
            tracing::warn!(task_id = %task_id, "Invalid Authorization header format for A2A SSE");
            AppError::auth_invalid("Invalid Authorization header format")
        })?;

        // Authenticate user
        let auth_result = resources
            .auth_middleware
            .authenticate_request(Some(&format!("Bearer {token}")))
            .await
            .map_err(|e| {
                tracing::warn!(task_id = %task_id, error = %e, "Failed to authenticate JWT token for A2A SSE");
                AppError::auth_invalid(format!("Authentication failed: {e}"))
            })?;

        tracing::info!(
            task_id = %task_id,
            user_id = %auth_result.user_id,
            "Authenticated A2A task SSE connection"
        );

        // Verify task exists in database
        let task = resources.database
            .get_a2a_task(&task_id)
            .await
            .map_err(|e| {
                tracing::error!(task_id = %task_id, error = %e, "Failed to fetch task for SSE streaming");
                AppError::internal(format!("Failed to fetch task: {e}"))
            })?
            .ok_or_else(|| {
                tracing::warn!(task_id = %task_id, "Task not found for SSE streaming");
                AppError::not_found(format!("Task {task_id} not found"))
            })?;

        let actual_client_id = task.client_id.clone();
        let mut receiver = manager
            .register_a2a_task_stream(task_id.clone(), actual_client_id)
            .await;
        let manager_clone = manager.clone();
        let task_id_clone = task_id.clone();

        let stream = async_stream::stream! {
            // Send initial connection event with current task status
            let mut event_id: u64 = 0;
            event_id += 1;

            // Send initial task state
            let initial_state = serde_json::json!({
                "task_id": task_id,
                "status": task.status,
                "created_at": task.created_at,
                "updated_at": task.updated_at,
            });

            yield Ok::<_, Infallible>(
                Event::default()
                    .id(event_id.to_string())
                    .data(serde_json::to_string(&initial_state).unwrap_or_else(|_| "{}".to_owned()))
                    .event("task_status")
            );

            // Listen for task updates
            loop {
                match receiver.recv().await {
                    Ok(message) => {
                        event_id += 1;
                        yield Ok(
                            Event::default()
                                .id(event_id.to_string())
                                .data(message)
                                .event("task_update")
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "SSE buffer overflow for task {}: {} messages dropped",
                            task_id_clone, skipped
                        );
                        // Continue operation
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("SSE channel closed for task: {}", task_id_clone);
                        break;
                    }
                }
            }

            // Clean up connection
            manager_clone.unregister_a2a_task_stream(&task_id_clone).await;
        };

        // Configure keepalive with 15-second interval
        Ok(Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keepalive"),
        ))
    }
}
