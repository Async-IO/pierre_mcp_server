// ABOUTME: Unified SSE route handlers for both OAuth notifications and MCP protocol streaming
// ABOUTME: Provides HTTP endpoints for establishing SSE connections with proper session management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::manager::SseManager;
use crate::config::environment::SseBufferStrategy;
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
            .with_state((manager, resources))
    }

    /// Handle OAuth notification SSE connection
    async fn handle_notification_sse(
        Path(user_id): Path<String>,
        State((manager, resources)): State<(Arc<SseManager>, Arc<ServerResources>)>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
        tracing::info!("New notification SSE connection for user: {}", user_id);

        let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
            tracing::warn!(user_id = %user_id, error = %e, "Invalid user ID format for SSE connection");
            AppError::invalid_input(format!("Invalid user ID format: {e}"))
        })?;

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
    async fn handle_protocol_sse(
        Path(session_id): Path<String>,
        State((manager, resources)): State<(Arc<SseManager>, Arc<ServerResources>)>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
        tracing::info!(
            "New MCP protocol SSE connection for session: {}",
            session_id
        );

        let mut receiver = manager
            .register_protocol_stream(session_id.clone(), None, resources.clone())
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
}
