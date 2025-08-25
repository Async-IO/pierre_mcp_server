# Database Layer

## Overview

Pierre MCP Server uses a plugin-based database architecture that supports multiple database backends through a unified interface. The system provides automatic encryption for sensitive data, comprehensive migration management, and tenant-isolated data access.

## Database Architecture

```mermaid
graph TB
    subgraph "Application Layer"
        Services[Business Services]
        Routes[API Routes]
        MCP[MCP Handlers]
        A2A[A2A Handlers]
    end
    
    subgraph "Database Abstraction"
        Factory[Database Factory]
        Provider[DatabaseProvider Trait]
        Encryption[Encryption Service]
    end
    
    subgraph "Database Implementations"
        SQLite[SQLite Provider]
        PostgreSQL[PostgreSQL Provider]
        MySQL[MySQL Provider - Future]
    end
    
    subgraph "Storage Backends"
        SQLiteFile[(SQLite File)]
        PostgresDB[(PostgreSQL)]
        CloudSQL[(Cloud SQL)]
    end
    
    subgraph "Data Categories"
        Users[Users & Auth]
        OAuth[OAuth Tokens]
        Analytics[Usage Analytics]
        A2ASystems[A2A Systems]
        Tenants[Tenant Data]
    end
    
    Services --> Factory
    Routes --> Factory
    MCP --> Factory
    A2A --> Factory
    
    Factory --> Provider
    Provider --> Encryption
    
    Provider --> SQLite
    Provider --> PostgreSQL
    
    SQLite --> SQLiteFile
    PostgreSQL --> PostgresDB
    PostgreSQL --> CloudSQL
    
    Provider --> Users
    Provider --> OAuth
    Provider --> Analytics
    Provider --> A2ASystems
    Provider --> Tenants
```

## Database Provider Interface

### Core Trait Definition

```rust
// src/database_plugins/mod.rs
#[async_trait]
pub trait DatabaseProvider: Send + Sync + Clone {
    // Connection and setup
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self>;
    async fn migrate(&self) -> Result<()>;
    
    // User management
    async fn create_user(&self, user: &User) -> Result<Uuid>;
    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn update_last_active(&self, user_id: Uuid) -> Result<()>;
    
    // OAuth token management
    async fn update_strava_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()>;
    
    async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>>;
    
    // API key management
    async fn create_api_key(&self, api_key: &ApiKey) -> Result<Uuid>;
    async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>>;
    async fn get_api_keys_for_user(&self, user_id: Uuid) -> Result<Vec<ApiKey>>;
    
    // Analytics and usage tracking
    async fn record_api_usage(&self, usage: &ApiKeyUsage) -> Result<()>;
    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()>;
    async fn get_usage_stats(&self, user_id: Uuid) -> Result<ApiKeyUsageStats>;
    
    // A2A system management
    async fn create_a2a_client(&self, client: &A2AClient) -> Result<Uuid>;
    async fn get_a2a_client(&self, client_id: Uuid) -> Result<Option<A2AClient>>;
    async fn get_system_user_by_api_key(&self, key_hash: &str) -> Result<Option<A2AClient>>;
    
    // Tenant management
    async fn create_tenant(&self, tenant: &Tenant) -> Result<Uuid>;
    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>>;
    async fn update_tenant(&self, tenant: &Tenant) -> Result<()>;
}
```

## Database Factory System

### Automatic Database Detection

```rust
// src/database_plugins/factory.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
}

pub enum Database {
    SQLite(SqliteDatabase),
    #[cfg(feature = "postgresql")]
    PostgreSQL(PostgresDatabase),
}

impl Database {
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        let db_type = detect_database_type(database_url)?;
        
        match db_type {
            DatabaseType::SQLite => {
                let db = SqliteDatabase::new(database_url, encryption_key).await?;
                Ok(Self::SQLite(db))
            }
            DatabaseType::PostgreSQL => {
                #[cfg(feature = "postgresql")]
                {
                    let db = PostgresDatabase::new(database_url, encryption_key).await?;
                    Ok(Self::PostgreSQL(db))
                }
                #[cfg(not(feature = "postgresql"))]
                {
                    Err(anyhow!("PostgreSQL support not compiled in"))
                }
            }
        }
    }
    
    pub const fn backend_info(&self) -> &'static str {
        match self {
            Self::SQLite(_) => "SQLite (Local Development)",
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_) => "PostgreSQL (Cloud-Ready)",
        }
    }
}

fn detect_database_type(database_url: &str) -> Result<DatabaseType> {
    if database_url.starts_with("sqlite:") {
        Ok(DatabaseType::SQLite)
    } else if database_url.starts_with("postgres://") 
           || database_url.starts_with("postgresql://") {
        Ok(DatabaseType::PostgreSQL)
    } else {
        Err(anyhow!("Unsupported database URL format: {}", database_url))
    }
}
```

### Database Provider Implementation

The factory delegates all calls to the underlying provider through ServerResources:

```rust
// ✅ CORRECT: Database created once at startup via ServerResources
impl MultiTenantMcpServer {
    pub fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }
    
    // Components access database through shared resources
    async fn handle_request(&self) -> Result<Response> {
        let user = self.resources.database.get_user(user_id).await?;
        // Use shared database reference, never clone or create new instances
    }
}

#[async_trait]
impl DatabaseProvider for Database {
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        Self::new(database_url, encryption_key).await
    }
    
    async fn migrate(&self) -> Result<()> {
        match self {
            Self::SQLite(db) => db.migrate().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.migrate().await,
        }
    }
    
    async fn create_user(&self, user: &User) -> Result<Uuid> {
        match self {
            Self::SQLite(db) => db.create_user(user).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_user(user).await,
        }
    }
    
    // ... other methods delegate similarly
}
```

## Database Schema

### Core Tables

#### Users Table
```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    firstname TEXT,
    lastname TEXT,
    status TEXT NOT NULL DEFAULT 'pending' 
        CHECK (status IN ('pending', 'active', 'suspended', 'deleted')),
    is_admin BOOLEAN NOT NULL DEFAULT 0,
    tier TEXT NOT NULL DEFAULT 'free' 
        CHECK (tier IN ('free', 'basic', 'professional', 'enterprise')),
    rate_limit_multiplier REAL NOT NULL DEFAULT 1.0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_active DATETIME,
    approved_at DATETIME,
    approved_by TEXT,
    suspended_at DATETIME,
    suspended_by TEXT,
    suspension_reason TEXT
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_status ON users(status);
CREATE INDEX idx_users_tier ON users(tier);
```

#### OAuth Tokens Table
```sql
CREATE TABLE user_oauth_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('strava', 'fitbit', 'garmin')),
    access_token_encrypted TEXT NOT NULL,
    refresh_token_encrypted TEXT,
    expires_at DATETIME,
    scope TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, provider)
);

CREATE INDEX idx_oauth_tokens_user_provider ON user_oauth_tokens(user_id, provider);
CREATE INDEX idx_oauth_tokens_expires ON user_oauth_tokens(expires_at);
```

#### API Keys Table
```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key_hash TEXT UNIQUE NOT NULL,
    key_prefix TEXT NOT NULL,
    tier TEXT NOT NULL DEFAULT 'trial' 
        CHECK (tier IN ('trial', 'standard', 'premium', 'custom')),
    scopes TEXT NOT NULL, -- JSON array
    is_active BOOLEAN NOT NULL DEFAULT 1,
    last_used DATETIME,
    usage_count INTEGER NOT NULL DEFAULT 0,
    rate_limit_override INTEGER,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT -- JSON object
);

CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_user ON api_keys(user_id);
CREATE INDEX idx_api_keys_active ON api_keys(is_active);
```

#### Tenants Table
```sql
CREATE TABLE tenants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    domain TEXT UNIQUE,
    plan TEXT NOT NULL DEFAULT 'starter' 
        CHECK (plan IN ('starter', 'professional', 'enterprise')),
    owner_user_id TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_tenants_slug ON tenants(slug);
CREATE INDEX idx_tenants_owner ON tenants(owner_user_id);
```

#### A2A Systems Table
```sql
CREATE TABLE a2a_clients (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    api_key_hash TEXT UNIQUE NOT NULL,
    capabilities TEXT NOT NULL, -- JSON array
    rate_limit_per_day INTEGER NOT NULL DEFAULT 1000,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used DATETIME,
    metadata TEXT -- JSON object
);

CREATE INDEX idx_a2a_clients_hash ON a2a_clients(api_key_hash);
CREATE INDEX idx_a2a_clients_active ON a2a_clients(is_active);
```

### Usage Analytics Tables

#### API Usage Tracking
```sql
CREATE TABLE api_key_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address TEXT,
    user_agent TEXT,
    error_message TEXT
);

CREATE INDEX idx_api_usage_key_time ON api_key_usage(api_key_id, timestamp);
CREATE INDEX idx_api_usage_endpoint ON api_key_usage(endpoint);
CREATE INDEX idx_api_usage_status ON api_key_usage(status_code);
```

#### JWT Usage Tracking
```sql
CREATE TABLE jwt_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address TEXT,
    user_agent TEXT
);

CREATE INDEX idx_jwt_usage_user_time ON jwt_usage(user_id, timestamp);
CREATE INDEX idx_jwt_usage_endpoint ON jwt_usage(endpoint);
```

## Data Encryption

### Encryption Service

```rust
// src/database/encryption.rs
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use rand::{thread_rng, RngCore};

pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    pub fn new(key: &[u8; 32]) -> Self {
        let key = Key::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        
        Self { cipher }
    }
    
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        
        Ok(base64::encode(result))
    }
    
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        let data = base64::decode(encrypted)
            .map_err(|e| anyhow!("Base64 decode failed: {}", e))?;
        
        if data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data"));
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        String::from_utf8(plaintext)
            .map_err(|e| anyhow!("Invalid UTF-8: {}", e))
    }
}
```

### Token Storage with Encryption

```rust
// src/database/user_oauth_tokens.rs
impl DatabaseProvider for SqliteDatabase {
    async fn update_strava_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()> {
        let encryption_service = EncryptionService::new(
            &self.encryption_key.as_slice().try_into()?
        );
        
        // Encrypt tokens before storage
        let encrypted_access = encryption_service.encrypt(access_token)?;
        let encrypted_refresh = encryption_service.encrypt(refresh_token)?;
        
        sqlx::query!(
            r#"
            INSERT INTO user_oauth_tokens 
                (id, user_id, provider, access_token_encrypted, 
                 refresh_token_encrypted, expires_at, scope, updated_at)
            VALUES (?1, ?2, 'strava', ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id, provider) DO UPDATE SET
                access_token_encrypted = excluded.access_token_encrypted,
                refresh_token_encrypted = excluded.refresh_token_encrypted,
                expires_at = excluded.expires_at,
                scope = excluded.scope,
                updated_at = CURRENT_TIMESTAMP
            "#,
            Uuid::new_v4().to_string(),
            user_id.to_string(),
            encrypted_access,
            encrypted_refresh,
            expires_at,
            scope,
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query!(
            "SELECT access_token_encrypted, refresh_token_encrypted, expires_at, scope
             FROM user_oauth_tokens 
             WHERE user_id = ?1 AND provider = 'strava'",
            user_id.to_string()
        )
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = row {
            let encryption_service = EncryptionService::new(
                &self.encryption_key.as_slice().try_into()?
            );
            
            let access_token = encryption_service.decrypt(&row.access_token_encrypted)?;
            let refresh_token = row.refresh_token_encrypted
                .as_ref()
                .map(|t| encryption_service.decrypt(t))
                .transpose()?;
            
            Ok(Some(DecryptedToken {
                access_token,
                refresh_token,
                expires_at: row.expires_at,
                scope: row.scope,
            }))
        } else {
            Ok(None)
        }
    }
}
```

## Database Migrations

### Migration System

```rust
// src/database/mod.rs
impl Database {
    pub async fn migrate(&self) -> Result<()> {
        // Run migrations in dependency order
        self.migrate_users().await?;
        self.migrate_api_keys().await?;
        self.migrate_analytics().await?;
        self.migrate_a2a().await?;
        self.migrate_admin().await?;
        self.migrate_user_oauth_tokens().await?;
        self.migrate_tenant_management().await?;
        
        Ok(())
    }
    
    async fn migrate_users(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                firstname TEXT,
                lastname TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                is_admin BOOLEAN NOT NULL DEFAULT 0,
                tier TEXT NOT NULL DEFAULT 'free',
                rate_limit_multiplier REAL NOT NULL DEFAULT 1.0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_active DATETIME,
                approved_at DATETIME,
                approved_by TEXT,
                suspended_at DATETIME,
                suspended_by TEXT,
                suspension_reason TEXT,
                CHECK (status IN ('pending', 'active', 'suspended', 'deleted')),
                CHECK (tier IN ('free', 'basic', 'professional', 'enterprise'))
            )
            "#
        )
        .execute(&self.pool)
        .await?;
        
        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_status ON users(status)")
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
}
```

### Schema Versioning

```rust
// Future enhancement for schema versioning
pub struct MigrationManager {
    database: Database,
}

impl MigrationManager {
    pub async fn get_schema_version(&self) -> Result<u32> {
        // Check if migration table exists
        let version = sqlx::query_scalar!(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1"
        )
        .fetch_optional(self.database.pool())
        .await?
        .unwrap_or(0);
        
        Ok(version as u32)
    }
    
    pub async fn migrate_to_version(&self, target_version: u32) -> Result<()> {
        let current_version = self.get_schema_version().await?;
        
        if current_version >= target_version {
            return Ok(());
        }
        
        for version in (current_version + 1)..=target_version {
            info!("Running migration {}", version);
            self.run_migration(version).await?;
            self.record_migration(version).await?;
        }
        
        Ok(())
    }
}
```

## Connection Management

### Connection Pooling

```rust
// src/database_plugins/sqlite.rs
#[derive(Clone)]
pub struct SqliteDatabase {
    pool: SqlitePool,
    encryption_key: Vec<u8>,
}

impl SqliteDatabase {
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Configure connection pool
        let connection_options = if database_url.starts_with("sqlite:") {
            format!("{}?mode=rwc", database_url)
        } else {
            database_url.to_string()
        };
        
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::from_str(&connection_options)?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .foreign_keys(true)
                .busy_timeout(Duration::from_secs(30))
        ).await?;
        
        // Set connection limits
        pool.set_max_connections(20).await;
        pool.set_min_connections(5).await;
        pool.set_acquire_timeout(Duration::from_secs(10)).await;
        
        Ok(Self {
            pool,
            encryption_key,
        })
    }
}
```

### PostgreSQL Connection

```rust
// src/database_plugins/postgres.rs
#[cfg(feature = "postgresql")]
pub struct PostgresDatabase {
    pool: PgPool,
    encryption_key: Vec<u8>,
}

#[cfg(feature = "postgresql")]
impl PostgresDatabase {
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .min_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await?;
        
        Ok(Self {
            pool,
            encryption_key,
        })
    }
}
```

## Tenant Data Isolation

### Tenant-Scoped Queries

```rust
// All data access includes tenant_id for isolation
impl DatabaseProvider for Database {
    async fn get_user_activities(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        limit: u32,
    ) -> Result<Vec<Activity>> {
        sqlx::query_as!(
            Activity,
            r#"
            SELECT id, tenant_id, user_id, name, type, distance, duration, 
                   start_date, created_at, updated_at
            FROM activities 
            WHERE tenant_id = ?1 AND user_id = ?2 
            ORDER BY start_date DESC 
            LIMIT ?3
            "#,
            tenant_id.to_string(),
            user_id.to_string(),
            limit as i32
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }
    
    async fn create_activity(
        &self,
        tenant_id: Uuid,
        activity: &Activity,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO activities 
                (id, tenant_id, user_id, name, type, distance, duration, 
                 start_date, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP)
            "#,
            id.to_string(),
            tenant_id.to_string(),
            activity.user_id.to_string(),
            activity.name,
            activity.activity_type,
            activity.distance,
            activity.duration,
            activity.start_date,
        )
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }
}
```

## Performance Optimization

### Query Optimization

```rust
// Use prepared statements and proper indexing
impl DatabaseProvider for Database {
    async fn get_user_stats_optimized(&self, user_id: Uuid) -> Result<UserStats> {
        // Single query with aggregation instead of multiple queries
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_activities,
                SUM(distance) as total_distance,
                SUM(duration) as total_time,
                AVG(distance) as avg_distance,
                MIN(start_date) as first_activity,
                MAX(start_date) as last_activity
            FROM activities 
            WHERE user_id = ?1 AND deleted_at IS NULL
            "#,
            user_id.to_string()
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(UserStats {
            total_activities: stats.total_activities as u32,
            total_distance: stats.total_distance.unwrap_or(0.0),
            total_time: stats.total_time.unwrap_or(0),
            avg_distance: stats.avg_distance.unwrap_or(0.0),
            first_activity: stats.first_activity,
            last_activity: stats.last_activity,
        })
    }
}
```

### Connection Pool Monitoring

```rust
// src/database/monitoring.rs
pub struct DatabaseMetrics {
    pub pool_size: Gauge,
    pub active_connections: Gauge,
    pub idle_connections: Gauge,
    pub query_duration: Histogram,
    pub queries_total: Counter,
    pub connection_errors: Counter,
}

impl DatabaseMetrics {
    pub async fn record_pool_stats(&self, pool: &SqlitePool) {
        let pool_options = pool.options();
        self.pool_size.set(pool_options.get_max_connections() as f64);
        
        // Note: SQLx doesn't expose detailed pool stats yet
        // This would be available in future versions
    }
    
    pub fn record_query(&self, duration: Duration, success: bool) {
        self.query_duration.observe(duration.as_secs_f64());
        self.queries_total.inc();
        
        if !success {
            self.connection_errors.inc();
        }
    }
}
```

## Database Configuration

### Environment Configuration

```bash
# SQLite configuration (development)
DATABASE_URL=sqlite:./data/pierre.db
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5
DATABASE_ACQUIRE_TIMEOUT=10
DATABASE_ENCRYPTION_KEY_PATH=./data/encryption.key

# PostgreSQL configuration (production)
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre_prod
DATABASE_MAX_CONNECTIONS=50
DATABASE_MIN_CONNECTIONS=10
DATABASE_ACQUIRE_TIMEOUT=30
DATABASE_IDLE_TIMEOUT=600
DATABASE_MAX_LIFETIME=1800
```

### Database URL Examples

```bash
# SQLite
DATABASE_URL="sqlite:./data/pierre.db"
DATABASE_URL="sqlite://./data/pierre.db?mode=rwc"

# PostgreSQL
DATABASE_URL="postgresql://user:password@localhost:5432/pierre"
DATABASE_URL="postgres://user:password@localhost:5432/pierre"

# PostgreSQL with connection options
DATABASE_URL="postgresql://user:password@localhost:5432/pierre?sslmode=require&application_name=pierre-mcp-server"

# Cloud PostgreSQL
DATABASE_URL="postgresql://user:password@db.example.com:5432/pierre?sslmode=require"
```

## Backup and Recovery

### SQLite Backup

```rust
// src/database/backup.rs
impl SqliteDatabase {
    pub async fn create_backup(&self, backup_path: &Path) -> Result<()> {
        // Use SQLite VACUUM INTO for consistent backup
        sqlx::query(&format!("VACUUM INTO '{}'", backup_path.display()))
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn restore_from_backup(&self, backup_path: &Path) -> Result<()> {
        // Close current connections
        self.pool.close().await;
        
        // Copy backup file to database location
        tokio::fs::copy(backup_path, &self.database_path).await?;
        
        // Reconnect
        self.pool = SqlitePool::connect(&self.connection_string).await?;
        
        Ok(())
    }
}
```

### PostgreSQL Backup

```bash
# Create backup
pg_dump $DATABASE_URL > backup.sql

# Restore backup
psql $DATABASE_URL < backup.sql

# Automated backup script
#!/bin/bash
BACKUP_DIR="/backups/pierre"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_FILE="$BACKUP_DIR/pierre_backup_$TIMESTAMP.sql"

pg_dump $DATABASE_URL > $BACKUP_FILE
gzip $BACKUP_FILE

# Keep only last 30 days
find $BACKUP_DIR -name "*.sql.gz" -mtime +30 -delete
```

## Testing Strategy

### Database Testing

```rust
// tests/database_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::multitenant::ServerResources;
    
    // ✅ CORRECT: Create test database and wrap in ServerResources
    async fn create_test_server_resources() -> Arc<ServerResources> {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let encryption_key = vec![0u8; 32]; // Test key
        
        let database = Arc::new(Database::new(&db_url, encryption_key).await.unwrap());
        
        // Create minimal ServerResources for testing
        Arc::new(ServerResources {
            database,
            // ... other test resources
        })
    }
    
    #[tokio::test]
    async fn test_user_creation() {
        let resources = create_test_server_resources().await;
        
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            firstname: Some("Test".to_string()),
            lastname: Some("User".to_string()),
            status: UserStatus::Active,
            tier: UserTier::Free,
            // ... other fields
        };
        
        let user_id = resources.database.create_user(&user).await.unwrap();
        assert_eq!(user_id, user.id);
        
        let retrieved = resources.database.get_user(user_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().email, user.email);
    }
    
    #[tokio::test]
    async fn test_tenant_isolation() {
        let resources = create_test_server_resources().await;
        
        // Create two tenants
        let tenant_a = create_test_tenant("tenant-a").await;
        let tenant_b = create_test_tenant("tenant-b").await;
        
        // Create data for each tenant
        let activity_a = create_test_activity(&tenant_a);
        let activity_b = create_test_activity(&tenant_b);
        
        resources.database.create_activity(tenant_a.id, &activity_a).await.unwrap();
        resources.database.create_activity(tenant_b.id, &activity_b).await.unwrap();
        
        // Verify isolation
        let activities_a = resources.database.get_user_activities(
            tenant_a.id, tenant_a.owner_id, 100
        ).await.unwrap();
        
        assert_eq!(activities_a.len(), 1);
        assert_eq!(activities_a[0].id, activity_a.id);
        
        let activities_b = resources.database.get_user_activities(
            tenant_b.id, tenant_b.owner_id, 100
        ).await.unwrap();
        
        assert_eq!(activities_b.len(), 1);
        assert_eq!(activities_b[0].id, activity_b.id);
    }
}
```

This comprehensive database layer provides a solid foundation for Pierre MCP Server with proper abstraction, encryption, tenant isolation, and performance optimization.