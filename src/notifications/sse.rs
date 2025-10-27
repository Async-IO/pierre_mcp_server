// ABOUTME: Server-Sent Events implementation for real-time OAuth notifications
// ABOUTME: Handles SSE connections with resource bounds, message broadcasting, and cleanup

use crate::database::oauth_notifications::OAuthNotification;
use crate::errors::AppError;
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::Instant;
use warp::{http::StatusCode, Filter, Rejection, Reply};

/// Metadata for an SSE connection (creation time for timeout enforcement)
#[derive(Clone)]
struct SseConnection {
    sender: broadcast::Sender<String>,
    created_at: Instant,
}

/// SSE connection manager for tracking active client connections with resource bounds
/// Enforces per-user connection limits and automatic cleanup of stale connections
#[derive(Clone)]
pub struct SseConnectionManager {
    connections: Arc<RwLock<std::collections::HashMap<String, Vec<SseConnection>>>>,
    max_per_user: usize,
    timeout: std::time::Duration,
}

impl Default for SseConnectionManager {
    fn default() -> Self {
        Self {
            connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_per_user: crate::constants::network_config::SSE_MAX_CONNECTIONS_PER_USER,
            timeout: std::time::Duration::from_secs(
                crate::constants::timeouts::SSE_CONNECTION_TIMEOUT_SECS,
            ),
        }
    }
}

impl SseConnectionManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new SSE connection for a user with enforced connection limits
    ///
    /// # Errors
    /// Returns `ResourceUnavailable` if user already has max connections
    pub async fn register_connection(
        &self,
        user_id: String,
    ) -> Result<broadcast::Receiver<String>, AppError> {
        let (tx, rx) =
            broadcast::channel(crate::constants::network_config::SSE_BROADCAST_CHANNEL_SIZE);

        let connection = SseConnection {
            sender: tx,
            created_at: Instant::now(),
        };

        // Minimize lock scope: only hold lock during actual entry modification
        let mut connections = self.connections.write().await;
        let user_conns = connections.entry(user_id.clone()).or_insert_with(Vec::new);

        // Enforce per-user connection limit
        if user_conns.len() >= self.max_per_user {
            return Err(AppError::new(
                crate::errors::ErrorCode::ResourceUnavailable,
                format!(
                    "Maximum SSE connections ({}) reached for user {}",
                    self.max_per_user, user_id
                ),
            ));
        }

        user_conns.push(connection);
        drop(connections); // Explicitly drop lock before async work

        tracing::info!("SSE connection registered for user: {}", user_id);
        Ok(rx)
    }

    /// Remove an SSE connection when client disconnects
    pub async fn unregister_connection(&self, user_id: &str) {
        self.connections.write().await.remove(user_id);
        tracing::info!("SSE connection unregistered for user: {}", user_id);
    }

    /// Send notification to a specific user via SSE
    ///
    /// # Errors
    /// Returns an error if no SSE connection exists for the user or if sending fails
    pub async fn send_notification(
        &self,
        user_id: &str,
        notification: &OAuthNotification,
    ) -> Result<()> {
        let connections = self.connections.read().await;

        connections
            .get(user_id)
            .map(|user_conns| {
                let sse_message = format!(
                    "data: {}\n\n",
                    json!({
                        "type": "oauth_notification",
                        "id": notification.id,
                        "provider": notification.provider,
                        "message": notification.message,
                        "success": notification.success,
                        "created_at": notification.created_at
                    })
                );

                for conn in user_conns {
                    // Silently drop failed sends (client disconnected)
                    // This is expected behavior - clients disconnect and we clean up
                    let _ = conn.sender.send(sse_message.clone());
                }

                tracing::info!("SSE notification sent successfully to user: {}", user_id);
            })
            .ok_or_else(|| {
                tracing::warn!("No SSE connection found for user: {}", user_id);
                AppError::not_found("No SSE connection found for user").into()
            })
    }

    /// Cleanup stale SSE connections older than the timeout
    /// Called periodically to prevent resource exhaustion from long-lived connections
    pub async fn cleanup_stale_connections(&self) {
        let now = Instant::now();
        let mut connections = self.connections.write().await;

        connections.retain(|_user_id, user_conns| {
            // Remove timed-out connections
            user_conns.retain(|conn| now.duration_since(conn.created_at) < self.timeout);

            // Remove user entry if no connections left
            !user_conns.is_empty()
        });

        tracing::debug!(
            "SSE cleanup completed - {} users have active connections",
            connections.len()
        );
    }

    /// Get count of active connections (for monitoring)
    pub async fn active_connections(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Spawn background cleanup task for periodic removal of stale connections
    ///
    /// This task runs every `SSE_CLEANUP_INTERVAL_SECS` and removes connections
    /// that have exceeded the configured timeout.
    ///
    /// # Returns
    /// `JoinHandle` to the spawned task for lifecycle management
    ///
    /// # Example
    /// ```no_run
    /// use pierre_mcp_server::notifications::sse::SseConnectionManager;
    /// use std::sync::Arc;
    ///
    /// let manager = Arc::new(SseConnectionManager::new());
    /// let cleanup_handle = SseConnectionManager::spawn_cleanup_task(manager.clone());
    /// // Task runs in background until handle is dropped or server shuts down
    /// ```
    #[must_use]
    pub fn spawn_cleanup_task(manager: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let cleanup_interval =
            std::time::Duration::from_secs(crate::constants::timeouts::SSE_CLEANUP_INTERVAL_SECS);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                tracing::debug!(
                    "Running SSE connection cleanup task (timeout={}s)",
                    crate::constants::timeouts::SSE_CONNECTION_TIMEOUT_SECS
                );
                manager.cleanup_stale_connections().await;
            }
        })
    }
}

/// Handle SSE connection endpoint
///
/// # Errors
/// Returns a rejection if the SSE stream cannot be created or connection limit exceeded
pub async fn handle_sse_connection(
    user_id: String,
    manager: Arc<SseConnectionManager>,
) -> Result<impl Reply, Rejection> {
    tracing::info!("New SSE connection request for user: {}", user_id);

    let mut receiver = manager
        .register_connection(user_id.clone())
        .await
        .map_err(warp::reject::custom)?;

    let manager_clone = manager.clone(); // Safe: Arc clone for async stream
    let user_id_clone = user_id.clone(); // Safe: String ownership for async stream

    let stream = async_stream::stream! {
        // Send initial connection established event
        yield Ok::<_, warp::Error>(warp::sse::Event::default()
            .data("connected")
            .event("connection"));

        // Listen for notifications
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    yield Ok(warp::sse::Event::default()
                        .data(message)
                        .event("notification"));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    tracing::warn!("SSE receiver lagged for user: {}", user_id);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!("SSE channel closed for user: {}", user_id);
                    break;
                }
            }
        }

        // Clean up connection
        manager_clone.unregister_connection(&user_id_clone).await;
    };

    // Configure keepalive with 15-second interval
    let keep = warp::sse::keep_alive()
        .interval(std::time::Duration::from_secs(15))
        .text(": keepalive\n\n");

    Ok(warp::sse::reply(keep.stream(stream)))
}

/// SSE route filter for user authentication
#[must_use]
pub fn sse_routes(
    manager: Arc<SseConnectionManager>,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("notifications")
        .and(warp::path("sse"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and_then({
            move |params: HashMap<String, String>| {
                let manager = manager.clone(); // Safe: Arc clone for HTTP handler closure
                async move {
                    let user_id = params
                        .get("user_id")
                        .ok_or_else(|| warp::reject::custom(InvalidUserIdError))?
                        .clone(); // Safe: String ownership from HashMap

                    // Accept user_id from query parameter for SSE connection

                    handle_sse_connection(user_id, manager).await
                }
            }
        })
}

/// Custom error for invalid user ID
#[derive(Debug)]
struct InvalidUserIdError;

impl warp::reject::Reject for InvalidUserIdError {}

/// Error handling for SSE endpoints
///
/// # Errors
/// This function cannot fail (returns Infallible) but processes warp rejections
pub fn handle_sse_rejection(err: &Rejection) -> Result<impl Reply, std::convert::Infallible> {
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
