// ABOUTME: Database abstraction layer for Pierre MCP Server
// ABOUTME: Plugin architecture for database support with SQLite and PostgreSQL backends
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::admin::jwks::JwksManager;
use crate::admin::models::{
    AdminToken, AdminTokenUsage, CreateAdminTokenRequest, GeneratedAdminToken,
};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::config::fitness::FitnessConfig;
use crate::dashboard_routes::{RequestLog, ToolUsage};
use crate::database::{
    ConversationRecord, ConversationSummary, CreateUserMcpTokenRequest, MessageRecord,
    UserMcpToken, UserMcpTokenCreated, UserMcpTokenInfo,
};
use crate::errors::AppResult;
use crate::models::OAuthNotification;
use crate::models::{
    AuthorizationCode, ConnectionType, OAuthApp, ProviderConnection, Tenant, TenantPlan,
    TenantToolOverride, ToolCatalogEntry, ToolCategory, User, UserOAuthApp, UserOAuthToken,
    UserStatus,
};
use crate::oauth2_client::OAuthClientState;
use crate::oauth2_server::models::{OAuth2AuthCode, OAuth2Client, OAuth2RefreshToken, OAuth2State};
use crate::pagination::{CursorPage, PaginationParams};
use crate::permissions::impersonation::ImpersonationSession;
use crate::rate_limiting::JwtUsage;
use crate::security::audit::AuditEvent;
use crate::security::key_rotation::KeyVersion;
use crate::tenant::llm_manager::{LlmCredentialRecord, LlmCredentialSummary};
use crate::tenant::oauth_manager::TenantOAuthCredentials;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

// Re-export the A2A types from the main database module

/// A2A usage tracking record
pub use crate::database::A2AUsage;
/// A2A usage statistics
pub use crate::database::A2AUsageStats;

/// Database provider factory
pub mod factory;
// Phase 3: sqlite.rs wrapper eliminated - Database now implements DatabaseProvider directly
// pub mod sqlite;

/// PostgreSQL database implementation
#[cfg(feature = "postgresql")]
pub mod postgres;

/// Shared database logic (enum conversions, validation, mappers, encryption, etc.)
pub mod shared;

/// Core database abstraction trait
///
/// All database implementations must implement this trait to provide
/// a consistent interface for the application layer.
#[async_trait]
pub trait DatabaseProvider: Send + Sync + Clone {
    /// Create a new database connection with encryption key
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> AppResult<Self>
    where
        Self: Sized;

    /// Run database migrations to set up schema
    async fn migrate(&self) -> AppResult<()>;

    // ================================
    // User Management
    // ================================

    /// Create a new user account
    async fn create_user(&self, user: &User) -> AppResult<Uuid>;

    /// Get user by ID
    async fn get_user(&self, user_id: Uuid) -> AppResult<Option<User>>;

    /// Get user by email address
    async fn get_user_by_email(&self, email: &str) -> AppResult<Option<User>>;

    /// Get user by email (required - fails if not found)
    async fn get_user_by_email_required(&self, email: &str) -> AppResult<User>;

    /// Get user by Firebase UID
    async fn get_user_by_firebase_uid(&self, firebase_uid: &str) -> AppResult<Option<User>>;

    /// Update user's last active timestamp
    async fn update_last_active(&self, user_id: Uuid) -> AppResult<()>;

    /// Get total number of users
    async fn get_user_count(&self) -> AppResult<i64>;

    /// Get users by status (pending, active, suspended), optionally scoped to a tenant
    async fn get_users_by_status(
        &self,
        status: &str,
        tenant_id: Option<Uuid>,
    ) -> AppResult<Vec<User>>;

    /// Get users by status with cursor-based pagination
    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<User>>;

    /// Update user status and approval information
    ///
    /// # Arguments
    /// * `user_id` - The user to update
    /// * `new_status` - The new status to set
    /// * `approved_by` - UUID of the admin user who approved (None for service token approvals)
    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: UserStatus,
        approved_by: Option<Uuid>,
    ) -> AppResult<User>;

    /// Update user's `tenant_id` to link them to a tenant (`tenant_id` should be UUID string)
    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> AppResult<()>;

    /// Update user's password hash
    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> AppResult<()>;

    /// Update user's display name
    async fn update_user_display_name(&self, user_id: Uuid, display_name: &str) -> AppResult<User>;

    /// Delete a user and all associated data
    ///
    /// This permanently removes the user from the database.
    /// Associated data (tokens, conversations, etc.) are cascade deleted.
    async fn delete_user(&self, user_id: Uuid) -> AppResult<()>;

    /// Get the first admin user by creation date
    ///
    /// Used for system seeding to associate with a valid admin user
    async fn get_first_admin_user(&self) -> AppResult<Option<User>>;

    // ================================
    // User OAuth Tokens (Multi-Tenant)
    // ================================

    /// Store or update user OAuth token for a tenant-provider combination
    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> AppResult<()>;

    /// Get user OAuth token for a specific tenant-provider combination
    async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Option<UserOAuthToken>>;

    /// Get all OAuth tokens for a user, optionally scoped to a specific tenant
    async fn get_user_oauth_tokens(
        &self,
        user_id: Uuid,
        tenant_id: Option<&str>,
    ) -> AppResult<Vec<UserOAuthToken>>;

    /// Get all OAuth tokens for a tenant-provider combination
    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Vec<UserOAuthToken>>;

    /// Delete user OAuth token for a tenant-provider combination
    async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<()>;

    /// Delete all OAuth tokens for a user within a tenant scope
    async fn delete_user_oauth_tokens(&self, user_id: Uuid, tenant_id: &str) -> AppResult<()>;

    /// Update OAuth token expiration and refresh info
    async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> AppResult<()>;

    // ================================
    // User OAuth App Credentials
    // ================================

    /// Store user OAuth app credentials (`client_id`, `client_secret`)
    async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> AppResult<()>;

    /// Get user OAuth app credentials for a provider
    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<UserOAuthApp>>;

    /// List all OAuth app providers configured for a user
    async fn list_user_oauth_apps(&self, user_id: Uuid) -> AppResult<Vec<UserOAuthApp>>;

    /// Remove user OAuth app credentials for a provider
    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> AppResult<()>;

    // ================================
    // User Profiles & Goals
    // ================================

    /// Upsert user profile data
    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> AppResult<()>;

    /// Get user profile data
    async fn get_user_profile(&self, user_id: Uuid) -> AppResult<Option<Value>>;

    /// Create a new goal for a user
    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> AppResult<String>;

    /// Get all goals for a user
    async fn get_user_goals(&self, user_id: Uuid) -> AppResult<Vec<Value>>;

    /// Update progress on a goal, scoped to the owning user
    async fn update_goal_progress(
        &self,
        goal_id: &str,
        user_id: Uuid,
        current_value: f64,
    ) -> AppResult<()>;

    /// Get user configuration data
    async fn get_user_configuration(&self, user_id: &str) -> AppResult<Option<String>>;

    /// Save user configuration data
    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> AppResult<()>;

    // ================================
    // Insights & Analytics
    // ================================

    /// Store an AI-generated insight
    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> AppResult<String>;

    /// Get insights for a user
    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<Value>>;

    // ================================
    // API Key Management
    // ================================

    /// Create a new API key
    async fn create_api_key(&self, api_key: &ApiKey) -> AppResult<()>;

    /// Get API key by its prefix and hash
    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> AppResult<Option<ApiKey>>;

    /// Get all API keys for a user
    async fn get_user_api_keys(&self, user_id: Uuid) -> AppResult<Vec<ApiKey>>;

    /// Update API key last used timestamp
    async fn update_api_key_last_used(&self, api_key_id: &str) -> AppResult<()>;

    /// Deactivate an API key
    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> AppResult<()>;

    /// Get API key by ID, optionally scoped to a specific user for ownership enforcement
    async fn get_api_key_by_id(
        &self,
        api_key_id: &str,
        user_id: Option<Uuid>,
    ) -> AppResult<Option<ApiKey>>;

    /// Get API keys with optional filters
    async fn get_api_keys_filtered(
        &self,
        user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> AppResult<Vec<ApiKey>>;

    /// Clean up expired API keys
    async fn cleanup_expired_api_keys(&self) -> AppResult<u64>;

    /// Get expired API keys
    async fn get_expired_api_keys(&self) -> AppResult<Vec<ApiKey>>;

    /// Record API key usage
    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> AppResult<()>;

    /// Get current usage count for an API key
    async fn get_api_key_current_usage(&self, api_key_id: &str) -> AppResult<u32>;

    /// Get usage statistics for an API key
    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<ApiKeyUsageStats>;

    // ================================
    // JWT Usage Tracking
    // ================================

    /// Record JWT token usage for rate limiting and analytics
    async fn record_jwt_usage(&self, usage: &JwtUsage) -> AppResult<()>;

    /// Get current JWT usage count for rate limiting (current month)
    async fn get_jwt_current_usage(&self, user_id: Uuid) -> AppResult<u32>;

    // ================================
    // Request Logs & System Stats
    // ================================

    /// Get request logs with filtering options
    ///
    /// When `user_id` is provided, results are scoped to that user's logs only.
    /// Filters on `api_key_id`, `status_filter`, and `tool_filter` are applied
    /// in the database query to avoid returning unrelated data.
    async fn get_request_logs(
        &self,
        user_id: Option<Uuid>,
        api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> AppResult<Vec<RequestLog>>;

    /// Get system statistics, optionally scoped to a tenant
    async fn get_system_stats(&self, tenant_id: Option<Uuid>) -> AppResult<(u64, u64)>;

    // ================================
    // A2A (Agent-to-Agent) Support
    // ================================

    /// Create a new A2A client
    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> AppResult<String>;

    /// Get A2A client by ID
    async fn get_a2a_client(&self, client_id: &str) -> AppResult<Option<A2AClient>>;

    /// Get A2A client by API key ID
    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> AppResult<Option<A2AClient>>;

    /// Get A2A client by name
    async fn get_a2a_client_by_name(&self, name: &str) -> AppResult<Option<A2AClient>>;

    /// List all A2A clients for a user
    async fn list_a2a_clients(&self, user_id: &Uuid) -> AppResult<Vec<A2AClient>>;

    /// Deactivate an A2A client
    async fn deactivate_a2a_client(&self, client_id: &str) -> AppResult<()>;

    /// Get client credentials for authentication
    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> AppResult<Option<(String, String)>>;

    /// Invalidate all active sessions for a client
    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> AppResult<()>;

    /// Deactivate all API keys associated with a client
    async fn deactivate_client_api_keys(&self, client_id: &str) -> AppResult<()>;

    /// Create a new A2A session
    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> AppResult<String>;

    /// Get A2A session by token
    async fn get_a2a_session(&self, session_token: &str) -> AppResult<Option<A2ASession>>;

    /// Update A2A session activity timestamp
    async fn update_a2a_session_activity(&self, session_token: &str) -> AppResult<()>;

    /// Get active sessions for a specific client
    async fn get_active_a2a_sessions(&self, client_id: &str) -> AppResult<Vec<A2ASession>>;

    /// Create a new A2A task
    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> AppResult<String>;

    /// Get A2A task by ID
    async fn get_a2a_task(&self, task_id: &str) -> AppResult<Option<A2ATask>>;

    /// List A2A tasks for a client with optional filtering
    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<A2ATask>>;

    /// Update A2A task status
    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> AppResult<()>;

    /// Record A2A usage for analytics
    async fn record_a2a_usage(&self, usage: &A2AUsage) -> AppResult<()>;

    /// Get current A2A usage count for a client
    async fn get_a2a_client_current_usage(&self, client_id: &str) -> AppResult<u32>;

    /// Get A2A usage statistics for a client
    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<A2AUsageStats>;

    /// Get A2A client usage history
    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> AppResult<Vec<(DateTime<Utc>, u32, u32)>>;

    // ================================
    // Provider Sync Tracking
    // ================================

    /// Get last sync timestamp for a provider within a specific tenant
    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Option<DateTime<Utc>>>;

    /// Update last sync timestamp for a provider within a specific tenant
    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> AppResult<()>;

    // ================================
    // Analytics & Intelligence
    // ================================

    /// Get top tools analysis for dashboard
    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AppResult<Vec<ToolUsage>>;

    // ================================
    // Admin Token Management
    // ================================

    /// Create a new admin token
    async fn create_admin_token(
        &self,
        request: &CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &JwksManager,
    ) -> AppResult<GeneratedAdminToken>;

    /// Get admin token by ID
    async fn get_admin_token_by_id(&self, token_id: &str) -> AppResult<Option<AdminToken>>;

    /// Get admin token by prefix for fast lookup
    async fn get_admin_token_by_prefix(&self, token_prefix: &str) -> AppResult<Option<AdminToken>>;

    /// List all admin tokens (super admin only)
    async fn list_admin_tokens(&self, include_inactive: bool) -> AppResult<Vec<AdminToken>>;

    /// Deactivate admin token
    async fn deactivate_admin_token(&self, token_id: &str) -> AppResult<()>;

    /// Update admin token last used timestamp
    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()>;

    /// Record admin token usage for audit trail
    async fn record_admin_token_usage(&self, usage: &AdminTokenUsage) -> AppResult<()>;

    /// Get admin token usage history
    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<AdminTokenUsage>>;

    /// Record API key provisioning by admin
    async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> AppResult<()>;

    /// Get admin provisioned keys history
    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<serde_json::Value>>;

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
        key_size_bits: i32,
    ) -> AppResult<()>;

    /// Load all RSA keypairs from database
    async fn load_rsa_keypairs(
        &self,
    ) -> AppResult<Vec<(String, String, String, DateTime<Utc>, bool)>>;

    /// Update active status of RSA keypair
    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> AppResult<()>;

    // ================================
    // User MCP Tokens (AI Client Authentication)
    // ================================

    /// Create a new user MCP token for AI client authentication
    async fn create_user_mcp_token(
        &self,
        user_id: Uuid,
        request: &CreateUserMcpTokenRequest,
    ) -> AppResult<UserMcpTokenCreated>;

    /// Validate a user MCP token and return the associated user ID
    async fn validate_user_mcp_token(&self, token_value: &str) -> AppResult<Uuid>;

    /// List all MCP tokens for a user
    async fn list_user_mcp_tokens(&self, user_id: Uuid) -> AppResult<Vec<UserMcpTokenInfo>>;

    /// Revoke a user MCP token
    async fn revoke_user_mcp_token(&self, token_id: &str, user_id: Uuid) -> AppResult<()>;

    /// Get a user MCP token by ID
    async fn get_user_mcp_token(
        &self,
        token_id: &str,
        user_id: Uuid,
    ) -> AppResult<Option<UserMcpToken>>;

    /// Cleanup expired user MCP tokens (mark as revoked)
    async fn cleanup_expired_user_mcp_tokens(&self) -> AppResult<u64>;

    // ================================
    // Multi-Tenant Management
    // ================================

    /// Create a new tenant
    async fn create_tenant(&self, tenant: &Tenant) -> AppResult<()>;

    /// Get tenant by ID
    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> AppResult<Tenant>;

    /// Get tenant by slug
    async fn get_tenant_by_slug(&self, slug: &str) -> AppResult<Tenant>;

    /// List tenants for a user
    async fn list_tenants_for_user(&self, user_id: Uuid) -> AppResult<Vec<Tenant>>;

    /// Store tenant OAuth credentials
    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &TenantOAuthCredentials,
    ) -> AppResult<()>;

    /// Get tenant OAuth providers
    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> AppResult<Vec<TenantOAuthCredentials>>;

    /// Get tenant OAuth credentials for specific provider
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<TenantOAuthCredentials>>;

    // ================================
    // OAuth App Registration
    // ================================

    /// Create OAuth application for MCP clients
    async fn create_oauth_app(&self, app: &OAuthApp) -> AppResult<()>;

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> AppResult<OAuthApp>;

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(&self, user_id: Uuid) -> AppResult<Vec<OAuthApp>>;

    // ================================
    // OAuth 2.0 Server (RFC 7591)
    // ================================

    /// Store OAuth 2.0 client registration
    async fn store_oauth2_client(&self, client: &OAuth2Client) -> AppResult<()>;

    /// Get OAuth 2.0 client by `client_id`
    async fn get_oauth2_client(&self, client_id: &str) -> AppResult<Option<OAuth2Client>>;

    /// Store OAuth 2.0 authorization code
    async fn store_oauth2_auth_code(&self, auth_code: &OAuth2AuthCode) -> AppResult<()>;

    /// Get OAuth 2.0 authorization code
    async fn get_oauth2_auth_code(&self, code: &str) -> AppResult<Option<OAuth2AuthCode>>;

    /// Update OAuth 2.0 authorization code (mark as used)
    async fn update_oauth2_auth_code(&self, auth_code: &OAuth2AuthCode) -> AppResult<()>;

    /// Store OAuth 2.0 refresh token
    async fn store_oauth2_refresh_token(&self, refresh_token: &OAuth2RefreshToken)
        -> AppResult<()>;

    /// Get OAuth 2.0 refresh token
    async fn get_oauth2_refresh_token(&self, token: &str) -> AppResult<Option<OAuth2RefreshToken>>;

    /// Revoke OAuth 2.0 refresh token
    async fn revoke_oauth2_refresh_token(&self, token: &str) -> AppResult<()>;

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
    /// * `client_id` - Expected `client_id` (validation)
    /// * `redirect_uri` - Expected `redirect_uri` (validation)
    /// * `now` - Current timestamp for expiration check
    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuth2AuthCode>>;

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
    /// * `client_id` - Expected `client_id` (validation)
    /// * `now` - Current timestamp for expiration check
    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuth2RefreshToken>>;

    /// Look up a refresh token by its value (without `client_id` constraint)
    ///
    /// This is used by the validate-and-refresh endpoint where we only have the token value
    /// and need to look it up to verify ownership.
    ///
    /// # Arguments
    /// * `token` - The refresh token value to look up
    ///
    /// # Returns
    /// The refresh token data if found, None if not found
    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> AppResult<Option<OAuth2RefreshToken>>;

    /// Store authorization code
    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> AppResult<()>;

    /// Get authorization code data
    async fn get_authorization_code(&self, code: &str) -> AppResult<AuthorizationCode>;

    /// Delete authorization code (after use)
    async fn delete_authorization_code(&self, code: &str) -> AppResult<()>;

    /// Store `OAuth2` state for CSRF protection
    async fn store_oauth2_state(&self, state: &OAuth2State) -> AppResult<()>;

    /// Consume `OAuth2` state (atomically check and mark as used)
    ///
    /// # Arguments
    /// * `state_value` - The state parameter to consume
    /// * `client_id` - Expected `client_id` (validation)
    /// * `now` - Current timestamp for expiration check
    async fn consume_oauth2_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuth2State>>;

    // ================================
    // OAuth Client State (CSRF + PKCE)
    // ================================

    /// Store OAuth client-side state for CSRF protection and PKCE verifier storage
    ///
    /// Used when Pierre acts as an OAuth client connecting to external providers.
    async fn store_oauth_client_state(&self, state: &OAuthClientState) -> AppResult<()>;

    /// Consume OAuth client state atomically (verify and mark as used)
    ///
    /// Returns the state if valid, not expired, and not already used.
    async fn consume_oauth_client_state(
        &self,
        state_value: &str,
        provider: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuthClientState>>;

    // ================================
    // Key Rotation & Security
    // ================================

    /// Store key version metadata
    async fn store_key_version(&self, version: &KeyVersion) -> AppResult<()>;

    /// Get all key versions for a tenant
    async fn get_key_versions(&self, tenant_id: Option<Uuid>) -> AppResult<Vec<KeyVersion>>;

    /// Get current active key version for a tenant
    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> AppResult<Option<KeyVersion>>;

    /// Update key version status (activate/deactivate)
    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> AppResult<()>;

    /// Delete old key versions
    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> AppResult<u64>;

    /// Get all tenants for key rotation check
    async fn get_all_tenants(&self) -> AppResult<Vec<Tenant>>;

    /// Store audit event
    async fn store_audit_event(&self, event: &AuditEvent) -> AppResult<()>;

    /// Get audit events with filters
    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<AuditEvent>>;

    // ================================
    // Tenant User Management
    // ================================

    /// Get user role for a specific tenant
    async fn get_user_tenant_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> AppResult<Option<String>>;

    // ================================
    // System Secret Management
    // ================================

    /// Get or create system secret (generates if not exists)
    async fn get_or_create_system_secret(&self, secret_type: &str) -> AppResult<String>;

    /// Get existing system secret
    async fn get_system_secret(&self, secret_type: &str) -> AppResult<String>;

    /// Update system secret (for rotation)
    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> AppResult<()>;

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
    ) -> AppResult<String>;

    /// Get unread OAuth notifications for a user
    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> AppResult<Vec<OAuthNotification>>;

    /// Mark OAuth notification as read
    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> AppResult<bool>;

    /// Mark all OAuth notifications as read for a user
    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> AppResult<u64>;

    /// Get all OAuth notifications for a user (read and unread)
    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> AppResult<Vec<OAuthNotification>>;

    // ================================
    // Fitness Configuration Management
    // ================================

    /// Save tenant-level fitness configuration
    async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &FitnessConfig,
    ) -> AppResult<String>;

    /// Save user-specific fitness configuration
    async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &FitnessConfig,
    ) -> AppResult<String>;

    /// Get tenant-level fitness configuration
    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> AppResult<Option<FitnessConfig>>;

    /// Get user-specific fitness configuration
    async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> AppResult<Option<FitnessConfig>>;

    /// List all tenant-level fitness configuration names
    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> AppResult<Vec<String>>;

    /// List all user-specific fitness configuration names
    async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<String>>;

    /// Delete fitness configuration (tenant or user-specific)
    async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> AppResult<bool>;

    // ================================
    // Impersonation Session Management
    // ================================

    /// Create a new impersonation session for audit trail
    async fn create_impersonation_session(&self, session: &ImpersonationSession) -> AppResult<()>;

    /// Get impersonation session by ID
    async fn get_impersonation_session(
        &self,
        session_id: &str,
    ) -> AppResult<Option<ImpersonationSession>>;

    /// Get active impersonation session where user is impersonator or target
    async fn get_active_impersonation_session(
        &self,
        user_id: Uuid,
    ) -> AppResult<Option<ImpersonationSession>>;

    /// End an impersonation session
    async fn end_impersonation_session(&self, session_id: &str) -> AppResult<()>;

    /// End all active impersonation sessions for an impersonator
    async fn end_all_impersonation_sessions(&self, impersonator_id: Uuid) -> AppResult<u64>;

    /// List impersonation sessions with optional filters
    async fn list_impersonation_sessions(
        &self,
        impersonator_id: Option<Uuid>,
        target_user_id: Option<Uuid>,
        active_only: bool,
        limit: u32,
    ) -> AppResult<Vec<ImpersonationSession>>;

    // ================================
    // LLM Credentials Management
    // ================================

    /// Store LLM credentials (user-specific or tenant-level)
    async fn store_llm_credentials(&self, record: &LlmCredentialRecord) -> AppResult<()>;

    /// Get LLM credentials for a specific provider
    ///
    /// # Arguments
    /// * `tenant_id` - Tenant ID
    /// * `user_id` - User ID (None for tenant-level default)
    /// * `provider` - LLM provider name (e.g., "gemini", "groq")
    async fn get_llm_credentials(
        &self,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        provider: &str,
    ) -> AppResult<Option<LlmCredentialRecord>>;

    /// List all LLM credentials for a tenant (for admin UI)
    async fn list_llm_credentials(&self, tenant_id: Uuid) -> AppResult<Vec<LlmCredentialSummary>>;

    /// Delete LLM credentials
    async fn delete_llm_credentials(
        &self,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        provider: &str,
    ) -> AppResult<bool>;

    /// Get admin config override value by key (for system-wide LLM API keys)
    async fn get_admin_config_override(
        &self,
        config_key: &str,
        tenant_id: Option<&str>,
    ) -> AppResult<Option<String>>;

    // ================================
    // Encryption Interface
    // ================================

    /// Encrypt data with AAD (Additional Authenticated Data)
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails (e.g., invalid key, nonce generation failure)
    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> AppResult<String>;

    /// Decrypt data with AAD
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails (e.g., invalid data, AAD mismatch, tampered data)
    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> AppResult<String>;

    // ================================
    // Tool Selection
    // ================================

    /// Get the complete tool catalog
    async fn get_tool_catalog(&self) -> AppResult<Vec<ToolCatalogEntry>>;

    /// Get a specific tool catalog entry by name
    async fn get_tool_catalog_entry(&self, tool_name: &str) -> AppResult<Option<ToolCatalogEntry>>;

    /// Get tools filtered by category
    async fn get_tools_by_category(
        &self,
        category: ToolCategory,
    ) -> AppResult<Vec<ToolCatalogEntry>>;

    /// Get tools available for a specific plan level
    async fn get_tools_by_min_plan(&self, plan: TenantPlan) -> AppResult<Vec<ToolCatalogEntry>>;

    /// Get all tool overrides for a tenant
    async fn get_tenant_tool_overrides(
        &self,
        tenant_id: Uuid,
    ) -> AppResult<Vec<TenantToolOverride>>;

    /// Get a specific tool override for a tenant
    async fn get_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
    ) -> AppResult<Option<TenantToolOverride>>;

    /// Create or update a tool override for a tenant
    async fn upsert_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
        is_enabled: bool,
        enabled_by_user_id: Option<Uuid>,
        reason: Option<String>,
    ) -> AppResult<TenantToolOverride>;

    /// Delete a tool override (revert to catalog default)
    async fn delete_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
    ) -> AppResult<bool>;

    /// Count enabled tools for a tenant
    async fn count_enabled_tools(&self, tenant_id: Uuid) -> AppResult<usize>;

    // ================================
    // Synthetic Provider Support
    // ================================

    /// Check if a user has synthetic activities seeded
    ///
    /// This is used by the providers endpoint to determine if the synthetic
    /// provider should be shown as "connected" for a user.
    async fn user_has_synthetic_activities(&self, user_id: Uuid) -> AppResult<bool>;

    // ================================
    // Provider Connections
    // ================================

    /// Register a provider connection (upsert)
    ///
    /// Creates or updates a record in `provider_connections` for the given user/tenant/provider.
    async fn register_provider_connection(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        connection_type: &ConnectionType,
        metadata: Option<&str>,
    ) -> AppResult<()>;

    /// Remove a provider connection
    async fn remove_provider_connection(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<()>;

    /// Get all provider connections for a user
    ///
    /// When `tenant_id` is None, returns cross-tenant view.
    /// When Some, scopes to that specific tenant.
    async fn get_user_provider_connections(
        &self,
        user_id: Uuid,
        tenant_id: Option<&str>,
    ) -> AppResult<Vec<ProviderConnection>>;

    /// Check if a specific provider is connected for a user (cross-tenant)
    async fn is_provider_connected(&self, user_id: Uuid, provider: &str) -> AppResult<bool>;

    // ================================
    // Chat Conversations & Messages
    // ================================

    /// Create a new chat conversation
    async fn chat_create_conversation(
        &self,
        user_id: &str,
        tenant_id: &str,
        title: &str,
        model: &str,
        system_prompt: Option<&str>,
    ) -> AppResult<ConversationRecord>;

    /// Get a conversation by ID with user/tenant isolation
    async fn chat_get_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<Option<ConversationRecord>>;

    /// List conversations for a user with pagination
    async fn chat_list_conversations(
        &self,
        user_id: &str,
        tenant_id: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<ConversationSummary>>;

    /// Update conversation title
    async fn chat_update_conversation_title(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: &str,
        title: &str,
    ) -> AppResult<bool>;

    /// Delete a conversation and its messages
    async fn chat_delete_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<bool>;

    /// Add a message to a conversation (verifies user owns the conversation)
    async fn chat_add_message(
        &self,
        conversation_id: &str,
        user_id: &str,
        role: &str,
        content: &str,
        token_count: Option<u32>,
        finish_reason: Option<&str>,
    ) -> AppResult<MessageRecord>;

    /// Get all messages for a conversation (verifies user owns the conversation)
    async fn chat_get_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<MessageRecord>>;

    /// Get recent messages for a conversation (verifies user owns the conversation)
    async fn chat_get_recent_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
        limit: i64,
    ) -> AppResult<Vec<MessageRecord>>;

    /// Get message count for a conversation (verifies user owns the conversation)
    async fn chat_get_message_count(&self, conversation_id: &str, user_id: &str) -> AppResult<i64>;

    /// Delete all conversations for a user
    async fn chat_delete_all_user_conversations(
        &self,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<i64>;

    // ================================
    // Password Reset Tokens
    // ================================

    /// Store a password reset token (hashed) for a user
    ///
    /// Returns the token ID. The raw token is never stored — only its SHA-256 hash.
    async fn store_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        created_by: &str,
    ) -> AppResult<Uuid>;

    /// Consume a password reset token by its hash
    ///
    /// Returns the `user_id` if the token is valid (exists, not expired, not used).
    /// Marks the token as used atomically.
    async fn consume_password_reset_token(&self, token_hash: &str) -> AppResult<Uuid>;

    /// Invalidate all unused reset tokens for a user
    ///
    /// Called after a successful password change to prevent stale tokens from being used.
    async fn invalidate_user_reset_tokens(&self, user_id: Uuid) -> AppResult<()>;
}
