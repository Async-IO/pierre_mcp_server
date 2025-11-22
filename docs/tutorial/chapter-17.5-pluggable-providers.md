# chapter 17.5: pluggable provider architecture

This chapter explores pierre's pluggable provider architecture that enables runtime registration of 1 to x fitness providers simultaneously. You'll learn about provider factories, dynamic discovery, environment-based configuration, and how to add new providers without modifying existing code.

## what you'll learn

- Provider registry and factory pattern
- Runtime provider discovery (1 to x providers)
- Environment-based provider configuration
- Shared request/response trait contracts
- Adding custom providers without code changes
- Synthetic provider for development/testing
- Multi-provider connection management
- Provider capability detection

## pluggable architecture overview

Pierre implements a **fully pluggable provider system** where fitness providers are registered at runtime through a factory pattern. The system supports **1 to x providers simultaneously**, meaning you can use just Strava, or Strava + Garmin + Fitbit + custom providers all at once.

```
┌─────────────────────────────────────────────────────────┐
│               ProviderRegistry (runtime)                │
│  Manages 1 to x providers with dynamic discovery        │
└────────────┬────────────────────────────────────────────┘
             │
    ┌────────┴────────┬───────────┬────────────┬──────────┐
    │                 │           │            │          │
    ▼                 ▼           ▼            ▼          ▼
┌─────────┐    ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐
│ Strava  │    │ Garmin  │  │ Fitbit  │  │Synthetic│  │ Custom  │
│ Factory │    │ Factory │  │ Factory │  │ Factory │  │ Factory │
└────┬────┘    └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘
     │              │           │            │           │
     ▼              ▼           ▼            ▼           ▼
┌─────────┐    ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐
│ Strava  │    │ Garmin  │  │ Fitbit  │  │Synthetic│  │ Custom  │
│Provider │    │Provider │  │Provider │  │Provider │  │Provider │
└─────────┘    └─────────┘  └─────────┘  └─────────┘  └─────────┘
     │              │           │            │           │
     └──────────────┴───────────┴────────────┴───────────┘
                           │
                           ▼
            ┌──────────────────────────┐
            │   FitnessProvider Trait  │
            │   (shared interface)     │
            └──────────────────────────┘
```

**Key benefit**: Add, remove, or swap providers without modifying tool code, connection handlers, or application logic.

## provider registry

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

## provider factory pattern

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

## environment-based configuration

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

## dynamic provider discovery

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

## synthetic provider (phase 1)

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

## adding a custom provider (1 to x)

Here's how to add a new provider to pierre's registry:

### step 1: implement FitnessProvider trait

**Source**: your_custom_provider.rs
```rust
use pierre_mcp_server::providers::core::{FitnessProvider, ProviderConfig, OAuth2Credentials};
use pierre_mcp_server::models::{Activity, Athlete, Stats};
use pierre_mcp_server::errors::AppResult;
use pierre_mcp_server::pagination::{PaginationParams, CursorPage};
use async_trait::async_trait;

pub struct WhooProvider {
    config: ProviderConfig,
    credentials: Arc<RwLock<Option<OAuth2Credentials>>>,
    http_client: reqwest::Client,
}

#[async_trait]
impl FitnessProvider for WhooProvider {
    fn name(&self) -> &'static str {
        "whoop"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        *self.credentials.write().await = Some(credentials);
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        self.credentials.read().await.is_some()
    }

    async fn get_athlete(&self) -> AppResult<Athlete> {
        // Fetch from Whoop API
        let response = self.http_client
            .get(&format!("{}/user/profile", self.config.api_base_url))
            .header("Authorization", format!("Bearer {}", self.access_token()?))
            .send()
            .await?;

        // Convert Whoop JSON to unified Athlete model
        let whoop_user: WhoopUserResponse = response.json().await?;
        Ok(Athlete {
            id: whoop_user.user_id.to_string(),
            username: Some(whoop_user.email),
            firstname: Some(whoop_user.first_name),
            lastname: Some(whoop_user.last_name),
            // ... map Whoop fields to Athlete model
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
        // Fetch from Whoop API
        // Convert Whoop workouts to Activity models
        todo!("Implement Whoop activity fetching")
    }

    // ... implement remaining trait methods
}
```

### step 2: create provider factory

```rust
pub struct WhoopProviderFactory;

impl ProviderFactory for WhoopProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(WhoopProvider::new(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["whoop"]
    }
}
```

### step 3: register in ProviderRegistry

**Source**: src/providers/registry.rs
```rust
impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
        };

        // Existing providers (Strava, Garmin, Fitbit, Synthetic)
        // ...

        // Register Whoop provider (NEW!)
        registry.register_factory(
            "whoop",  // Provider name
            Box::new(WhoopProviderFactory),
        );
        let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
            crate::config::environment::load_provider_env_config(
                "whoop",
                "https://api.prod.whoop.com/oauth/authorize",
                "https://api.prod.whoop.com/oauth/token",
                "https://api.prod.whoop.com/developer/v1",
                Some("https://api.prod.whoop.com/oauth/revoke"),
                &["read:workout".to_owned(), "read:profile".to_owned()],
            );
        registry.set_default_config(
            "whoop",
            ProviderConfig {
                name: "whoop".to_owned(),
                auth_url,
                token_url,
                api_base_url,
                revoke_url,
                default_scopes: scopes,
            },
        );

        registry
    }
}
```

### step 4: add to constants

**Source**: src/constants/oauth/providers.rs
```rust
pub const WHOOP: &str = "whoop";

#[must_use]
pub const fn all() -> &'static [&'static str] {
    &[STRAVA, FITBIT, GARMIN, SYNTHETIC, WHOOP]  // Add WHOOP
}
```

### step 5: configure environment

**Source**: .envrc
```bash
# Whoop provider configuration
export PIERRE_WHOOP_CLIENT_ID=your-whoop-client-id
export PIERRE_WHOOP_CLIENT_SECRET=your-whoop-secret
export PIERRE_WHOOP_SCOPES="read:workout,read:profile"
```

**That's it!** Whoop is now:
- ✅ Available in `supported_providers()`
- ✅ Discoverable via `is_supported("whoop")`
- ✅ Creatable via `create_provider("whoop")`
- ✅ Listed in connection status responses
- ✅ Supported in `connect_provider` tool

**No changes needed**:
- ❌ Connection handlers (dynamic discovery)
- ❌ Tool implementations (use FitnessProvider trait)
- ❌ MCP schema generation (automatic)
- ❌ Test fixtures (provider-agnostic)

## managing 1 to x providers simultaneously

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

## shared request/response traits

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

## rust idioms: trait object factory

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

## rust idioms: arc<rwlock<t>> for interior mutability

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

## configuration best practices

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

## key takeaways

1. **Pluggable architecture**: Providers registered at runtime through factory pattern, no compile-time coupling.

2. **1 to x providers**: System supports unlimited providers simultaneously - just Strava, or Strava + Garmin + Fitbit + custom providers.

3. **Dynamic discovery**: `supported_providers()` and `is_supported()` enable runtime introspection and automatic tool adaptation.

4. **Environment-based config**: Cloud-native deployment using `PIERRE_<PROVIDER>_*` environment variables (no TOML files).

5. **Synthetic provider**: OAuth-free development provider perfect for CI/CD, demos, and rapid iteration.

6. **Factory pattern**: `ProviderFactory` trait enables lazy provider instantiation with configuration injection.

7. **Shared interface**: `FitnessProvider` trait ensures uniform request/response patterns across all providers.

8. **Trait objects**: `Box<dyn ProviderFactory>` enables storing heterogeneous factory types in registry.

9. **Interior mutability**: `Arc<RwLock<T>>` pattern allows mutation through `&self` in async trait methods.

10. **Zero code changes**: Adding providers doesn't require modifying connection handlers, tools, or application logic.

11. **Backward compatibility**: Legacy env vars (`STRAVA_CLIENT_ID`) supported alongside new format (`PIERRE_STRAVA_CLIENT_ID`).

12. **Type safety**: Compiler enforces that all providers implement complete `FitnessProvider` interface.

---

**Next Chapter**: [Chapter 18: A2A Protocol - Agent-to-Agent Communication](./chapter-18-a2a-protocol.md) - Learn how Pierre implements the Agent-to-Agent (A2A) protocol for secure inter-agent communication with Ed25519 signatures.

**Previous Chapter**: [Chapter 17: Provider Data Models & Rate Limiting](./chapter-17-provider-models.md) - Explore trait-based provider abstraction, unified data models, and retry logic with exponential backoff.
