# Building Pierre MCP Server with Private Provider Repository

## Architecture Overview

### Current Structure (Monorepo)
```
pierre_mcp_server/
├── src/
│   ├── providers/
│   │   ├── core.rs              # FitnessProvider trait (stays)
│   │   ├── spi.rs               # ProviderDescriptor SPI (stays)
│   │   ├── registry.rs          # ProviderRegistry (stays)
│   │   ├── strava_provider.rs   # → MOVES to private repo
│   │   ├── garmin_provider.rs   # → MOVES to private repo
│   │   └── synthetic_provider.rs # → MOVES to private repo
│   └── ...
└── Cargo.toml
```

### Future Structure (Separated Repositories)

**Public Repository: `pierre_mcp_server`**
```
pierre_mcp_server/
├── src/
│   ├── providers/
│   │   ├── core.rs              # FitnessProvider trait (PUBLIC)
│   │   ├── spi.rs               # ProviderDescriptor SPI (PUBLIC)
│   │   ├── registry.rs          # ProviderRegistry (PUBLIC)
│   │   └── mod.rs               # Public provider exports
│   └── ...
└── Cargo.toml                   # Dependencies on private provider crates
```

**Private Repository: `pierre-fitness-providers`**
```
pierre-fitness-providers/         # Private Git repository
├── Cargo.toml                    # Workspace configuration
├── providers/
│   ├── strava/
│   │   ├── Cargo.toml           # pierre-provider-strava crate
│   │   └── src/
│   │       ├── lib.rs           # StravaProvider + StravaDescriptor
│   │       └── ...
│   ├── garmin/
│   │   ├── Cargo.toml           # pierre-provider-garmin crate
│   │   └── src/
│   │       ├── lib.rs           # GarminProvider + GarminDescriptor
│   │       └── ...
│   └── synthetic/
│       ├── Cargo.toml           # pierre-provider-synthetic crate
│       └── src/
│           ├── lib.rs           # SyntheticProvider + SyntheticDescriptor
│           └── ...
└── README.md
```

---

## Build Configuration

### 1. Private Repository Cargo.toml (Workspace)

**File: `pierre-fitness-providers/Cargo.toml`**
```toml
[workspace]
members = [
    "providers/strava",
    "providers/garmin",
    "providers/synthetic",
]
resolver = "2"

[workspace.package]
version = "0.2.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/Async-IO/pierre-fitness-providers"

[workspace.dependencies]
# Shared dependencies for all providers
pierre_mcp_server = { version = "0.2", default-features = false }
tokio = { version = "1.45", features = ["rt-multi-thread", "macros"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
```

### 2. Individual Provider Crate Configuration

**File: `pierre-fitness-providers/providers/strava/Cargo.toml`**
```toml
[package]
name = "pierre-provider-strava"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Strava fitness provider for Pierre MCP Server"

[lib]
name = "pierre_provider_strava"
path = "src/lib.rs"

[dependencies]
# Import public SPI from main crate (without provider implementations)
pierre_mcp_server = { workspace = true, default-features = false, features = ["sqlite"] }

# Provider-specific dependencies
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }

# Strava-specific dependencies
base64 = "0.22"
sha2 = "0.10"
```

**Similar configuration for Garmin and Synthetic providers**

### 3. Main Server Cargo.toml (Updated)

**File: `pierre_mcp_server/Cargo.toml`** (Changes highlighted)
```toml
[package]
name = "pierre_mcp_server"
version = "0.2.0"
edition = "2021"
# ... (unchanged)

[features]
default = ["sqlite", "all-providers"]
sqlite = []
postgresql = ["sqlx/postgres"]

# Provider feature flags - now pull from private repository
provider-strava = ["pierre-provider-strava"]
provider-garmin = ["pierre-provider-garmin"]
provider-synthetic = ["pierre-provider-synthetic"]
all-providers = ["provider-strava", "provider-garmin", "provider-synthetic"]

[dependencies]
# ... (all existing dependencies unchanged)

# ============================================================================
# PRIVATE PROVIDER DEPENDENCIES (Git-based)
# ============================================================================
# These providers are in a private GitHub repository accessible via SSH or
# personal access token (PAT). CI/CD must configure git credentials.

# Option 1: SSH-based (recommended for development)
pierre-provider-strava = { git = "ssh://git@github.com/Async-IO/pierre-fitness-providers.git", optional = true }
pierre-provider-garmin = { git = "ssh://git@github.com/Async-IO/pierre-fitness-providers.git", optional = true }
pierre-provider-synthetic = { git = "ssh://git@github.com/Async-IO/pierre-fitness-providers.git", optional = true }

# Option 2: HTTPS with PAT (for CI/CD - configured via .cargo/config.toml)
# pierre-provider-strava = { git = "https://github.com/Async-IO/pierre-fitness-providers.git", optional = true }
# pierre-provider-garmin = { git = "https://github.com/Async-IO/pierre-fitness-providers.git", optional = true }
# pierre-provider-synthetic = { git = "https://github.com/Async-IO/pierre-fitness-providers.git", optional = true }

# Optional: Pin to specific branch/tag/commit for reproducibility
# pierre-provider-strava = { git = "...", branch = "main", optional = true }
# pierre-provider-strava = { git = "...", tag = "v0.2.0", optional = true }
# pierre-provider-strava = { git = "...", rev = "abc123def", optional = true }
```

---

## Build Process

### Local Development Build

```bash
# 1. Ensure SSH key is configured for private repository access
ssh -T git@github.com  # Verify GitHub SSH access

# 2. Build with all providers (default)
cargo build --release

# 3. Build with specific providers only
cargo build --release --no-default-features --features "sqlite,provider-strava"

# 4. Build without any providers (core server only)
cargo build --release --no-default-features --features "sqlite"
```

### CI/CD Build Configuration

#### GitHub Actions Example

**File: `.github/workflows/build.yml`**
```yaml
name: Build Pierre MCP Server

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      # CRITICAL: Configure Git credentials for private provider repository
      - name: Configure Git credentials
        run: |
          git config --global url."https://${{ secrets.PROVIDER_REPO_PAT }}@github.com/".insteadOf "https://github.com/"

      # Alternative: Use SSH key
      # - name: Configure SSH key
      #   uses: webfactory/ssh-agent@v0.9.0
      #   with:
      #     ssh-private-key: ${{ secrets.PROVIDER_REPO_SSH_KEY }}

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Build with all providers
        run: cargo build --release --all-features

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
```

**Required GitHub Secrets:**
- `PROVIDER_REPO_PAT`: Personal Access Token with `repo` scope for `pierre-fitness-providers`
- **OR** `PROVIDER_REPO_SSH_KEY`: SSH private key with read access

#### Alternative: Use `.cargo/config.toml` for Credential Management

**File: `.cargo/config.toml`** (Git-ignored, per-developer)
```toml
# Local development: Use SSH
[net]
git-fetch-with-cli = true

# CI/CD: Use HTTPS with token substitution
# [net]
# git-fetch-with-cli = false
#
# [http]
# proxy = "http://proxy.example.com:8080"  # Optional
#
# [registries.crates-io]
# protocol = "sparse"
```

**Environment Variable Approach (CI/CD)**
```bash
# Set before cargo build
export CARGO_NET_GIT_FETCH_WITH_CLI=true
git config --global credential.helper store
echo "https://${GITHUB_TOKEN}@github.com" > ~/.git-credentials
```

---

## Provider Registration (Code Changes)

### Updated Provider Registry

**File: `src/providers/registry.rs`**
```rust
use super::core::{FitnessProvider, ProviderConfig};
use super::spi::ProviderDescriptor;
use crate::constants::oauth_providers;
use std::collections::HashMap;
use std::sync::Arc;

// Import provider descriptors from separate crates (conditional compilation)
#[cfg(feature = "provider-strava")]
use pierre_provider_strava::{StravaDescriptor, StravaProviderFactory};

#[cfg(feature = "provider-garmin")]
use pierre_provider_garmin::{GarminDescriptor, GarminProviderFactory};

#[cfg(feature = "provider-synthetic")]
use pierre_provider_synthetic::{SyntheticDescriptor, SyntheticProviderFactory};

pub struct ProviderRegistry {
    factories: HashMap<String, Box<dyn ProviderFactory>>,
    descriptors: HashMap<String, Box<dyn ProviderDescriptor>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            descriptors: HashMap::new(),
        };

        // Conditionally register Strava provider
        #[cfg(feature = "provider-strava")]
        {
            registry.register_factory(
                oauth_providers::STRAVA,
                Box::new(StravaProviderFactory),
            );
            registry.register_descriptor(
                oauth_providers::STRAVA,
                Box::new(StravaDescriptor),
            );
        }

        // Conditionally register Garmin provider
        #[cfg(feature = "provider-garmin")]
        {
            registry.register_factory(
                oauth_providers::GARMIN,
                Box::new(GarminProviderFactory),
            );
            registry.register_descriptor(
                oauth_providers::GARMIN,
                Box::new(GarminDescriptor),
            );
        }

        // Conditionally register Synthetic provider
        #[cfg(feature = "provider-synthetic")]
        {
            registry.register_factory(
                oauth_providers::SYNTHETIC,
                Box::new(SyntheticProviderFactory),
            );
            registry.register_descriptor(
                oauth_providers::SYNTHETIC,
                Box::new(SyntheticDescriptor),
            );
        }

        registry
    }

    // ... (rest of implementation unchanged)
}
```

### Provider Crate Public Interface

**File: `pierre-fitness-providers/providers/strava/src/lib.rs`**
```rust
//! Strava fitness provider for Pierre MCP Server
//!
//! This crate provides Strava API integration through the Pierre provider SPI.

// Re-export public types from main crate
pub use pierre_mcp_server::providers::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
pub use pierre_mcp_server::providers::spi::{
    OAuthEndpoints, OAuthParams, ProviderDescriptor, ProviderFactory,
};

mod provider;
mod descriptor;

// Public exports
pub use provider::StravaProvider;
pub use provider::StravaProviderFactory;
pub use descriptor::StravaDescriptor;
```

---

## Deployment Scenarios

### 1. Public Server (Open Source)
```bash
# Build without proprietary providers
cargo build --release --no-default-features \
  --features "sqlite,provider-synthetic"

# Only synthetic provider available for testing
```

### 2. Commercial Deployment (Full Providers)
```bash
# Build with all providers (requires private repository access)
cargo build --release --features "all-providers"

# Deploy with environment credentials
export PIERRE_STRAVA_CLIENT_ID="..."
export PIERRE_STRAVA_CLIENT_SECRET="..."
export PIERRE_GARMIN_CLIENT_ID="..."
export PIERRE_GARMIN_CLIENT_SECRET="..."
```

### 3. Single-Provider Deployment
```bash
# Build with only Strava support
cargo build --release --no-default-features \
  --features "sqlite,provider-strava"
```

---

## Advantages of This Architecture

### 1. **Access Control**
- ✅ Core server is public (MIT/Apache-2.0)
- ✅ Provider implementations are private (proprietary licenses possible)
- ✅ Git-based access control via GitHub repository permissions

### 2. **Flexible Licensing**
- ✅ Public SPI allows third-party providers
- ✅ Official providers can have different licenses
- ✅ Commercial vs. open-source provider options

### 3. **Feature Flag Control**
- ✅ Compile-time provider selection
- ✅ Reduced binary size when providers excluded
- ✅ Zero-cost abstraction (providers compiled out if disabled)

### 4. **Dependency Isolation**
- ✅ Provider-specific dependencies isolated to provider crates
- ✅ Main server doesn't carry unused provider dependencies
- ✅ Easier security audits (smaller public surface area)

### 5. **Development Workflow**
- ✅ Public contributors can develop without provider access
- ✅ Core SPI development doesn't require provider credentials
- ✅ Provider teams can work independently in private repo

---

## Migration Checklist

### Phase 1: Prepare Public SPI
- [x] FitnessProvider trait finalized
- [x] ProviderDescriptor trait with OAuth metadata
- [x] ProviderRegistry supports dynamic registration
- [x] Feature flags for conditional compilation
- [x] Documentation for SPI usage

### Phase 2: Create Private Repository
- [ ] Create `pierre-fitness-providers` private repository
- [ ] Set up Cargo workspace with provider crates
- [ ] Move Strava provider implementation
- [ ] Move Garmin provider implementation
- [ ] Move Synthetic provider implementation
- [ ] Configure GitHub access controls

### Phase 3: Update Main Repository
- [ ] Update `Cargo.toml` with git dependencies
- [ ] Update `src/providers/mod.rs` to import from external crates
- [ ] Update `src/providers/registry.rs` imports
- [ ] Update CI/CD with credential configuration
- [ ] Update documentation with build instructions

### Phase 4: Testing & Validation
- [ ] Test build with all providers enabled
- [ ] Test build with individual providers
- [ ] Test build with no providers (core only)
- [ ] Verify CI/CD pipeline works with credentials
- [ ] Validate feature flag behavior
- [ ] Run architectural validation script

### Phase 5: Documentation
- [ ] Update README.md with build instructions
- [ ] Document credential configuration for CI/CD
- [ ] Create developer guide for private provider access
- [ ] Document public SPI for third-party developers
- [ ] Update deployment documentation

---

## Troubleshooting

### Build Error: "Unable to update git repository"

**Problem**: Cargo cannot access private repository
```
error: failed to load source for dependency `pierre-provider-strava`
Caused by: Unable to update https://github.com/Async-IO/pierre-fitness-providers
```

**Solution**:
```bash
# Ensure git credentials are configured
git config --global credential.helper store

# Or use SSH instead of HTTPS
# Update Cargo.toml to use: git = "ssh://git@github.com/..."

# Verify access
git ls-remote ssh://git@github.com/Async-IO/pierre-fitness-providers.git
```

### Build Error: "Feature X is not enabled"

**Problem**: Missing feature flag
```
error[E0432]: unresolved import `pierre_provider_strava`
```

**Solution**:
```bash
# Enable required features explicitly
cargo build --features "provider-strava,provider-garmin"
```

### CI/CD Error: "Permission denied (publickey)"

**Problem**: SSH key not configured in CI environment

**Solution**: Use HTTPS with PAT instead
```yaml
# In .github/workflows/build.yml
- name: Configure Git credentials
  run: |
    git config --global url."https://${{ secrets.PROVIDER_REPO_PAT }}@github.com/".insteadOf "https://github.com/"
```

---

## Summary

This architecture provides:

1. **Clear separation** between public core and private providers
2. **Flexible builds** via feature flags
3. **Git-based access control** for proprietary code
4. **Zero runtime overhead** (compile-time provider selection)
5. **Third-party extensibility** through public SPI

**Build Command Examples:**
```bash
# Full build (requires private repo access)
cargo build --release --all-features

# Public build (synthetic provider only)
cargo build --release --no-default-features --features "sqlite,provider-synthetic"

# Custom build (Strava only)
cargo build --release --no-default-features --features "sqlite,provider-strava"
```
