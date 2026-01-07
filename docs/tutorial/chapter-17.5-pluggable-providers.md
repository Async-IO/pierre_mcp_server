<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 17.5: Pluggable Provider Architecture

This chapter explores pierre's pluggable provider architecture that enables runtime registration of 1 to x fitness providers simultaneously. You'll learn about provider factories, dynamic discovery, environment-based configuration, and how to add new providers without modifying existing code.

## What You'll Learn

- Provider registry and factory pattern
- Runtime provider discovery (1 to x providers)
- Environment-based provider configuration
- Shared request/response trait contracts
- **Service Provider Interface (SPI)** for external providers
- **Feature flags** for compile-time provider selection
- **Bitflags-based capabilities** detection
- Adding custom providers without code changes
- Synthetic provider for development/testing
- Multi-provider connection management

## Pluggable Architecture Overview

Pierre implements a **fully pluggable provider system** where fitness providers are registered at runtime through a factory pattern. The system supports **1 to x providers simultaneously**, meaning you can use just Strava, or Strava + Garmin + Fitbit + custom providers all at once.

```
┌───────────────────────────────────────────────────────────────────────┐
│                      ProviderRegistry (runtime)                        │
│           Manages 1 to x providers with dynamic discovery              │
└────────────┬──────────────────────────────────────────────────────────┘
             │
    ┌────────┴────────┬───────────┬────────────┬──────────┬─────────┬─────────────┐
    │                 │           │            │          │         │             │
    ▼                 ▼           ▼            ▼          ▼         ▼             ▼
┌─────────┐    ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────┐  ┌─────────┐  ┌─────────┐
│ Strava  │    │ Garmin  │  │  Terra  │  │ Fitbit  │  │ WHOOP │  │Synthetic│  │ Custom  │
│ Factory │    │ Factory │  │ Factory │  │ Factory │  │Factory│  │ Factory │  │ Factory │
└────┬────┘    └────┬────┘  └────┬────┘  └────┬────┘  └───┬───┘  └────┬────┘  └────┬────┘
     │              │           │            │           │            │            │
     ▼              ▼           ▼            ▼           ▼            ▼            ▼
┌─────────┐    ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────┐  ┌─────────┐  ┌─────────┐
│ Strava  │    │ Garmin  │  │  Terra  │  │ Fitbit  │  │ WHOOP │  │Synthetic│  │ Custom  │
│Provider │    │Provider │  │Provider │  │Provider │  │Provdr │  │Provider │  │Provider │
└─────────┘    └─────────┘  └─────────┘  └─────────┘  └───────┘  └─────────┘  └─────────┘
     │              │           │            │           │            │            │
     └──────────────┴───────────┴────────────┴───────────┴────────────┴────────────┘
                                    │
                                    ▼
                     ┌──────────────────────────┐
                     │   FitnessProvider Trait  │
                     │   (shared interface)     │
                     └──────────────────────────┘
```

**Key benefit**: Add, remove, or swap providers without modifying tool code, connection handlers, or application logic.

## Feature Flags (compile-Time Selection)

Pierre uses Cargo feature flags for compile-time provider selection. This allows minimal binaries with only the providers you need:

**Source**: Cargo.toml
```toml
# Provider feature flags - enable/disable individual fitness data providers
provider-strava = []
provider-garmin = []
provider-terra = []
provider-fitbit = []
provider-whoop = []
provider-synthetic = []
all-providers = ["provider-strava", "provider-garmin", "provider-terra", "provider-fitbit", "provider-whoop", "provider-synthetic"]
```

**Build with specific providers**:
```bash
# All providers (default)
cargo build --release

# Only Strava
cargo build --release --no-default-features --features "sqlite,provider-strava"

# Strava + Garmin (no synthetic)
cargo build --release --no-default-features --features "sqlite,provider-strava,provider-garmin"
```

**Conditional compilation in code**:
```rust
// Provider modules conditionally compiled
#[cfg(feature = "provider-strava")]
pub mod strava_provider;

#[cfg(feature = "provider-garmin")]
pub mod garmin_provider;

#[cfg(feature = "provider-whoop")]
pub mod whoop_provider;

#[cfg(feature = "provider-synthetic")]
pub mod synthetic_provider;
```

## Service Provider Interface (SPI)

The SPI defines the contract for pluggable providers, enabling external crates to register providers without modifying core code.

### Providerdescriptor Trait

**Source**: src/providers/spi.rs:129-177
```rust
/// Service Provider Interface (SPI) for pluggable fitness providers
///
/// External provider crates implement this trait to describe their capabilities.
pub trait ProviderDescriptor: Send + Sync {
    /// Unique provider identifier (e.g., "strava", "garmin", "whoop")
    fn name(&self) -> &'static str;

    /// Human-readable display name (e.g., "Strava", "Garmin Connect")
    fn display_name(&self) -> &'static str;

    /// Provider capabilities using bitflags
    fn capabilities(&self) -> ProviderCapabilities;

    /// OAuth endpoints (None for non-OAuth providers like synthetic)
    fn oauth_endpoints(&self) -> Option<OAuthEndpoints>;

    /// OAuth parameters (scope separator, PKCE, etc.)
    fn oauth_params(&self) -> Option<OAuthParams>;

    /// Base URL for API requests
    fn api_base_url(&self) -> &'static str;

    /// Default OAuth scopes for this provider
    fn default_scopes(&self) -> &'static [&'static str];
}
```

### Providercapabilities (Bitflags)

Provider capabilities use bitflags for efficient storage and combinators:

**Source**: src/providers/spi.rs:95-126
```rust
bitflags::bitflags! {
    /// Provider capability flags using bitflags for efficient storage
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ProviderCapabilities: u8 {
        /// Provider supports OAuth 2.0 authentication
        const OAUTH = 0b0000_0001;
        /// Provider supports activity data (workouts, runs, rides)
        const ACTIVITIES = 0b0000_0010;
        /// Provider supports sleep tracking data
        const SLEEP_TRACKING = 0b0000_0100;
        /// Provider supports recovery metrics (HRV, strain)
        const RECOVERY_METRICS = 0b0000_1000;
        /// Provider supports health metrics (weight, body composition)
        const HEALTH_METRICS = 0b0001_0000;
    }
}

impl ProviderCapabilities {
    /// Full fitness provider (OAuth + activities)
    pub const fn full_fitness() -> Self {
        Self::OAUTH.union(Self::ACTIVITIES)
    }

    /// Full health provider (all capabilities)
    pub const fn full_health() -> Self {
        Self::OAUTH
            .union(Self::ACTIVITIES)
            .union(Self::SLEEP_TRACKING)
            .union(Self::RECOVERY_METRICS)
            .union(Self::HEALTH_METRICS)
    }
}
```

**Using capabilities**:
```rust
// Check specific capability
if provider.capabilities().contains(ProviderCapabilities::SLEEP_TRACKING) {
    // Provider supports sleep data
}

// Combine capabilities
let caps = ProviderCapabilities::OAUTH | ProviderCapabilities::ACTIVITIES;

// Use convenience constructors
let full_health = ProviderCapabilities::full_health();
```

### Oauthparams

OAuth configuration varies by provider (scope separators, PKCE support):

**Source**: src/providers/spi.rs:85-93
```rust
/// OAuth parameters for provider-specific configuration
#[derive(Debug, Clone)]
pub struct OAuthParams {
    /// Scope separator character (space for Fitbit, comma for Strava)
    pub scope_separator: &'static str,
    /// Whether to use PKCE (recommended for public clients)
    pub use_pkce: bool,
    /// Additional query parameters for authorization URL
    pub additional_auth_params: &'static [(&'static str, &'static str)],
}
```

## Provider Registry

The `ProviderRegistry` is the central hub for managing all fitness providers:

**Source**: src/providers/registry.rs:13-60
```rust
/// Central registry for all fitness providers with factory pattern
pub struct ProviderRegistry {
    /// Map of provider names to their factories
    factories: HashMap<String, Box<dyn ProviderFactory>>,
    /// Default configurations for each provider (loaded from environment)
    default_configs: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    /// Create registry and auto-register all known providers
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
        };

        // Register Strava provider with environment-based config
        registry.register_factory(
            oauth_providers::STRAVA,
            Box::new(StravaProviderFactory),
        );
        let config = load_provider_env_config(
            oauth_providers::STRAVA,
            "https://www.strava.com/oauth/authorize",
            "https://www.strava.com/oauth/token",
            "https://www.strava.com/api/v3",
            Some("https://www.strava.com/oauth/deauthorize"),
            &[oauth_providers::STRAVA_DEFAULT_SCOPES.to_owned()],
        );
        registry.set_default_config(oauth_providers::STRAVA, /* config */);

        // Register Garmin provider
        registry.register_factory(
            oauth_providers::GARMIN,
            Box::new(GarminProviderFactory),
        );
        // ... Garmin config

        // Register Synthetic provider (no OAuth needed!)
        registry.register_factory(
            oauth_providers::SYNTHETIC,
            Box::new(SyntheticProviderFactory),
        );
        // ... Synthetic config

        registry
    }

    /// Register a provider factory for runtime creation
    pub fn register_factory(&mut self, name: &str, factory: Box<dyn ProviderFactory>) {
        self.factories.insert(name.to_owned(), factory);
    }

    /// Check if provider is supported (dynamic discovery)
    #[must_use]
    pub fn is_supported(&self, provider: &str) -> bool {
        self.factories.contains_key(provider)
    }

    /// Get all supported provider names (1 to x providers)
    #[must_use]
    pub fn supported_providers(&self) -> Vec<String> {
        self.factories.keys().map(ToString::to_string).collect()
    }

    /// Create provider instance from factory
    pub fn create_provider(&self, name: &str) -> Option<Box<dyn FitnessProvider>> {
        let factory = self.factories.get(name)?;
        let config = self.default_configs.get(name)?.clone();
        Some(factory.create(config))
    }
}
```

**Registry responsibilities**:
- **Factory storage**: Maps provider names to factory implementations
- **Dynamic discovery**: `is_supported()` and `supported_providers()` enable runtime introspection
- **Configuration management**: Stores default configs loaded from environment
- **Provider creation**: `create_provider()` instantiates providers on-demand

## Provider Factory Pattern

Each provider implements a `ProviderFactory` trait for creation:

**Source**: src/providers/core.rs:173-180
```rust
/// Provider factory for creating instances
pub trait ProviderFactory: Send + Sync {
    /// Create a new provider instance with the given configuration
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider>;

    /// Get supported provider names (for multi-provider factories)
    fn supported_providers(&self) -> &'static [&'static str];
}
```

**Example: Strava factory**:

**Source**: src/providers/registry.rs:20-28
```rust
/// Factory for creating Strava provider instances
struct StravaProviderFactory;

impl ProviderFactory for StravaProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(StravaProvider::new(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["strava"]
    }
}
```

**Example: Synthetic factory** (Phase 1):

**Source**: src/providers/registry.rs:30-38
```rust
/// Factory for creating Synthetic provider instances
struct SyntheticProviderFactory;

impl ProviderFactory for SyntheticProviderFactory {
    fn create(&self, _config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(SyntheticProvider::default())
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["synthetic"]
    }
}
```

**Factory pattern benefits**:
- **Lazy instantiation**: Providers created only when needed
- **Configuration injection**: Factory receives config at creation time
- **Type erasure**: Returns `Box<dyn FitnessProvider>` for uniform handling

## Environment-Based Configuration

Pierre loads provider configuration from environment variables for **cloud-native deployment** (GCP, AWS, etc.):

**Configuration schema**:
```bash
# Default provider (1 required, used when no provider specified)
export PIERRE_DEFAULT_PROVIDER=strava  # or garmin, synthetic, custom

# Per-provider configuration (repeat for each provider 1 to x)
export PIERRE_STRAVA_CLIENT_ID=your-client-id
export PIERRE_STRAVA_CLIENT_SECRET=your-secret
export PIERRE_STRAVA_AUTH_URL=https://www.strava.com/oauth/authorize
export PIERRE_STRAVA_TOKEN_URL=https://www.strava.com/oauth/token
export PIERRE_STRAVA_API_BASE_URL=https://www.strava.com/api/v3
export PIERRE_STRAVA_REVOKE_URL=https://www.strava.com/oauth/deauthorize
export PIERRE_STRAVA_SCOPES="activity:read_all,profile:read_all"

# Garmin provider
export PIERRE_GARMIN_CLIENT_ID=your-consumer-key
export PIERRE_GARMIN_CLIENT_SECRET=your-consumer-secret
# ... Garmin URLs and scopes

# Synthetic provider (no OAuth needed - perfect for dev/testing!)
# No env vars required - automatically available
```

**Loading configuration**:

**Source**: src/config/environment.rs:2093-2174
```rust
/// Load provider-specific configuration from environment variables
///
/// Falls back to provided defaults if environment variables are not set.
/// Supports legacy env vars (STRAVA_CLIENT_ID) for backward compatibility.
#[must_use]
pub fn load_provider_env_config(
    provider: &str,
    default_auth_url: &str,
    default_token_url: &str,
    default_api_base_url: &str,
    default_revoke_url: Option<&str>,
    default_scopes: &[String],
) -> ProviderEnvConfig {
    let provider_upper = provider.to_uppercase();

    // Load client credentials with fallback to legacy env vars
    let client_id = env::var(format!("PIERRE_{provider_upper}_CLIENT_ID"))
        .or_else(|_| env::var(format!("{provider_upper}_CLIENT_ID")))
        .ok();

    let client_secret = env::var(format!("PIERRE_{provider_upper}_CLIENT_SECRET"))
        .or_else(|_| env::var(format!("{provider_upper}_CLIENT_SECRET")))
        .ok();

    // Load URLs with defaults
    let auth_url = env::var(format!("PIERRE_{provider_upper}_AUTH_URL"))
        .unwrap_or_else(|_| default_auth_url.to_owned());

    // ... load other fields

    (client_id, client_secret, auth_url, token_url, api_base_url, revoke_url, scopes)
}
```

**Backward compatibility**:
- **New format**: `PIERRE_STRAVA_CLIENT_ID` (preferred)
- **Legacy format**: `STRAVA_CLIENT_ID` (still supported)
- **Graceful fallback**: Tries new format first, then legacy

## Dynamic Provider Discovery

Connection tools automatically discover available providers at runtime:

**Source**: src/protocols/universal/handlers/connections.rs:84-88
```rust
// Multi-provider mode - check all supported providers from registry
let providers_to_check = executor.resources.provider_registry.supported_providers();
let mut providers_status = serde_json::Map::new();

for provider in providers_to_check {
    let is_connected = matches!(
        executor
            .auth_service
            .get_valid_token(user_uuid, provider, request.tenant_id.as_deref())
            .await,
        Ok(Some(_))
    );

    providers_status.insert(
        provider.to_owned(),
        serde_json::json!({
            "connected": is_connected,
            "status": if is_connected { "connected" } else { "disconnected" }
        }),
    );
}
```

**Dynamic provider validation**:

**Source**: src/protocols/universal/handlers/connections.rs:224-228
```rust
/// Validate that provider is supported using provider registry
fn is_provider_supported(
    provider: &str,
    provider_registry: &crate::providers::ProviderRegistry,
) -> bool {
    provider_registry.is_supported(provider)
}
```

**Dynamic error messages**:

**Source**: src/protocols/universal/handlers/connections.rs:333-340
```rust
if !is_provider_supported(provider, &executor.resources.provider_registry) {
    let supported_providers = executor
        .resources
        .provider_registry
        .supported_providers()
        .join(", ");
    return Ok(connection_error(format!(
        "Provider '{provider}' is not supported. Supported providers: {supported_providers}"
    )));
}
```

**Result**: Error messages automatically update when you add/remove providers. No hardcoded lists!

## Synthetic Provider (phase 1)

Pierre includes a **synthetic provider** for development and testing **without OAuth**:

**Source**: src/providers/synthetic_provider.rs:30-79
```rust
/// Synthetic fitness provider for development and testing (no OAuth required!)
///
/// This provider generates realistic fitness data without connecting to external APIs.
/// Perfect for:
/// - Development without OAuth credentials
/// - Integration tests
/// - Demo environments
/// - CI/CD pipelines
pub struct SyntheticProvider {
    activities: Arc<RwLock<Vec<Activity>>>,
    activity_index: Arc<RwLock<HashMap<String, Activity>>>,
    config: ProviderConfig,
}

impl SyntheticProvider {
    /// Create provider with pre-populated synthetic activities
    #[must_use]
    pub fn with_activities(activities: Vec<Activity>) -> Self {
        let mut index = HashMap::new();
        for activity in &activities {
            index.insert(activity.id.clone(), activity.clone());
        }

        Self {
            activities: Arc::new(RwLock::new(activities)),
            activity_index: Arc::new(RwLock::new(index)),
            config: ProviderConfig {
                name: oauth_providers::SYNTHETIC.to_owned(),
                auth_url: "http://localhost:8081/synthetic/auth".to_owned(),
                token_url: "http://localhost:8081/synthetic/token".to_owned(),
                api_base_url: "http://localhost:8081/synthetic/api".to_owned(),
                revoke_url: None,
                default_scopes: vec!["read:all".to_owned()],
            },
        }
    }
}
```

**Synthetic provider benefits**:
- **No OAuth dance**: Skip authorization flows during development
- **Deterministic data**: Same activities every time for testing
- **Fast iteration**: No network calls, instant responses
- **CI/CD friendly**: No API keys or secrets needed
- **Always available**: Listed in `supported_providers()`

**Default provider selection**:

**Source**: src/config/environment.rs:2060-2078
```rust
/// Get default provider from PIERRE_DEFAULT_PROVIDER or fallback to "synthetic"
#[must_use]
pub fn default_provider() -> String {
    use crate::constants::oauth_providers;
    env::var("PIERRE_DEFAULT_PROVIDER")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| oauth_providers::SYNTHETIC.to_owned())
}
```

**Fallback hierarchy**:
1. `PIERRE_DEFAULT_PROVIDER=strava` → use Strava
2. `PIERRE_DEFAULT_PROVIDER=garmin` → use Garmin
3. Not set or empty → use Synthetic (OAuth-free development)

## Adding a Custom Provider SPI Approach)

Here's how to add a new provider using the SPI architecture:

### Step 1: Add Feature Flag

**Source**: Cargo.toml
```toml
[features]
provider-whoop = []
all-providers = ["provider-strava", "provider-garmin", "provider-synthetic", "provider-whoop"]
```

### Step 2: Implement Providerdescriptor (SPI)

**Source**: src/providers/spi.rs
```rust
use pierre_mcp_server::providers::spi::{
    ProviderDescriptor, OAuthEndpoints, OAuthParams, ProviderCapabilities
};

/// WHOOP provider descriptor for SPI registration
#[cfg(feature = "provider-whoop")]
pub struct WhoopDescriptor;

#[cfg(feature = "provider-whoop")]
impl ProviderDescriptor for WhoopDescriptor {
    fn name(&self) -> &'static str {
        "whoop"
    }

    fn display_name(&self) -> &'static str {
        "WHOOP"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        // WHOOP supports all health features - use bitflags combinator
        ProviderCapabilities::full_health()
    }

    fn oauth_endpoints(&self) -> Option<OAuthEndpoints> {
        Some(OAuthEndpoints {
            auth_url: "https://api.prod.whoop.com/oauth/oauth2/auth",
            token_url: "https://api.prod.whoop.com/oauth/oauth2/token",
            revoke_url: Some("https://api.prod.whoop.com/oauth/oauth2/revoke"),
        })
    }

    fn oauth_params(&self) -> Option<OAuthParams> {
        Some(OAuthParams {
            scope_separator: " ",  // Space-separated scopes
            use_pkce: true,        // PKCE recommended
            additional_auth_params: &[],
        })
    }

    fn api_base_url(&self) -> &'static str {
        "https://api.prod.whoop.com/developer/v1"
    }

    fn default_scopes(&self) -> &'static [&'static str] {
        &["read:profile", "read:workout", "read:sleep", "read:recovery"]
    }
}
```

### Step 3: Implement Fitnessprovider Trait

**Source**: src/providers/whoop_provider.rs
```rust
use pierre_mcp_server::providers::core::{FitnessProvider, ProviderConfig, OAuth2Credentials};
use pierre_mcp_server::models::{Activity, Athlete, Stats};
use pierre_mcp_server::errors::AppResult;
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

#[cfg(feature = "provider-whoop")]
pub struct WhoopProvider {
    config: ProviderConfig,
    credentials: Arc<RwLock<Option<OAuth2Credentials>>>,
    http_client: reqwest::Client,
}

#[cfg(feature = "provider-whoop")]
#[async_trait]
impl FitnessProvider for WhoopProvider {
    fn name(&self) -> &'static str {
        "whoop"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        // Store credentials using RwLock for interior mutability
        let mut creds = self.credentials.write()
            .map_err(|_| pierre_mcp_server::providers::errors::ProviderError::ConfigurationError(
                "Failed to acquire credentials lock".to_owned()
            ))?;
        *creds = Some(credentials);
        Ok(())
    }

    async fn get_athlete(&self) -> AppResult<Athlete> {
        // Real implementation: fetch from WHOOP API and convert to unified model
        Ok(Athlete {
            id: "whoop-user-123".to_owned(),
            username: "athlete".to_owned(),
            firstname: Some("WHOOP".to_owned()),
            lastname: Some("User".to_owned()),
            profile_picture: None,
            provider: "whoop".to_owned(),
        })
    }

    async fn get_activities(
        &self,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
        // Real implementation: fetch workouts from WHOOP API
        Ok(vec![])
    }

    // ... implement remaining trait methods
}
```

### Step 4: Create Provider Factory and Register

**Source**: src/providers/registry.rs
```rust
#[cfg(feature = "provider-whoop")]
use super::whoop_provider::WhoopProvider;
#[cfg(feature = "provider-whoop")]
use super::spi::WhoopDescriptor;

/// Factory for creating WHOOP provider instances
#[cfg(feature = "provider-whoop")]
struct WhoopProviderFactory;

#[cfg(feature = "provider-whoop")]
impl ProviderFactory for WhoopProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(WhoopProvider::new(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["whoop"]
    }
}

// In ProviderRegistry::new():
#[cfg(feature = "provider-whoop")]
{
    let descriptor = WhoopDescriptor;
    registry.register_factory("whoop", Box::new(WhoopProviderFactory));
    // Config loaded from descriptor's oauth_endpoints() and default_scopes()
}
```

### Step 5: Add to Constants and Module Exports

**Source**: src/constants/oauth/providers.rs
```rust
#[cfg(feature = "provider-whoop")]
pub const WHOOP: &str = "whoop";

#[cfg(feature = "provider-whoop")]
pub const WHOOP_DEFAULT_SCOPES: &str = "read:profile read:workout read:sleep read:recovery";
```

**Source**: src/providers/mod.rs
```rust
#[cfg(feature = "provider-whoop")]
pub mod whoop_provider;

#[cfg(feature = "provider-whoop")]
pub use spi::WhoopDescriptor;
```

### Step 6: Configure Environment

**Source**: .envrc
```bash
# WHOOP provider configuration
export WHOOP_CLIENT_ID=your-whoop-client-id
export WHOOP_CLIENT_SECRET=your-whoop-secret
export WHOOP_REDIRECT_URI=http://localhost:8081/api/oauth/callback/whoop
```

**That's it!** WHOOP is now:
- ✅ Conditionally compiled with `--features provider-whoop`
- ✅ Available in `supported_providers()` when feature enabled
- ✅ Discoverable via `is_supported("whoop")`
- ✅ Creatable via `create_provider("whoop")`
- ✅ Listed in connection status responses
- ✅ Supported in `connect_provider` tool
- ✅ Capabilities queryable via bitflags

**No changes needed**:
- ❌ Connection handlers (dynamic discovery)
- ❌ Tool implementations (use FitnessProvider trait)
- ❌ MCP schema generation (automatic)
- ❌ Test fixtures (provider-agnostic)

## Managing 1 to X Providers Simultaneously

Pierre's architecture supports **multiple active providers per tenant/user**:

**Multi-provider connection status**:
```json
{
  "success": true,
  "result": {
    "providers": {
      "strava": {
        "connected": true,
        "status": "connected"
      },
      "garmin": {
        "connected": true,
        "status": "connected"
      },
      "fitbit": {
        "connected": false,
        "status": "disconnected"
      },
      "synthetic": {
        "connected": true,
        "status": "connected"
      },
      "whoop": {
        "connected": true,
        "status": "connected"
      }
    }
  }
}
```

**Data aggregation across providers**:
```rust
// Pseudo-code for fetching activities from all connected providers
async fn get_all_activities(user_id: Uuid, tenant_id: Uuid) -> Vec<Activity> {
    let mut all_activities = Vec::new();

    for provider_name in registry.supported_providers() {
        if let Ok(Some(provider)) = create_authenticated_provider(
            user_id,
            tenant_id,
            provider_name,
        ).await {
            if let Ok(activities) = provider.get_activities(Some(50), None).await {
                all_activities.extend(activities);
            }
        }
    }

    // Deduplicate and merge activities from multiple providers
    all_activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));
    all_activities
}
```

**Provider switching**:
```rust
// Tools accept optional provider parameter
let provider_name = request
    .parameters
    .get("provider")
    .and_then(|v| v.as_str())
    .unwrap_or(&default_provider());

let provider = registry.create_provider(provider_name)
    .ok_or_else(|| ProtocolError::ProviderNotFound)?;
```

## Shared request/response Traits

All providers implement the same `FitnessProvider` trait, ensuring **uniform request/response patterns**:

**Request side (method parameters)**:
- **IDs**: `&str` for activity/athlete IDs
- **Pagination**: `PaginationParams` struct
- **Date ranges**: `DateTime<Utc>` for time-based queries
- **Options**: `Option<T>` for optional filters

**Response side (domain models)**:
- **Activity**: Unified workout representation
- **Athlete**: User profile information
- **Stats**: Aggregate performance metrics
- **PersonalRecord**: Best achievements
- **SleepSession**, **RecoveryMetrics**, **HealthMetrics**: Health data

**Shared error handling**:
- **AppResult<T>**: All providers return the same result type
- **ProviderError**: Structured error enum with retry information
- **Consistent mapping**: Provider-specific errors → `ProviderError`

**Benefits**:
1. **Swappable**: Change from Strava to Garmin without modifying tool code
2. **Testable**: Mock any provider using `FitnessProvider` trait
3. **Type-safe**: Compiler enforces contract across all providers
4. **Extensible**: New providers must implement complete interface

## Rust Idioms: Trait Object Factory

**Source**: src/providers/registry.rs:43-46
```rust
pub fn register_factory(&mut self, name: &str, factory: Box<dyn ProviderFactory>) {
    self.factories.insert(name.to_owned(), factory);
}
```

**Trait objects**:
- **`Box<dyn ProviderFactory>`**: Heap-allocated trait object with dynamic dispatch
- **Dynamic dispatch**: Method calls resolved at runtime (vtable lookup)
- **Polymorphism**: Registry stores different factory types (Strava, Garmin, etc.)
- **Type erasure**: Concrete factory type erased, only trait methods accessible

**Alternative (static dispatch)**:
```rust
// Generic approach (static dispatch)
pub fn register_factory<F: ProviderFactory + 'static>(&mut self, name: &str, factory: F) {
    // Can't store different F types in same HashMap!
}
```

**Why trait objects**: Registry needs to store heterogeneous factory types in single collection.

## Rust Idioms: arc<rwlock<t>> for Interior Mutability

**Source**: src/providers/synthetic_provider.rs:34-36
```rust
pub struct SyntheticProvider {
    activities: Arc<RwLock<Vec<Activity>>>,
    activity_index: Arc<RwLock<HashMap<String, Activity>>>,
    config: ProviderConfig,
}
```

**Pattern explanation**:
- **Arc**: Atomic reference counting for shared ownership across threads
- **RwLock**: Reader-writer lock allowing multiple readers OR single writer
- **Interior mutability**: Mutate data inside `&self` (FitnessProvider trait uses `&self`)

**Why needed**:
```rust
#[async_trait]
pub trait FitnessProvider: Send + Sync {
    async fn get_activities(&self, ...) -> Result<Vec<Activity>>;
    //                      ^^^^^ immutable reference
}
```

**Without RwLock** (doesn't compile):
```rust
impl FitnessProvider for SyntheticProvider {
    async fn get_activities(&self, ...) -> Result<Vec<Activity>> {
        self.activities.push(...); // ❌ Can't mutate through &self
    }
}
```

**With RwLock** (compiles):
```rust
impl FitnessProvider for SyntheticProvider {
    async fn get_activities(&self, ...) -> Result<Vec<Activity>> {
        let activities = self.activities.read().await; // ✅ Interior mutability
        Ok(activities.clone())
    }
}
```

## Provider Resilience Patterns

Pierre implements multiple resilience patterns to handle provider failures gracefully.

### Retry with Exponential Backoff

**Source**: `src/providers/core.rs` (conceptual)
```rust
/// Retry configuration for provider requests
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay between retries (doubles each attempt)
    pub base_delay_ms: u64,
    /// Maximum delay cap
    pub max_delay_ms: u64,
    /// Jitter factor (0.0 to 1.0) to prevent thundering herd
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            jitter_factor: 0.1,
        }
    }
}
```

**Retry logic**:
```rust
async fn fetch_with_retry<T, F, Fut>(
    operation: F,
    config: &RetryConfig,
) -> Result<T, ProviderError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ProviderError>>,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < config.max_retries => {
                attempt += 1;
                let delay = calculate_backoff(attempt, config);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

fn calculate_backoff(attempt: u32, config: &RetryConfig) -> u64 {
    let base = config.base_delay_ms * 2u64.pow(attempt - 1);
    let jitter = (base as f64 * config.jitter_factor * rand::random::<f64>()) as u64;
    (base + jitter).min(config.max_delay_ms)
}
```

### Rate Limit Respect

Providers return `Retry-After` headers when rate limited:

```rust
match provider.get_activities().await {
    Err(ProviderError::RateLimitExceeded { retry_after_secs, .. }) => {
        tracing::warn!(
            provider = %provider.name(),
            retry_after = retry_after_secs,
            "Provider rate limited, scheduling retry"
        );
        // Queue for later execution
        scheduler.schedule_retry(request, retry_after_secs).await;
        Ok(PendingResult::Scheduled)
    }
    result => result,
}
```

### Token Auto-Refresh

OAuth tokens are automatically refreshed before expiration:

**Source**: `src/oauth2_client/flow_manager.rs` (conceptual)
```rust
/// Check if token needs refresh (5 minute buffer)
fn needs_refresh(token: &UserOAuthToken) -> bool {
    if let Some(expires_at) = token.expires_at {
        let refresh_buffer = Duration::from_secs(300); // 5 minutes
        expires_at - refresh_buffer < Utc::now()
    } else {
        false
    }
}

/// Transparently refresh token before provider call
async fn ensure_valid_token(
    db: &Database,
    user_id: Uuid,
    tenant_id: &str,
    provider: &str,
) -> Result<String, ProviderError> {
    let token = db.oauth_tokens().get(user_id, tenant_id, provider).await?;

    if needs_refresh(&token) {
        let refreshed = refresh_token(&token).await?;
        db.oauth_tokens().upsert(&refreshed).await?;
        Ok(refreshed.access_token)
    } else {
        Ok(token.access_token)
    }
}
```

### Graceful Degradation

When a provider is unavailable, Pierre continues serving from cache:

```rust
/// Fetch activities with cache fallback
async fn get_activities_resilient(
    provider: &dyn FitnessProvider,
    cache: &Cache,
    user_id: Uuid,
) -> Result<Vec<Activity>, ProviderError> {
    let cache_key = format!("activities:{}:{}", provider.name(), user_id);

    match provider.get_activities(user_id).await {
        Ok(activities) => {
            // Update cache on success
            cache.set(&cache_key, &activities, Duration::from_secs(3600)).await;
            Ok(activities)
        }
        Err(e) if e.is_transient() => {
            // Try cache on transient errors
            if let Some(cached) = cache.get::<Vec<Activity>>(&cache_key).await {
                tracing::warn!(
                    provider = %provider.name(),
                    error = %e,
                    "Provider unavailable, serving from cache"
                );
                Ok(cached)
            } else {
                Err(e)
            }
        }
        Err(e) => Err(e),
    }
}
```

### Provider Health Checks

Monitor provider availability proactively:

```rust
/// Provider health status
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub provider: String,
    pub is_healthy: bool,
    pub last_check: DateTime<Utc>,
    pub consecutive_failures: u32,
    pub average_latency_ms: f64,
}

/// Check provider health via lightweight endpoint
async fn check_provider_health(provider: &dyn FitnessProvider) -> ProviderHealth {
    let start = Instant::now();
    let result = provider.health_check().await;
    let latency = start.elapsed().as_millis() as f64;

    ProviderHealth {
        provider: provider.name().to_string(),
        is_healthy: result.is_ok(),
        last_check: Utc::now(),
        consecutive_failures: if result.is_ok() { 0 } else { 1 },
        average_latency_ms: latency,
    }
}
```

### Multi-Provider Fallback

When primary provider fails, try alternatives:

```rust
/// Try multiple providers in order
async fn get_activities_multi_provider(
    registry: &ProviderRegistry,
    user_id: Uuid,
    preferred_providers: &[&str],
) -> Result<Vec<Activity>, ProviderError> {
    let mut last_error = None;

    for provider_name in preferred_providers {
        if let Some(provider) = registry.get(provider_name) {
            match provider.get_activities(user_id).await {
                Ok(activities) => return Ok(activities),
                Err(e) => {
                    tracing::warn!(
                        provider = provider_name,
                        error = %e,
                        "Provider failed, trying next"
                    );
                    last_error = Some(e);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| ProviderError::NoProvidersAvailable))
}
```

### Resilience Configuration

Per-provider resilience settings:

```toml
# config/providers.toml (conceptual)
[strava]
max_retries = 3
base_delay_ms = 100
timeout_secs = 30
circuit_breaker_threshold = 5
circuit_breaker_reset_secs = 60

[garmin]
max_retries = 5  # Garmin is slower, more retries
base_delay_ms = 200
timeout_secs = 60
```

## Caching Provider Decorator

Pierre provides a `CachingFitnessProvider` decorator that wraps any `FitnessProvider` with transparent caching using the cache-aside pattern. This significantly reduces API calls to external providers.

### Cache-Aside Pattern

**Source**: src/providers/caching_provider.rs
```rust
/// Caching wrapper for any FitnessProvider implementation
pub struct CachingFitnessProvider<C: CacheProvider> {
    /// The underlying provider being wrapped
    inner: Box<dyn FitnessProvider>,
    /// Cache backend (Redis or in-memory)
    cache: Arc<C>,
    /// Tenant ID for cache key isolation
    tenant_id: Uuid,
    /// User ID for cache key isolation
    user_id: Uuid,
    /// TTL configuration for different resource types
    ttl_config: CacheTtlConfig,
}
```

**How it works**:
1. Check cache for requested data
2. If cache hit: return cached data immediately
3. If cache miss: fetch from provider API, store in cache, return data

```rust
// Create a caching provider
let cached_provider = CachingFitnessProvider::new(
    provider,        // Any Box<dyn FitnessProvider>
    cache,           // InMemoryCache or RedisCache
    tenant_id,
    user_id,
);

// Use normally - caching is transparent
let activities = cached_provider.get_activities(Some(10), None).await?;
```

### Cache Policy Control

The `CachePolicy` enum allows explicit control over caching behavior:

**Source**: src/providers/caching_provider.rs
```rust
/// Cache policy for controlling caching behavior per-request
pub enum CachePolicy {
    /// Use cache if available, fetch and cache on miss (default)
    UseCache,
    /// Bypass cache entirely, always fetch fresh data
    Bypass,
    /// Invalidate existing cache entry, fetch fresh, update cache
    Refresh,
}
```

**Usage**:
```rust
// Default behavior - use cache
let activities = cached_provider.get_activities(Some(10), None).await?;

// Force fresh data (user-triggered refresh)
let fresh = cached_provider
    .get_activities_with_policy(Some(10), None, CachePolicy::Refresh)
    .await?;

// Bypass cache entirely (debugging)
let uncached = cached_provider
    .get_activities_with_policy(Some(10), None, CachePolicy::Bypass)
    .await?;
```

### TTL Configuration

Different resources have different cache durations based on data volatility:

| Resource | TTL | Rationale |
|----------|-----|-----------|
| `AthleteProfile` | 24 hours | Profiles rarely change |
| `ActivityList` | 15 minutes | Need fresh for new activities |
| `Activity` | 1 hour | Activity details immutable after creation |
| `Stats` | 6 hours | Aggregates don't need real-time freshness |

**Source**: src/constants/cache.rs
```rust
pub const DEFAULT_PROFILE_TTL_SECS: u64 = 86_400;      // 24 hours
pub const DEFAULT_ACTIVITY_LIST_TTL_SECS: u64 = 900;   // 15 minutes
pub const DEFAULT_ACTIVITY_TTL_SECS: u64 = 3_600;      // 1 hour
pub const DEFAULT_STATS_TTL_SECS: u64 = 21_600;        // 6 hours
```

### Cache Key Structure

Cache keys include tenant/user/provider isolation for multi-tenant safety:

```
tenant:{tenant_id}:user:{user_id}:provider:{provider}:{resource_type}
```

**Examples**:
```
tenant:abc123:user:def456:provider:strava:athlete_profile
tenant:abc123:user:def456:provider:strava:activity_list:page:1:per_page:50
tenant:abc123:user:def456:provider:strava:activity:12345678
```

### Cache Invalidation

**Automatic invalidation on disconnect**:
```rust
// When user disconnects, cache is automatically cleared
impl<C: CacheProvider> FitnessProvider for CachingFitnessProvider<C> {
    async fn disconnect(&self) -> AppResult<()> {
        // Invalidate all user's cache entries
        self.invalidate_user_cache().await?;
        self.inner.disconnect().await
    }
}
```

**Manual invalidation (for webhooks)**:
```rust
// Invalidate when new activity detected via webhook
cached_provider.invalidate_activity_list_cache().await?;

// Invalidate all user cache
cached_provider.invalidate_user_cache().await?;
```

### Factory Methods

**Using the registry**:
```rust
// Create a caching provider via registry
let cached_provider = registry
    .create_caching_provider("strava", cache_config, tenant_id, user_id)
    .await?;

// Or use the global convenience function
let cached_provider = create_caching_provider_global(
    "strava",
    cache_config,
    tenant_id,
    user_id,
).await?;
```

### Cache Backend Selection

The caching provider supports both in-memory and Redis backends:

```bash
# Use Redis (production/multi-instance)
export REDIS_URL=redis://localhost:6379

# No REDIS_URL = use in-memory LRU cache (dev/single-instance)
```

**Benefits of caching**:
- **Reduced API calls**: Bounded by TTL, not request volume
- **Faster responses**: Sub-millisecond cache hits vs 100ms+ API calls
- **Rate limit protection**: Fewer calls = less risk of hitting limits
- **Resilience**: Cache can serve stale data during provider outages

## Configuration Best Practices

**Cloud deployment (.envrc for GCP/AWS)**:
```bash
# Production: Configure only active providers
export PIERRE_DEFAULT_PROVIDER=strava
export PIERRE_STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID}
export PIERRE_STRAVA_CLIENT_SECRET=${STRAVA_CLIENT_SECRET}

# Multi-provider setup
export PIERRE_DEFAULT_PROVIDER=strava
export PIERRE_STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID}
export PIERRE_STRAVA_CLIENT_SECRET=${STRAVA_CLIENT_SECRET}
export PIERRE_GARMIN_CLIENT_ID=${GARMIN_CONSUMER_KEY}
export PIERRE_GARMIN_CLIENT_SECRET=${GARMIN_CONSUMER_SECRET}
export PIERRE_FITBIT_CLIENT_ID=${FITBIT_CLIENT_ID}
export PIERRE_FITBIT_CLIENT_SECRET=${FITBIT_CLIENT_SECRET}

# Development: Use synthetic provider (no secrets!)
export PIERRE_DEFAULT_PROVIDER=synthetic
# No other vars needed - synthetic provider works out of the box
```

**Testing environments**:
```bash
# Integration tests: Use synthetic provider
export PIERRE_DEFAULT_PROVIDER=synthetic

# OAuth tests: Override to real provider
export PIERRE_DEFAULT_PROVIDER=strava
export PIERRE_STRAVA_CLIENT_ID=test-client-id
export PIERRE_STRAVA_CLIENT_SECRET=test-secret
```

## Key Takeaways

1. **Pluggable architecture**: Providers registered at runtime through factory pattern, no compile-time coupling.

2. **Feature flags**: Compile-time provider selection via `provider-strava`, `provider-garmin`, `provider-synthetic` for minimal binaries.

3. **Service Provider Interface (SPI)**: `ProviderDescriptor` trait enables external providers to register without core code changes.

4. **Bitflags capabilities**: `ProviderCapabilities` uses efficient bitflags with combinators like `full_health()` and `full_fitness()`.

5. **1 to x providers**: System supports unlimited providers simultaneously - just Strava, or Strava + Garmin + custom providers.

6. **Dynamic discovery**: `supported_providers()` and `is_supported()` enable runtime introspection and automatic tool adaptation.

7. **Environment-based config**: Cloud-native deployment using `PIERRE_<PROVIDER>_*` environment variables.

8. **Synthetic provider**: OAuth-free development provider perfect for CI/CD, demos, and rapid iteration.

9. **OAuth parameters**: `OAuthParams` struct captures provider-specific OAuth differences (scope separator, PKCE).

10. **Factory pattern**: `ProviderFactory` trait enables lazy provider instantiation with configuration injection.

11. **Shared interface**: `FitnessProvider` trait ensures uniform request/response patterns across all providers.

12. **Trait objects**: `Box<dyn ProviderFactory>` enables storing heterogeneous factory types in registry.

13. **Interior mutability**: `Arc<RwLock<T>>` pattern allows mutation through `&self` in async trait methods.

14. **Zero code changes**: Adding providers doesn't require modifying connection handlers, tools, or application logic.

15. **Type safety**: Compiler enforces that all providers implement complete `FitnessProvider` interface.

16. **Caching decorator**: `CachingFitnessProvider` wraps any provider with transparent cache-aside caching to reduce API calls.

17. **Cache policy control**: `CachePolicy` enum (`UseCache`, `Bypass`, `Refresh`) enables per-request cache behavior control.

18. **Multi-tenant cache isolation**: Cache keys include tenant/user/provider for safe multi-tenant deployments.

---

**Next Chapter**: [Chapter 18: A2A Protocol - Agent-to-Agent Communication](./chapter-18-a2a-protocol.md) - Learn how Pierre implements the Agent-to-Agent (A2A) protocol for secure inter-agent communication with Ed25519 signatures.

**Previous Chapter**: [Chapter 17: Provider Data Models & Rate Limiting](./chapter-17-provider-models.md) - Explore trait-based provider abstraction, unified data models, and retry logic with exponential backoff.
