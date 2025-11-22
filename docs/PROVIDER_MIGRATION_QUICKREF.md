# Provider Migration Quick Reference

## TL;DR: How It Works

```
┌─────────────────────────────────────────────────────────────┐
│  PUBLIC REPO: pierre_mcp_server                             │
│  ├── Core server code                                       │
│  ├── FitnessProvider trait (SPI)                            │
│  ├── ProviderDescriptor trait (SPI)                         │
│  └── Cargo.toml                                             │
│      └── [dependencies]                                     │
│          pierre-provider-strava = { git = "ssh://..." }     │
│          pierre-provider-garmin = { git = "ssh://..." }     │
│                                                             │
│          ↓ Cargo fetches via Git                            │
│                                                             │
│  PRIVATE REPO: pierre-fitness-providers                     │
│  ├── providers/strava/    (proprietary Strava client)       │
│  ├── providers/garmin/    (proprietary Garmin client)       │
│  └── providers/synthetic/ (test provider)                   │
└─────────────────────────────────────────────────────────────┘
```

## Key Build Commands

### Development Build (All Providers)
```bash
# Requires SSH access to private repository
cargo build --release
```

### Public Build (No Proprietary Providers)
```bash
# Only includes synthetic test provider
cargo build --release --no-default-features \
  --features "sqlite,provider-synthetic"
```

### Single Provider Build
```bash
# Strava only
cargo build --release --no-default-features \
  --features "sqlite,provider-strava"

# Garmin only
cargo build --release --no-default-features \
  --features "sqlite,provider-garmin"
```

## Cargo.toml Dependencies (After Migration)

```toml
[features]
provider-strava = ["pierre-provider-strava"]
provider-garmin = ["pierre-provider-garmin"]
provider-synthetic = ["pierre-provider-synthetic"]
all-providers = ["provider-strava", "provider-garmin", "provider-synthetic"]

[dependencies]
# Git dependencies with SSH authentication
pierre-provider-strava = {
    git = "ssh://git@github.com/Async-IO/pierre-fitness-providers.git",
    optional = true
}
pierre-provider-garmin = {
    git = "ssh://git@github.com/Async-IO/pierre-fitness-providers.git",
    optional = true
}
pierre-provider-synthetic = {
    git = "ssh://git@github.com/Async-IO/pierre-fitness-providers.git",
    optional = true
}
```

## CI/CD Configuration

### GitHub Actions (with Personal Access Token)
```yaml
steps:
  - name: Configure Git credentials for private providers
    run: |
      git config --global url."https://${{ secrets.PROVIDER_REPO_PAT }}@github.com/".insteadOf "https://github.com/"

  - name: Build with all providers
    run: cargo build --release --all-features
```

### GitHub Actions (with SSH Key)
```yaml
steps:
  - name: Configure SSH key for private providers
    uses: webfactory/ssh-agent@v0.9.0
    with:
      ssh-private-key: ${{ secrets.PROVIDER_REPO_SSH_KEY }}

  - name: Build with all providers
    run: cargo build --release --all-features
```

## What Changes in Code

### Before (Monorepo)
```rust
// src/providers/registry.rs
use super::strava_provider::StravaProviderFactory;  // Local module
use super::spi::StravaDescriptor;                   // Local module

#[cfg(feature = "provider-strava")]
{
    registry.register_factory("strava", Box::new(StravaProviderFactory));
    registry.register_descriptor("strava", Box::new(StravaDescriptor));
}
```

### After (Private Repo)
```rust
// src/providers/registry.rs
use pierre_provider_strava::{StravaProviderFactory, StravaDescriptor}; // External crate

#[cfg(feature = "provider-strava")]
{
    registry.register_factory("strava", Box::new(StravaProviderFactory));
    registry.register_descriptor("strava", Box::new(StravaDescriptor));
}
```

**Key Difference**: Import from external crate instead of local module.

## What Stays Public (SPI)

These remain in the **public** `pierre_mcp_server` repository:

```rust
// src/providers/core.rs
pub trait FitnessProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn get_athlete(&self) -> AppResult<Athlete>;
    async fn get_activities(...) -> AppResult<Vec<Activity>>;
    // ... full trait definition
}

// src/providers/spi.rs
pub trait ProviderDescriptor: Send + Sync {
    fn name(&self) -> &'static str;
    fn oauth_endpoints(&self) -> Option<OAuthEndpoints>;
    fn oauth_params(&self) -> Option<OAuthParams>;
    fn default_scopes(&self) -> &'static [&'static str];
}

pub trait ProviderFactory: Send + Sync {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider>;
    fn supported_providers(&self) -> &'static [&'static str];
}
```

**Anyone** can implement these traits to create third-party providers.

## What Moves to Private Repo

These move to **private** `pierre-fitness-providers` repository:

```
pierre-fitness-providers/
├── providers/strava/src/
│   ├── lib.rs                    # StravaProvider + StravaDescriptor
│   ├── provider.rs               # impl FitnessProvider for StravaProvider
│   └── descriptor.rs             # impl ProviderDescriptor for StravaDescriptor
│
├── providers/garmin/src/
│   ├── lib.rs                    # GarminProvider + GarminDescriptor
│   ├── provider.rs               # impl FitnessProvider for GarminProvider
│   └── descriptor.rs             # impl ProviderDescriptor for GarminDescriptor
│
└── providers/synthetic/src/
    └── lib.rs                    # SyntheticProvider (test provider)
```

## Binary Size Impact

### With All Providers
```bash
$ cargo build --release
$ ls -lh target/release/pierre-mcp-server
-rwxr-xr-x  1 user  staff   14M pierre-mcp-server
```

### With Synthetic Only
```bash
$ cargo build --release --no-default-features --features "sqlite,provider-synthetic"
$ ls -lh target/release/pierre-mcp-server
-rwxr-xr-x  1 user  staff   9.2M pierre-mcp-server  # ~35% smaller
```

**Benefit**: Unused providers are completely compiled out (zero runtime cost).

## Security Implications

### Before Migration
- ✅ Public: Core server
- ❌ Public: Strava API client code (with OAuth secrets handling)
- ❌ Public: Garmin API client code (with OAuth secrets handling)

### After Migration
- ✅ Public: Core server
- ✅ Public: Provider SPI (trait definitions only)
- ✅ Private: Strava API client implementation
- ✅ Private: Garmin API client implementation
- ✅ Public: Synthetic test provider (no real credentials)

**Benefit**: Proprietary provider implementations not visible to public.

## Third-Party Provider Development

Even with private providers, **anyone** can create their own provider:

```rust
// external-provider-crate/src/lib.rs
use pierre_mcp_server::providers::core::{FitnessProvider, ProviderConfig};
use pierre_mcp_server::providers::spi::ProviderDescriptor;
use async_trait::async_trait;

pub struct MyCustomProvider { /* ... */ }

#[async_trait]
impl FitnessProvider for MyCustomProvider {
    fn name(&self) -> &'static str { "mycustom" }
    async fn get_athlete(&self) -> AppResult<Athlete> {
        // Custom implementation
    }
    // ... implement all required methods
}

pub struct MyCustomDescriptor;
impl ProviderDescriptor for MyCustomDescriptor {
    fn name(&self) -> &'static str { "mycustom" }
    fn oauth_endpoints(&self) -> Option<OAuthEndpoints> { /* ... */ }
    fn oauth_params(&self) -> Option<OAuthParams> { /* ... */ }
    fn default_scopes(&self) -> &'static [&'static str] { /* ... */ }
}
```

**Then register it:**
```rust
registry.register_factory("mycustom", Box::new(MyCustomProviderFactory));
registry.register_descriptor("mycustom", Box::new(MyCustomDescriptor));
```

## Migration Timeline Estimate

| Phase | Duration | Tasks |
|-------|----------|-------|
| **Phase 1**: SPI Preparation | ✅ Complete | Trait definitions, feature flags |
| **Phase 2**: Private Repo Setup | 2-4 hours | Create repo, workspace setup |
| **Phase 3**: Code Migration | 4-6 hours | Move providers, update imports |
| **Phase 4**: CI/CD Configuration | 2-3 hours | GitHub Actions, credentials |
| **Phase 5**: Testing & Validation | 3-4 hours | Build matrix, integration tests |
| **Total** | ~1-2 days | Full migration and validation |

## Next Steps

1. **Complete Task 4 validation** (confirm SPI is stable)
2. **Run comprehensive tests** (`cargo test --all-features`)
3. **Create private repository** `pierre-fitness-providers`
4. **Migrate providers** one at a time (Strava → Garmin → Synthetic)
5. **Update CI/CD** with credential configuration
6. **Update documentation** with new build instructions

---

## Quick Verification Commands

```bash
# Verify current features
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "pierre_mcp_server") | .features'

# Test build without providers
cargo check --no-default-features --features sqlite

# Test build with single provider
cargo check --no-default-features --features "sqlite,provider-strava"

# Verify feature flag conditional compilation
cargo expand --features provider-strava | grep "StravaDescriptor"
```

## Reference Documentation

- Full details: [PRIVATE_PROVIDERS_BUILD.md](./PRIVATE_PROVIDERS_BUILD.md)
- Provider SPI: [src/providers/spi.rs](../src/providers/spi.rs)
- Current architecture: [ARCHITECTURE.md](./ARCHITECTURE.md)
