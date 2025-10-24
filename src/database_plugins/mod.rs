// ABOUTME: Database abstraction layer for Pierre MCP Server
// ABOUTME: Plugin architecture for database support with SQLite and PostgreSQL backends
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::models::{User, UserOAuthApp, UserOAuthToken};
use crate::rate_limiting::JwtUsage;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

// Re-export the A2A types from the main database module
pub use crate::database::{A2AUsage, A2AUsageStats};

pub mod factory;
pub mod sqlite;

#[cfg(feature = "postgresql")]
pub mod postgres;

/// Core database abstraction trait
///
/// All database implementations must implement this trait to provide
/// a consistent interface for the application layer.
#[async_trait]
pub trait DatabaseProvider: Send + Sync + Clone {
    /// Create a new database connection with encryption key
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self>
    where
        Self: Sized;

    /// Run database migrations to set up schema
    async fn migrate(&self) -> Result<()>;

    // ================================
    // User Management
    // ================================

    /// Create a new user account
    async fn create_user(&self, user: &User) -> Result<Uuid>;

    /// Get user by ID
    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>>;

    /// Get user by email address
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;

    /// Get user by email (required - fails if not found)
    async fn get_user_by_email_required(&self, email: &str) -> Result<User>;

    /// Update user's last active timestamp
    async fn update_last_active(&self, user_id: Uuid) -> Result<()>;

    /// Get total number of users
    async fn get_user_count(&self) -> Result<i64>;

    /// Get users by status (pending, active, suspended)
    async fn get_users_by_status(&self, status: &str) -> Result<Vec<User>>;

    /// Get users by status with cursor-based pagination
    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> Result<crate::pagination::CursorPage<User>>;

    /// Update user status and approval information
    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> Result<User>;

    /// Update user's tenant_id to link them to a tenant (tenant_id should be UUID string)
    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> Result<()>;

    // ================================
    // User OAuth Tokens (Multi-Tenant)
    // ================================

    /// Store or update user OAuth token for a tenant-provider combination
    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> Result<()>;

    /// Get user OAuth token for a specific tenant-provider combination
    async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>>;

    /// Get all OAuth tokens for a user across all tenants
    async fn get_user_oauth_tokens(&self, user_id: Uuid) -> Result<Vec<UserOAuthToken>>;

    /// Get all OAuth tokens for a tenant-provider combination
    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>>;

    /// Delete user OAuth token for a tenant-provider combination
    async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<()>;

    /// Delete all OAuth tokens for a user (when user is deleted)
    async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()>;

    /// Update OAuth token expiration and refresh info
    async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()>;

    // ================================
    // User OAuth App Credentials
    // ================================

    /// Store user OAuth app credentials (client_id, client_secret)
    async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<()>;

    /// Get user OAuth app credentials for a provider
    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>>;

    /// List all OAuth app providers configured for a user
    async fn list_user_oauth_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>>;

    /// Remove user OAuth app credentials for a provider
    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()>;

    // ================================
    // User Profiles & Goals
    // ================================

    /// Upsert user profile data
    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> Result<()>;

    /// Get user profile data
    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<Value>>;

    /// Create a new goal for a user
    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String>;

    /// Get all goals for a user
    async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<Value>>;

    /// Update progress on a goal
    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()>;

    /// Get user configuration data
    async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>>;

    /// Save user configuration data
    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()>;

    // ================================
    // Insights & Analytics
    // ================================

    /// Store an AI-generated insight
    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> Result<String>;

    /// Get insights for a user
    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>>;

    // ================================
    // API Key Management
    // ================================

    /// Create a new API key
    async fn create_api_key(&self, api_key: &ApiKey) -> Result<()>;

    /// Get API key by its prefix and hash
    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> Result<Option<ApiKey>>;

    /// Get all API keys for a user
    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>>;

    /// Update API key last used timestamp
    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()>;

    /// Deactivate an API key
    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()>;

    /// Get API key by ID
    async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<ApiKey>>;

    /// Get API keys with optional filters
    async fn get_api_keys_filtered(
        &self,
        user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>>;

    /// Clean up expired API keys
    async fn cleanup_expired_api_keys(&self) -> Result<u64>;

    /// Get expired API keys
    async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>>;

    /// Record API key usage
    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()>;

    /// Get current usage count for an API key
    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32>;

    /// Get usage statistics for an API key
    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats>;

    // ================================
    // JWT Usage Tracking
    // ================================

    /// Record JWT token usage for rate limiting and analytics
    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()>;

    /// Get current JWT usage count for rate limiting (current month)
    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32>;

    // ================================
    // Request Logs & System Stats
    // ================================

    /// Get request logs with filtering options
    async fn get_request_logs(
        &self,
        api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>>;

    /// Get system-wide statistics
    async fn get_system_stats(&self) -> Result<(u64, u64)>;

    // ================================
    // A2A (Agent-to-Agent) Support
    // ================================

    /// Create a new A2A client
    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String>;

    /// Get A2A client by ID
    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>>;

    /// Get A2A client by API key ID
    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> Result<Option<A2AClient>>;

    /// Get A2A client by name
    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>>;

    /// List all A2A clients for a user
    async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>>;

    /// Deactivate an A2A client
    async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()>;

    /// Get client credentials for authentication
    async fn get_a2a_client_credentials(&self, client_id: &str)
        -> Result<Option<(String, String)>>;

    /// Invalidate all active sessions for a client
    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()>;

    /// Deactivate all API keys associated with a client
    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()>;

    /// Create a new A2A session
    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String>;

    /// Get A2A session by token
    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>>;

    /// Update A2A session activity timestamp
    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()>;

    /// Get active sessions for a specific client
    async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>>;

    /// Create a new A2A task
    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String>;

    /// Get A2A task by ID
    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>>;

    /// List A2A tasks for a client with optional filtering
    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>>;

    /// Update A2A task status
    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()>;

    /// Record A2A usage for analytics
    async fn record_a2a_usage(&self, usage: &A2AUsage) -> Result<()>;

    /// Get current A2A usage count for a client
    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32>;

    /// Get A2A usage statistics for a client
    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<A2AUsageStats>;

    /// Get A2A client usage history
    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>>;

    // ================================
    // Provider Sync Tracking
    // ================================

    /// Get last sync timestamp for a provider
    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>>;

    /// Update last sync timestamp for a provider
    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()>;

    // ================================
    // Analytics & Intelligence
    // ================================

    /// Get top tools analysis for dashboard
    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>>;

    // ================================
    // Admin Token Management
    // ================================

    /// Create a new admin token
    async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken>;

    /// Get admin token by ID
    async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>>;

    /// Get admin token by prefix for fast lookup
    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>>;

    /// List all admin tokens (super admin only)
    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>>;

    /// Deactivate admin token
    async fn deactivate_admin_token(&self, token_id: &str) -> Result<()>;

    /// Update admin token last used timestamp
    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<()>;

    /// Record admin token usage for audit trail
    async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<()>;

    /// Get admin token usage history
    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>>;

    /// Record API key provisioning by admin
    async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<()>;

    /// Get admin provisioned keys history
    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>>;

    // ================================
    // RSA Key Persistence for JWT Signing
    // ================================

    /// Save RSA keypair to database for persistence across restarts
    async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
        key_size_bits: usize,
    ) -> Result<()>;

    /// Load all RSA keypairs from database
    async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>, bool)>>;

    /// Update active status of RSA keypair
    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> Result<()>;

    // ================================
    // Multi-Tenant Management
    // ================================

    /// Create a new tenant
    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()>;

    /// Get tenant by ID
    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<crate::models::Tenant>;

    /// Get tenant by slug
    async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant>;

    /// List tenants for a user
    async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>>;

    /// Store tenant OAuth credentials
    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()>;

    /// Get tenant OAuth providers
    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>>;

    /// Get tenant OAuth credentials for specific provider
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>>;

    // ================================
    // OAuth App Registration
    // ================================

    /// Create OAuth application for MCP clients
    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()>;

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp>;

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(&self, user_id: Uuid)
        -> Result<Vec<crate::models::OAuthApp>>;

    // ================================
    // OAuth 2.0 Server (RFC 7591)
    // ================================

    /// Store OAuth 2.0 client registration
    async fn store_oauth2_client(&self, client: &crate::oauth2::models::OAuth2Client)
        -> Result<()>;

    /// Get OAuth 2.0 client by client_id
    async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2Client>>;

    /// Store OAuth 2.0 authorization code
    async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2::models::OAuth2AuthCode,
    ) -> Result<()>;

    /// Get OAuth 2.0 authorization code
    async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2AuthCode>>;

    /// Update OAuth 2.0 authorization code (mark as used)
    async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2::models::OAuth2AuthCode,
    ) -> Result<()>;

    /// Store OAuth 2.0 refresh token
    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2::models::OAuth2RefreshToken,
    ) -> Result<()>;

    /// Get OAuth 2.0 refresh token
    async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2RefreshToken>>;

    /// Revoke OAuth 2.0 refresh token
    async fn revoke_oauth2_refresh_token(&self, token: &str) -> Result<()>;

    /// Atomically consume OAuth 2.0 authorization code (check-and-set in single operation)
    ///
    /// This method prevents race conditions by performing validation and marking as used
    /// in a single atomic database operation using UPDATE...WHERE...RETURNING.
    ///
    /// Returns `Some(auth_code)` if the code was valid, unused, and successfully consumed.
    /// Returns `None` if the code is invalid, already used, expired, or validation failed.
    ///
    /// # Arguments
    /// * `code` - The authorization code to consume
    /// * `client_id` - Expected client_id (validation)
    /// * `redirect_uri` - Expected redirect_uri (validation)
    /// * `now` - Current timestamp for expiration check
    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2::models::OAuth2AuthCode>>;

    /// Atomically consume OAuth 2.0 refresh token (check-and-revoke in single operation)
    ///
    /// This method prevents race conditions by performing validation and revoking
    /// in a single atomic database operation using UPDATE...WHERE...RETURNING.
    ///
    /// Returns `Some(refresh_token)` if the token was valid and successfully consumed.
    /// Returns `None` if the token is invalid, already revoked, or validation failed.
    ///
    /// # Arguments
    /// * `token` - The refresh token to consume
    /// * `client_id` - Expected client_id (validation)
    /// * `now` - Current timestamp for expiration check
    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2::models::OAuth2RefreshToken>>;

    /// Store authorization code
    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> Result<()>;

    /// Get authorization code data
    async fn get_authorization_code(&self, code: &str) -> Result<crate::models::AuthorizationCode>;

    /// Delete authorization code (after use)
    async fn delete_authorization_code(&self, code: &str) -> Result<()>;

    // ================================
    // Key Rotation & Security
    // ================================

    /// Store key version metadata
    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()>;

    /// Get all key versions for a tenant
    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>>;

    /// Get current active key version for a tenant
    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>>;

    /// Update key version status (activate/deactivate)
    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<()>;

    /// Delete old key versions
    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64>;

    /// Get all tenants for key rotation check
    async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>>;

    /// Store audit event
    async fn store_audit_event(&self, event: &crate::security::audit::AuditEvent) -> Result<()>;

    /// Get audit events with filters
    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>>;

    // ================================
    // Tenant User Management
    // ================================

    /// Get user role for a specific tenant
    async fn get_user_tenant_role(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<String>>;

    // ================================
    // System Secret Management
    // ================================

    /// Get or create system secret (generates if not exists)
    async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String>;

    /// Get existing system secret
    async fn get_system_secret(&self, secret_type: &str) -> Result<String>;

    /// Update system secret (for rotation)
    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()>;

    // ================================
    // OAuth Notifications
    // ================================

    /// Store OAuth completion notification for MCP resource delivery
    async fn store_oauth_notification(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String>;

    /// Get unread OAuth notifications for a user
    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>>;

    /// Mark OAuth notification as read
    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool>;

    /// Mark all OAuth notifications as read for a user
    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64>;

    /// Get all OAuth notifications for a user (read and unread)
    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>>;

    // ================================
    // Fitness Configuration Management
    // ================================

    /// Save tenant-level fitness configuration
    async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String>;

    /// Save user-specific fitness configuration
    async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String>;

    /// Get tenant-level fitness configuration
    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>>;

    /// Get user-specific fitness configuration
    async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>>;

    /// List all tenant-level fitness configuration names
    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>>;

    /// List all user-specific fitness configuration names
    async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>>;

    /// Delete fitness configuration (tenant or user-specific)
    async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool>;
}
