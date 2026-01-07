<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 1: Project Architecture & Module Organization

> **Learning Objectives**: Understand the Pierre codebase structure, Rust module system, and how the project is organized for maintainability and scalability.
>
> **Prerequisites**: Basic Rust syntax, familiarity with `cargo` commands
>
> **Estimated Time**: 2-3 hours

---

## Introduction

Pierre Fitness Platform is a production Rust application with 290 source files organized into a coherent module hierarchy. This chapter teaches you how to navigate the codebase, understand the module system, and recognize organizational patterns used throughout.

The codebase follows a **"library + binaries"** pattern where most functionality lives in `src/lib.rs` and binary entry points import from the library.

---

## Project Structure Overview

```
pierre_mcp_server/
├── src/                        # main rust source (library + modules)
│   ├── lib.rs                  # library root - central module hub
│   ├── bin/                    # binary entry points
│   │   ├── pierre-mcp-server.rs   # main server binary
│   │   └── admin_setup.rs         # admin utilities
│   ├── mcp/                    # mcp protocol implementation
│   ├── a2a/                    # agent-to-agent protocol
│   ├── protocols/              # universal protocol layer
│   ├── oauth2_server/          # oauth2 authorization server
│   ├── oauth2_client/          # oauth2 client (for providers)
│   ├── providers/              # fitness provider integrations
│   ├── intelligence/           # sports science algorithms
│   ├── database/               # database repositories (13 focused traits)
│   ├── database_plugins/       # pluggable database backends (SQLite/PostgreSQL)
│   ├── middleware/             # http middleware (auth, tracing, etc)
│   ├── routes/                 # http route handlers
│   └── [30+ other modules]
│
├── sdk/                        # typescript sdk for stdio transport
│   ├── src/
│   │   ├── bridge.ts           # stdio ↔ http bridge (2309 lines)
│   │   ├── types.ts            # auto-generated tool types
│   │   └── secure-storage.ts   # os keychain integration
│   └── test/                   # sdk e2e tests
│
├── tests/                      # integration & e2e tests
│   ├── helpers/
│   │   ├── synthetic_data.rs   # fitness data generator
│   │   └── test_utils.rs       # shared test utilities
│   └── [194 test files]
│
├── scripts/                    # build & utility scripts
│   ├── generate-sdk-types.js   # typescript type generation
│   └── lint-and-test.sh        # ci validation script
│
├── templates/                  # html templates (oauth pages)
├── docs/                       # documentation
└── Cargo.toml                  # rust dependencies & config
```

**Key Observation**: The codebase is split into **library code** (`src/lib.rs`) and **binary code** (`src/bin/`). This is a Rust best practice for testability and reusability.

---

## The Library Root: src/lib.rs

The `src/lib.rs` file is the central hub of the Pierre library. It declares all public modules and controls what's exported to consumers.

### File Header Pattern

**Source**: `src/lib.rs:1-9`

```rust
// ABOUTME: Main library entry point for Pierre fitness API platform
// ABOUTME: Provides MCP, A2A, and REST API protocols for fitness data analysis
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![recursion_limit = "256"]
#![deny(unsafe_code)]
```

**Rust Idioms Explained**:

1. **`// ABOUTME:` comments** - Human-readable file purpose (not rustdoc)
   - Quick context for developers scanning the codebase
   - Appears at top of all 290 source files

2. **Crate-level attributes** `#![...]`
   - `#![recursion_limit = "256"]`: Increases macro recursion limit
     - Required for complex derive macros (serde, thiserror)
     - Default is 128, Pierre uses 256 for deeply nested types

   - `#![deny(unsafe_code)]`: **Zero-tolerance unsafe code policy**
     - Compiler error if `unsafe` block appears anywhere
     - Exception: `src/health.rs` (Windows FFI, approved via validation script)
     - See: `scripts/architectural-validation.sh` for enforcement

**Reference**: [Rust Reference - Crate Attributes](https://doc.rust-lang.org/reference/attributes.html)

### Module Documentation

**Source**: `src/lib.rs:10-55`

```rust
//! # Pierre MCP Server
//!
//! A Model Context Protocol (MCP) server for fitness data aggregation and analysis.
//! This server provides a unified interface to access fitness data from various providers
//! like Strava and Fitbit through the MCP protocol.
//!
//! ## Features
//!
//! - **Multi-provider support**: Connect to Strava, Fitbit, and more
//! - **OAuth2 authentication**: Secure authentication flow for fitness providers
//! - **MCP protocol**: Standard interface for Claude and other AI assistants
//! - **Real-time data**: Access to activities, athlete profiles, and statistics
//! - **Extensible architecture**: Easy to add new fitness providers
//!
//! ## Quick Start
//!
//! 1. Set up authentication credentials using the `auth-setup` binary
//! 2. Start the MCP server with `pierre-mcp-server`
//! 3. Connect from Claude or other MCP clients
//!
//! ## Architecture
//!
//! The server follows a modular architecture:
//! - **Providers**: Abstract fitness provider implementations
//! - **Models**: Common data structures for fitness data
//! - **MCP**: Model Context Protocol server implementation
//! - **OAuth2**: Authentication client for secure API access
//! - **Config**: Configuration management and persistence
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::config::environment::ServerConfig;
//! use pierre_mcp_server::errors::AppResult;
//!
//! #[tokio::main]
//! async fn main() -> AppResult<()> {
//!     // Load configuration
//!     let config = ServerConfig::from_env()?;
//!
//!     // Start Pierre MCP Server with loaded configuration
//!     println!("Pierre MCP Server configured with port: HTTP={}",
//!              config.http_port);
//!
//!     Ok(())
//! }
//! ```
```

**Rust Idioms Explained**:

1. **Module-level docs** `//!` (three slashes + bang)
   - Appears in `cargo doc` output
   - Documents the containing module/crate
   - Markdown formatted for rich documentation

2. **```rust,no_run` code blocks**
   - Syntax highlighted in docs
   - `,no_run` flag: compile-checked but not executed in doc tests
   - Ensures examples stay up-to-date with code

**Reference**: [Rust Book - Documentation Comments](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#making-useful-documentation-comments)

### Module Declarations

**Source**: `src/lib.rs:57-189`

```rust
/// Fitness provider implementations for various services
pub mod providers;

/// Common data models for fitness data
pub mod models;

/// Cursor-based pagination for efficient data traversal
pub mod pagination;

/// Configuration management and persistence
pub mod config;

/// Focused dependency injection contexts
pub mod context;

/// Application constants and configuration values
pub mod constants;

/// OAuth 2.0 client (Pierre as client to fitness providers)
pub mod oauth2_client;

/// Model Context Protocol server implementation
pub mod mcp;

/// Athlete Intelligence for activity analysis and insights
pub mod intelligence;

/// External API clients (USDA, weather services)
pub mod external;

/// Configuration management and runtime parameter system
pub mod configuration;

// ... 30+ more module declarations
```

**Rust Idioms Explained**:

1. **`pub mod` declarations**
   - Makes module public to external crates
   - Each `pub mod foo;` looks for:
     - `src/foo.rs` (single-file module), OR
     - `src/foo/mod.rs` (directory module)

2. **Documentation comments** `///` (three slashes)
   - Documents the item below (not the containing module)
   - Brief one-line summaries for each module
   - Visible in IDE tooltips and `cargo doc`

3. **Module ordering** - Logical grouping:
   - Core domain (providers, models, pagination)
   - Configuration (config, context, constants)
   - Protocols (oauth2_client, mcp, a2a, protocols)
   - Data layer (database, database_plugins, cache)
   - Infrastructure (auth, crypto, routes, middleware)
   - Features (intelligence, external, plugins)
   - Utilities (types, utils, test_utils)

**Reference**: [Rust Book - Modules](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)

### Conditional Compilation

**Source**: `src/lib.rs:188-189`

```rust
/// Test utilities for creating consistent test data
#[cfg(any(test, feature = "testing"))]
pub mod test_utils;
```

**Rust Idioms Explained**:

1. **`#[cfg(...)]` attribute**
   - Conditional compilation based on configuration
   - Code only included if conditions are met

2. **`#[cfg(any(test, feature = "testing"))]`**
   - **`test`**: Built-in flag when running `cargo test`
   - **`feature = "testing"`**: Custom feature flag from `Cargo.toml:47`
   - **`any(...)`**: Include if ANY condition is true

3. **Why use this?**
   - `test_utils` module only needed during testing
   - Excluded from production binary (reduces binary size)
   - Can be enabled in other crates via `features = ["testing"]`

**Cargo.toml configuration**:

```toml
[features]
default = ["sqlite"]
sqlite = []
postgresql = ["sqlx/postgres"]
testing = []  # Feature flag for test utilities
```

**Reference**: [Rust Book - Conditional Compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)

---

## Binary Entry Points: src/bin/

Rust crates can define multiple binary targets. Pierre has two main binaries:

### Main Server Binary

**Source**: `src/bin/pierre-mcp-server.rs:1-61`

```rust
// ABOUTME: Server implementation for serving users with isolated data access
// ABOUTME: Production-ready server with authentication and user isolation capabilities

#![recursion_limit = "256"]
#![deny(unsafe_code)]

//! # Pierre Fitness API Server Binary
//!
//! This binary starts the multi-protocol Pierre Fitness API with user authentication,
//! secure token storage, and database management.

use anyhow::Result;
use clap::Parser;
use pierre_mcp_server::{
    config::environment::{ServerConfig, TokioRuntimeConfig},
    database_plugins::factory::Database,
    mcp::{multitenant::MultiTenantMcpServer, resources::ServerResources},
    // ... other imports
};
use tokio::runtime::{Builder, Runtime};

/// Command-line arguments for the Pierre MCP server
#[derive(Parser)]
#[command(name = "pierre-mcp-server")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Pierre Fitness API - Multi-protocol fitness data API for LLMs")]
pub struct Args {
    /// Configuration file path for providers
    #[arg(short, long)]
    config: Option<String>,

    /// Override HTTP port
    #[arg(long)]
    http_port: Option<u16>,
}

fn main() -> Result<()> {
    let args = parse_args_or_default();

    // Load runtime config first to build the Tokio runtime
    let runtime_config = TokioRuntimeConfig::from_env();
    let runtime = build_tokio_runtime(&runtime_config)?;

    // Run the async server on our configured runtime
    runtime.block_on(async {
        let config = setup_configuration(&args)?;
        bootstrap_server(config).await
    })
}
```

**Rust Idioms Explained**:

1. **Binary crate attributes** - Same as library (`#![...]`)
   - Each binary can have its own attributes
   - Often mirrors library settings

2. **`use pierre_mcp_server::...`** - Importing from library
   - Binary depends on library crate
   - Imports only what's needed
   - Absolute paths from crate root

3. **`clap::Parser` derive macro**
   - Auto-generates CLI argument parser
   - `#[command(...)]` attributes for metadata
   - `#[arg(...)]` attributes for options
   - Generates `--help` automatically

4. **Manual Tokio runtime building**
   - Pierre uses `TokioRuntimeConfig::from_env()` for configurable runtime
   - Worker threads and stack size configurable via environment
   - More control than `#[tokio::main]` macro:

```rust
// Pierre's configurable runtime builder
fn build_tokio_runtime(config: &TokioRuntimeConfig) -> Result<Runtime> {
    let mut builder = Builder::new_multi_thread();
    if let Some(workers) = config.worker_threads {
        builder.worker_threads(workers);
    }
    builder.enable_all().build().map_err(Into::into)
}
```

**Reference**:
- [Rust Book - Separating Concerns with Binary Crates](https://doc.rust-lang.org/book/ch12-03-improving-error-handling-and-modularity.html)
- [Tokio - Runtime](https://tokio.rs/tokio/topics/runtime)

### Cargo.toml Binary Declarations

**Source**: `Cargo.toml:14-29`

```toml
[lib]
name = "pierre_mcp_server"
path = "src/lib.rs"

[[bin]]
name = "pierre-mcp-server"
path = "src/bin/pierre-mcp-server.rs"

[[bin]]
name = "admin-setup"
path = "src/bin/admin_setup.rs"

[[bin]]
name = "diagnose-weather-api"
path = "src/bin/diagnose_weather_api.rs"
```

**Explanation**:
- `[lib]`: Single library target
- `[[bin]]`: Multiple binary targets (double brackets = array)
- Binary names can differ from file names (kebab-case vs snake_case)

**Build commands**:
```bash
# Build all binaries
cargo build --release

# Run specific binary
cargo run --bin pierre-mcp-server

# Install binary to ~/.cargo/bin
cargo install --path . --bin pierre-mcp-server
```

---

## Module Organization Patterns

Pierre uses several module organization patterns consistently.

### Single-File Modules

**Example**: `src/errors.rs`

```
src/
├── lib.rs          # Contains: pub mod errors;
└── errors.rs       # The module implementation
```

When module fits in one file (~100-500 lines), use single-file pattern.

### Directory Modules

**Example**: `src/mcp/` directory

```
src/
├── lib.rs                  # Contains: pub mod mcp;
└── mcp/
    ├── mod.rs              # Module root, declares submodules
    ├── protocol.rs         # Submodule
    ├── tool_handlers.rs    # Submodule
    ├── multitenant.rs      # Submodule
    └── [8 more files]
```

**Source**: `src/mcp/mod.rs:1-40`

```rust
// ABOUTME: MCP (Model Context Protocol) server implementation
// ABOUTME: JSON-RPC 2.0 protocol for AI assistant tool execution

//! MCP Protocol Implementation
//!
//! This module implements the Model Context Protocol (MCP) for AI assistant integration.
//! MCP is a JSON-RPC 2.0 based protocol that enables AI assistants like Claude to execute
//! tools and access resources from external services.

// Submodule declarations
pub mod protocol;
pub mod tool_handlers;
pub mod multitenant;
pub mod resources;
pub mod tenant_isolation;
pub mod oauth_flow_manager;
pub mod transport_manager;
pub mod mcp_request_processor;
pub mod server_lifecycle;
pub mod progress;
pub mod schema;

// Re-exports for convenience
pub use multitenant::MultiTenantMcpServer;
pub use resources::ServerResources;
pub use protocol::{McpRequest, McpResponse};
```

**Rust Idioms Explained**:

1. **`mod.rs` convention**
   - Directory modules need a `mod.rs` file
   - Acts as the "index" file for the directory
   - Declares and organizes submodules

2. **Re-exports** `pub use ...`
   - Makes deeply nested types accessible at module root
   - Users can write `use pierre_mcp_server::mcp::MultiTenantMcpServer`
   - Instead of `use pierre_mcp_server::mcp::multitenant::MultiTenantMcpServer`

3. **Submodule visibility**
   - `pub mod` makes submodule public
   - `mod` (without `pub`) keeps it private to parent module
   - All Pierre submodules are public for flexibility

**Reference**: [Rust Book - Separating Modules into Different Files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html)

### Nested Directory Modules

**Example**: `src/protocols/universal/handlers/`

```
src/
└── protocols/
    ├── mod.rs
    └── universal/
        ├── mod.rs
        └── handlers/
            ├── mod.rs
            ├── strava_api.rs
            ├── intelligence.rs
            ├── goals.rs
            ├── configuration.rs
            ├── sleep_recovery.rs
            ├── nutrition.rs
            └── connections.rs
```

**Source**: `src/protocols/universal/handlers/mod.rs`

```rust
//! MCP tool handlers for all tool categories

pub mod strava_api;
pub mod intelligence;
pub mod goals;
pub mod configuration;
pub mod sleep_recovery;
pub mod nutrition;
pub mod connections;

// Re-export all handler functions
pub use strava_api::*;
pub use intelligence::*;
pub use goals::*;
pub use configuration::*;
pub use sleep_recovery::*;
pub use nutrition::*;
pub use connections::*;
```

**Pattern**: Deep hierarchies use `mod.rs` at each level to organize related functionality.

---

## Feature Flags & Conditional Compilation

Pierre uses feature flags for optional dependencies and database backends.

**Source**: `Cargo.toml:42-47`

```toml
[features]
default = ["sqlite"]
sqlite = []
postgresql = ["sqlx/postgres"]
testing = []
telemetry = []
```

### Feature Flag Usage

**1. Default features**

```toml
default = ["sqlite"]
```

Builds with SQLite by default. Users can opt out:
```bash
cargo build --no-default-features
```

**2. Database backend selection**

**Source**: `src/database_plugins/factory.rs:30-50`

```rust
pub async fn new(
    connection_string: &str,
    encryption_key: Vec<u8>,
    #[cfg(feature = "postgresql")]
    postgres_pool_config: &PostgresPoolConfig,
) -> Result<Self, DatabaseError> {
    #[cfg(feature = "sqlite")]
    if connection_string.starts_with("sqlite:") {
        let sqlite_db = SqliteDatabase::new(connection_string, encryption_key).await?;
        return Ok(Database::Sqlite(sqlite_db));
    }

    #[cfg(feature = "postgresql")]
    if connection_string.starts_with("postgres://") || connection_string.starts_with("postgresql://") {
        let postgres_db = PostgresDatabase::new(
            connection_string,
            encryption_key,
            postgres_pool_config,
        ).await?;
        return Ok(Database::Postgres(postgres_db));
    }

    Err(DatabaseError::ConfigurationError(
        "Unsupported database type in connection string".to_string()
    ))
}
```

**Rust Idioms Explained**:

1. **`#[cfg(feature = "...")]`**
   - Code only compiled if feature is enabled
   - `sqlite` feature compiles SQLite code
   - `postgresql` feature compiles PostgreSQL code

2. **Function parameter attributes**
   ```rust
   #[cfg(feature = "postgresql")]
   postgres_pool_config: &PostgresPoolConfig,
   ```
   - Parameter only exists if feature is enabled
   - Type checking happens only when feature is active

3. **Build commands**:
   ```bash
   # SQLite (default)
   cargo build

   # PostgreSQL
   cargo build --no-default-features --features postgresql

   # Both
   cargo build --features postgresql
   ```

**Reference**: [Cargo Book - Features](https://doc.rust-lang.org/cargo/reference/features.html)

---

## Documentation Patterns

Pierre follows consistent documentation practices across all 287 source files.

### Dual-Comment Pattern

Every file has both `// ABOUTME:` and `//!` comments:

```rust
// ABOUTME: Brief human-readable purpose
// ABOUTME: Additional context
//
// License header

//! # Module Title
//!
//! Detailed rustdoc documentation
//! with markdown formatting
```

**Benefits**:
- `// ABOUTME:`: Quick context when browsing files (shows in editors)
- `//!`: Full documentation for `cargo doc` output
- Separation of concerns: quick ref vs comprehensive docs

### Rustdoc Formatting

**Source**: `src/mcp/protocol.rs:1-30`

```rust
//! # MCP Protocol Implementation
//!
//! Core protocol handlers for the Model Context Protocol (MCP).
//! Implements JSON-RPC 2.0 message handling and tool execution.
//!
//! ## Supported Methods
//!
//! - `initialize`: Protocol version negotiation
//! - `tools/list`: List available tools
//! - `tools/call`: Execute a tool
//! - `resources/list`: List available resources
//!
//! ## Example
//!
//! ```rust,ignore
//! use pierre_mcp_server::mcp::protocol::handle_mcp_request;
//!
//! let response = handle_mcp_request(request).await?;
//! ```
```

**Markdown features**:
- `#` headers (h1, h2, h3)
- `- ` bullet lists
- `` ` `` inline code
- ` ```rust ` code blocks
- `**bold**` and `*italic*`

**Generate docs**:
```bash
cargo doc --open  # Generate & open in browser
```

**Reference**: [Rust Book - Documentation](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html)

---

## Import Conventions

Pierre follows consistent import patterns for clarity.

### Absolute vs Relative Imports

**Preferred**: Absolute imports from crate root

```rust
use crate::errors::AppError;
use crate::database::DatabaseError;
use crate::providers::ProviderError;
```

**Avoid**: Relative imports

```rust
// Don't do this
use super::super::errors::AppError;
use ../database::DatabaseError;  // Not valid Rust
```

**Exception**: Sibling modules in same directory can use `super::`

```rust
// In src/protocols/universal/handlers/goals.rs
use super::super::executor::UniversalToolExecutor;  // Acceptable
use crate::protocols::universal::executor::UniversalToolExecutor;  // Better
```

### Grouping Imports

**Source**: `src/bin/pierre-mcp-server.rs:15-24`

```rust
// Group 1: External crates
use clap::Parser;
use std::sync::Arc;
use tracing::{error, info};

// Group 2: Internal crate imports
use pierre_mcp_server::{
    auth::AuthManager,
    cache::factory::Cache,
    config::environment::ServerConfig,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppResult,
    logging,
    mcp::{multitenant::MultiTenantMcpServer, resources::ServerResources},
};
```

**Convention**:
1. External dependencies (`clap`, `std`, `tracing`)
2. Internal crate (`pierre_mcp_server::...`)
3. Blank line between groups

**Reference**: [Rust Style Guide](https://github.com/rust-lang/rust/blob/master/src/doc/style-guide/src/cargo.md)

---

## Navigating the Codebase

### Finding Functionality

**Strategy 1**: Start from `src/lib.rs` module declarations
- Each `pub mod` has a one-line summary
- Navigate to module's `mod.rs` for details

**Strategy 2**: Use `grep` or IDE search
```bash
# Find all files containing "OAuth"
grep -r "OAuth" src/

# Find struct definitions
grep -r "pub struct" src/
```

**Strategy 3**: Follow imports
- Open a file, read its `use` statements
- Imports show dependencies and related modules

### Understanding Module Responsibilities

**MCP Protocol** (`src/mcp/`):
- **protocol.rs**: Core JSON-RPC handlers
- **tool_handlers.rs**: Tool execution routing
- **multitenant.rs**: Multi-tenant server wrapper
- **resources.rs**: Shared server resources

**Protocols Layer** (`src/protocols/universal/`):
- **tool_registry.rs**: Type-safe tool routing
- **executor.rs**: Tool execution engine
- **handlers/**: Business logic for each tool

**Database** (`src/database_plugins/`):
- **factory.rs**: Database abstraction
- **sqlite.rs**: SQLite implementation
- **postgres.rs**: PostgreSQL implementation

---

## Diagram: Module Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                         src/lib.rs                          │
│                    (central module hub)                     │
└─────────────┬────────────────────────┬──────────────────────┘
              │                        │
     ┌────────▼────────┐      ┌───────▼────────┐
     │  src/bin/       │      │  Public Modules │
     │  - main server  │      │  (40+ modules)  │
     │  - admin tools  │      └───────┬─────────┘
     └─────────────────┘              │
                                      │
              ┌───────────────────────┼──────────────────────┐
              │                       │                      │
     ┌────────▼────────┐   ┌─────────▼────────┐  ┌─────────▼────────┐
     │   Protocols     │   │   Data Layer     │  │  Infrastructure  │
     │  - mcp/         │   │  - database/     │  │  - auth          │
     │  - a2a/         │   │  - database_     │  │  - middleware    │
     │  - protocols/   │   │    plugins/      │  │  - routes        │
     │  - jsonrpc/     │   │  - cache/        │  │  - logging       │
     └─────────────────┘   └──────────────────┘  └──────────────────┘
              │                       │                      │
              │                       │                      │
     ┌────────▼────────────────┐     │         ┌────────────▼──────────┐
     │   Domain Logic          │     │         │   External Services   │
     │  - providers/           │     │         │  - oauth2_client      │
     │  - intelligence/        │     │         │  - oauth2_server      │
     │  - configuration/       │     │         │  - external/          │
     └─────────────────────────┘     │         └───────────────────────┘
                                     │
                        ┌────────────▼──────────┐
                        │   Shared Utilities    │
                        │  - models             │
                        │  - errors             │
                        │  - types              │
                        │  - utils              │
                        │  - constants          │
                        └───────────────────────┘
```

**Key Observations**:
- **lib.rs** is the hub connecting all modules
- **Protocols** layer is protocol-agnostic (shared by MCP & A2A)
- **Data layer** is abstracted (pluggable backends)
- **Infrastructure** is cross-cutting (auth, middleware, logging)
- **Domain logic** is isolated (providers, intelligence)

---

## Practical Exercises

### Exercise 1: Explore Module Structure

1. Open `src/lib.rs` and count the `pub mod` declarations
2. For each protocol module (`mcp`, `a2a`, `protocols`), open its `mod.rs`
3. Draw a mental map of 3-level deep module hierarchy

**Expected output**: Understanding of how modules nest and relate

### Exercise 2: Trace an Import Path

1. Open `src/bin/pierre-mcp-server.rs`
2. Find the import: `use pierre_mcp_server::mcp::multitenant::MultiTenantMcpServer`
3. Navigate the path:
   - `src/lib.rs` → `pub mod mcp;`
   - `src/mcp/mod.rs` → `pub mod multitenant;`
   - `src/mcp/multitenant.rs` → `pub struct MultiTenantMcpServer`

**Expected output**: Comfortable navigating nested modules

### Exercise 3: Identify Feature Flags

1. Search `Cargo.toml` for `[features]` section
2. Find all `#[cfg(feature = "...")]` in `src/database_plugins/factory.rs`
3. Run build with different features:
   ```bash
   cargo build --no-default-features --features postgresql
   ```

**Expected output**: Understanding conditional compilation

---

## Rust Idioms Summary

| Idiom | Purpose | Example Location |
|-------|---------|-----------------|
| **Crate attributes** `#![...]` | Set compiler flags/limits | `src/lib.rs:7-8` |
| **Module docs** `//!` | Document containing module | All `mod.rs` files |
| **Item docs** `///` | Document following item | `src/lib.rs:58+` |
| **`pub mod`** | Public module declaration | `src/lib.rs:57-189` |
| **Re-exports** `pub use` | Convenience exports | `src/mcp/mod.rs:24-26` |
| **Feature flags** `#[cfg(...)]` | Conditional compilation | `src/database_plugins/factory.rs` |
| **Binary targets** `[[bin]]` | Multiple executables | `Cargo.toml:15-29` |

**References**:
- [Rust Book - Packages and Crates](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html)
- [Rust Reference - Attributes](https://doc.rust-lang.org/reference/attributes.html)
- [Cargo Book - Targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html)

---

## Key Takeaways

1. **Library + Binaries Pattern**: Core logic in `lib.rs`, entry points in `bin/`
2. **Module Hierarchy**: Use `pub mod` in parent, `mod.rs` for directory modules
3. **Dual Documentation**: `// ABOUTME:` for humans, `//!` for rustdoc
4. **Feature Flags**: Enable optional functionality (`sqlite`, `postgresql`, `testing`)
5. **Import Conventions**: Absolute paths from `crate::`, grouped by origin
6. **Zero Unsafe Code**: `#![deny(unsafe_code)]` enforced via CI

---

## Next Chapter

[Chapter 2: Error Handling & Type-Safe Errors](./chapter-02-error-handling.md) - Learn how Pierre uses `thiserror` for structured error types and eliminates `anyhow!` from production code.
