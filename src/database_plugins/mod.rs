// ABOUTME: Database abstraction layer for Pierre MCP Server
// ABOUTME: Plugin architecture for database support with SQLite and PostgreSQL backends

use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::models::{DecryptedToken, User};
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

    // ================================
    // OAuth Token Management
    // ================================

    /// Update Strava OAuth tokens for a user
    async fn update_strava_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()>;

    /// Get Strava tokens for a user
    async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>>;

    /// Update Fitbit OAuth tokens for a user
    async fn update_fitbit_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()>;

    /// Get Fitbit tokens for a user
    async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>>;

    /// Clear Strava tokens for a user
    async fn clear_strava_token(&self, user_id: Uuid) -> Result<()>;

    /// Clear Fitbit tokens for a user
    async fn clear_fitbit_token(&self, user_id: Uuid) -> Result<()>;

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

    /// Create OAuth application (for Claude Desktop, ChatGPT, etc.)
    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()>;

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp>;

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(&self, user_id: Uuid)
        -> Result<Vec<crate::models::OAuthApp>>;

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
}
