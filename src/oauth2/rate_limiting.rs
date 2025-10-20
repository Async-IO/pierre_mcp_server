// ABOUTME: OAuth2 endpoint rate limiting with RFC-compliant headers and rejection handling
// ABOUTME: Implements per-IP token bucket rate limiting for authorization, token, and registration endpoints

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use warp::{Filter, Rejection, Reply};

/// `OAuth2` rate limiter with per-IP tracking
#[derive(Clone)]
pub struct OAuth2RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
    config: crate::rate_limiting::OAuth2RateLimitConfig,
}

struct RateLimiterState {
    /// Per-IP request tracking: IP -> (`request_count`, `window_start`)
    requests: HashMap<IpAddr, (u32, Instant)>,
}

impl OAuth2RateLimiter {
    /// Create new `OAuth2` rate limiter with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                requests: HashMap::new(),
            })),
            config: crate::rate_limiting::OAuth2RateLimitConfig::new(),
        }
    }

    /// Create `OAuth2` rate limiter with custom configuration
    #[must_use]
    pub fn with_config(config: crate::rate_limiting::OAuth2RateLimitConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                requests: HashMap::new(),
            })),
            config,
        }
    }

    /// Check rate limit for a specific endpoint and IP
    #[must_use]
    pub fn check_rate_limit(
        &self,
        endpoint: &str,
        client_ip: IpAddr,
    ) -> crate::rate_limiting::OAuth2RateLimitStatus {
        let limit = self.config.get_limit(endpoint);
        let now = Instant::now();
        let window = Duration::from_secs(60); // 1 minute window

        let (is_limited, remaining, window_start) = {
            let mut state = self.state.lock().unwrap_or_else(|poisoned| {
                tracing::warn!("OAuth2 rate limiter lock poisoned, recovering");
                poisoned.into_inner()
            });

            // Clean up old entries (older than 2 minutes)
            state.requests.retain(|_ip, (_count, start)| {
                now.duration_since(*start) < Duration::from_secs(120)
            });

            // Get or create entry for this IP
            let entry = state.requests.entry(client_ip).or_insert((0, now));
            let (count, window_start) = entry;

            // Reset window if expired
            if now.duration_since(*window_start) >= window {
                *count = 0;
                *window_start = now;
            }

            let remaining = limit.saturating_sub(*count);
            let is_limited = *count >= limit;

            // Increment count if not limited
            if !is_limited {
                *count += 1;
            }

            let result = (is_limited, remaining, *window_start);
            drop(state); // Explicitly drop mutex guard before time-consuming operations
            result
        };

        // Calculate reset time (convert Instant to Unix timestamp)
        let now_system = std::time::SystemTime::now();
        let elapsed_from_window_start = now.duration_since(window_start);
        let reset_system = now_system + (window - elapsed_from_window_start);
        #[allow(clippy::cast_possible_wrap)]
        // Safe: Unix timestamps fit in i64 range for next several centuries
        let reset_at = reset_system
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as i64;

        crate::rate_limiting::OAuth2RateLimitStatus {
            is_limited,
            limit,
            remaining,
            reset_at,
            retry_after_seconds: None,
        }
        .with_retry_after()
    }

    /// Create warp filter for rate limiting
    #[must_use]
    pub fn filter(
        &self,
        endpoint: &'static str,
    ) -> impl Filter<Extract = (crate::rate_limiting::OAuth2RateLimitStatus,), Error = Rejection> + Clone
    {
        let limiter = self.clone();
        warp::addr::remote().and_then(move |addr: Option<std::net::SocketAddr>| {
            let limiter = limiter.clone();
            async move {
                let client_ip =
                    addr.map_or_else(|| IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), |a| a.ip());

                let status = limiter.check_rate_limit(endpoint, client_ip);

                if status.is_limited {
                    tracing::warn!(
                        "OAuth2 rate limit exceeded for {} from IP {}: {}/{} requests",
                        endpoint,
                        client_ip,
                        status.limit,
                        status.limit
                    );
                    Err(warp::reject::custom(OAuth2RateLimitExceeded { status }))
                } else {
                    Ok(status)
                }
            }
        })
    }
}

impl Default for OAuth2RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// `OAuth2` rate limit exceeded rejection
#[derive(Debug)]
pub struct OAuth2RateLimitExceeded {
    pub status: crate::rate_limiting::OAuth2RateLimitStatus,
}

impl warp::reject::Reject for OAuth2RateLimitExceeded {}

/// Add rate limit headers to any reply
pub fn with_rate_limit_headers<T: Reply>(
    reply: T,
    status: &crate::rate_limiting::OAuth2RateLimitStatus,
) -> impl Reply {
    let reply = warp::reply::with_header(reply, "X-RateLimit-Limit", status.limit.to_string());
    let reply =
        warp::reply::with_header(reply, "X-RateLimit-Remaining", status.remaining.to_string());
    warp::reply::with_header(reply, "X-RateLimit-Reset", status.reset_at.to_string())
}

/// Handle rate limit rejection and return 429 response with proper headers
///
/// # Errors
///
/// Returns the original rejection if it is not an `OAuth2RateLimitExceeded` rejection
#[allow(clippy::option_if_let_else)]
// Clippy's suggested map_or_else pattern doesn't work here due to borrow checker constraints
pub async fn handle_rate_limit_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(rate_limit_exceeded) = err.find::<OAuth2RateLimitExceeded>() {
        let retry_after = rate_limit_exceeded.status.retry_after_seconds.unwrap_or(60);

        let json = warp::reply::json(&serde_json::json!({
            "error": "rate_limit_exceeded",
            "error_description": format!(
                "Rate limit exceeded. Retry after {} seconds.",
                retry_after
            )
        }));

        let reply = warp::reply::with_status(json, warp::http::StatusCode::TOO_MANY_REQUESTS);
        let reply = warp::reply::with_header(reply, "Retry-After", retry_after.to_string());
        let reply = warp::reply::with_header(
            reply,
            "X-RateLimit-Limit",
            rate_limit_exceeded.status.limit.to_string(),
        );
        let reply = warp::reply::with_header(
            reply,
            "X-RateLimit-Remaining",
            rate_limit_exceeded.status.remaining.to_string(),
        );
        let reply = warp::reply::with_header(
            reply,
            "X-RateLimit-Reset",
            rate_limit_exceeded.status.reset_at.to_string(),
        );

        Ok(Box::new(reply) as Box<dyn Reply>)
    } else {
        Err(err)
    }
}
