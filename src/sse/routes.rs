// ABOUTME: Unified SSE route handlers for both OAuth notifications and MCP protocol streaming
// ABOUTME: Provides HTTP endpoints for establishing SSE connections with proper session management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::manager::SseManager;
use crate::mcp::resources::ServerResources;
use anyhow::Result;
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use uuid::Uuid;
use warp::{http::StatusCode, Filter, Rejection, Reply};

/// Handle OAuth notification SSE connection
///
/// # Errors
///
/// Returns a rejection if the user ID is invalid or cannot be parsed as a UUID
pub async fn handle_notification_sse(
    user_id: String,
    manager: Arc<SseManager>,
    resources: Arc<ServerResources>,
) -> Result<impl Reply, Rejection> {
    tracing::info!("New notification SSE connection for user: {}", user_id);

    let user_uuid =
        Uuid::parse_str(&user_id).map_err(|_| warp::reject::custom(InvalidUserIdError))?;

    let mut receiver = manager.register_notification_stream(user_uuid).await;
    let manager_clone = manager.clone();
    let user_id_clone = user_uuid;
    let overflow_strategy = resources.config.sse.buffer_overflow_strategy;

    let stream = async_stream::stream! {
        // Send initial connection established event with sequential event IDs
        let mut event_id: u64 = 0;
        event_id += 1;
        yield Ok::<_, warp::Error>(warp::sse::Event::default()
            .id(event_id.to_string())
            .data("connected")
            .event("connection"));

        // Listen for notifications
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    event_id += 1;
                    yield Ok(warp::sse::Event::default()
                        .id(event_id.to_string())
                        .data(message)
                        .event("notification"));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    use crate::config::environment::SseBufferStrategy;
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
    let keep = warp::sse::keep_alive()
        .interval(std::time::Duration::from_secs(15))
        .text(": keepalive\n\n");

    Ok(warp::sse::reply(keep.stream(stream)))
}

/// Handle MCP protocol SSE connection with session management
///
/// # Errors
///
/// This function returns an error if the SSE stream registration fails
pub async fn handle_protocol_sse(
    session_id: Option<String>,
    authorization: Option<String>,
    manager: Arc<SseManager>,
    resources: Arc<ServerResources>,
) -> Result<impl Reply, Rejection> {
    let session_id = session_id.unwrap_or_else(|| format!("session_{}", uuid::Uuid::new_v4()));

    tracing::info!(
        "New MCP protocol SSE connection for session: {}",
        session_id
    );

    let overflow_strategy = resources.config.sse.buffer_overflow_strategy;
    let mut receiver = manager
        .register_protocol_stream(session_id.clone(), authorization, resources)
        .await;
    let manager_clone = manager.clone();
    let session_id_clone = session_id.clone();

    let stream = async_stream::stream! {
        // Send initial connection established event with sequential event IDs
        let mut event_id: u64 = 0;
        event_id += 1;
        yield Ok::<_, warp::Error>(warp::sse::Event::default()
            .id(event_id.to_string())
            .data("MCP protocol stream ready")
            .event("connected"));

        // Listen for MCP messages
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    event_id += 1;
                    yield Ok(warp::sse::Event::default()
                        .id(event_id.to_string())
                        .data(message)
                        .event("message"));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    use crate::config::environment::SseBufferStrategy;
                    tracing::warn!(
                        "SSE buffer overflow for session {}: {} messages dropped (strategy: {:?})",
                        session_id_clone, skipped, overflow_strategy
                    );

                    // Handle overflow based on configured strategy
                    match overflow_strategy {
                        SseBufferStrategy::DropOldest => {
                            // Continue operation - this is the default broadcast behavior
                            tracing::debug!("Continuing with DropOldest strategy for session {}", session_id_clone);
                        }
                        SseBufferStrategy::DropNew => {
                            // Note: broadcast channels inherently drop oldest, not newest
                            tracing::warn!(
                                "DropNew strategy configured but broadcast channels drop oldest. \
                                Consider using bounded mpsc for true DropNew behavior."
                            );
                        }
                        SseBufferStrategy::CloseConnection => {
                            tracing::info!(
                                "Closing SSE connection for session {} due to buffer overflow",
                                session_id_clone
                            );
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("SSE protocol channel closed for session: {}", session_id_clone);
                    break;
                }
            }
        }

        // Clean up connection
        manager_clone.unregister_protocol_stream(&session_id_clone).await;
    };

    // Configure keepalive with 15-second interval and include session ID in response headers
    let keep = warp::sse::keep_alive()
        .interval(std::time::Duration::from_secs(15))
        .text(": keepalive\n\n");
    let response = warp::sse::reply(keep.stream(stream));
    Ok(warp::reply::with_header(
        response,
        "Mcp-Session-Id",
        session_id,
    ))
}

/// OAuth notification SSE route filter
#[must_use]
pub fn notification_sse_routes(
    manager: Arc<SseManager>,
    resources: Arc<ServerResources>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("notifications")
        .and(warp::path("sse"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and_then({
            move |params: HashMap<String, String>| {
                let manager = manager.clone();
                let resources = resources.clone();
                async move {
                    let user_id = params
                        .get("user_id")
                        .ok_or_else(|| warp::reject::custom(InvalidUserIdError))?
                        .clone();

                    handle_notification_sse(user_id, manager, resources).await
                }
            }
        })
}

/// MCP protocol SSE route filter
#[must_use]
pub fn protocol_sse_routes(
    manager: Arc<SseManager>,
    resources: Arc<ServerResources>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("mcp")
        .and(warp::path("sse"))
        .and(warp::path::end())
        .and(warp::get())
        .and(extract_session_id())
        .and(extract_authorization())
        .and_then({
            move |session_id: Option<String>, authorization: Option<String>| {
                let manager = manager.clone();
                let resources = resources.clone();
                async move {
                    handle_protocol_sse(session_id, authorization, manager, resources).await
                }
            }
        })
}

/// Combined SSE routes
#[must_use]
pub fn sse_routes(
    manager: Arc<SseManager>,
    resources: Arc<ServerResources>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    notification_sse_routes(manager.clone(), resources.clone())
        .or(protocol_sse_routes(manager, resources))
}

/// Extract session ID from headers or query parameters
fn extract_session_id() -> impl Filter<Extract = (Option<String>,), Error = warp::Rejection> + Clone
{
    warp::header::optional::<String>("mcp-session-id")
        .or(warp::query::<HashMap<String, String>>()
            .map(|params: HashMap<String, String>| params.get("session_id").cloned()))
        .unify()
}

/// Extract authorization header
fn extract_authorization(
) -> impl Filter<Extract = (Option<String>,), Error = warp::Rejection> + Clone {
    warp::header::optional::<String>("authorization")
}

/// Custom error for invalid user ID
#[derive(Debug)]
struct InvalidUserIdError;

impl warp::reject::Reject for InvalidUserIdError {}

/// Error handling for SSE endpoints
///
/// # Errors
///
/// This function never returns an error - it converts all rejections to appropriate HTTP responses
pub fn handle_sse_rejection(err: &Rejection) -> Result<impl Reply, Infallible> {
    if err.find::<InvalidUserIdError>().is_some() {
        Ok(warp::reply::with_status(
            "Invalid or missing user_id parameter",
            StatusCode::BAD_REQUEST,
        ))
    } else {
        Ok(warp::reply::with_status(
            "Internal server error",
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}
