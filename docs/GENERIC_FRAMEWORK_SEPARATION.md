# Generic Framework Separation Architecture

## Vision: Pierre as a Generic A2A/MCP/REST Framework

### Current Problem
`pierre_mcp_server` is currently a **monolithic fitness application** with:
- Protocol servers (MCP, A2A, REST) - **GENERIC**
- Fitness providers (Strava, Garmin) - **DOMAIN-SPECIFIC**
- Fitness intelligence (16,257 lines) - **DOMAIN-SPECIFIC**
- Multi-tenant infrastructure - **GENERIC**

**Goal**: Separate into **generic framework** + **fitness application** built on top of it.

---

## Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 1: PUBLIC GENERIC FRAMEWORK (pierre-framework)          │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  • Protocol Servers: MCP, A2A, REST                             │
│  • Multi-tenant auth & authorization                            │
│  • Database abstraction (SQLite, PostgreSQL)                    │
│  • Cache layer (Redis, in-memory)                               │
│  • OAuth2 server infrastructure                                 │
│  • Generic Provider SPI                                         │
│  • Admin panel framework                                        │
│  • Health monitoring                                            │
│  • Metrics/telemetry                                            │
│                                                                 │
│  LICENSE: MIT/Apache-2.0 (Open Source)                          │
└─────────────────────────────────────────────────────────────────┘
                               ↑
                               │ Uses framework
                               │
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 2: PRIVATE FITNESS APPLICATION (pierre-fitness-app)     │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  • Fitness data models (Activity, Athlete, Stats)               │
│  • Intelligence handlers (16,257 lines):                        │
│    - Performance analysis (VO2max, FTP, VDOT, TSS, TRIMP)       │
│    - Training load & recovery                                   │
│    - Goal tracking & recommendations                            │
│    - Nutrition analysis                                         │
│    - Sleep quality analysis                                     │
│    - Weather integration                                        │
│  • Fitness-specific MCP tools                                   │
│  • Fitness configuration profiles                               │
│                                                                 │
│  LICENSE: Proprietary (Closed Source)                           │
└─────────────────────────────────────────────────────────────────┘
                               ↑
                               │ Uses intelligence + models
                               │
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 3: PRIVATE PROVIDER IMPLEMENTATIONS                      │
│           (pierre-fitness-providers)                            │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  • Strava API client & OAuth flow                               │
│  • Garmin API client & OAuth flow                               │
│  • Fitbit API client & OAuth flow (future)                      │
│  • Synthetic test provider                                      │
│                                                                 │
│  LICENSE: Proprietary (Closed Source)                           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Detailed Separation Plan

### LAYER 1: Generic Framework (pierre-framework)

**What Stays PUBLIC:**

#### Core Infrastructure
```
src/
├── admin/                      # Admin panel framework (generic)
├── cache/                      # Cache abstraction (generic)
├── database_plugins/           # Database abstraction (generic)
├── oauth2_server/              # OAuth2 server (generic)
├── errors/                     # Error handling (generic)
├── health.rs                   # Health monitoring (generic)
├── utils/                      # HTTP client, helpers (generic)
├── constants/oauth/            # OAuth constants (generic)
└── context/                    # Dependency injection (generic)
```

#### Protocol Servers
```
src/protocols/
├── mcp/                        # MCP protocol server (generic)
├── a2a/                        # A2A protocol server (generic)
└── universal/
    ├── executor.rs             # Tool executor (generic framework)
    ├── auth_service.rs         # Auth service (generic)
    └── handlers/
        ├── connections.rs      # Generic OAuth connections
        └── configuration.rs    # Generic configuration (MODIFIED)
```

#### Generic Provider SPI
```
src/providers/
├── core.rs                     # GENERIC DataProvider trait (renamed from FitnessProvider)
├── spi.rs                      # GENERIC ProviderDescriptor trait
├── registry.rs                 # GENERIC ProviderRegistry
└── errors.rs                   # Provider errors (generic)
```

**Key Change**: `FitnessProvider` → `DataProvider` (generic trait for ANY data source)

**Framework Capabilities:**
- ✅ Multi-protocol server (MCP, A2A, REST, WebSocket)
- ✅ Multi-tenant authentication & authorization
- ✅ OAuth2 provider & consumer
- ✅ Pluggable data provider architecture
- ✅ Database abstraction (SQLite, PostgreSQL, custom)
- ✅ Caching layer (Redis, in-memory, custom)
- ✅ Health monitoring & metrics
- ✅ Admin panel infrastructure
- ✅ Generic tool registration & execution

---

### LAYER 2: Fitness Application (pierre-fitness-app)

**What Moves to PRIVATE FITNESS APP:**

#### Fitness Domain Models
```
pierre-fitness-app/
├── src/
│   ├── models/                 # MOVE FROM pierre_mcp_server/src/models.rs
│   │   ├── activity.rs         # Activity, SportType, HeartRateZone, etc.
│   │   ├── athlete.rs          # Athlete profile
│   │   ├── stats.rs            # Fitness statistics
│   │   ├── sleep.rs            # Sleep sessions, stages
│   │   ├── recovery.rs         # Recovery metrics
│   │   ├── health.rs           # Health metrics
│   │   └── nutrition.rs        # Nutrition data
│   │
│   ├── intelligence/           # MOVE FROM pierre_mcp_server/src/intelligence/
│   │   ├── mod.rs              # (16,257 lines total)
│   │   ├── activity_analyzer.rs
│   │   ├── performance_analyzer.rs
│   │   ├── performance_analyzer_v2.rs
│   │   ├── recommendation_engine.rs
│   │   ├── goal_engine.rs
│   │   ├── nutrition_calculator.rs
│   │   ├── sleep_analysis.rs
│   │   ├── recovery_calculator.rs
│   │   ├── training_load.rs
│   │   ├── weather.rs
│   │   ├── metrics.rs
│   │   ├── insights.rs
│   │   └── algorithms/
│   │       ├── vo2max.rs       # VO2max estimation
│   │       ├── ftp.rs          # Functional Threshold Power
│   │       ├── vdot.rs         # Running performance
│   │       ├── tss.rs          # Training Stress Score
│   │       ├── trimp.rs        # Training Impulse
│   │       ├── lthr.rs         # Lactate Threshold HR
│   │       └── ...
│   │
│   ├── handlers/               # MOVE FROM pierre_mcp_server/src/protocols/universal/handlers/
│   │   ├── fitness_api.rs      # Fitness provider API handlers
│   │   ├── intelligence.rs     # Intelligence tool handlers
│   │   ├── goals.rs            # Goal tracking handlers
│   │   ├── nutrition.rs        # Nutrition analysis handlers
│   │   └── sleep_recovery.rs   # Sleep/recovery handlers
│   │
│   └── config/
│       └── intelligence_config.rs  # Fitness-specific configuration
│
└── Cargo.toml
    [dependencies]
    pierre-framework = { version = "0.3", features = ["mcp", "a2a"] }
    pierre-fitness-providers = { git = "...", optional = true }
```

**Fitness App Features:**
- ✅ Performance analysis (VO2max, FTP, VDOT, TSS, TRIMP, etc.)
- ✅ Training load & recovery tracking
- ✅ Goal setting & progress tracking
- ✅ Nutrition analysis & meal planning
- ✅ Sleep quality analysis & recommendations
- ✅ Weather-aware training suggestions
- ✅ Pattern detection & insights
- ✅ Personalized recommendations

---

### LAYER 3: Provider Implementations (pierre-fitness-providers)

**Already documented in PRIVATE_PROVIDERS_BUILD.md**

```
pierre-fitness-providers/
├── providers/
│   ├── strava/                 # Strava API client
│   ├── garmin/                 # Garmin API client
│   ├── fitbit/                 # Fitbit API client (future)
│   └── synthetic/              # Test provider
└── Cargo.toml
    [dependencies]
    pierre-framework = { version = "0.3", features = ["provider-spi"] }
    pierre-fitness-app = { version = "0.2" }  # For fitness models
```

---

## Generic Framework SPI (Renamed Traits)

### Current (Fitness-Specific)
```rust
// src/providers/core.rs
pub trait FitnessProvider: Send + Sync {
    async fn get_athlete(&self) -> AppResult<Athlete>;
    async fn get_activities(&self, ...) -> AppResult<Vec<Activity>>;
    async fn get_stats(&self) -> AppResult<Stats>;
    // ... fitness-specific methods
}
```

### Future (Generic)
```rust
// pierre-framework/src/providers/core.rs
pub trait DataProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &'static str;

    /// Get provider configuration
    fn config(&self) -> &ProviderConfig;

    /// Set OAuth2 credentials
    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()>;

    /// Check authentication status
    async fn is_authenticated(&self) -> bool;

    /// Refresh token if needed
    async fn refresh_token_if_needed(&self) -> AppResult<()>;

    /// Generic data fetching (domain-agnostic)
    /// Applications define specific methods via trait extension
    async fn fetch_data(&self, query: DataQuery) -> AppResult<DataResponse>;

    /// Disconnect provider
    async fn disconnect(&self) -> AppResult<()>;
}

/// Generic data query (applications can extend)
pub struct DataQuery {
    pub resource_type: String,
    pub filters: HashMap<String, Value>,
    pub pagination: Option<PaginationParams>,
}

/// Generic data response (applications can extend)
pub struct DataResponse {
    pub data: Vec<Value>,
    pub metadata: HashMap<String, Value>,
}
```

### Fitness App Extension (Private)
```rust
// pierre-fitness-app/src/providers/fitness_provider.rs
use pierre_framework::providers::core::DataProvider;
use crate::models::{Activity, Athlete, Stats};

/// Fitness-specific provider trait (extends generic DataProvider)
#[async_trait]
pub trait FitnessProvider: DataProvider {
    /// Get athlete profile
    async fn get_athlete(&self) -> AppResult<Athlete> {
        let query = DataQuery {
            resource_type: "athlete".to_owned(),
            filters: HashMap::new(),
            pagination: None,
        };
        let response = self.fetch_data(query).await?;
        // Convert generic response to Athlete
        Ok(serde_json::from_value(response.data[0].clone())?)
    }

    /// Get activities
    async fn get_activities(&self, limit: Option<usize>, offset: Option<usize>) -> AppResult<Vec<Activity>>;

    /// Get statistics
    async fn get_stats(&self) -> AppResult<Stats>;

    // ... other fitness-specific methods
}
```

---

## Build Configuration After Separation

### 1. Generic Framework (pierre-framework)

**Cargo.toml:**
```toml
[package]
name = "pierre-framework"
version = "0.3.0"
description = "Generic multi-protocol server framework (MCP, A2A, REST)"
license = "MIT OR Apache-2.0"

[features]
default = ["mcp", "a2a", "rest"]
mcp = []                        # MCP protocol server
a2a = []                        # A2A protocol server
rest = []                       # REST API server
websocket = []                  # WebSocket support
sqlite = []                     # SQLite database
postgresql = ["sqlx/postgres"]  # PostgreSQL database
redis-cache = ["redis"]         # Redis caching
provider-spi = []               # Data provider SPI
oauth2-server = []              # OAuth2 server support
admin-panel = []                # Admin panel infrastructure

[dependencies]
tokio = { version = "1.45", features = ["rt-multi-thread", "macros"] }
axum = { version = "0.7", features = ["ws", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# ... (core dependencies only, no domain-specific ones)
```

**Usage by Applications:**
```toml
# Any application (fitness, finance, IoT, etc.)
[dependencies]
pierre-framework = { version = "0.3", features = ["mcp", "a2a", "sqlite"] }
```

---

### 2. Fitness Application (pierre-fitness-app)

**Cargo.toml:**
```toml
[package]
name = "pierre-fitness-app"
version = "0.2.0"
description = "Fitness intelligence application built on Pierre Framework"
license = "Proprietary"

[features]
default = ["all-providers"]
provider-strava = ["pierre-fitness-providers/strava"]
provider-garmin = ["pierre-fitness-providers/garmin"]
all-providers = ["provider-strava", "provider-garmin"]

[dependencies]
# Generic framework
pierre-framework = { version = "0.3", features = ["mcp", "a2a", "rest", "sqlite", "redis-cache"] }

# Private provider implementations
pierre-fitness-providers = { git = "ssh://git@github.com/Async-IO/pierre-fitness-providers.git", optional = true }

# Fitness-specific dependencies
chrono = { version = "0.4", features = ["serde"] }
# ... (domain-specific dependencies)
```

---

### 3. Final Deployment Build

**Fitness Application Binary:**
```bash
# Build complete fitness application with all providers
cd pierre-fitness-app
cargo build --release

# This pulls:
# 1. pierre-framework (public, from crates.io)
# 2. pierre-fitness-providers (private, from Git)
# 3. pierre-fitness-app code (private)
```

**Custom Application Using Framework:**
```rust
// custom-crm-app/src/main.rs
use pierre_framework::{Server, DataProvider};

#[tokio::main]
async fn main() {
    // Build a CRM application using the same framework
    let server = Server::builder()
        .with_mcp_protocol()
        .with_a2a_protocol()
        .with_database("sqlite://crm.db")
        .register_provider("salesforce", SalesforceProvider::new())
        .build()
        .await
        .unwrap();

    server.run().await.unwrap();
}
```

---

## Migration Impact Analysis

### Lines of Code Movement

| Component | Current Location | Lines | Future Location |
|-----------|-----------------|-------|-----------------|
| **Generic Framework** | `pierre_mcp_server` | ~8,000 | `pierre-framework` (public) |
| **Fitness Models** | `src/models.rs` | ~1,200 | `pierre-fitness-app/src/models/` |
| **Intelligence** | `src/intelligence/` | 16,257 | `pierre-fitness-app/src/intelligence/` |
| **Fitness Handlers** | `src/protocols/universal/handlers/` | ~3,500 | `pierre-fitness-app/src/handlers/` |
| **Provider Impls** | `src/providers/*_provider.rs` | ~5,000 | `pierre-fitness-providers/` |
| **Total Domain-Specific** | - | ~26,000 | **PRIVATE** |
| **Total Generic** | - | ~8,000 | **PUBLIC** |

**Separation Ratio**: 76% domain-specific (private) / 24% framework (public)

---

## Example: Third-Party Application Using Framework

### IoT Sensor Application
```rust
// iot-sensor-app/src/main.rs
use pierre_framework::{Server, DataProvider, ProviderDescriptor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SensorReading {
    sensor_id: String,
    temperature: f64,
    humidity: f64,
    timestamp: DateTime<Utc>,
}

struct IoTProvider { /* ... */ }

#[async_trait]
impl DataProvider for IoTProvider {
    async fn fetch_data(&self, query: DataQuery) -> AppResult<DataResponse> {
        // Fetch from IoT devices
    }
    // ... implement other methods
}

#[tokio::main]
async fn main() {
    let server = Server::builder()
        .with_mcp_protocol()
        .with_rest_api()
        .register_provider("aws-iot", IoTProvider::new())
        .build()
        .await
        .unwrap();

    server.run().await.unwrap();
}
```

### Financial Trading Application
```rust
// trading-app/src/main.rs
use pierre_framework::{Server, DataProvider};

struct AlpacaProvider { /* ... */ }
struct InteractiveBrokersProvider { /* ... */ }

#[tokio::main]
async fn main() {
    let server = Server::builder()
        .with_a2a_protocol()
        .register_provider("alpaca", AlpacaProvider::new())
        .register_provider("interactive-brokers", InteractiveBrokersProvider::new())
        .build()
        .await
        .unwrap();

    server.run().await.unwrap();
}
```

---

## Migration Checklist

### Phase 1: Framework Extraction (2-3 weeks)
- [ ] Create `pierre-framework` repository (public)
- [ ] Move generic infrastructure code
- [ ] Rename `FitnessProvider` → `DataProvider` (generic)
- [ ] Rename `ProviderDescriptor` → generic version
- [ ] Remove fitness-specific dependencies
- [ ] Update protocol servers to be domain-agnostic
- [ ] Publish `pierre-framework` v0.3.0 to crates.io

### Phase 2: Fitness App Extraction (1-2 weeks)
- [ ] Create `pierre-fitness-app` repository (private)
- [ ] Move fitness domain models (`src/models.rs`)
- [ ] Move intelligence layer (`src/intelligence/`)
- [ ] Move fitness handlers (`src/protocols/universal/handlers/`)
- [ ] Update imports to use `pierre-framework`
- [ ] Configure build with provider dependencies

### Phase 3: Provider Separation (Already Documented)
- [ ] Create `pierre-fitness-providers` repository (private)
- [ ] Move provider implementations
- [ ] Configure workspace
- [ ] Update CI/CD credentials

### Phase 4: Documentation & Testing (1 week)
- [ ] Framework documentation for third-party developers
- [ ] Fitness app deployment guide
- [ ] Migration guide for existing deployments
- [ ] Comprehensive integration tests
- [ ] Performance benchmarks

---

## Benefits of Complete Separation

### 1. Open Source Generic Framework
- ✅ Attract third-party developers for non-fitness use cases
- ✅ Community contributions to framework infrastructure
- ✅ Broader adoption (IoT, finance, CRM, etc.)
- ✅ Framework can evolve independently

### 2. Proprietary Fitness Intelligence
- ✅ Protect 16,257 lines of fitness algorithms (VO2max, FTP, etc.)
- ✅ Monetize fitness-specific features
- ✅ Control access to provider implementations
- ✅ Competitive advantage in fitness domain

### 3. Clean Architecture
- ✅ Clear separation of concerns (framework vs. application)
- ✅ Independent versioning (framework v0.3, app v0.2)
- ✅ Reduced coupling
- ✅ Easier maintenance

### 4. Business Flexibility
- ✅ Open-source framework → community growth
- ✅ Closed-source app → revenue generation
- ✅ Multiple applications on same framework
- ✅ Licensing flexibility

---

## Summary: Three-Repository Strategy

```
1. pierre-framework (PUBLIC on crates.io)
   - Generic MCP/A2A/REST server framework
   - Multi-tenant infrastructure
   - Provider SPI
   - ~8,000 lines
   - License: MIT/Apache-2.0

2. pierre-fitness-app (PRIVATE Git repo)
   - Fitness domain models
   - Intelligence algorithms (16,257 lines)
   - Fitness-specific handlers
   - ~21,000 lines
   - License: Proprietary

3. pierre-fitness-providers (PRIVATE Git repo)
   - Strava, Garmin, Fitbit providers
   - ~5,000 lines
   - License: Proprietary
```

**Final Build:**
```bash
cd pierre-fitness-app
cargo build --release  # Pulls framework (public) + providers (private)
```

**Third-Party Build:**
```bash
cd my-custom-app
cargo build --release  # Pulls framework (public) + custom providers
```

This architecture makes Pierre a **true generic framework** while protecting your fitness-specific intellectual property! 🎯
