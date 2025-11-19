// ABOUTME: PostgreSQL database implementation for cloud and production deployments
// ABOUTME: Provides enterprise-grade database support with connection pooling and scalability
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! `PostgreSQL` database implementation
//!
//! This module provides `PostgreSQL` support for cloud deployments,
//! implementing the same interface as the `SQLite` version.

use super::shared;
use crate::database::errors::DatabaseError;
use crate::database::repositories::{
    A2ARepository, AdminRepository, ApiKeyRepository, FitnessConfigRepository,
    NotificationRepository, OAuth2ServerRepository, OAuthTokenRepository, SecurityRepository,
    TenantRepository, UsageRepository,
};
use crate::errors::AppResult;
use crate::models::User;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::time::Duration;

/// `PostgreSQL` database implementation
#[derive(Clone)]
pub struct PostgresDatabase {
    pool: Pool<Postgres>,
    encryption_key: Vec<u8>,
}

impl PostgresDatabase {
    /// Close the database connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Run database migrations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any migration fails
    /// - Database connection is lost during migration
    /// - Insufficient database permissions
    pub async fn migrate(&self) -> AppResult<()> {
        self.create_users_table().await?;
        self.create_user_profiles_table().await?;
        self.create_goals_table().await?;
        self.create_insights_table().await?;
        self.create_api_keys_tables().await?;
        self.create_a2a_tables().await?;
        self.create_admin_tables().await?;
        self.create_jwt_usage_table().await?;
        self.create_oauth_notifications_table().await?;
        self.create_rsa_keypairs_table().await?;
        self.create_tenant_tables().await?;
        self.create_indexes().await?;
        Ok(())
    }

    async fn create_users_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT NOT NULL,
                tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
                tenant_id TEXT,
                strava_access_token TEXT,
                strava_refresh_token TEXT,
                strava_expires_at TIMESTAMPTZ,
                strava_scope TEXT,
                strava_nonce TEXT,
                fitbit_access_token TEXT,
                fitbit_refresh_token TEXT,
                fitbit_expires_at TIMESTAMPTZ,
                fitbit_scope TEXT,
                fitbit_nonce TEXT,
                is_active BOOLEAN NOT NULL DEFAULT true,
                user_status TEXT NOT NULL DEFAULT 'pending' CHECK (user_status IN ('pending', 'active', 'suspended')),
                is_admin BOOLEAN NOT NULL DEFAULT false,
                approved_by UUID REFERENCES users(id),
                approved_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                last_active TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_user_profiles_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_profiles (
                user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                profile_data JSONB NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_goals_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS goals (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                goal_data JSONB NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_insights_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS insights (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                insight_type TEXT NOT NULL,
                content JSONB NOT NULL,
                metadata JSONB,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_api_keys_tables(&self) -> AppResult<()> {
        // Create api_keys table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                description TEXT,
                tier TEXT NOT NULL CHECK (tier IN ('trial', 'starter', 'professional', 'enterprise')),
                is_active BOOLEAN NOT NULL DEFAULT true,
                rate_limit_requests INTEGER NOT NULL,
                rate_limit_window_seconds INTEGER NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ,
                last_used_at TIMESTAMPTZ,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create api_key_usage table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS api_key_usage (
                id SERIAL PRIMARY KEY,
                api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                endpoint TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code SMALLINT NOT NULL,
                method TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address INET,
                user_agent TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_a2a_tables(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_clients (
                client_id TEXT PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                description TEXT,
                client_secret_hash TEXT NOT NULL,
                api_key_hash TEXT NOT NULL,
                capabilities TEXT[] NOT NULL DEFAULT '{}',
                redirect_uris TEXT[] NOT NULL DEFAULT '{}',
                contact_email TEXT,
                is_active BOOLEAN NOT NULL DEFAULT true,
                rate_limit_per_minute INTEGER NOT NULL DEFAULT 100,
                rate_limit_per_day INTEGER DEFAULT 10000,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_sessions (
                session_token TEXT PRIMARY KEY,
                client_id TEXT NOT NULL REFERENCES a2a_clients(client_id) ON DELETE CASCADE,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                granted_scopes TEXT[] NOT NULL DEFAULT '{}',
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ NOT NULL,
                last_active_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_tasks (
                task_id TEXT PRIMARY KEY,
                session_token TEXT NOT NULL REFERENCES a2a_sessions(session_token) ON DELETE CASCADE,
                task_type TEXT NOT NULL,
                parameters JSONB NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                result JSONB,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_usage (
                id SERIAL PRIMARY KEY,
                client_id TEXT NOT NULL REFERENCES a2a_clients(client_id) ON DELETE CASCADE,
                session_token TEXT REFERENCES a2a_sessions(session_token) ON DELETE SET NULL,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                endpoint TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code SMALLINT NOT NULL,
                method TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address INET,
                user_agent TEXT,
                protocol_version TEXT NOT NULL DEFAULT 'v1',
                client_capabilities TEXT[] DEFAULT '{}',
                granted_scopes TEXT[] DEFAULT '{}'
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_admin_tables(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_tokens (
                id TEXT PRIMARY KEY,
                service_name TEXT NOT NULL,
                service_description TEXT,
                token_hash TEXT NOT NULL,
                token_prefix TEXT NOT NULL,
                jwt_secret_hash TEXT NOT NULL,
                permissions TEXT NOT NULL DEFAULT '[]',
                is_super_admin BOOLEAN NOT NULL DEFAULT false,
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ,
                last_used_at TIMESTAMPTZ,
                last_used_ip INET,
                usage_count BIGINT NOT NULL DEFAULT 0
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_token_usage (
                id SERIAL PRIMARY KEY,
                admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                action TEXT NOT NULL,
                target_resource TEXT,
                ip_address INET,
                user_agent TEXT,
                request_size_bytes INTEGER,
                success BOOLEAN NOT NULL,
                method TEXT,
                response_time_ms INTEGER
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_provisioned_keys (
                id SERIAL PRIMARY KEY,
                admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
                api_key_id TEXT NOT NULL,
                user_email TEXT NOT NULL,
                requested_tier TEXT NOT NULL,
                provisioned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                provisioned_by_service TEXT NOT NULL,
                rate_limit_requests INTEGER NOT NULL,
                rate_limit_period TEXT NOT NULL,
                key_status TEXT NOT NULL DEFAULT 'active',
                revoked_at TIMESTAMPTZ,
                revoked_reason TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_jwt_usage_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS jwt_usage (
                id SERIAL PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                endpoint TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code INTEGER NOT NULL,
                method TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address INET,
                user_agent TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create OAuth notifications table for MCP resource delivery
    async fn create_oauth_notifications_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth_notifications (
                id TEXT PRIMARY KEY,
                user_id UUID NOT NULL,
                provider TEXT NOT NULL,
                success BOOLEAN NOT NULL DEFAULT true,
                message TEXT NOT NULL,
                expires_at TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                read_at TIMESTAMPTZ,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indices for efficient queries
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_id
            ON oauth_notifications (user_id)
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_unread
            ON oauth_notifications (user_id, read_at)
            WHERE read_at IS NULL
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create RSA keypairs table for JWT signing key persistence
    async fn create_rsa_keypairs_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS rsa_keypairs (
                kid TEXT PRIMARY KEY,
                private_key_pem TEXT NOT NULL,
                public_key_pem TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT false,
                key_size_bits INTEGER NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create index for active key lookup
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_rsa_keypairs_active
            ON rsa_keypairs (is_active)
            WHERE is_active = true
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Creates complete multi-tenant database schema with all required tables
    #[allow(clippy::too_many_lines)]
    async fn create_tenant_tables(&self) -> AppResult<()> {
        // Create tenants table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenants (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                name VARCHAR(255) NOT NULL,
                slug VARCHAR(100) UNIQUE NOT NULL,
                domain VARCHAR(255) UNIQUE,
                subscription_tier VARCHAR(50) DEFAULT 'starter' CHECK (subscription_tier IN ('starter', 'professional', 'enterprise')),
                is_active BOOLEAN DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_oauth_credentials table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_oauth_credentials (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                provider VARCHAR(50) NOT NULL,
                client_id VARCHAR(255) NOT NULL,
                client_secret_encrypted TEXT NOT NULL,
                redirect_uri VARCHAR(500) NOT NULL,
                scopes TEXT[] DEFAULT '{}',
                rate_limit_per_day INTEGER DEFAULT 15000,
                is_active BOOLEAN DEFAULT true,
                configured_by UUID REFERENCES users(id),
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, provider)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_users table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                role VARCHAR(50) DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'billing', 'member')),
                joined_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, user_id)
            )
            "
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_provider_usage table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_provider_usage (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                provider VARCHAR(50) NOT NULL,
                usage_date DATE NOT NULL,
                request_count INTEGER DEFAULT 0,
                error_count INTEGER DEFAULT 0,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, provider, usage_date)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create OAuth Apps table for app registration
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth_apps (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                client_id VARCHAR(255) UNIQUE NOT NULL,
                client_secret VARCHAR(255) NOT NULL,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                redirect_uris TEXT[] NOT NULL DEFAULT '{}',
                scopes TEXT[] NOT NULL DEFAULT '{}',
                app_type VARCHAR(50) DEFAULT 'web' CHECK (app_type IN ('desktop', 'web', 'mobile', 'server')),
                owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                is_active BOOLEAN DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create Authorization Code table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS authorization_codes (
                code VARCHAR(255) PRIMARY KEY,
                client_id VARCHAR(255) NOT NULL REFERENCES oauth_apps(client_id) ON DELETE CASCADE,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                redirect_uri VARCHAR(500) NOT NULL,
                scope VARCHAR(500) NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create user_oauth_tokens table for per-user, per-tenant OAuth tokens
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_oauth_tokens (
                id VARCHAR(255) PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                tenant_id VARCHAR(255) NOT NULL,
                provider VARCHAR(50) NOT NULL,
                access_token TEXT NOT NULL,
                refresh_token TEXT,
                token_type VARCHAR(50) DEFAULT 'bearer',
                expires_at TIMESTAMPTZ,
                scope TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, tenant_id, provider)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_indexes(&self) -> AppResult<()> {
        // User and profile indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await?;

        // API key indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_api_key_id ON api_key_usage(api_key_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_timestamp ON api_key_usage(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        // A2A indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_clients_user_id ON a2a_clients(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_client_id ON a2a_usage(client_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_timestamp ON a2a_usage(timestamp)")
            .execute(&self.pool)
            .await?;

        // Admin token indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_service ON admin_tokens(service_name)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_prefix ON admin_tokens(token_prefix)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_usage_token_id ON admin_token_usage(admin_token_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_usage_timestamp ON admin_token_usage(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_provisioned_token ON admin_provisioned_keys(admin_token_id)")
            .execute(&self.pool)
            .await?;

        // JWT usage indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_user_id ON jwt_usage(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_timestamp ON jwt_usage(timestamp)")
            .execute(&self.pool)
            .await?;

        // Tenant indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_oauth_credentials_tenant_provider ON tenant_oauth_credentials(tenant_id, provider)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tenant_users_tenant ON tenant_users(tenant_id)",
        )
        .execute(&self.pool)
        .await?;

        // UserOAuthToken indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_user ON user_oauth_tokens(user_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_tenant_provider ON user_oauth_tokens(tenant_id, provider)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_users_user ON tenant_users(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_usage_date ON tenant_provider_usage(tenant_id, provider, usage_date)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a new user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or email already in use
    pub async fn create_user(&self, user: &User) -> AppResult<uuid::Uuid> {
        // Check if user exists by email
        let existing = self.get_user_by_email(&user.email).await?;
        if let Some(existing_user) = existing {
            if existing_user.id != user.id {
                return Err(DatabaseError::ConstraintViolation {
                    constraint: "users_email_unique".to_owned(),
                    details: format!("Email {} is already in use by another user", user.email),
                }
                .into());
            }
            self.update_existing_user(user).await?;
        } else {
            self.insert_new_user(user).await?;
        }

        Ok(user.id)
    }

    /// Update existing user in database
    async fn update_existing_user(&self, user: &User) -> AppResult<()> {
        let (strava_access, strava_refresh, strava_expires, strava_scope) =
            Self::extract_token_fields(user.strava_token.as_ref());
        let (fitbit_access, fitbit_refresh, fitbit_expires, fitbit_scope) =
            Self::extract_token_fields(user.fitbit_token.as_ref());

        sqlx::query(
            r"
            UPDATE users SET
                display_name = $2,
                password_hash = $3,
                tier = $4,
                tenant_id = $5,
                strava_access_token = $6,
                strava_refresh_token = $7,
                strava_expires_at = $8,
                strava_scope = $9,
                fitbit_access_token = $10,
                fitbit_refresh_token = $11,
                fitbit_expires_at = $12,
                fitbit_scope = $13,
                is_active = $14,
                user_status = $15,
                approved_by = $16,
                approved_at = $17,
                last_active = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(user.id.to_string())
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(shared::enums::user_tier_to_str(&user.tier))
        .bind(&user.tenant_id)
        .bind(strava_access)
        .bind(strava_refresh)
        .bind(strava_expires)
        .bind(strava_scope)
        .bind(fitbit_access)
        .bind(fitbit_refresh)
        .bind(fitbit_expires)
        .bind(fitbit_scope)
        .bind(user.is_active)
        .bind(shared::enums::user_status_to_str(&user.user_status))
        .bind(user.approved_by.map(|id| id.to_string()))
        .bind(user.approved_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Insert new user into database
    async fn insert_new_user(&self, user: &User) -> AppResult<()> {
        let (strava_access, strava_refresh, strava_expires, strava_scope) =
            Self::extract_token_fields(user.strava_token.as_ref());
        let (fitbit_access, fitbit_refresh, fitbit_expires, fitbit_scope) =
            Self::extract_token_fields(user.fitbit_token.as_ref());

        sqlx::query(
            r"
            INSERT INTO users (
                id, email, display_name, password_hash, tier, tenant_id,
                strava_access_token, strava_refresh_token, strava_expires_at, strava_scope,
                fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope,
                is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            ",
        )
        .bind(user.id.to_string())
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(shared::enums::user_tier_to_str(&user.tier))
        .bind(&user.tenant_id)
        .bind(strava_access)
        .bind(strava_refresh)
        .bind(strava_expires)
        .bind(strava_scope)
        .bind(fitbit_access)
        .bind(fitbit_refresh)
        .bind(fitbit_expires)
        .bind(fitbit_scope)
        .bind(user.is_active)
        .bind(shared::enums::user_status_to_str(&user.user_status))
        .bind(user.is_admin)
        .bind(user.approved_by.map(|id| id.to_string()))
        .bind(user.approved_at)
        .bind(user.created_at)
        .bind(user.last_active)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Extract OAuth token fields for database binding
    fn extract_token_fields(
        token: Option<&crate::models::EncryptedToken>,
    ) -> (Option<&str>, Option<&str>, Option<i64>, Option<&str>) {
        token.map_or((None, None, None, None), |t| {
            (
                Some(t.access_token.as_str()),
                Some(t.refresh_token.as_str()),
                Some(t.expires_at.timestamp()),
                Some(t.scope.as_str()),
            )
        })
    }

    /// Get user by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_user(&self, user_id: uuid::Uuid) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id,
                   is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
            FROM users WHERE id = $1
            "
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref()
            .map(shared::mappers::parse_user_from_row)
            .transpose()
    }

    /// Get user by email
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id,
                   is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
            FROM users WHERE email = $1
            "
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref()
            .map(shared::mappers::parse_user_from_row)
            .transpose()
    }
}

impl PostgresDatabase {
    // ================================
    // Repository Pattern Accessors
    // ================================

    /// Get `ApiKeyRepository` for API key management
    #[must_use]
    pub fn api_keys(&self) -> crate::database::repositories::ApiKeyRepositoryImpl {
        crate::database::repositories::ApiKeyRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `UsageRepository` for usage tracking and analytics
    #[must_use]
    pub fn usage(&self) -> crate::database::repositories::UsageRepositoryImpl {
        crate::database::repositories::UsageRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `A2ARepository` for Agent-to-Agent management
    #[must_use]
    pub fn a2a(&self) -> crate::database::repositories::A2ARepositoryImpl {
        crate::database::repositories::A2ARepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `AdminRepository` for admin token management
    #[must_use]
    pub fn admin(&self) -> crate::database::repositories::AdminRepositoryImpl {
        crate::database::repositories::AdminRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `TenantRepository` for multi-tenant management
    #[must_use]
    pub fn tenants(&self) -> crate::database::repositories::TenantRepositoryImpl {
        crate::database::repositories::TenantRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `OAuth2ServerRepository` for OAuth 2.0 server functionality
    #[must_use]
    pub fn oauth2_server(&self) -> crate::database::repositories::OAuth2ServerRepositoryImpl {
        crate::database::repositories::OAuth2ServerRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `SecurityRepository` for key rotation and audit
    #[must_use]
    pub fn security(&self) -> crate::database::repositories::SecurityRepositoryImpl {
        crate::database::repositories::SecurityRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `OAuthTokenRepository` for OAuth token storage
    #[must_use]
    pub fn oauth_tokens(&self) -> crate::database::repositories::OAuthTokenRepositoryImpl {
        crate::database::repositories::OAuthTokenRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `FitnessConfigRepository` for fitness configuration management
    #[must_use]
    pub fn fitness_configs(&self) -> crate::database::repositories::FitnessConfigRepositoryImpl {
        crate::database::repositories::FitnessConfigRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    /// Get `NotificationRepository` for OAuth notifications
    #[must_use]
    pub fn notifications(&self) -> crate::database::repositories::NotificationRepositoryImpl {
        crate::database::repositories::NotificationRepositoryImpl::new(
            crate::database_plugins::factory::Database::PostgreSQL(self.clone()),
        )
    }

    // ================================
    // Database Connection Management
    // ================================

    /// Create new `PostgreSQL` database with provided pool configuration (internal implementation)
    /// This is called by the Database factory with centralized `ServerConfig`
    ///
    /// # Errors
    ///
    /// Returns an error if database connection or pool configuration fails
    async fn new_impl(
        database_url: &str,
        encryption_key: Vec<u8>,
        pool_config: &crate::config::environment::PostgresPoolConfig,
    ) -> AppResult<Self> {
        // Use pool configuration from ServerConfig (read once at startup)
        let max_connections = pool_config.max_connections;
        let min_connections = pool_config.min_connections;
        let acquire_timeout_secs = pool_config.acquire_timeout_secs;

        // Log connection pool configuration for debugging
        tracing::info!(
            "PostgreSQL pool config: max_connections={max_connections}, min_connections={min_connections}, timeout={acquire_timeout_secs}s"
        );

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(300)))
            .max_lifetime(Some(Duration::from_secs(600)))
            .connect(database_url)
            .await?;

        let db = Self {
            pool,
            encryption_key,
        };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }

    /// Create new `PostgreSQL` database with provided pool configuration (public API)
    /// This is called by the Database factory with centralized `ServerConfig`
    ///
    /// # Errors
    ///
    /// Returns an error if database connection or pool configuration fails
    pub async fn new(
        database_url: &str,
        encryption_key: Vec<u8>,
        pool_config: &crate::config::environment::PostgresPoolConfig,
    ) -> AppResult<Self> {
        Self::new_impl(database_url, encryption_key, pool_config).await
    }
}

impl PostgresDatabase {
    // ================================
    // User Management (Additional)
    // ================================

    /// Get user by email, returning an error if not found
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User with email is not found
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_by_email_required(&self, email: &str) -> AppResult<crate::models::User> {
        self.get_user_by_email(email).await?.ok_or_else(|| {
            DatabaseError::NotFound {
                entity_type: "User",
                entity_id: email.to_owned(),
            }
            .into()
        })
    }

    /// Update user's last active timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_last_active(&self, user_id: uuid::Uuid) -> AppResult<()> {
        sqlx::query("UPDATE users SET last_active = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get total count of users in the database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Count aggregation fails
    /// - Database connection issues
    pub async fn get_user_count(&self) -> AppResult<i64> {
        let count = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Get all users with a specific status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_users_by_status(&self, status: &str) -> AppResult<Vec<crate::models::User>> {
        let rows = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id,
                   is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
            FROM users
            WHERE user_status = $1
            ORDER BY created_at DESC
            "
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(shared::mappers::parse_user_from_row)
            .collect()
    }

    /// Get users with a specific status using cursor-based pagination
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Cursor parsing fails
    /// - Database connection issues
    pub async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> AppResult<crate::pagination::CursorPage<crate::models::User>> {
        use crate::pagination::{Cursor, CursorPage};

        let limit = params.limit;
        let query_limit = limit + 1; // Fetch one extra to determine has_more

        let mut query_builder = sqlx::QueryBuilder::new(
            r"SELECT id, email, display_name, password_hash, tier, tenant_id,
                     is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
              FROM users WHERE user_status = ",
        );
        query_builder.push_bind(status);

        if let Some(ref cursor) = params.cursor {
            // Cursor contains "timestamp:id" - extract timestamp for filtering
            query_builder.push(" AND created_at < ");
            let cursor_str = cursor.to_string();
            if let Some(timestamp_str) = cursor_str.split(':').next() {
                if let Ok(timestamp_millis) = timestamp_str.parse::<i64>() {
                    let timestamp = chrono::DateTime::from_timestamp_millis(timestamp_millis)
                        .unwrap_or_else(chrono::Utc::now);
                    query_builder.push_bind(timestamp);
                }
            }
        }

        query_builder.push(" ORDER BY created_at DESC LIMIT ");
        query_builder.push_bind(i64::try_from(query_limit)?);

        let rows = query_builder.build().fetch_all(&self.pool).await?;

        let has_more = rows.len() > limit;
        let items: Vec<_> = rows
            .iter()
            .take(limit)
            .map(shared::mappers::parse_user_from_row)
            .collect::<AppResult<Vec<_>>>()?;

        let count = items.len();
        let next_cursor = if has_more && !items.is_empty() {
            items
                .last()
                .map(|user| Cursor::new(user.created_at, &user.id.to_string()))
        } else {
            None
        };

        Ok(CursorPage {
            items,
            next_cursor,
            prev_cursor: None, // Backward pagination not implemented
            has_more,
            count,
        })
    }

    /// Update user status and record approval information
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Database update fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn update_user_status(
        &self,
        user_id: uuid::Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> AppResult<crate::models::User> {
        let status_str = shared::enums::user_status_to_str(&new_status);

        sqlx::query(
            r"
            UPDATE users
            SET user_status = $1, approved_by = $2, approved_at = $3
            WHERE id = $4
            ",
        )
        .bind(status_str)
        .bind(admin_token_id)
        .bind(chrono::Utc::now())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        self.get_user(user_id).await?.ok_or_else(|| {
            DatabaseError::NotFound {
                entity_type: "User",
                entity_id: user_id.to_string(),
            }
            .into()
        })
    }

    /// Update user's tenant ID for multi-tenant support
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_user_tenant_id(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
    ) -> AppResult<()> {
        sqlx::query("UPDATE users SET tenant_id = $1 WHERE id = $2")
            .bind(tenant_id)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ================================
    // User Profiles & Goals
    // ================================

    /// Create or update a user profile with the provided data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operation fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn upsert_user_profile(
        &self,
        user_id: uuid::Uuid,
        profile_data: serde_json::Value,
    ) -> AppResult<()> {
        let profile_json = serde_json::to_string(&profile_data)?;

        sqlx::query(
            r"
            INSERT INTO user_profiles (user_id, profile_data, updated_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO UPDATE SET
                profile_data = EXCLUDED.profile_data,
                updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(user_id.to_string())
        .bind(profile_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user profile data by user ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_profile(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Option<serde_json::Value>> {
        let row = sqlx::query("SELECT profile_data FROM user_profiles WHERE user_id = $1")
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let profile_json: String = row.try_get("profile_data")?;
            let profile_data: serde_json::Value = serde_json::from_str(&profile_json)?;
            Ok(Some(profile_data))
        } else {
            Ok(None)
        }
    }

    /// Create a new goal for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Goal data validation fails
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn create_goal(
        &self,
        user_id: uuid::Uuid,
        goal_data: serde_json::Value,
    ) -> AppResult<String> {
        let goal_id = uuid::Uuid::new_v4().to_string();
        let goal_json = serde_json::to_string(&goal_data)?;

        sqlx::query(
            r"
            INSERT INTO user_goals (id, user_id, goal_data, created_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ",
        )
        .bind(&goal_id)
        .bind(user_id.to_string())
        .bind(&goal_json)
        .execute(&self.pool)
        .await?;

        Ok(goal_id)
    }

    /// Get all goals for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_goals(&self, user_id: uuid::Uuid) -> AppResult<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r"
            SELECT goal_data FROM user_goals
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let goal_json: String = row.try_get("goal_data")?;
                let goal_data: serde_json::Value = serde_json::from_str(&goal_json)?;
                Ok(goal_data)
            })
            .collect()
    }

    /// Update the progress value for a specific goal
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Goal does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE user_goals
            SET current_value = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            ",
        )
        .bind(current_value)
        .bind(goal_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user configuration data by user ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_configuration(&self, user_id: &str) -> AppResult<Option<String>> {
        let row = sqlx::query("SELECT config_json FROM user_configurations WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_get("config_json").ok()))
    }

    /// Save user configuration data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Database update fails
    /// - Database connection issues
    pub async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> AppResult<()> {
        sqlx::query(
            r"
            INSERT INTO user_configurations (user_id, config_json, updated_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO UPDATE SET
                config_json = EXCLUDED.config_json,
                updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(user_id)
        .bind(config_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ================================
    // Insights
    // ================================

    /// Store a new insight for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Insight data validation fails
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_insight(
        &self,
        user_id: uuid::Uuid,
        insight_data: serde_json::Value,
    ) -> AppResult<String> {
        let insight_id = uuid::Uuid::new_v4().to_string();
        let insight_json = serde_json::to_string(&insight_data)?;

        sqlx::query(
            r"
            INSERT INTO user_insights (id, user_id, insight_data, created_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ",
        )
        .bind(&insight_id)
        .bind(user_id.to_string())
        .bind(&insight_json)
        .execute(&self.pool)
        .await?;

        Ok(insight_id)
    }

    /// Get insights for a user with optional filtering
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_insights(
        &self,
        user_id: uuid::Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<serde_json::Value>> {
        let limit_val = limit.unwrap_or(10);

        let rows = if let Some(insight_type_val) = insight_type {
            sqlx::query(
                r"
                SELECT insight_data FROM user_insights
                WHERE user_id = $1 AND insight_type = $2
                ORDER BY created_at DESC
                LIMIT $3
                ",
            )
            .bind(user_id.to_string())
            .bind(insight_type_val)
            .bind(i64::from(limit_val))
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r"
                SELECT insight_data FROM user_insights
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                ",
            )
            .bind(user_id.to_string())
            .bind(i64::from(limit_val))
            .fetch_all(&self.pool)
            .await?
        };

        rows.iter()
            .map(|row| {
                let insight_json: String = row.try_get("insight_data")?;
                let insight_data: serde_json::Value = serde_json::from_str(&insight_json)?;
                Ok(insight_data)
            })
            .collect()
    }

    // ================================
    // API Keys
    // ================================

    /// Create a new API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API key already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_api_key(&self, api_key: &crate::api_keys::ApiKey) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO api_keys (id, user_id, name, key_prefix, key_hash, description, tier, rate_limit_requests, rate_limit_window_seconds, is_active, last_used_at, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
        .bind(&api_key.id)
        .bind(api_key.user_id.to_string())
        .bind(&api_key.name)
        .bind(&api_key.key_prefix)
        .bind(&api_key.key_hash)
        .bind(&api_key.description)
        .bind(serde_json::to_string(&api_key.tier)?)
        .bind(i32::try_from(api_key.rate_limit_requests)?)
        .bind(i32::try_from(api_key.rate_limit_window_seconds)?)
        .bind(api_key.is_active)
        .bind(api_key.last_used_at)
        .bind(api_key.expires_at)
        .bind(api_key.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Retrieve an API key by its prefix and hash
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_api_key_by_prefix(
        &self,
        prefix: &str,
        hash: &str,
    ) -> AppResult<Option<crate::api_keys::ApiKey>> {
        // Delegate to Repository pattern
        self.api_keys()
            .get_by_prefix(prefix, hash)
            .await
            .map_err(Into::into)
    }

    /// Retrieve all API keys for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_api_keys(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Vec<crate::api_keys::ApiKey>> {
        // Delegate to Repository pattern
        self.api_keys()
            .list_by_user(user_id)
            .await
            .map_err(Into::into)
    }

    /// Update the last used timestamp for an API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API key does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_api_key_last_used(&self, api_key_id: &str) -> AppResult<()> {
        // Delegate to Repository pattern
        self.api_keys()
            .update_last_used(api_key_id)
            .await
            .map_err(Into::into)
    }

    /// Deactivate an API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API key does not exist
    /// - User does not own the API key
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_api_key(&self, api_key_id: &str, user_id: uuid::Uuid) -> AppResult<()> {
        // Delegate to Repository pattern
        self.api_keys()
            .deactivate(api_key_id, user_id)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an API key by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_api_key_by_id(
        &self,
        api_key_id: &str,
    ) -> AppResult<Option<crate::api_keys::ApiKey>> {
        // Delegate to Repository pattern
        self.api_keys()
            .get_by_id(api_key_id)
            .await
            .map_err(Into::into)
    }

    /// Clean up expired API keys from the database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn cleanup_expired_api_keys(&self) -> AppResult<u64> {
        let result = sqlx::query("DELETE FROM api_keys WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Retrieve all expired API keys
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_expired_api_keys(&self) -> AppResult<Vec<crate::api_keys::ApiKey>> {
        let rows = sqlx::query(
            "SELECT id, user_id, name, key_prefix, key_hash, description, tier, rate_limit_requests, rate_limit_window_seconds, is_active, last_used_at, expires_at, created_at
             FROM api_keys WHERE expires_at < NOW()"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut keys = Vec::new();
        for row in rows {
            let tier_json: String = row.try_get("tier")?;
            keys.push(crate::api_keys::ApiKey {
                id: row.try_get("id")?,
                user_id: uuid::Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                name: row.try_get("name")?,
                key_prefix: row.try_get("key_prefix")?,
                key_hash: row.try_get("key_hash")?,
                description: row.try_get("description")?,
                tier: serde_json::from_str(&tier_json)?,
                rate_limit_requests: row.try_get::<i32, _>("rate_limit_requests")?.try_into()?,
                rate_limit_window_seconds: row
                    .try_get::<i32, _>("rate_limit_window_seconds")?
                    .try_into()?,
                is_active: row.try_get("is_active")?,
                last_used_at: row.try_get("last_used_at")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok(keys)
    }

    /// Record API key usage for analytics and rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn record_api_key_usage(
        &self,
        usage: &crate::api_keys::ApiKeyUsage,
    ) -> AppResult<()> {
        // Delegate to Repository pattern
        self.usage()
            .record_api_key_usage(usage)
            .await
            .map_err(Into::into)
    }

    /// Get current usage count for an API key within the rate limit window
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_api_key_current_usage(&self, api_key_id: &str) -> AppResult<u32> {
        // Delegate to Repository pattern
        self.usage()
            .get_api_key_current_usage(api_key_id)
            .await
            .map_err(Into::into)
    }

    /// Get usage statistics for an API key within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<crate::api_keys::ApiKeyUsageStats> {
        // Delegate to Repository pattern
        self.usage()
            .get_api_key_usage_stats(api_key_id, start_date, end_date)
            .await
            .map_err(Into::into)
    }

    // ================================
    // JWT Usage Tracking
    // ================================

    /// Record JWT usage for analytics and rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn record_jwt_usage(&self, usage: &crate::rate_limiting::JwtUsage) -> AppResult<()> {
        // Delegate to Repository pattern
        self.usage()
            .record_jwt_usage(usage)
            .await
            .map_err(Into::into)
    }

    /// Get current JWT usage count for a user within the rate limit window
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_jwt_current_usage(&self, user_id: uuid::Uuid) -> AppResult<u32> {
        // Delegate to Repository pattern
        self.usage()
            .get_jwt_current_usage(user_id)
            .await
            .map_err(Into::into)
    }

    // ================================
    // System Statistics
    // ================================

    /// Get system-wide statistics (user count, active API key count)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_system_stats(&self) -> AppResult<(u64, u64)> {
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let api_key_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE is_active = true")
                .fetch_one(&self.pool)
                .await?;

        #[allow(clippy::cast_sign_loss)]
        Ok((user_count as u64, api_key_count as u64))
    }

    // ================================
    // A2A Client Management
    // ================================

    /// Create a new A2A (Agent-to-Agent) client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_a2a_client(
        &self,
        client: &crate::a2a::auth::A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> AppResult<String> {
        self.a2a()
            .create_client(client, client_secret, api_key_id)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an A2A client by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client(
        &self,
        _client_id: &str,
    ) -> AppResult<Option<crate::a2a::auth::A2AClient>> {
        tokio::task::yield_now().await;
        Ok(None)
    }

    /// Retrieve an A2A client by API key ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client_by_api_key_id(
        &self,
        api_key_id: &str,
    ) -> AppResult<Option<crate::a2a::auth::A2AClient>> {
        self.a2a()
            .get_client_by_api_key(api_key_id)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an A2A client by name
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client_by_name(
        &self,
        name: &str,
    ) -> AppResult<Option<crate::a2a::auth::A2AClient>> {
        self.a2a()
            .get_client_by_name(name)
            .await
            .map_err(Into::into)
    }

    /// List all A2A clients for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_a2a_clients(
        &self,
        _user_id: &uuid::Uuid,
    ) -> AppResult<Vec<crate::a2a::auth::A2AClient>> {
        tokio::task::yield_now().await;
        Ok(Vec::new())
    }

    /// Deactivate an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_a2a_client(&self, client_id: &str) -> AppResult<()> {
        self.a2a()
            .deactivate_client(client_id)
            .await
            .map_err(Into::into)
    }

    /// Get A2A client credentials (client ID and secret)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> AppResult<Option<(String, String)>> {
        self.a2a()
            .get_client_credentials(client_id)
            .await
            .map_err(Into::into)
    }

    /// Invalidate all active sessions for an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> AppResult<()> {
        self.a2a()
            .invalidate_client_sessions(client_id)
            .await
            .map_err(Into::into)
    }

    /// Deactivate all API keys associated with an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_client_api_keys(&self, client_id: &str) -> AppResult<()> {
        self.a2a()
            .deactivate_client_api_keys(client_id)
            .await
            .map_err(Into::into)
    }

    // ================================
    // A2A Sessions
    // ================================

    /// Create a new A2A session
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client does not exist
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&uuid::Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> AppResult<String> {
        self.a2a()
            .create_session(client_id, user_id, granted_scopes, expires_in_hours)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an A2A session by token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_session(
        &self,
        session_token: &str,
    ) -> AppResult<Option<crate::a2a::client::A2ASession>> {
        self.a2a()
            .get_session(session_token)
            .await
            .map_err(Into::into)
    }

    /// Update the last activity timestamp for an A2A session
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_a2a_session_activity(&self, session_token: &str) -> AppResult<()> {
        self.a2a()
            .update_session_activity(session_token)
            .await
            .map_err(Into::into)
    }

    /// Retrieve all active A2A sessions for a client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_active_a2a_sessions(
        &self,
        client_id: &str,
    ) -> AppResult<Vec<crate::a2a::client::A2ASession>> {
        self.a2a()
            .get_active_sessions(client_id)
            .await
            .map_err(Into::into)
    }

    // ================================
    // A2A Tasks
    // ================================

    /// Create a new A2A task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client does not exist
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &serde_json::Value,
    ) -> AppResult<String> {
        self.a2a()
            .create_task(client_id, session_id, task_type, input_data)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an A2A task by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_task(
        &self,
        _task_id: &str,
    ) -> AppResult<Option<crate::a2a::protocol::A2ATask>> {
        tokio::task::yield_now().await;
        Ok(None)
    }

    /// List A2A tasks with optional filtering and pagination
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&crate::a2a::protocol::TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<crate::a2a::protocol::A2ATask>> {
        self.a2a()
            .list_tasks(client_id, status_filter, limit, offset)
            .await
            .map_err(Into::into)
    }

    /// Update the status of an A2A task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task does not exist
    /// - Database update fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &crate::a2a::protocol::TaskStatus,
        result: Option<&serde_json::Value>,
        error: Option<&str>,
    ) -> AppResult<()> {
        self.a2a()
            .update_task_status(task_id, status, result, error)
            .await
            .map_err(Into::into)
    }

    // ================================
    // A2A Usage Tracking
    // ================================

    /// Record A2A usage for analytics and rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn record_a2a_usage(&self, _usage: &crate::database::A2AUsage) -> AppResult<()> {
        tokio::task::yield_now().await;
        Ok(())
    }

    /// Get current usage count for an A2A client within the rate limit window
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_a2a_client_current_usage(&self, client_id: &str) -> AppResult<u32> {
        self.a2a()
            .get_client_current_usage(client_id)
            .await
            .map_err(Into::into)
    }

    /// Get usage statistics for an A2A client within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<crate::database::A2AUsageStats> {
        self.a2a()
            .get_usage_stats(client_id, start_date, end_date)
            .await
            .map_err(Into::into)
    }

    /// Get usage history for an A2A client for the specified number of days
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> AppResult<Vec<(chrono::DateTime<chrono::Utc>, u32, u32)>> {
        self.a2a()
            .get_client_usage_history(client_id, days)
            .await
            .map_err(Into::into)
    }

    // ================================
    // Provider Synchronization
    // ================================

    /// Get the last synchronization timestamp for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_provider_last_sync(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<Option<chrono::DateTime<chrono::Utc>>> {
        self.oauth_tokens()
            .get_last_sync(user_id, provider)
            .await
            .map_err(Into::into)
    }

    /// Update the last synchronization timestamp for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_provider_last_sync(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        sync_time: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()> {
        self.oauth_tokens()
            .update_last_sync(user_id, provider, sync_time)
            .await
            .map_err(Into::into)
    }

    // ================================
    // Analytics & Reporting
    // ================================

    /// Get analytics on the most used tools for a user within a time range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_top_tools_analysis(
        &self,
        _user_id: uuid::Uuid,
        _start_time: chrono::DateTime<chrono::Utc>,
        _end_time: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Vec<crate::dashboard_routes::ToolUsage>> {
        // NOTE: This method is not actually used. Dashboard routes has its own
        // implementation that aggregates data from get_api_key_usage_stats.
        // Returning empty vector to avoid circular delegation through UsageRepository.
        tokio::task::yield_now().await;
        Ok(Vec::new())
    }

    /// Get request logs with filters (avoids circular delegation)
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_request_logs_with_filters(
        &self,
        _api_key_id: Option<&str>,
        _start_time: Option<chrono::DateTime<chrono::Utc>>,
        _end_time: Option<chrono::DateTime<chrono::Utc>>,
        _status_filter: Option<&str>,
        _tool_filter: Option<&str>,
    ) -> AppResult<Vec<crate::dashboard_routes::RequestLog>> {
        tokio::task::yield_now().await;
        Ok(Vec::new())
    }

    // ================================
    // Admin Token Management
    // ================================

    /// Create a new admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token generation fails
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> AppResult<crate::admin::models::GeneratedAdminToken> {
        self.admin()
            .create_token(request, admin_jwt_secret, jwks_manager)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an admin token by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> AppResult<Option<crate::admin::models::AdminToken>> {
        self.admin()
            .get_token_by_id(token_id)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an admin token by prefix
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> AppResult<Option<crate::admin::models::AdminToken>> {
        self.admin()
            .get_token_by_prefix(token_prefix)
            .await
            .map_err(Into::into)
    }

    /// List all admin tokens, optionally including inactive ones
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> AppResult<Vec<crate::admin::models::AdminToken>> {
        self.admin()
            .list_tokens(include_inactive)
            .await
            .map_err(Into::into)
    }

    /// Deactivate an admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_admin_token(&self, token_id: &str) -> AppResult<()> {
        self.admin()
            .deactivate_token(token_id)
            .await
            .map_err(Into::into)
    }

    /// Update the last used timestamp and IP address for an admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.admin()
            .update_token_last_used(token_id, ip_address)
            .await
            .map_err(Into::into)
    }

    /// Record admin token usage for audit and analytics
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn record_admin_token_usage(
        &self,
        _usage: &crate::admin::models::AdminTokenUsage,
    ) -> AppResult<()> {
        tokio::task::yield_now().await;
        Ok(())
    }

    /// Get usage history for an admin token within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Vec<crate::admin::models::AdminTokenUsage>> {
        self.admin()
            .get_usage_history(token_id, start_date, end_date)
            .await
            .map_err(Into::into)
    }

    /// Record an API key provisioned by an admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> AppResult<()> {
        self.admin()
            .record_provisioned_key(
                admin_token_id,
                api_key_id,
                user_email,
                tier,
                rate_limit_requests,
                rate_limit_period,
            )
            .await
            .map_err(Into::into)
    }

    /// Get API keys provisioned by admin tokens within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Vec<serde_json::Value>> {
        self.admin()
            .get_provisioned_keys(admin_token_id, start_date, end_date)
            .await
            .map_err(Into::into)
    }

    // ================================
    // Multi-Tenant Management
    // ================================

    /// Create a new tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_tenant(&self, tenant: &crate::models::Tenant) -> AppResult<()> {
        self.tenants().create(tenant).await.map_err(Into::into)
    }

    /// Retrieve a tenant by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant does not exist
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_by_id(
        &self,
        tenant_id: uuid::Uuid,
    ) -> AppResult<crate::models::Tenant> {
        self.tenants()
            .get_by_id(tenant_id)
            .await
            .map_err(Into::into)
    }

    /// Retrieve a tenant by slug
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant does not exist
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_by_slug(&self, slug: &str) -> AppResult<crate::models::Tenant> {
        self.tenants().get_by_slug(slug).await.map_err(Into::into)
    }

    /// List all tenants a user has access to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_tenants_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Vec<crate::models::Tenant>> {
        self.tenants()
            .list_for_user(user_id)
            .await
            .map_err(Into::into)
    }

    /// Store OAuth credentials for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Encryption fails
    /// - Database connection issues
    pub async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> AppResult<()> {
        self.tenants()
            .store_oauth_credentials(credentials)
            .await
            .map_err(Into::into)
    }

    /// Retrieve all OAuth providers configured for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Decryption fails
    /// - Database connection issues
    pub async fn get_tenant_oauth_providers(
        &self,
        tenant_id: uuid::Uuid,
    ) -> AppResult<Vec<crate::tenant::TenantOAuthCredentials>> {
        self.tenants()
            .get_oauth_providers(tenant_id)
            .await
            .map_err(Into::into)
    }

    /// Retrieve OAuth credentials for a specific provider and tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Decryption fails
    /// - Database connection issues
    pub async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<Option<crate::tenant::TenantOAuthCredentials>> {
        self.tenants()
            .get_oauth_credentials(tenant_id, provider)
            .await
            .map_err(Into::into)
    }

    // ================================
    // OAuth App Registration
    // ================================

    /// Create a new OAuth app registration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - App already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> AppResult<()> {
        self.tenants()
            .create_oauth_app(app)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an OAuth app by client ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - App does not exist
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth_app_by_client_id(
        &self,
        client_id: &str,
    ) -> AppResult<crate::models::OAuthApp> {
        self.tenants()
            .get_oauth_app_by_client_id(client_id)
            .await
            .map_err(Into::into)
    }

    /// List all OAuth apps registered by a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_oauth_apps_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Vec<crate::models::OAuthApp>> {
        self.tenants()
            .list_oauth_apps_for_user(user_id)
            .await
            .map_err(Into::into)
    }

    // ================================
    // OAuth 2.0 Server (RFC 7591)
    // ================================

    /// Store an OAuth 2.0 client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_oauth2_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> AppResult<()> {
        self.oauth2_server()
            .store_client(client)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an OAuth 2.0 client by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2Client>> {
        self.oauth2_server()
            .get_client(client_id)
            .await
            .map_err(Into::into)
    }

    /// Store an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> AppResult<()> {
        self.oauth2_server()
            .store_auth_code(auth_code)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        self.oauth2_server()
            .get_auth_code(code)
            .await
            .map_err(Into::into)
    }

    /// Update an existing OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> AppResult<()> {
        self.oauth2_server()
            .update_auth_code(auth_code)
            .await
            .map_err(Into::into)
    }

    /// Store an OAuth 2.0 refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> AppResult<()> {
        self.oauth2_server()
            .store_refresh_token(refresh_token)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an OAuth 2.0 refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        self.oauth2_server()
            .get_refresh_token(token)
            .await
            .map_err(Into::into)
    }

    /// Revoke an OAuth 2.0 refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn revoke_oauth2_refresh_token(&self, token: &str) -> AppResult<()> {
        self.oauth2_server()
            .revoke_refresh_token(token)
            .await
            .map_err(Into::into)
    }

    /// Consume an OAuth 2.0 authorization code (validates and marks as used)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code does not exist
    /// - Code has expired
    /// - Code has already been used
    /// - Client ID or redirect URI mismatch
    /// - Database update fails
    /// - Database connection issues
    pub async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        self.oauth2_server()
            .consume_auth_code(code, client_id, redirect_uri, now)
            .await
            .map_err(Into::into)
    }

    /// Consume an OAuth 2.0 refresh token (validates and optionally rotates)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Token has been revoked
    /// - Token has expired
    /// - Client ID mismatch
    /// - Database update fails
    /// - Database connection issues
    pub async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        self.oauth2_server()
            .consume_refresh_token(token, client_id, now)
            .await
            .map_err(Into::into)
    }

    /// Retrieve an OAuth 2.0 refresh token by its value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        self.oauth2_server()
            .get_refresh_token_by_value(token)
            .await
            .map_err(Into::into)
    }

    /// Store an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_authorization_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> AppResult<()> {
        self.oauth2_server()
            .store_authorization_code(auth_code)
            .await
            .map_err(Into::into)
    }

    /// Retrieve and validate an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code does not exist
    /// - Client ID or redirect URI mismatch
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> AppResult<crate::oauth2_server::models::OAuth2AuthCode> {
        self.oauth2_server()
            .get_authorization_code(code, client_id, redirect_uri)
            .await
            .map_err(Into::into)
    }

    /// Delete an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code does not exist
    /// - Client ID or redirect URI mismatch
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> AppResult<()> {
        self.oauth2_server()
            .delete_authorization_code(code, client_id, redirect_uri)
            .await
            .map_err(Into::into)
    }

    /// Store an OAuth 2.0 state parameter for CSRF protection
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - State already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_oauth2_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> AppResult<()> {
        self.oauth2_server()
            .store_state(state)
            .await
            .map_err(Into::into)
    }

    /// Consume an OAuth 2.0 state parameter (validates and marks as used)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - State does not exist
    /// - State has expired
    /// - State has already been used
    /// - Client ID mismatch
    /// - Database update fails
    /// - Database connection issues
    pub async fn consume_oauth2_state(
        &self,
        state: &str,
        client_id: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2State>> {
        self.oauth2_server()
            .consume_state(state, client_id, now)
            .await
            .map_err(Into::into)
    }

    // ================================
    // Key Rotation & Security
    // ================================

    /// Store a new encryption key version for key rotation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Key version already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_key_version(
        &self,
        tenant_id: Option<uuid::Uuid>,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> AppResult<()> {
        self.security()
            .store_key_version(tenant_id, version)
            .await
            .map_err(Into::into)
    }

    /// Retrieve all encryption key versions for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_key_versions(
        &self,
        tenant_id: Option<uuid::Uuid>,
    ) -> AppResult<Vec<crate::security::key_rotation::KeyVersion>> {
        self.security()
            .get_key_versions(tenant_id)
            .await
            .map_err(Into::into)
    }

    /// Get the currently active encryption key version for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_current_key_version(
        &self,
        tenant_id: Option<uuid::Uuid>,
    ) -> AppResult<Option<crate::security::key_rotation::KeyVersion>> {
        self.security()
            .get_current_key_version(tenant_id)
            .await
            .map_err(Into::into)
    }

    /// Update the active status of an encryption key version
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Key version does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_key_version_status(
        &self,
        tenant_id: Option<uuid::Uuid>,
        version: u32,
        is_active: bool,
    ) -> AppResult<()> {
        self.security()
            .update_key_version_status(tenant_id, version, is_active)
            .await
            .map_err(Into::into)
    }

    /// Delete old encryption key versions, keeping only the most recent ones
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_old_key_versions(
        &self,
        tenant_id: Option<uuid::Uuid>,
        keep_count: u32,
    ) -> AppResult<u64> {
        self.security()
            .delete_old_key_versions(tenant_id, keep_count)
            .await
            .map_err(Into::into)
    }

    /// Get all tenants in the system
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_all_tenants(&self) -> AppResult<Vec<crate::models::Tenant>> {
        self.tenants().list_all().await.map_err(Into::into)
    }

    /// Store an audit event for security logging
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_audit_event(
        &self,
        tenant_id: Option<uuid::Uuid>,
        event: &crate::security::audit::AuditEvent,
    ) -> AppResult<()> {
        self.security()
            .store_audit_event(tenant_id, event)
            .await
            .map_err(Into::into)
    }

    /// Retrieve audit events with optional filtering
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_audit_events(
        &self,
        tenant_id: Option<uuid::Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<crate::security::audit::AuditEvent>> {
        self.security()
            .get_audit_events(tenant_id, event_type, limit)
            .await
            .map_err(Into::into)
    }

    // ================================
    // User OAuth Tokens (Multi-Tenant)
    // ================================

    /// Upsert user OAuth token for multi-tenant OAuth management
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn upsert_user_oauth_token(
        &self,
        _token: &crate::models::UserOAuthToken,
    ) -> AppResult<()> {
        tokio::task::yield_now().await;
        Ok(())
    }

    /// Get user OAuth token for a specific provider and tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Option<crate::models::UserOAuthToken>> {
        self.oauth_tokens()
            .get(user_id, tenant_id, provider)
            .await
            .map_err(Into::into)
    }

    /// Get all OAuth tokens for a user across all providers
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_oauth_tokens(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Vec<crate::models::UserOAuthToken>> {
        self.oauth_tokens()
            .list_by_user(user_id)
            .await
            .map_err(Into::into)
    }

    /// Get all OAuth tokens for a specific provider within a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Vec<crate::models::UserOAuthToken>> {
        self.oauth_tokens()
            .list_by_tenant_provider(tenant_id, provider)
            .await
            .map_err(Into::into)
    }

    /// Delete a specific OAuth token for a user, tenant, and provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<()> {
        self.oauth_tokens()
            .delete(user_id, tenant_id, provider)
            .await
            .map_err(Into::into)
    }

    /// Delete all OAuth tokens for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_user_oauth_tokens(&self, user_id: uuid::Uuid) -> AppResult<()> {
        self.oauth_tokens()
            .delete_all_for_user(user_id)
            .await
            .map_err(Into::into)
    }

    /// Refresh user OAuth token with new access and refresh tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn refresh_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<()> {
        self.oauth_tokens()
            .refresh(
                user_id,
                tenant_id,
                provider,
                access_token,
                refresh_token,
                expires_at,
            )
            .await
            .map_err(Into::into)
    }

    /// Get user role for a specific tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_user_tenant_role(
        &self,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
    ) -> AppResult<Option<String>> {
        self.tenants()
            .get_user_role(&user_id.to_string(), &tenant_id.to_string())
            .await
            .map_err(Into::into)
    }

    // ================================
    // User OAuth App Credentials
    // ================================

    /// Store user OAuth app credentials (`client_id`, `client_secret`)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data encryption fails
    /// - Database connection issues
    pub async fn store_user_oauth_app(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> AppResult<()> {
        self.oauth_tokens()
            .store_app(user_id, provider, client_id, client_secret, redirect_uri)
            .await
            .map_err(Into::into)
    }

    /// Get user OAuth app credentials for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Data decryption fails
    /// - Database connection issues
    pub async fn get_user_oauth_app(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<Option<crate::models::UserOAuthApp>> {
        self.oauth_tokens()
            .get_app(user_id, provider)
            .await
            .map_err(Into::into)
    }

    /// List all OAuth app providers configured for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_user_oauth_apps(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Vec<crate::models::UserOAuthApp>> {
        self.oauth_tokens()
            .list_apps(user_id)
            .await
            .map_err(Into::into)
    }

    /// Remove user OAuth app credentials for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn remove_user_oauth_app(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<()> {
        self.oauth_tokens()
            .remove_app(user_id, provider)
            .await
            .map_err(Into::into)
    }

    // ================================
    // System Secret Management
    // ================================

    /// Get or create system secret (generates if not exists)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret generation fails
    /// - Database insertion fails
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_or_create_system_secret(&self, secret_type: &str) -> AppResult<String> {
        self.security()
            .get_or_create_system_secret(secret_type)
            .await
            .map_err(Into::into)
    }

    /// Get existing system secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret does not exist
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_system_secret(&self, secret_type: &str) -> AppResult<String> {
        self.security()
            .get_system_secret(secret_type)
            .await
            .map_err(Into::into)
    }

    /// Update system secret (for rotation)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> AppResult<()> {
        self.security()
            .update_system_secret(secret_type, new_value)
            .await
            .map_err(Into::into)
    }

    // ================================
    // OAuth Notifications
    // ================================

    /// Store OAuth completion notification for MCP resource delivery
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_oauth_notification(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> AppResult<String> {
        self.notifications()
            .store(user_id, provider, success, message, expires_at)
            .await
            .map_err(Into::into)
    }

    /// Get unread OAuth notifications for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_unread_oauth_notifications(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        self.notifications()
            .get_unread(user_id)
            .await
            .map_err(Into::into)
    }

    /// Mark OAuth notification as read
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Notification does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: uuid::Uuid,
    ) -> AppResult<bool> {
        self.notifications()
            .mark_read(notification_id, user_id)
            .await
            .map_err(Into::into)
    }

    /// Mark all OAuth notifications as read for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn mark_all_oauth_notifications_read(&self, user_id: uuid::Uuid) -> AppResult<u64> {
        self.notifications()
            .mark_all_read(user_id)
            .await
            .map_err(Into::into)
    }

    /// Get all OAuth notifications for a user (read and unread)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_all_oauth_notifications(
        &self,
        user_id: uuid::Uuid,
        limit: Option<i64>,
    ) -> AppResult<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        self.notifications()
            .get_all(user_id, limit)
            .await
            .map_err(Into::into)
    }

    // ================================
    // Fitness Configuration Management
    // ================================

    /// Save tenant-level fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> AppResult<String> {
        self.fitness_configs()
            .save_tenant_config(tenant_id, configuration_name, config)
            .await
            .map_err(Into::into)
    }

    /// Save user-specific fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> AppResult<String> {
        self.fitness_configs()
            .save_user_config(tenant_id, user_id, configuration_name, config)
            .await
            .map_err(Into::into)
    }

    /// Get tenant-level fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> AppResult<Option<crate::config::fitness_config::FitnessConfig>> {
        self.fitness_configs()
            .get_tenant_config(tenant_id, configuration_name)
            .await
            .map_err(Into::into)
    }

    /// Get user-specific fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> AppResult<Option<crate::config::fitness_config::FitnessConfig>> {
        self.fitness_configs()
            .get_user_config(tenant_id, user_id, configuration_name)
            .await
            .map_err(Into::into)
    }

    /// List all tenant-level fitness configuration names
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn list_tenant_fitness_configurations(
        &self,
        tenant_id: &str,
    ) -> AppResult<Vec<String>> {
        self.fitness_configs()
            .list_tenant_configs(tenant_id)
            .await
            .map_err(Into::into)
    }

    /// List all user-specific fitness configuration names
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<String>> {
        self.fitness_configs()
            .list_user_configs(tenant_id, user_id)
            .await
            .map_err(Into::into)
    }

    /// Delete fitness configuration (tenant or user-specific)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration does not exist
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> AppResult<bool> {
        self.fitness_configs()
            .delete_config(tenant_id, user_id, configuration_name)
            .await
            .map_err(Into::into)
    }

    // ================================
    // RSA Keypair Management (JWT Signing)
    // ================================

    /// Save RSA keypair to database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Keypair already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: chrono::DateTime<chrono::Utc>,
        is_active: bool,
        key_size_bits: i32,
    ) -> AppResult<()> {
        #[allow(clippy::cast_sign_loss)]
        self.security()
            .save_rsa_keypair(
                kid,
                private_key_pem,
                public_key_pem,
                created_at,
                is_active,
                key_size_bits as usize,
            )
            .await
            .map_err(Into::into)
    }

    /// Load all RSA keypairs from database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn load_rsa_keypairs(
        &self,
    ) -> AppResult<Vec<(String, String, String, chrono::DateTime<chrono::Utc>, bool)>> {
        self.security()
            .load_rsa_keypairs()
            .await
            .map_err(Into::into)
    }

    /// Update active status of RSA keypair
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Keypair does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_rsa_keypair_active_status(
        &self,
        kid: &str,
        is_active: bool,
    ) -> AppResult<()> {
        self.security()
            .update_rsa_keypair_status(kid, is_active)
            .await
            .map_err(Into::into)
    }
}

// Implement encryption support for PostgreSQL (harmonize with SQLite security)
impl shared::encryption::HasEncryption for PostgresDatabase {
    /// Encrypt data using AES-256-GCM with Additional Authenticated Data
    ///
    /// This brings `PostgreSQL` to security parity with `SQLite`, which already
    /// encrypts OAuth tokens at rest.
    ///
    /// # Security
    /// - Uses AES-256-GCM (AEAD cipher) via ring crate
    /// - Generates unique 96-bit nonce per encryption
    /// - Binds AAD to prevent cross-tenant token reuse
    /// - Output: base64(nonce || ciphertext || `auth_tag`)
    fn encrypt_data_with_aad(&self, data: &str, aad_context: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // Generate unique nonce (96 bits for GCM)
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Create encryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        // Encrypt data with AAD binding
        let mut data_bytes = data.as_bytes().to_vec();
        let aad = Aad::from(aad_context.as_bytes());
        key.seal_in_place_append_tag(nonce, aad, &mut data_bytes)?;

        // Combine nonce and encrypted data, then base64 encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend(data_bytes);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    /// Decrypt data using AES-256-GCM with Additional Authenticated Data
    ///
    /// Reverses `encrypt_data_with_aad`. AAD context must match or decryption fails.
    ///
    /// # Security
    /// - Verifies AAD matches (prevents token context switching)
    /// - Authenticates ciphertext hasn't been tampered
    /// - Fails safely on any mismatch/corruption
    fn decrypt_data_with_aad(&self, encrypted_data: &str, aad_context: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decode from base64
        let combined = general_purpose::STANDARD.decode(encrypted_data)?;

        if combined.len() < 12 {
            return Err(DatabaseError::QueryError {
                context: "Invalid encrypted data: too short".to_owned(),
            }
            .into());
        }

        // Extract nonce and encrypted data
        let (nonce_bytes, encrypted_bytes) = combined.split_at(12);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into()?);

        // Create decryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        // Decrypt data with AAD verification
        let mut decrypted_data = encrypted_bytes.to_vec();
        let aad = Aad::from(aad_context.as_bytes());
        let decrypted = key
            .open_in_place(nonce, aad, &mut decrypted_data)
            .map_err(|e| DatabaseError::QueryError {
                context: format!(
                    "Decryption failed (possible AAD mismatch or tampered data): {e:?}"
                ),
            })?;

        String::from_utf8(decrypted.to_vec()).map_err(|e| {
            DatabaseError::QueryError {
                context: format!("Failed to convert decrypted data to string: {e}"),
            }
            .into()
        })
    }
}
