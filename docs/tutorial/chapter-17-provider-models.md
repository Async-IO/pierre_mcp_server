<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 17: Provider Data Models & Rate Limiting

This chapter explores how Pierre abstracts fitness provider APIs through unified interfaces and handles rate limiting across multiple providers. You'll learn about trait-based provider abstraction, provider-agnostic data models, retry logic, and tenant-aware provider wrappers.

## What You'll Learn

- Trait-based provider abstraction
- Provider-agnostic data models
- Async trait implementation
- Rate limit handling with exponential backoff
- Provider error types with structured context
- Type conversion utilities for API data
- Tenant-aware provider wrappers
- Cursor-based vs offset-based pagination
- Optional trait methods with default implementations

## Provider Abstraction Architecture

Pierre uses a trait-based approach to abstract fitness provider differences:

```
┌──────────────────────────────────────────────────────────┐
│                 FitnessProvider Trait                     │
│  (Unified interface for all fitness data providers)      │
└──────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Strava     │    │   Fitbit     │    │   Garmin     │
│  Provider    │    │  Provider    │    │  Provider    │
└──────────────┘    └──────────────┘    └──────────────┘
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ Strava API   │    │ Fitbit API   │    │ Garmin API   │
│ (REST/JSON)  │    │ (REST/JSON)  │    │ (REST/JSON)  │
└──────────────┘    └──────────────┘    └──────────────┘
```

**Key benefit**: Pierre tools call `FitnessProvider` methods without knowing which provider implementation they're using.

## Fitnessprovider Trait

The trait defines a uniform interface for all fitness providers:

**Source**: src/providers/core.rs:52-171
```rust
/// Core fitness data provider trait - single interface for all providers
#[async_trait]
pub trait FitnessProvider: Send + Sync {
    /// Get provider name (e.g., "strava", "fitbit")
    fn name(&self) -> &'static str;

    /// Get provider configuration
    fn config(&self) -> &ProviderConfig;

    /// Set `OAuth2` credentials for this provider
    async fn set_credentials(&self, credentials: OAuth2Credentials) -> Result<()>;

    /// Check if provider has valid authentication
    async fn is_authenticated(&self) -> bool;

    /// Refresh access token if needed
    async fn refresh_token_if_needed(&self) -> Result<()>;

    /// Get user's athlete profile
    async fn get_athlete(&self) -> Result<Athlete>;

    /// Get user's activities with offset-based pagination (legacy)
    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>>;

    /// Get user's activities with cursor-based pagination (recommended)
    ///
    /// This method provides efficient, consistent pagination using opaque cursors.
    /// Cursors prevent duplicates and missing items when data changes during pagination.
    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> Result<CursorPage<Activity>>;

    /// Get specific activity by ID
    async fn get_activity(&self, id: &str) -> Result<Activity>;

    /// Get user's aggregate statistics
    async fn get_stats(&self) -> Result<Stats>;

    /// Get user's personal records
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>>;

    /// Get sleep sessions for a date range
    ///
    /// Returns sleep data from providers that support sleep tracking (Fitbit, Garmin).
    /// Providers without sleep data support return `UnsupportedFeature` error.
    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        let date_range = format!(
            "{} to {}",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );
        Err(ProviderError::UnsupportedFeature {
            provider: self.name().to_owned(),
            feature: format!("sleep_sessions (requested: {date_range})"),
        })
    }

    /// Revoke access tokens (disconnect)
    async fn disconnect(&self) -> Result<()>;
}
```

**Trait design**:
- **#[async_trait]**: Required for async methods in traits (trait desugaring for async)
- **Send + Sync**: Required for sharing across threads in async Rust
- **Default implementations**: Optional methods like `get_sleep_sessions` have defaults that return `UnsupportedFeature`

## Rust Idioms: Async Trait

**Source**: src/providers/core.rs:53
```rust
#[async_trait]
pub trait FitnessProvider: Send + Sync {
    async fn get_athlete(&self) -> Result<Athlete>;
    // ... other async methods
}
```

**Why async_trait**:
- **Trait async limitation**: Rust doesn't natively support `async fn` in traits (as of Rust 1.75)
- **Macro expansion**: `#[async_trait]` macro transforms async methods into `Pin<Box<dyn Future>>`
- **Send + Sync**: Required for async traits to ensure thread safety across await points

**Expanded version** (conceptual):
```rust
trait FitnessProvider: Send + Sync {
    fn get_athlete(&self) -> Pin<Box<dyn Future<Output = Result<Athlete>> + Send + '_>>;
}
```

## Provider-Agnostic Data Models

Pierre defines unified data models that work across all providers:

**Activity model** (src/models.rs:246-350 - conceptual):
```rust
/// Represents a single fitness activity from any provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Unique identifier (provider-specific)
    pub id: String,
    /// Activity name/title
    pub name: String,
    /// Sport type (run, ride, swim, etc.)
    pub sport_type: SportType,
    /// Activity distance in meters
    pub distance: Option<f64>,
    /// Total duration in seconds
    pub duration: Option<u64>,
    /// Moving time in seconds (excludes rest/stops)
    pub moving_time: Option<u64>,
    /// Total elevation gain in meters
    pub total_elevation_gain: Option<f64>,
    /// Activity start time (UTC)
    pub start_date: DateTime<Utc>,
    /// Average speed in m/s
    pub average_speed: Option<f32>,
    /// Average heart rate in BPM
    pub average_heartrate: Option<u32>,
    /// Maximum heart rate in BPM
    pub max_heartrate: Option<u32>,
    /// Average power in watts (cycling)
    pub average_watts: Option<u32>,
    /// Total energy in kilojoules
    pub kilojoules: Option<f32>,
    /// Calories burned
    pub calories: Option<u32>,
    /// Whether activity used a trainer/treadmill
    pub trainer: Option<bool>,
    /// GPS route polyline (encoded)
    pub map: Option<ActivityMap>,
    // ... 30+ more optional fields
}
```

**Design principles**:
- **Provider-agnostic**: Fields common across all providers (id, name, distance, etc.)
- **Optional fields**: Use `Option<T>` for provider-specific or missing data
- **Normalized units**: Standardize on meters, seconds, BPM (not provider-specific units)
- **Extensible**: New providers can omit fields they don't support

**Athlete model** (src/models.rs:400-450 - conceptual):
```rust
/// Athlete profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Athlete {
    pub id: String,
    pub username: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub sex: Option<String>,
    pub weight: Option<f32>,
    pub profile_medium: Option<String>,
    pub profile: Option<String>,
    pub ftp: Option<u32>, // Functional Threshold Power (cycling)
    // ... provider-specific fields
}
```

## Provider Error Types

Pierre defines structured errors with retry information:

**Source**: src/providers/errors.rs:10-101
```rust
/// Provider operation errors with structured context
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Provider API is unavailable or returning errors
    #[error("Provider {provider} API error: {status_code} - {message}")]
    ApiError {
        /// Name of the fitness provider (e.g., "strava", "garmin")
        provider: String,
        /// HTTP status code from the provider
        status_code: u16,
        /// Error message from the provider
        message: String,
        /// Whether this error can be retried
        retryable: bool,
    },

    /// Rate limit exceeded with retry information
    #[error("Rate limit exceeded for {provider}: retry after {retry_after_secs} seconds")]
    RateLimitExceeded {
        /// Name of the fitness provider
        provider: String,
        /// Seconds to wait before retrying
        retry_after_secs: u64,
        /// Type of rate limit hit (e.g., "15-minute", "daily")
        limit_type: String,
    },

    /// Authentication failed or token expired
    #[error("Authentication failed for {provider}: {reason}")]
    AuthenticationFailed {
        /// Name of the fitness provider
        provider: String,
        /// Reason for authentication failure
        reason: String,
    },

    /// Resource not found
    #[error("{resource_type} '{resource_id}' not found in {provider}")]
    NotFound {
        provider: String,
        resource_type: String,
        resource_id: String,
    },

    /// Feature not supported by provider
    #[error("Provider {provider} does not support {feature}")]
    UnsupportedFeature {
        provider: String,
        feature: String,
    },

    // ... more error variants
}
```

**Structured errors**:
- **thiserror**: Generates `Error` trait implementation with `#[error]` messages
- **Named fields**: Structured data (provider, status_code, retry_after_secs)
- **Display message**: `#[error(...)]` macro generates user-friendly error messages

**Retry logic**:

**Source**: src/providers/errors.rs:104-130
```rust
impl ProviderError {
    /// Check if error is retryable
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::ApiError { retryable, .. } => *retryable,
            Self::RateLimitExceeded { .. } | Self::NetworkError(_) => true,
            Self::AuthenticationFailed { .. }
            | Self::TokenRefreshFailed { .. }
            | Self::NotFound { .. }
            | Self::InvalidData { .. }
            | Self::ConfigurationError { .. }
            | Self::UnsupportedFeature { .. }
            | Self::Other(_) => false,
        }
    }

    /// Get retry delay in seconds if applicable
    #[must_use]
    pub const fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::RateLimitExceeded {
                retry_after_secs, ..
            } => Some(*retry_after_secs),
            _ => None,
        }
    }
}
```

**Retryable errors**: Rate limits and network errors can be retried; authentication failures and not-found errors cannot.

## Retry Logic with Exponential Backoff

Pierre implements automatic retry with exponential backoff for rate limits:

**Source**: src/providers/utils.rs:17-39
```rust
/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// HTTP status codes that should trigger retries
    pub retryable_status_codes: Vec<StatusCode>,
    /// Estimated block duration for user-facing error messages (seconds)
    pub estimated_block_duration_secs: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            retryable_status_codes: vec![StatusCode::TOO_MANY_REQUESTS],
            estimated_block_duration_secs: 3600, // 1 hour
        }
    }
}
```

**Retry implementation**:

**Source**: src/providers/utils.rs:97-175
```rust
/// Make an authenticated HTTP GET request with retry logic
pub async fn api_request_with_retry<T>(
    client: &Client,
    url: &str,
    access_token: &str,
    provider_name: &str,
    retry_config: &RetryConfig,
) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    tracing::info!("Starting {provider_name} API request to: {url}");

    let mut attempt = 0;
    loop {
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .with_context(|| format!("Failed to send request to {provider_name} API"))?;

        let status = response.status();
        tracing::info!("Received HTTP response with status: {status}");

        if retry_config.retryable_status_codes.contains(&status) {
            attempt += 1;
            if attempt >= retry_config.max_retries {
                let max_retries = retry_config.max_retries;
                warn!(
                    "{provider_name} API rate limit exceeded - max retries ({max_retries}) reached"
                );
                let minutes = retry_config.estimated_block_duration_secs / 60;
                let status_code = status.as_u16();
                return Err(ProviderError::RateLimitExceeded {
                    provider: provider_name.to_owned(),
                    retry_after_secs: retry_config.estimated_block_duration_secs,
                    limit_type: format!(
                        "API rate limit ({status_code}) - max retries reached - wait ~{minutes} minutes"
                    ),
                }.into());
            }

            let backoff_ms = retry_config.initial_backoff_ms * 2_u64.pow(attempt - 1);
            let max_retries = retry_config.max_retries;
            let status_code = status.as_u16();
            warn!(
                "{provider_name} API rate limit hit ({status_code}) - retry {attempt}/{max_retries} after {backoff_ms}ms backoff"
            );

            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            continue;
        }

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                provider: provider_name.to_owned(),
                status_code: status.as_u16(),
                message: format!("{provider_name} API request failed with status {status}: {text}"),
                retryable: false,
            }
            .into());
        }

        return response
            .json()
            .await
            .with_context(|| format!("Failed to parse {provider_name} API response"));
    }
}
```

**Exponential backoff**:
```
Attempt 1: initial_backoff_ms * 2^0 = 1000ms  (1 second)
Attempt 2: initial_backoff_ms * 2^1 = 2000ms  (2 seconds)
Attempt 3: initial_backoff_ms * 2^2 = 4000ms  (4 seconds)
```

**Why exponential backoff**: Prevents thundering herd problem where all clients retry simultaneously.

## Rust Idioms: Hrtb for Generic Deserialize

**Source**: src/providers/utils.rs:104-105
```rust
where
    T: for<'de> Deserialize<'de>,
```

**HRTB (Higher-Ranked Trait Bound)**:
- **`for<'de>`**: Type `T` must implement `Deserialize` for any lifetime `'de`
- **Needed for serde**: `Deserialize` has a lifetime parameter for borrowed data
- **Generic deserialization**: Allows function to return any deserializable type

**Without HRTB** (doesn't compile):
```rust
where
    T: Deserialize<'static>, // Too restrictive - only works for 'static lifetime
```

## Type Conversion Utilities

Providers return float values that need safe conversion to integers:

**Source**: src/providers/utils.rs:42-86
```rust
/// Type conversion utilities for safe float-to-integer conversions
pub mod conversions {
    use num_traits::ToPrimitive;

    /// Safely convert f64 to u64, clamping to valid range
    /// Used for duration values from APIs that return floats
    #[must_use]
    pub fn f64_to_u64(value: f64) -> u64 {
        if !value.is_finite() {
            return 0;
        }
        let t = value.trunc();
        if t.is_sign_negative() {
            return 0;
        }
        t.to_u64().map_or(u64::MAX, |v| v)
    }

    /// Safely convert f32 to u32, clamping to valid range
    /// Used for metrics like heart rate, power, cadence
    #[must_use]
    pub fn f32_to_u32(value: f32) -> u32 {
        if !value.is_finite() {
            return 0;
        }
        let t = value.trunc();
        if t.is_sign_negative() {
            return 0;
        }
        t.to_u32().map_or(u32::MAX, |v| v)
    }

    /// Safely convert f64 to u32, clamping to valid range
    /// Used for calorie values and other metrics
    #[must_use]
    pub fn f64_to_u32(value: f64) -> u32 {
        if !value.is_finite() {
            return 0;
        }
        let t = value.trunc();
        if t.is_sign_negative() {
            return 0;
        }
        t.to_u32().map_or(u32::MAX, |v| v)
    }
}
```

**Safety checks**:
1. **is_finite()**: Reject NaN and infinity
2. **is_sign_negative()**: Reject negative values (durations/HR/power can't be negative)
3. **trunc()**: Remove fractional part before conversion
4. **map_or()**: Clamp to max value if conversion overflows

**Usage example**:
```rust
let duration_secs: f64 = activity_json["duration"].as_f64().unwrap_or(0.0);
let duration: u64 = conversions::f64_to_u64(duration_secs);
```

## Tenant-Aware Provider Wrapper

Pierre wraps providers with tenant context for isolation:

**Source**: src/providers/core.rs:182-211
```rust
/// Tenant-aware provider wrapper that handles multi-tenancy
pub struct TenantProvider {
    inner: Box<dyn FitnessProvider>,
    tenant_id: Uuid,
    user_id: Uuid,
}

impl TenantProvider {
    /// Create a new tenant-aware provider
    #[must_use]
    pub fn new(inner: Box<dyn FitnessProvider>, tenant_id: Uuid, user_id: Uuid) -> Self {
        Self {
            inner,
            tenant_id,
            user_id,
        }
    }

    /// Get tenant ID
    #[must_use]
    pub const fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Get user ID
    #[must_use]
    pub const fn user_id(&self) -> Uuid {
        self.user_id
    }
}
```

**Delegation pattern**:

**Source**: src/providers/core.rs:213-276
```rust
#[async_trait]
impl FitnessProvider for TenantProvider {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> Result<()> {
        // Add tenant-specific logging/metrics here
        tracing::info!(
            "Setting credentials for provider {} in tenant {} for user {}",
            self.name(),
            self.tenant_id,
            self.user_id
        );
        self.inner.set_credentials(credentials).await
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        self.inner.get_athlete().await
    }

    // ... delegate all other methods to inner
}
```

**Wrapper benefits**:
- **Logging**: Tenant/user context in all log messages
- **Metrics**: Track usage per tenant/user
- **Isolation**: Prevent cross-tenant data leaks
- **Transparent**: Tools don't know they're using wrapped provider

## Cursor-Based Pagination

Pierre supports cursor-based pagination for efficient data access:

**Conceptual implementation**:
```rust
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}
```

**Cursor vs offset pagination**:

| Offset-based | Cursor-based |
|--------------|--------------|
| `?limit=10&offset=20` | `?limit=10&cursor=abc123` |
| Can miss items if data changes | Consistent even if data changes |
| Simple to implement | Requires opaque cursor generation |
| Slow for large offsets | Fast for any cursor position |

**Why cursors**:
- **Consistency**: Prevent duplicate/missing items when data inserted during pagination
- **Performance**: Database can seek to cursor position efficiently
- **Provider support**: Strava, Fitbit, Garmin all support cursor pagination

## Key Takeaways

1. **Trait-based abstraction**: `FitnessProvider` trait unifies all provider implementations.

2. **async_trait**: Required for async methods in traits (Rust limitation workaround).

3. **Send + Sync**: Required for sharing trait objects across async tasks/threads.

4. **Provider-agnostic models**: Unified `Activity`, `Athlete`, `Stats` types work across all providers.

5. **Structured errors**: `ProviderError` with named fields and retry information.

6. **Exponential backoff**: `2^attempt * initial_backoff_ms` prevents thundering herd.

7. **Type conversion**: Safe float-to-integer conversion handles NaN, infinity, negative values.

8. **HRTB**: `for<'de> Deserialize<'de>` allows generic deserialization with any lifetime.

9. **Tenant wrapper**: `TenantProvider` adds tenant/user context without changing trait interface.

10. **Cursor pagination**: More reliable than offset pagination for dynamic data.

11. **Default trait methods**: Optional provider features (sleep, recovery) have default "unsupported" implementations.

12. **Retry config**: Configurable retry attempts, backoff, and status codes per provider.

---

**Next Chapter**: [Chapter 18: A2A Protocol - Agent-to-Agent Communication](./chapter-18-a2a-protocol.md) - Learn how Pierre implements the Agent-to-Agent (A2A) protocol for secure inter-agent communication with Ed25519 signatures.
