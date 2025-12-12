# provider registration guide

This guide shows how pierre's pluggable provider architecture supports **1 to x providers simultaneously** and how new providers are registered.

## provider registration flow

```
┌──────────────────────────────────────────────────────┐
│  Step 1: Application Startup                         │
│  ProviderRegistry::new() called                      │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 2: Factory Registration (1 to x providers)     │
│                                                       │
│  registry.register_factory("strava", StravaFactory)  │
│  registry.register_factory("garmin", GarminFactory)  │
│  registry.register_factory("fitbit", FitbitFactory)  │
│  registry.register_factory("synthetic", SynthFactory)│
│  registry.register_factory("whoop", WhoopFactory)    │ <- built-in
│  registry.register_factory("terra", TerraFactory)    │ <- built-in
│  registry.register_factory("polar", PolarFactory)    │ <- custom example
│  ... unlimited providers ...                         │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 3: Environment Configuration Loading           │
│                                                       │
│  For each registered provider:                       │
│    config = load_provider_env_config(                │
│      provider_name,                                  │
│      default_auth_url,                               │
│      default_token_url,                              │
│      default_api_base_url,                           │
│      default_revoke_url,                             │
│      default_scopes                                  │
│    )                                                 │
│    registry.set_default_config(provider, config)     │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 4: Runtime Usage                               │
│                                                       │
│  // Check if provider is available                   │
│  if registry.is_supported("strava") { ... }          │
│                                                       │
│  // List all available providers                     │
│  let providers = registry.supported_providers();     │
│  // ["strava", "garmin", "fitbit", "synthetic",      │
│  //  "whoop", "polar", ...]                          │
│                                                       │
│  // Create provider instance                         │
│  let provider = registry.create_provider("strava");  │
│                                                       │
│  // Use provider through FitnessProvider trait       │
│  let activities = provider.get_activities(...).await;│
└──────────────────────────────────────────────────────┘
```

## how providers are registered

### example: registering strava (built-in)

**Location**: `src/providers/registry.rs:71-94`

```rust
impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
        };

        // 1. Register factory
        registry.register_factory(
            oauth_providers::STRAVA,  // "strava"
            Box::new(StravaProviderFactory),
        );

        // 2. Load environment configuration
        let (_client_id, _client_secret, auth_url, token_url,
             api_base_url, revoke_url, scopes) =
            crate::config::environment::load_provider_env_config(
                oauth_providers::STRAVA,
                "https://www.strava.com/oauth/authorize",
                "https://www.strava.com/oauth/token",
                "https://www.strava.com/api/v3",
                Some("https://www.strava.com/oauth/deauthorize"),
                &[oauth_providers::STRAVA_DEFAULT_SCOPES.to_owned()],
            );

        // 3. Set default configuration
        registry.set_default_config(
            oauth_providers::STRAVA,
            ProviderConfig {
                name: oauth_providers::STRAVA.to_owned(),
                auth_url,
                token_url,
                api_base_url,
                revoke_url,
                default_scopes: scopes,
            },
        );

        // Repeat for Garmin, Fitbit, Synthetic, etc.
        // ...

        registry
    }
}
```

### example: registering custom provider (whoop)

**Location**: `src/providers/registry.rs` (add to `new()` method)

```rust
// Register Whoop provider
registry.register_factory(
    "whoop",
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
```

**That's it!** Whoop is now registered and available alongside Strava, Garmin, and others.

## environment variables for 1 to x providers

Pierre supports **unlimited providers simultaneously**. Just set environment variables for each:

```bash
# Default provider (required)
export PIERRE_DEFAULT_PROVIDER=strava

# Provider 1: Strava
export PIERRE_STRAVA_CLIENT_ID=abc123
export PIERRE_STRAVA_CLIENT_SECRET=secret123

# Provider 2: Garmin
export PIERRE_GARMIN_CLIENT_ID=xyz789
export PIERRE_GARMIN_CLIENT_SECRET=secret789

# Provider 3: Fitbit
export PIERRE_FITBIT_CLIENT_ID=fitbit123
export PIERRE_FITBIT_CLIENT_SECRET=fitbit_secret

# Provider 4: Synthetic (no credentials needed!)
# Automatically available - no env vars required

# Provider 5: Custom Whoop
export PIERRE_WHOOP_CLIENT_ID=whoop_client
export PIERRE_WHOOP_CLIENT_SECRET=whoop_secret

# Provider 6: Custom Polar
export PIERRE_POLAR_CLIENT_ID=polar_client
export PIERRE_POLAR_CLIENT_SECRET=polar_secret

# ... unlimited providers ...
```

## dynamic discovery at runtime

Tools automatically discover all registered providers:

### connection status for all providers

**Request**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "get_connection_status"
  }
}
```

**Response** (discovers all 1 to x providers):
```json
{
  "success": true,
  "result": {
    "providers": {
      "strava": { "connected": true, "status": "connected" },
      "garmin": { "connected": true, "status": "connected" },
      "fitbit": { "connected": false, "status": "disconnected" },
      "synthetic": { "connected": true, "status": "connected" },
      "whoop": { "connected": true, "status": "connected" },
      "polar": { "connected": false, "status": "disconnected" }
    }
  }
}
```

**Implementation** (`src/protocols/universal/handlers/connections.rs:84-110`):
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

**Key benefit**: No hardcoded provider lists! Add/remove providers without changing tool code.

### dynamic error messages

**Request** (invalid provider):
```json
{
  "method": "tools/call",
  "params": {
    "name": "connect_provider",
    "arguments": {
      "provider": "unknown_provider"
    }
  }
}
```

**Response** (automatically lists all registered providers):
```json
{
  "success": false,
  "error": "Provider 'unknown_provider' is not supported. Supported providers: strava, garmin, fitbit, synthetic, whoop, polar"
}
```

**Implementation** (`src/protocols/universal/handlers/connections.rs:332-340`):
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

## provider factory implementations

Each provider implements `ProviderFactory`:

### strava factory

```rust
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

### synthetic factory (oauth-free!)

```rust
struct SyntheticProviderFactory;

impl ProviderFactory for SyntheticProviderFactory {
    fn create(&self, _config: ProviderConfig) -> Box<dyn FitnessProvider> {
        // Ignores config - generates synthetic data
        Box::new(SyntheticProvider::default())
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["synthetic"]
    }
}
```

### custom whoop factory (example)

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

## simultaneous multi-provider usage

Users can connect to **all providers simultaneously** and aggregate data:

### example: aggregating activities from all connected providers

```rust
pub async fn get_all_activities_from_all_providers(
    user_id: Uuid,
    tenant_id: Uuid,
    registry: &ProviderRegistry,
    auth_service: &AuthService,
) -> Vec<Activity> {
    let mut all_activities = Vec::new();

    // Iterate through all registered providers
    for provider_name in registry.supported_providers() {
        // Check if user is connected to this provider
        if let Ok(Some(credentials)) = auth_service
            .get_valid_token(user_id, &provider_name, Some(&tenant_id.to_string()))
            .await
        {
            // Create provider instance
            if let Some(provider) = registry.create_provider(&provider_name) {
                // Set credentials
                if provider.set_credentials(credentials).await.is_ok() {
                    // Fetch activities
                    if let Ok(activities) = provider.get_activities(Some(50), None).await {
                        all_activities.extend(activities);
                    }
                }
            }
        }
    }

    // Sort by date (most recent first)
    all_activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));

    // Deduplicate if needed (same activity synced to multiple providers)
    all_activities
}
```

**Result**: Activities from Strava, Garmin, Fitbit, Whoop, Polar all in one unified list!

## configuration best practices

### development (single provider)
```bash
# Use synthetic provider - no OAuth needed
export PIERRE_DEFAULT_PROVIDER=synthetic
```

### production (multi-provider deployment)
```bash
# Default to strava
export PIERRE_DEFAULT_PROVIDER=strava

# Configure all active providers
export PIERRE_STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID_SECRET}
export PIERRE_STRAVA_CLIENT_SECRET=${STRAVA_SECRET}

export PIERRE_GARMIN_CLIENT_ID=${GARMIN_KEY}
export PIERRE_GARMIN_CLIENT_SECRET=${GARMIN_SECRET}

export PIERRE_FITBIT_CLIENT_ID=${FITBIT_KEY}
export PIERRE_FITBIT_CLIENT_SECRET=${FITBIT_SECRET}
```

### testing (mix synthetic + real)
```bash
# Test with both synthetic and real provider
export PIERRE_DEFAULT_PROVIDER=synthetic
export PIERRE_STRAVA_CLIENT_ID=test_id
export PIERRE_STRAVA_CLIENT_SECRET=test_secret
```

## summary

**1 to x providers simultaneously**:
- ✅ Register unlimited providers via factory pattern
- ✅ Each provider independently configured via environment variables
- ✅ Runtime discovery via `supported_providers()` and `is_supported()`
- ✅ Zero code changes to add/remove providers
- ✅ Tools automatically adapt to available providers
- ✅ Users can connect to all providers at once
- ✅ Data aggregation across multiple providers
- ✅ Synthetic provider for OAuth-free development

**Key files**:
- `src/providers/registry.rs` - Central registry managing all providers
- `src/providers/core.rs` - `FitnessProvider` trait and `ProviderFactory` trait
- `src/config/environment.rs` - Environment-based configuration loading
- `src/protocols/universal/handlers/connections.rs` - Dynamic provider discovery

For detailed implementation guide, see [Chapter 17.5: Pluggable Provider Architecture](tutorial/chapter-17.5-pluggable-providers.md).
