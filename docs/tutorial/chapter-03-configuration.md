<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 3: Configuration Management & Environment Variables

> **Learning Objectives**: Master environment-driven configuration in Rust, understand type-safe config patterns, and learn how Pierre implements the algorithm selection system.
>
> **Prerequisites**: Chapters 1-2, basic understanding of environment variables
>
> **Estimated Time**: 2-3 hours

---

## Introduction

Production applications require flexible configuration that works across development, staging, and production environments. Pierre uses a multi-layered configuration system:

1. **Environment variables** - Runtime configuration (highest priority)
2. **Type-safe enums** - Compile-time validation of config values
3. **Default values** - Sensible fallbacks for missing configuration
4. **Algorithm selection** - Runtime choice of sports science algorithms

This chapter teaches you how to build configuration systems that are both flexible and type-safe.

---

## Environment Variables with Dotenvy

Pierre uses `dotenvy` to load environment variables from `.envrc` files in development.

### .envrc File Pattern

**Source**: `.envrc.example` (root directory)

```bash
# Database configuration
export DATABASE_URL="sqlite:./data/users.db"
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"

# Server configuration
export HTTP_PORT=8081
export RUST_LOG=info
export JWT_EXPIRY_HOURS=24

# OAuth provider credentials
export STRAVA_CLIENT_ID=your_client_id
export STRAVA_CLIENT_SECRET=your_client_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava

# Algorithm configuration
export PIERRE_MAXHR_ALGORITHM=tanaka
export PIERRE_TSS_ALGORITHM=avg_power
export PIERRE_VDOT_ALGORITHM=daniels
```

**Loading at startup**:

**Source**: `src/bin/pierre-mcp-server.rs` (implicit via dotenvy)

```rust
use crate::errors::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Load .envrc if present (development only)
    dotenvy::dotenv().ok();  // ← Silently ignores if file doesn't exist

    // Parse configuration from environment
    let config = ServerConfig::from_env()?;

    // Rest of initialization...
    Ok(())
}
```

**Rust Idioms Explained**:

1. **`.ok()` to ignore errors**
   - Converts `Result<T, E>` to `Option<T>`
   - Discards error (file not found is okay in production)
   - Production deployments use real env vars, not files

2. **`dotenvy::dotenv()` behavior**
   - Searches for `.env` file in current/parent directories
   - Loads variables into process environment
   - Does NOT override existing env vars (existing take precedence)

**Reference**: [dotenvy crate documentation](https://docs.rs/dotenvy/)

---

## Type-Safe Configuration Enums

Pierre uses enums to represent configuration values, gaining compile-time type safety.

### Loglevel Enum

**Source**: `src/config/environment.rs:25-63`

```rust
/// Strongly typed log level configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Error level - only critical errors
    Error,
    /// Warning level - potential issues
    Warn,
    /// Info level - normal operational messages (default)
    #[default]
    Info,
    /// Debug level - detailed debugging information
    Debug,
    /// Trace level - very verbose tracing
    Trace,
}

impl LogLevel {
    /// Convert to `tracing::Level`
    #[must_use]
    pub const fn to_tracing_level(&self) -> tracing::Level {
        match self {
            Self::Error => tracing::Level::ERROR,
            Self::Warn => tracing::Level::WARN,
            Self::Info => tracing::Level::INFO,
            Self::Debug => tracing::Level::DEBUG,
            Self::Trace => tracing::Level::TRACE,
        }
    }

    /// Parse from string with fallback
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => Self::Error,
            "warn" => Self::Warn,
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            _ => Self::Info, // Default fallback
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}
```

**Rust Idioms Explained**:

1. **`#[derive(Default)]` with `#[default]` variant**
   - New in Rust 1.62+
   - Marks which variant is the default
   - `LogLevel::default()` returns `LogLevel::Info`

2. **`#[serde(rename_all = "lowercase")]`**
   - Serializes `LogLevel::Error` as `"error"` (not `"Error"`)
   - Matches common configuration conventions

3. **`from_str_or_default` pattern**
   - Infallible parsing (never panics)
   - Returns sensible default for invalid input
   - Used throughout Pierre for config parsing

4. **`Display` trait implementation**
   - Allows `format!("{}", log_level)`
   - Converts enum back to string for logging

**Usage example**:

```rust
// Parse from environment variable
let log_level = env::var("RUST_LOG")
    .map(|s| LogLevel::from_str_or_default(&s))
    .unwrap_or_default();  // Falls back to LogLevel::Info

// Convert to tracing level
let tracing_level = log_level.to_tracing_level();

// Use in logger initialization
tracing_subscriber::fmt()
    .with_max_level(tracing_level)
    .init();
```

**Reference**: [Rust Book - Default Trait](https://doc.rust-lang.org/std/default/trait.Default.html)

### Environment Enum (development vs Production)

**Source**: `src/config/environment.rs:73-124`

```rust
/// Environment type for security and other configurations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Development environment (default)
    #[default]
    Development,
    /// Production environment with stricter security
    Production,
    /// Testing environment for automated tests
    Testing,
}

impl Environment {
    /// Parse from string with fallback
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            "testing" | "test" => Self::Testing,
            _ => Self::Development, // Default fallback
        }
    }

    /// Check if this is a production environment
    #[must_use]
    pub const fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }

    /// Check if this is a development environment
    #[must_use]
    pub const fn is_development(&self) -> bool {
        matches!(self, Self::Development)
    }
}
```

**Rust Idioms Explained**:

1. **`matches!` macro** - Pattern matching that returns bool
   - `matches!(value, pattern)` → `true` if matches, `false` otherwise
   - Const fn compatible (can use in const contexts)
   - Cleaner than manual `match` with `true/false` arms

2. **Multiple patterns with `|`**
   - `"production" | "prod"` accepts either string
   - Allows flexibility in configuration values

3. **Helper methods for boolean checks**
   - `is_production()`, `is_development()` provide readable API
   - Enable conditional logic: `if env.is_production() { ... }`

**Usage example**:

```rust
let env = Environment::from_str_or_default(
    &env::var("PIERRE_ENV").unwrap_or_default()
);

// Conditional security settings
if env.is_production() {
    // Enforce HTTPS
    // Enable strict CORS
    // Disable debug endpoints
} else {
    // Allow HTTP for localhost
    // Permissive CORS for development
}
```

**Reference**: [Rust Reference - matches! macro](https://doc.rust-lang.org/std/macro.matches.html)

---

## Database Configuration with Type-Safe Enums

Pierre uses an enum to represent different database types, avoiding string-based type checking.

**Source**: `src/config/environment.rs:126-198`

```rust
/// Type-safe database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseUrl {
    /// SQLite database with file path
    SQLite {
        path: PathBuf,
    },
    /// PostgreSQL connection
    PostgreSQL {
        connection_string: String,
    },
    /// In-memory SQLite (for testing)
    Memory,
}

impl DatabaseUrl {
    /// Parse from string with validation
    pub fn parse_url(s: &str) -> Result<Self> {
        if s.starts_with("sqlite:") {
            let path_str = s.strip_prefix("sqlite:").unwrap_or(s);
            if path_str == ":memory:" {
                Ok(Self::Memory)
            } else {
                Ok(Self::SQLite {
                    path: PathBuf::from(path_str),
                })
            }
        } else if s.starts_with("postgresql://") || s.starts_with("postgres://") {
            Ok(Self::PostgreSQL {
                connection_string: s.to_owned(),
            })
        } else {
            // Fallback: treat as SQLite file path
            Ok(Self::SQLite {
                path: PathBuf::from(s),
            })
        }
    }

    /// Convert to connection string
    #[must_use]
    pub fn to_connection_string(&self) -> String {
        match self {
            Self::SQLite { path } => format!("sqlite:{}", path.display()),
            Self::PostgreSQL { connection_string } => connection_string.clone(),
            Self::Memory => "sqlite::memory:".into(),
        }
    }

    /// Check if this is a SQLite database
    #[must_use]
    pub const fn is_sqlite(&self) -> bool {
        matches!(self, Self::SQLite { .. } | Self::Memory)
    }

    /// Check if this is a PostgreSQL database
    #[must_use]
    pub const fn is_postgresql(&self) -> bool {
        matches!(self, Self::PostgreSQL { .. })
    }
}
```

**Rust Idioms Explained**:

1. **Enum variants with different data**
   - `SQLite { path: PathBuf }` - struct variant with field
   - `PostgreSQL { connection_string: String }` - different struct variant
   - `Memory` - unit variant (no data)

2. **`.strip_prefix()` method**
   - Removes prefix from string if present
   - Returns `Option<&str>` (None if prefix not found)
   - Safer than manual slicing

3. **`.into()` generic conversion**
   - `"sqlite::memory:".into()` converts `&str` → `String`
   - Type inference determines target type
   - Cleaner than explicit `.to_string()` or `.to_owned()`

4. **Pattern matching with `..` (field wildcards)**
   - `Self::SQLite { .. }` matches any SQLite variant
   - Ignores field values (don't care about path here)

**Usage example**:

```rust
// Parse from environment
let db_url = DatabaseUrl::parse_url(&env::var("DATABASE_URL")?)?;

// Type-specific logic
match db_url {
    DatabaseUrl::SQLite { ref path } => {
        println!("Using SQLite: {}", path.display());
        // SQLite-specific initialization
    }
    DatabaseUrl::PostgreSQL { ref connection_string } => {
        println!("Using PostgreSQL: {}", connection_string);
        // PostgreSQL-specific initialization
    }
    DatabaseUrl::Memory => {
        println!("Using in-memory database");
        // Test-only configuration
    }
}
```

**Reference**: [Rust Book - Enum Variants](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)

---

## Algorithm Selection System

Pierre allows runtime selection of sports science algorithms via environment variables.

**Source**: `src/config/intelligence_config.rs:75-133`

```rust
/// Algorithm Selection Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// TSS calculation algorithm: `avg_power`, `normalized_power`, or `hybrid`
    #[serde(default = "default_tss_algorithm")]
    pub tss: String,

    /// Max HR estimation algorithm: `fox`, `tanaka`, `nes`, or `gulati`
    #[serde(default = "default_maxhr_algorithm")]
    pub maxhr: String,

    /// FTP estimation algorithm: `20min_test`, `from_vo2max`, `ramp_test`, etc.
    #[serde(default = "default_ftp_algorithm")]
    pub ftp: String,

    /// LTHR estimation algorithm: `from_maxhr`, `from_30min`, etc.
    #[serde(default = "default_lthr_algorithm")]
    pub lthr: String,

    /// VO2max estimation algorithm: `from_vdot`, `cooper_test`, etc.
    #[serde(default = "default_vo2max_algorithm")]
    pub vo2max: String,
}

/// Default TSS algorithm (`avg_power` for backwards compatibility)
fn default_tss_algorithm() -> String {
    "avg_power".to_owned()
}

/// Default Max HR algorithm (tanaka as most accurate)
fn default_maxhr_algorithm() -> String {
    "tanaka".to_owned()
}

// ... more defaults

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            tss: default_tss_algorithm(),
            maxhr: default_maxhr_algorithm(),
            ftp: default_ftp_algorithm(),
            lthr: default_lthr_algorithm(),
            vo2max: default_vo2max_algorithm(),
        }
    }
}
```

**Rust Idioms Explained**:

1. **`#[serde(default = "function_name")]` attribute**
   - Calls function if field is missing during deserialization
   - Function must have signature `fn() -> T`
   - Each field can have different default function

2. **Default functions pattern**
   - Separate function per default value
   - Allows documentation of why each default was chosen
   - Better than inline values in struct initialization

3. **Manual `Default` implementation**
   - Calls each default function explicitly
   - Could use `#[derive(Default)]`, but manual gives more control
   - Ensures consistency between serde defaults and Default trait

**Configuration via environment**:

```bash
# .envrc
export PIERRE_TSS_ALGORITHM=normalized_power
export PIERRE_MAXHR_ALGORITHM=tanaka
export PIERRE_VDOT_ALGORITHM=daniels
```

**Loading algorithm config**:

```rust
fn load_algorithm_config() -> AlgorithmConfig {
    AlgorithmConfig {
        tss: env::var("PIERRE_TSS_ALGORITHM")
            .unwrap_or_else(|_| default_tss_algorithm()),
        maxhr: env::var("PIERRE_MAXHR_ALGORITHM")
            .unwrap_or_else(|_| default_maxhr_algorithm()),
        // ... other algorithms
    }
}
```

**Algorithm dispatch example**:

**Source**: `src/intelligence/algorithms/maxhr.rs` (conceptual)

```rust
pub fn calculate_max_hr(age: u32, gender: Gender, algorithm: &str) -> u16 {
    match algorithm {
        "fox" => {
            // Fox formula: 220 - age
            220 - age as u16
        }
        "tanaka" => {
            // Tanaka formula: 208 - (0.7 × age)
            (208.0 - (0.7 * age as f64)) as u16
        }
        "nes" => {
            // Nes formula: 211 - (0.64 × age)
            (211.0 - (0.64 * age as f64)) as u16
        }
        "gulati" if matches!(gender, Gender::Female) => {
            // Gulati formula (women): 206 - (0.88 × age)
            (206.0 - (0.88 * age as f64)) as u16
        }
        _ => {
            // Default to Tanaka (most accurate for general population)
            (208.0 - (0.7 * age as f64)) as u16
        }
    }
}
```

**Benefits of algorithm selection**:
- **Scientific accuracy**: Different formulas for different populations
- **Research validation**: Can A/B test algorithms
- **Backwards compatibility**: Can maintain old algorithm while testing new ones
- **User customization**: Advanced users can choose preferred formulas

**Reference**: See `docs/intelligence-methodology.md` for algorithm details

---

## Global Static Configuration with Oncelock

Pierre uses `OnceLock` for global configuration that's initialized once at startup.

**Source**: `src/constants/mod.rs` (conceptual pattern)

```rust
use std::sync::OnceLock;

/// Global server configuration (initialized once at startup)
static SERVER_CONFIG: OnceLock<ServerConfig> = OnceLock::new();

/// Initialize global configuration (call once at startup)
pub fn init_server_config() -> AppResult<()> {
    let config = ServerConfig::from_env()?;
    SERVER_CONFIG.set(config)
        .map_err(|_| AppError::internal("Config already initialized"))?;
    Ok(())
}

/// Get immutable reference to server config (call after init)
pub fn get_server_config() -> &'static ServerConfig {
    SERVER_CONFIG.get()
        .expect("Server config not initialized - call init_server_config() first")
}
```

**Rust Idioms Explained**:

1. **`OnceLock<T>`** - Thread-safe lazy initialization (Rust 1.70+)
   - Can be set exactly once
   - Returns `&'static T` after initialization
   - Replaces older `lazy_static!` macro

2. **Static lifetime `&'static`**
   - Reference valid for entire program duration
   - No need to pass config around everywhere
   - Can be shared across threads safely

3. **Initialization pattern**
   - Call `init_server_config()` once in `main()`
   - All other code calls `get_server_config()`
   - Panics if accessed before initialization (intentional - programming error)

**Usage in binary**:

**Source**: `src/bin/pierre-mcp-server.rs:119`

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize static server configuration
    pierre_mcp_server::constants::init_server_config()?;
    info!("Static server configuration initialized");

    // Rest of application can now use get_server_config()
    bootstrap_server(config).await
}
```

**Accessing global config**:

```rust
use crate::constants::get_server_config;

fn some_function() -> Result<()> {
    let config = get_server_config();
    println!("HTTP port: {}", config.http_port);
    Ok(())
}
```

**When to use global config**:
- ✅ **Read-only configuration** - Never changes after startup
- ✅ **Widely used values** - Accessed from many modules
- ✅ **Performance critical** - Avoid passing around large structs
- ❌ **Mutable state** - Use `Arc<Mutex<T>>` or message passing instead
- ❌ **Request-scoped data** - Use function parameters or context structs

**Reference**: [Rust std::sync::OnceLock](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)

---

## Const Generics for Compile-Time Validation

Pierre uses const generics to track validation state at compile time.

**Source**: `src/config/intelligence_config.rs:135-150`

```rust
/// Main intelligence configuration container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig<const VALIDATED: bool = false> {
    pub recommendation_engine: RecommendationEngineConfig,
    pub performance_analyzer: PerformanceAnalyzerConfig,
    pub goal_engine: GoalEngineConfig,
    // ... more fields
}

impl IntelligenceConfig<false> {
    /// Validate configuration and return validated version
    pub fn validate(self) -> Result<IntelligenceConfig<true>, ConfigError> {
        // Validate all fields
        self.recommendation_engine.validate()?;
        self.performance_analyzer.validate()?;
        // ... more validation

        // Return with VALIDATED = true
        Ok(IntelligenceConfig::<true> {
            recommendation_engine: self.recommendation_engine,
            performance_analyzer: self.performance_analyzer,
            // ... copy all fields
        })
    }
}

// Only validated configs can be used
impl IntelligenceConfig<true> {
    pub fn use_in_production(&self) {
        // Only callable on validated config
    }
}
```

**Rust Idioms Explained**:

1. **Const generic parameter** `<const VALIDATED: bool>`
   - Type parameter with a constant value
   - `IntelligenceConfig<false>` and `IntelligenceConfig<true>` are different types
   - Type system enforces validation

2. **Type-state pattern**
   - Use types to represent state machine states
   - `false` = unvalidated, `true` = validated
   - Compiler prevents using unvalidated config in production

3. **Default const generic** `<const VALIDATED: bool = false>`
   - `IntelligenceConfig` without generic defaults to `<false>`
   - Convenient for API consumers

**Usage example**:

```rust
// Load config (unvalidated)
let config: IntelligenceConfig<false> = load_from_env();

// This would compile-time error (config is unvalidated):
// config.use_in_production();

// Validate config
let validated_config: IntelligenceConfig<true> = config.validate()?;

// Now we can use it (compile-time enforced)
validated_config.use_in_production();
```

**Reference**: [Rust Book - Const Generics](https://doc.rust-lang.org/reference/items/generics.html#const-generics)

---

## Diagram: Configuration Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Configuration Layers                     │
└─────────────────────────────────────────────────────────────┘

                         ┌─────────────────┐
                         │  Binary Launch  │
                         └────────┬────────┘
                                  │
                                  ▼
                  ┌───────────────────────────┐
                  │  1. Load .envrc (dev)     │
                  │     dotenvy::dotenv()     │
                  └───────────┬───────────────┘
                              │
                              ▼
                  ┌───────────────────────────┐
                  │  2. Parse Environment     │
                  │     ServerConfig::from_env()│
                  └───────────┬───────────────┘
                              │
                              ▼
         ┌────────────────────┼────────────────────┐
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌──────────────────┐
│  Type-Safe Enums │  │ Algorithm Config│  │  Database Config │
│  - LogLevel      │  │  - TSS variants │  │  - SQLite/Postgres│
│  - Environment   │  │  - MaxHR variants│  │  - Type-safe URL │
└─────────────────┘  └─────────────────┘  └──────────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                              ▼
                  ┌───────────────────────────┐
                  │  3. Validate Config       │
                  │     IntelligenceConfig    │
                  │     <VALIDATED = true>    │
                  └───────────┬───────────────┘
                              │
                              ▼
                  ┌───────────────────────────┐
                  │  4. Initialize Global     │
                  │     OnceLock::set(config) │
                  └───────────┬───────────────┘
                              │
                              ▼
                  ┌───────────────────────────┐
                  │  5. Application Runtime   │
                  │     get_server_config()   │
                  └───────────────────────────┘
```

---

## Practical Exercises

### Exercise 1: Create a Custom Config Enum

Define a `CacheBackend` enum for cache configuration:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheBackend {
    // TODO: Add variants for:
    // - Memory (in-process LRU cache)
    // - Redis (with connection string)
    // - Disabled (no caching)
}

impl CacheBackend {
    pub fn from_env() -> Self {
        // TODO: Parse from CACHE_BACKEND environment variable
        // Default to Memory if not set
    }
}
```

**Solution**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheBackend {
    Memory,
    Redis { connection_string: String },
    Disabled,
}

impl CacheBackend {
    pub fn from_env() -> Self {
        match env::var("CACHE_BACKEND").ok() {
            Some(s) if s.starts_with("redis://") => Self::Redis {
                connection_string: s,
            },
            Some(s) if s == "disabled" => Self::Disabled,
            _ => Self::Memory,  // Default
        }
    }
}
```

### Exercise 2: Add Validation to Config Struct

Implement validation for algorithm configuration:

```rust
impl AlgorithmConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // TODO: Validate that algorithm names are known
        // Valid TSS algorithms: avg_power, normalized_power, hybrid
        // Valid MaxHR algorithms: fox, tanaka, nes, gulati
    }
}
```

**Solution**:
```rust
impl AlgorithmConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate TSS algorithm
        match self.tss.as_str() {
            "avg_power" | "normalized_power" | "hybrid" => {}
            _ => return Err(ConfigError::InvalidValue {
                field: "tss".to_string(),
                value: self.tss.clone(),
            }),
        }

        // Validate MaxHR algorithm
        match self.maxhr.as_str() {
            "fox" | "tanaka" | "nes" | "gulati" => {}
            _ => return Err(ConfigError::InvalidValue {
                field: "maxhr".to_string(),
                value: self.maxhr.clone(),
            }),
        }

        Ok(())
    }
}
```

---

## Rust Idioms Summary

| Idiom | Purpose | Example Location |
|-------|---------|-----------------|
| **`#[derive(Default)]` with `#[default]`** | Mark default enum variant | `src/config/environment.rs:21` |
| **`#[serde(rename_all = "...")]`** | Customize serialization format | `src/config/environment.rs:20` |
| **`#[serde(default = "function")]`** | Custom default per field | `src/config/intelligence_config.rs:78` |
| **`matches!` macro** | Pattern matching to bool | `src/config/environment.rs:100` |
| **`.strip_prefix()` method** | Safe string prefix removal | `src/config/environment.rs:151` |
| **Enum variants with data** | Different data per variant | `src/config/environment.rs:128-140` |
| **`OnceLock<T>`** | Thread-safe lazy static | `src/constants/mod.rs` |
| **Const generics** | Compile-time state tracking | `src/config/intelligence_config.rs:137` |

**References**:
- [Rust Book - Environment Variables](https://doc.rust-lang.org/book/ch12-05-working-with-environment-variables.html)
- [Serde Documentation](https://serde.rs/)
- [dotenvy crate](https://docs.rs/dotenvy/)

---

## Key Takeaways

1. **Environment variables for flexibility** - Runtime configuration without recompilation
2. **Type-safe enums over strings** - Compiler catches configuration errors
3. **`from_str_or_default` pattern** - Infallible parsing with sensible defaults
4. **Algorithm selection via env vars** - Runtime choice of sports science formulas
5. **`OnceLock` for global config** - Thread-safe lazy initialization
6. **Const generics for validation** - Type-state pattern enforces validation
7. **`#[serde(default)]` for resilience** - Graceful handling of missing fields

---

## Next Chapter

[Chapter 4: Dependency Injection with Context Pattern](./chapter-04-dependency-injection.md) - Learn how Pierre avoids the "AppState" anti-pattern with focused dependency injection contexts.
