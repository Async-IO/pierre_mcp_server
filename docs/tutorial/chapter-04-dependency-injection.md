<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 4: Dependency Injection with Context Pattern

> **Learning Objectives**: Understand dependency injection in Rust using Arc<T>, learn the service locator anti-pattern and how Pierre is evolving toward focused contexts, master shared ownership patterns.
>
> **Prerequisites**: Chapters 1-3, understanding of ownership and borrowing
>
> **Estimated Time**: 3-4 hours

---

## Introduction

Rust's ownership system makes dependency injection (DI) different from languages with garbage collection. You can't just pass references everywhere - you need to think about lifetimes and ownership.

Pierre uses **Arc<T>** (Atomic Reference Counting) for dependency injection, allowing shared ownership of expensive resources across threads.

**Key concepts**:
- **Dependency Injection**: Providing dependencies to a struct rather than creating them internally
- **Arc<T>**: Thread-safe reference-counted smart pointer
- **Service Locator**: Anti-pattern where a single struct holds all dependencies
- **Focused Contexts**: Better pattern with separate contexts for different domains

---

## The Problem: Expensive Resource Creation

Consider what happens without dependency injection:

```rust
// ANTI-PATTERN: Creating expensive resources repeatedly
async fn handle_request(user_id: &str) -> Result<Response> {
    // Creates new database connection (expensive!)
    let database = Database::new(&config.database_url).await?;

    // Creates new auth manager (unnecessary!)
    let auth_manager = AuthManager::new(24);

    // Use them...
    let user = database.get_user(user_id).await?;
    let token = auth_manager.create_token(&user)?;

    Ok(response)
}
```

**Problems**:
1. **Performance**: Database connection pool created per request
2. **Resource exhaustion**: Each connection uses memory/file descriptors
3. **Configuration duplication**: Same config loaded repeatedly
4. **No sharing**: Can't share state (caches, metrics) between requests

---

## Solution 1: Dependency Injection with Arc\<T\>

Arc (Atomic Reference Counting) enables shared ownership across threads.

### Arc Basics

```rust
use std::sync::Arc;

// Create an expensive resource once
let database = Arc::new(Database::new(&config).await?);

// Clone the Arc (cheap - just increments counter)
let db_clone = Arc::clone(&database);  // Or database.clone()

// Both point to the same underlying Database
// When last Arc is dropped, Database is dropped
```

**Rust Idioms Explained**:

1. **`Arc::new(value)`** - Wrap value in atomic reference counter
   - Allocates on heap
   - Returns `Arc<T>`
   - Thread-safe (uses atomic operations)

2. **`Arc::clone(&arc)` vs `.clone()`**
   - Both do the same thing (increment counter)
   - `Arc::clone` makes it explicit (recommended in docs)
   - `.clone()` is shorter (common in Pierre)

3. **Drop semantics**
   - Each `Arc::clone()` increments counter
   - Each drop decrements counter
   - When counter reaches 0, inner value is dropped

4. **Cost**
   - Creating Arc: One heap allocation
   - Cloning Arc: Increment atomic counter (~1-2 CPU instructions)
   - Accessing data: No overhead (just deref)

**Reference**: [Rust Book - Arc](https://doc.rust-lang.org/book/ch16-03-shared-state.html#atomic-reference-counting-with-arct)

### Dependency Injection Example

```rust
use std::sync::Arc;

// 1. Create expensive resources once at startup
#[tokio::main]
async fn main() -> Result<()> {
    let database = Arc::new(Database::new(&config).await?);
    let auth_manager = Arc::new(AuthManager::new(24));

    // 2. Pass to HTTP handlers via Axum state
    let app = Router::new()
        .route("/users/:id", get(get_user_handler))
        .with_state(AppState { database, auth_manager });

    // 3. Listen for requests
    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

// Handler receives dependencies via State extractor
async fn get_user_handler(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<User>, AppError> {
    // database and auth_manager are Arc clones (cheap)
    let user = state.database.get_user(&user_id).await?;
    let token = state.auth_manager.create_token(&user)?;
    Ok(Json(user))
}

#[derive(Clone)]
struct AppState {
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
}
```

**Pattern**:
- Create once → Wrap in Arc → Share via cloning Arc

**Reference**: [Axum - Sharing State](https://docs.rs/axum/latest/axum/extract/struct.State.html)

---

## Serverresources: Centralized Dependency Container

Pierre uses `ServerResources` as a central container for all dependencies.

**Source**: `src/mcp/resources.rs:35-77`

```rust
/// Centralized resource container for dependency injection
#[derive(Clone)]
pub struct ServerResources {
    /// Database connection pool for persistent storage operations
    pub database: Arc<Database>,
    /// Authentication manager for user identity verification
    pub auth_manager: Arc<AuthManager>,
    /// JSON Web Key Set manager for RS256 JWT signing and verification
    pub jwks_manager: Arc<JwksManager>,
    /// Authentication middleware for MCP request validation
    pub auth_middleware: Arc<McpAuthMiddleware>,
    /// WebSocket connection manager for real-time updates
    pub websocket_manager: Arc<WebSocketManager>,
    /// Server-Sent Events manager for streaming notifications
    pub sse_manager: Arc<crate::sse::SseManager>,
    /// OAuth client for multi-tenant authentication flows
    pub tenant_oauth_client: Arc<TenantOAuthClient>,
    /// Registry of fitness data providers (Strava, Fitbit, Garmin, WHOOP, Terra)
    pub provider_registry: Arc<ProviderRegistry>,
    /// Secret key for admin JWT token generation
    pub admin_jwt_secret: Arc<str>,
    /// Server configuration loaded from environment
    pub config: Arc<crate::config::environment::ServerConfig>,
    /// AI-powered fitness activity analysis engine
    pub activity_intelligence: Arc<ActivityIntelligence>,
    /// A2A protocol client manager
    pub a2a_client_manager: Arc<A2AClientManager>,
    /// Service for managing A2A system user accounts
    pub a2a_system_user_service: Arc<A2ASystemUserService>,
    /// Broadcast channel for OAuth completion notifications
    pub oauth_notification_sender: Option<broadcast::Sender<OAuthCompletedNotification>>,
    /// Cache layer for performance optimization
    pub cache: Arc<Cache>,
    /// Optional plugin executor for custom tool implementations
    pub plugin_executor: Option<Arc<PluginToolExecutor>>,
    /// Configuration for PII redaction in logs and responses
    pub redaction_config: Arc<RedactionConfig>,
    /// Rate limiter for OAuth2 endpoints
    pub oauth2_rate_limiter: Arc<crate::oauth2_server::rate_limiting::OAuth2RateLimiter>,
}
```

**Rust Idioms Explained**:

1. **`#[derive(Clone)]`** on struct with `Arc` fields
   - Cloning `ServerResources` clones all the `Arc`s (cheap)
   - Does NOT clone underlying data (Database, AuthManager, etc.)
   - Enables passing resources around without lifetime parameters

2. **`Arc<str>` for string secrets**
   - More memory efficient than `Arc<String>`
   - Immutable (strings never change)
   - Implements `AsRef<str>` for easy access

3. **`Option<Arc<T>>` for optional dependencies**
   - `plugin_executor` may not be initialized
   - `None` means feature disabled
   - `Some(Arc<...>)` when enabled

### Creating Serverresources

**Source**: `src/mcp/resources.rs:85-150`

```rust
impl ServerResources {
    pub fn new(
        database: Database,
        auth_manager: AuthManager,
        admin_jwt_secret: &str,
        config: Arc<crate::config::environment::ServerConfig>,
        cache: Cache,
        rsa_key_size_bits: usize,
        jwks_manager: Option<Arc<JwksManager>>,
    ) -> Self {
        // Wrap expensive resources in Arc once
        let database_arc = Arc::new(database);
        let auth_manager_arc = Arc::new(auth_manager);

        // Create dependent resources
        let tenant_oauth_client = Arc::new(TenantOAuthClient::new(
            TenantOAuthManager::new(Arc::new(config.oauth.clone()))
        ));
        let provider_registry = Arc::new(ProviderRegistry::new());

        // Create intelligence engine
        let activity_intelligence = Self::create_default_intelligence();

        // Create A2A components
        let a2a_system_user_service = Arc::new(
            A2ASystemUserService::new(database_arc.clone())
        );
        let a2a_client_manager = Arc::new(A2AClientManager::new(
            database_arc.clone(),
            a2a_system_user_service.clone(),
        ));

        // Wrap cache
        let cache_arc = Arc::new(cache);

        // Load or create JWKS manager
        let jwks_manager_arc = jwks_manager.unwrap_or_else(|| {
            // Load from database or create new
            // ... (initialization logic)
            Arc::new(new_jwks)
        });

        Self {
            database: database_arc,
            auth_manager: auth_manager_arc,
            jwks_manager: jwks_manager_arc,
            tenant_oauth_client,
            provider_registry,
            // ... all other fields
        }
    }
}
```

**Pattern observations**:

1. **Accept owned values** (`database: Database`)
   - Not `Arc<Database>` in parameters
   - Caller doesn't need to know about Arc
   - `new()` wraps in Arc internally

2. **Return `Self` (not `Arc<Self>`)**
   - Caller decides if they need Arc
   - Typical usage: `Arc::new(ServerResources::new(...))`

3. **`.clone()` on Arc is explicit**
   - Shows resource sharing happening
   - Comments explain why (see line 9 note about "Safe" clones)

### Using Serverresources

**Source**: `src/bin/pierre-mcp-server.rs:182-220`

```rust
fn create_server(
    database: Database,
    auth_manager: AuthManager,
    jwt_secret: &str,
    config: &ServerConfig,
    cache: Cache,
) -> MultiTenantMcpServer {
    let rsa_key_size = get_rsa_key_size();

    // Create resources (wraps everything in Arc)
    let mut resources_instance = ServerResources::new(
        database,
        auth_manager,
        jwt_secret,
        Arc::new(config.clone()),
        cache,
        rsa_key_size,
        None,  // Generate new JWKS
    );

    // Wrap in Arc for sharing
    let resources_arc = Arc::new(resources_instance.clone());

    // Initialize plugin system (needs Arc<ServerResources>)
    let plugin_executor = PluginToolExecutor::new(resources_arc);

    // Set plugin executor back on resources
    resources_instance.set_plugin_executor(Arc::new(plugin_executor));

    // Final Arc wrapping
    let resources = Arc::new(resources_instance);

    // Create server with resources
    MultiTenantMcpServer::new(resources)
}
```

**Pattern**: Create → Arc wrap → Share → Modify → Re-wrap

---

## The Service Locator Anti-Pattern

While `ServerResources` works, it's a **service locator anti-pattern**.

**Problems with service locator**:

1. **God object** - Single struct knows about everything
2. **Hidden dependencies** - Functions take `ServerResources` but only use 1-2 fields
3. **Testing complexity** - Must mock entire `ServerResources` even for simple tests
4. **Tight coupling** - Adding new dependency requires changing one big struct
5. **Unclear requirements** - Can't tell from signature what function needs

**Example of the problem**:

```rust
// What does this function actually need?
async fn process_activity(
    resources: &ServerResources,
    activity_id: &str,
) -> Result<ProcessedActivity> {
    // Uses only database and intelligence
    let activity = resources.database.get_activity(activity_id).await?;
    let analysis = resources.activity_intelligence.analyze(&activity)?;
    Ok(analysis)
}

// Better: explicit dependencies
async fn process_activity(
    database: &Database,
    intelligence: &ActivityIntelligence,
    activity_id: &str,
) -> Result<ProcessedActivity> {
    // Clear what's needed!
    let activity = database.get_activity(activity_id).await?;
    let analysis = intelligence.analyze(&activity)?;
    Ok(analysis)
}
```

**Reference**: [Service Locator Anti-Pattern](https://blog.ploeh.dk/2010/02/03/ServiceLocatorisanAnti-Pattern/)

---

## Solution 2: Focused Context Pattern

Pierre is evolving toward focused contexts that group related dependencies.

**Source**: `src/context/mod.rs:1-40`

```rust
//! Focused dependency injection contexts
//!
//! This module replaces the `ServerResources` service locator anti-pattern with
//! focused contexts that provide only the dependencies needed for specific operations.
//!
//! # Architecture
//!
//! - `AuthContext`: Authentication and authorization dependencies
//! - `DataContext`: Database and data provider dependencies
//! - `ConfigContext`: Configuration and OAuth management dependencies
//! - `NotificationContext`: WebSocket and SSE notification dependencies

/// Authentication context
pub mod auth;
/// Configuration context
pub mod config;
/// Data context
pub mod data;
/// Notification context
pub mod notification;
/// Server context combining all focused contexts
pub mod server;

// Re-exports
pub use auth::AuthContext;
pub use config::ConfigContext;
pub use data::DataContext;
pub use notification::NotificationContext;
pub use server::ServerContext;
```

### Focused Context Example

```rust
// Conceptual example of focused contexts

/// Context for authentication operations
#[derive(Clone)]
pub struct AuthContext {
    pub auth_manager: Arc<AuthManager>,
    pub jwks_manager: Arc<JwksManager>,
    pub middleware: Arc<McpAuthMiddleware>,
}

/// Context for data operations
#[derive(Clone)]
pub struct DataContext {
    pub database: Arc<Database>,
    pub provider_registry: Arc<ProviderRegistry>,
    pub cache: Arc<Cache>,
}

/// Context for configuration operations
#[derive(Clone)]
pub struct ConfigContext {
    pub config: Arc<ServerConfig>,
    pub tenant_oauth_client: Arc<TenantOAuthClient>,
}

// Use specific contexts
async fn authenticate_user(
    auth_ctx: &AuthContext,
    token: &str,
) -> Result<User> {
    // Only has access to auth-related dependencies
    auth_ctx.auth_manager.validate_token(token)
}

async fn fetch_activities(
    data_ctx: &DataContext,
    user_id: &str,
) -> Result<Vec<Activity>> {
    // Only has access to data-related dependencies
    data_ctx.database.get_activities(user_id).await
}
```

**Benefits**:

1. ✅ **Clear dependencies** - Function signature shows what it needs
2. ✅ **Easier testing** - Mock only relevant context
3. ✅ **Better organization** - Related dependencies grouped
4. ✅ **Loose coupling** - Changes to one context don't affect others
5. ✅ **Type safety** - Compiler prevents using wrong context

---

## Arc\<T\> vs Rc\<T\> vs Box\<T\>

Understanding when to use each smart pointer:

| Type | Thread-Safe? | Overhead | Use When |
|------|-------------|----------|----------|
| `Box<T>` | N/A | Single allocation | Single ownership, heap allocation |
| `Rc<T>` | ❌ No | Non-atomic counter | Shared ownership, single thread |
| `Arc<T>` | ✅ Yes | Atomic counter | Shared ownership, multi-threaded |

**Pierre uses `Arc<T>` because**:
- Axum handlers run on different threads
- Need to share resources across concurrent requests
- Thread safety is non-negotiable in async runtime

**When to use each**:

```rust
// Box<T> - Single ownership
let config = Box::new(Config::from_file("config.toml")?);
drop(config);  // Config is dropped

// Rc<T> - Shared ownership, single thread
use std::rc::Rc;
let data = Rc::new(vec![1, 2, 3]);
let data2 = Rc::clone(&data);
// Both point to same Vec, single-threaded only

// Arc<T> - Shared ownership, multi-threaded
use std::sync::Arc;
let database = Arc::new(Database::new()?);
tokio::spawn(async move {
    database.query(...).await  // Can use in another thread
});
```

**Reference**: [Rust Book - Smart Pointers](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html)

---

## Interior Mutability with Arc\<Mutex\<T\>\>

Arc provides shared ownership, but data is immutable. For mutable shared state, use `Mutex`.

```rust
use std::sync::{Arc, Mutex};

// Shared mutable counter
let counter = Arc::new(Mutex::new(0));

// Spawn multiple tasks that increment counter
for _ in 0..10 {
    let counter_clone = Arc::clone(&counter);
    tokio::spawn(async move {
        let mut num = counter_clone.lock().unwrap();  // Acquire lock
        *num += 1;
    });  // Lock automatically released when `num` is dropped
}
```

**Rust Idioms Explained**:

1. **`Arc<Mutex<T>>` pattern**
   - `Arc` for shared ownership
   - `Mutex` for exclusive access
   - Common pattern for shared mutable state

2. **`.lock()` returns `MutexGuard`**
   - RAII guard that unlocks on drop
   - Implements `Deref` and `DerefMut`
   - Access inner value with `*guard`

3. **When to use**:
   - ✅ Occasional writes (metrics, caches)
   - ❌ Frequent writes (use channels/actors instead)
   - ❌ Async code (use `tokio::sync::Mutex` instead)

**Pierre examples**:
- `WebSocketManager` uses `DashMap` (concurrent HashMap)
- `Cache` uses `Mutex` for LRU eviction
- Most resources are immutable after creation

**Reference**: [Rust Book - Mutex](https://doc.rust-lang.org/book/ch16-03-shared-state.html)

---

## Diagram: Dependency Injection Flow

```
┌──────────────────────────────────────────────────────────┐
│                     Application Startup                   │
└──────────────────────────────────────────────────────────┘
                           │
                           ▼
         ┌─────────────────────────────────────┐
         │  Create Expensive Resources Once    │
         │  - Database (connection pool)       │
         │  - AuthManager (key material)       │
         │  - JwksManager (RSA keys)           │
         │  - Cache (LRU storage)              │
         └─────────────────┬───────────────────┘
                           │
                           ▼
         ┌─────────────────────────────────────┐
         │  Wrap in Arc<T>                     │
         │  - Arc::new(database)               │
         │  - Arc::new(auth_manager)           │
         │  - Arc::new(jwks_manager)           │
         └─────────────────┬───────────────────┘
                           │
                           ▼
         ┌─────────────────────────────────────┐
         │  Create ServerResources             │
         │  (or focused contexts)              │
         └─────────────────┬───────────────────┘
                           │
                           ▼
         ┌─────────────────────────────────────┐
         │  Wrap ServerResources in Arc        │
         │  Arc::new(resources)                │
         └─────────────────┬───────────────────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
   ┌──────────┐     ┌──────────┐     ┌──────────┐
   │Handler 1 │     │Handler 2 │     │Handler N │
   │resources │     │resources │     │resources │
   │.clone()  │     │.clone()  │     │.clone()  │
   └────┬─────┘     └────┬─────┘     └────┬─────┘
        │                │                │
        └────────────────┼────────────────┘
                         │
                         ▼
         ┌──────────────────────────────────┐
         │  All point to same resources     │
         │  (Arc counter = N)               │
         │  Memory allocated once           │
         └──────────────────────────────────┘
```

---

## Practical Exercises

### Exercise 1: Implement Dependency Injection

Create a simple service with dependency injection:

```rust
use std::sync::Arc;

struct Database {
    connection_string: String,
}

impl Database {
    fn new(conn: &str) -> Self {
        Self { connection_string: conn.to_string() }
    }

    fn query(&self, sql: &str) -> String {
        format!("Querying {} with: {}", self.connection_string, sql)
    }
}

// TODO: Create UserService that depends on Database
// Use Arc for shared ownership
```

**Solution**:
```rust
struct UserService {
    database: Arc<Database>,
}

impl UserService {
    fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    fn get_user(&self, id: u32) -> String {
        self.database.query(&format!("SELECT * FROM users WHERE id = {}", id))
    }
}

// Usage
fn main() {
    let db = Arc::new(Database::new("postgres://localhost"));
    let user_service = UserService::new(Arc::clone(&db));
    println!("{}", user_service.get_user(1));
}
```

### Exercise 2: Convert to Focused Contexts

Refactor this service locator into focused contexts:

```rust
struct AppState {
    database: Arc<Database>,
    auth: Arc<AuthManager>,
    cache: Arc<Cache>,
    mailer: Arc<Mailer>,
    logger: Arc<Logger>,
}

async fn authenticate(state: &AppState, token: &str) -> Result<User> {
    // Only uses auth and database
    todo!()
}

// TODO: Create AuthContext and use it instead
```

**Solution**:
```rust
#[derive(Clone)]
struct AuthContext {
    auth: Arc<AuthManager>,
    database: Arc<Database>,
}

async fn authenticate(ctx: &AuthContext, token: &str) -> Result<User> {
    let claims = ctx.auth.verify_token(token)?;
    let user = ctx.database.get_user(&claims.user_id).await?;
    Ok(user)
}
```

---

## Rust Idioms Summary

| Idiom | Purpose | Example Location |
|-------|---------|-----------------|
| **`Arc<T>`** | Shared ownership across threads | `src/mcp/resources.rs:40-77` |
| **`Arc::clone()`** | Increment reference count | `src/mcp/resources.rs:98-113` |
| **`#[derive(Clone)]` on Arc struct** | Cheap struct cloning | `src/mcp/resources.rs:39` |
| **`Arc<str>`** | Efficient immutable string sharing | `src/mcp/resources.rs:58` |
| **`Option<Arc<T>>`** | Optional shared dependencies | `src/mcp/resources.rs:72` |
| **Focused contexts** | Domain-specific DI containers | `src/context/mod.rs` |

**References**:
- [Rust Book - Arc](https://doc.rust-lang.org/book/ch16-03-shared-state.html)
- [Rust Book - Smart Pointers](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html)
- [Axum State Management](https://docs.rs/axum/latest/axum/extract/struct.State.html)

---

## Key Takeaways

1. **Arc<T> enables shared ownership** - Thread-safe reference counting
2. **Cloning Arc is cheap** - Just increments atomic counter
3. **Create once, share everywhere** - Wrap expensive resources in Arc at startup
4. **Service locator is an anti-pattern** - Use focused contexts instead
5. **Explicit dependencies** - Function signatures should show what's needed
6. **Arc vs Rc vs Box** - Choose based on threading and ownership needs
7. **Interior mutability** - Use `Mutex` or `RwLock` for mutable shared state

---

## Next Chapter

[Chapter 5: Cryptographic Key Management](./chapter-05-cryptographic-keys.md) - Learn Pierre's two-tier key management system (MEK + DEK), RSA key generation for JWT signing, and the `zeroize` crate for secure memory cleanup.
