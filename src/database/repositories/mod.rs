// ABOUTME: Repository trait definitions for database abstraction
// ABOUTME: Breaks down the monolithic DatabaseProvider into cohesive, focused repositories
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::a2a::auth::A2AClient;
use pierre_core::models::TenantId;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::database::{A2AUsage, A2AUsageStats, DatabaseError};
use crate::models::{User, UserOAuthApp, UserOAuthToken, UserStatus};
use crate::pagination::{CursorPage, PaginationParams};
use crate::rate_limiting::JwtUsage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

// Repository implementations
/// Agent-to-Agent repository implementation
pub mod a2a_repository;
/// Admin token management repository implementation
pub mod admin_repository;
/// API key management repository implementation
pub mod api_key_repository;
/// Fitness configuration repository implementation
pub mod fitness_config_repository;
/// AI-generated insights repository implementation
pub mod insight_repository;
/// Recipe storage repository implementation
pub mod recipe_repository;
/// Coaches (custom AI personas) repository implementation
pub mod coaches_repository;
/// OAuth notification repository implementation
pub mod notification_repository;
/// OAuth 2.0 server repository implementation
pub mod oauth2_server_repository;
/// OAuth token storage repository implementation
pub mod oauth_token_repository;
/// User profile repository implementation
pub mod profile_repository;
/// Security and key rotation repository implementation
pub mod security_repository;
/// Multi-tenant management repository implementation
pub mod tenant_repository;
/// Tool selection and per-tenant tool configuration repository implementation
pub mod tool_selection_repository;
/// Usage tracking and analytics repository implementation
pub mod usage_repository;
/// User account management repository implementation
pub mod user_repository;
/// Mobility (stretching/yoga) repository implementation
pub mod mobility_repository;
/// Social features (friend connections, shared insights) repository implementation
pub mod social_repository;

// Re-export implementations
pub use a2a_repository::A2ARepositoryImpl;
pub use admin_repository::AdminRepositoryImpl;
pub use api_key_repository::ApiKeyRepositoryImpl;
pub use fitness_config_repository::FitnessConfigRepositoryImpl;
pub use insight_repository::InsightRepositoryImpl;
pub use notification_repository::NotificationRepositoryImpl;
pub use oauth2_server_repository::OAuth2ServerRepositoryImpl;
pub use oauth_token_repository::OAuthTokenRepositoryImpl;
pub use profile_repository::ProfileRepositoryImpl;
pub use recipe_repository::RecipeRepositoryImpl;
pub use coaches_repository::CoachesRepositoryImpl;
pub use security_repository::SecurityRepositoryImpl;
pub use tenant_repository::TenantRepositoryImpl;
pub use tool_selection_repository::ToolSelectionRepositoryImpl;
pub use usage_repository::UsageRepositoryImpl;
pub use user_repository::UserRepositoryImpl;
pub use mobility_repository::MobilityRepositoryImpl;
pub use social_repository::SocialRepositoryImpl;

// ================================
// Repository Trait Definitions
// ================================

/// User account management repository
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Create a new user account
    async fn create(&self, user: &User) -> Result<Uuid, DatabaseError>;

    /// Get user by ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, DatabaseError>;

    /// Get user by email address
    async fn get_by_email(&self, email: &str) -> Result<Option<User>, DatabaseError>;

    /// Get user by email (required - fails if not found)
    async fn get_by_email_required(&self, email: &str) -> Result<User, DatabaseError>;

    /// Update user's last active timestamp
    async fn update_last_active(&self, id: Uuid) -> Result<(), DatabaseError>;

    /// Get total number of users
    async fn get_count(&self) -> Result<i64, DatabaseError>;

    /// Get users by status (pending, active, suspended), optionally scoped to a tenant
    async fn list_by_status(
        &self,
        status: &str,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<User>, DatabaseError>;

    /// Get users by status with cursor-based pagination
    async fn list_by_status_paginated(
        &self,
        status: &str,
        pagination: &PaginationParams,
    ) -> Result<CursorPage<User>, DatabaseError>;

    /// Update user status and approval information
    ///
    /// # Arguments
    /// * `id` - The user to update
    /// * `new_status` - The new status to set
    /// * `approved_by` - UUID of the admin user who approved (None for service token approvals)
    async fn update_status(
        &self,
        id: Uuid,
        new_status: UserStatus,
        approved_by: Option<Uuid>,
    ) -> Result<User, DatabaseError>;

    /// Update user's `tenant_id` to link them to a tenant
    async fn update_tenant_id(&self, id: Uuid, tenant_id: TenantId) -> Result<(), DatabaseError>;
}

/// OAuth token storage repository (tenant-scoped)
#[async_trait]
pub trait OAuthTokenRepository: Send + Sync {
    /// Store or update user OAuth token for a tenant-provider combination
    async fn upsert(&self, token: &UserOAuthToken) -> Result<(), DatabaseError>;

    /// Get user OAuth token for a specific tenant-provider combination
    async fn get(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>, DatabaseError>;

    /// Get all OAuth tokens for a user, optionally scoped to a tenant
    async fn list_by_user(
        &self,
        user_id: Uuid,
        tenant_id: Option<TenantId>,
    ) -> Result<Vec<UserOAuthToken>, DatabaseError>;

    /// Get all OAuth tokens for a tenant-provider combination
    async fn list_by_tenant_provider(
        &self,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>, DatabaseError>;

    /// Delete user OAuth token for a tenant-provider combination
    async fn delete(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<(), DatabaseError>;

    /// Delete all OAuth tokens for a user within a tenant scope
    async fn delete_all_for_user(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<(), DatabaseError>;

    /// Update OAuth token expiration and refresh info
    async fn refresh(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), DatabaseError>;

    /// Store user OAuth app credentials (`client_id`, `client_secret`)
    async fn store_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<(), DatabaseError>;

    /// Get user OAuth app credentials for a provider
    async fn get_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>, DatabaseError>;

    /// List all OAuth app providers configured for a user
    async fn list_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>, DatabaseError>;

    /// Remove user OAuth app credentials for a provider
    async fn remove_app(&self, user_id: Uuid, provider: &str) -> Result<(), DatabaseError>;

    /// Get last sync timestamp for a provider within a specific tenant
    async fn get_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>, DatabaseError>;

    /// Update last sync timestamp for a provider within a specific tenant
    async fn update_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<(), DatabaseError>;
}

/// API key management repository
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    /// Create a new API key
    async fn create(&self, key: &ApiKey) -> Result<(), DatabaseError>;

    /// Get API key by its prefix and hash
    async fn get_by_prefix(
        &self,
        prefix: &str,
        hash: &str,
    ) -> Result<Option<ApiKey>, DatabaseError>;

    /// Get API key by ID, optionally scoped to a user for ownership enforcement
    async fn get_by_id(
        &self,
        id: &str,
        user_id: Option<Uuid>,
    ) -> Result<Option<ApiKey>, DatabaseError>;

    /// Get all API keys for a user
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<ApiKey>, DatabaseError>;

    /// Get API keys with optional filters
    async fn list_filtered(
        &self,
        user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>, DatabaseError>;

    /// Update API key last used timestamp
    async fn update_last_used(&self, id: &str) -> Result<(), DatabaseError>;

    /// Deactivate an API key
    async fn deactivate(&self, id: &str, user_id: Uuid) -> Result<(), DatabaseError>;

    /// Clean up expired API keys
    async fn cleanup_expired(&self) -> Result<u64, DatabaseError>;

    /// Get expired API keys
    async fn get_expired(&self) -> Result<Vec<ApiKey>, DatabaseError>;
}

/// Usage tracking and analytics repository
#[async_trait]
pub trait UsageRepository: Send + Sync {
    /// Record API key usage
    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<(), DatabaseError>;

    /// Get current usage count for an API key
    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32, DatabaseError>;

    /// Get usage statistics for an API key
    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats, DatabaseError>;

    /// Record JWT token usage for rate limiting and analytics
    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<(), DatabaseError>;

    /// Get current JWT usage count for rate limiting (current month)
    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32, DatabaseError>;

    /// Get request logs with filtering options
    ///
    /// When `user_id` is provided, results are scoped to that user's logs.
    async fn get_request_logs(
        &self,
        user_id: Option<Uuid>,
        api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>, DatabaseError>;

    /// Get system statistics, optionally scoped to a tenant
    async fn get_system_stats(&self, tenant_id: Option<Uuid>) -> Result<(u64, u64), DatabaseError>;

    /// Get top tools analysis for dashboard
    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>, DatabaseError>;
}

/// A2A (Agent-to-Agent) client and session management repository
#[async_trait]
pub trait A2ARepository: Send + Sync {
    /// Create a new A2A client
    async fn create_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String, DatabaseError>;

    /// Get A2A client by ID
    async fn get_client(&self, id: &str) -> Result<Option<A2AClient>, DatabaseError>;

    /// Get A2A client by API key ID
    async fn get_client_by_api_key(
        &self,
        api_key_id: &str,
    ) -> Result<Option<A2AClient>, DatabaseError>;

    /// Get A2A client by name
    async fn get_client_by_name(&self, name: &str) -> Result<Option<A2AClient>, DatabaseError>;

    /// List all A2A clients for a user
    async fn list_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>, DatabaseError>;

    /// Deactivate an A2A client
    async fn deactivate_client(&self, id: &str) -> Result<(), DatabaseError>;

    /// Get client credentials for authentication
    async fn get_client_credentials(
        &self,
        id: &str,
    ) -> Result<Option<(String, String)>, DatabaseError>;

    /// Invalidate all active sessions for a client
    async fn invalidate_client_sessions(&self, client_id: &str) -> Result<(), DatabaseError>;

    /// Deactivate all API keys associated with a client
    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<(), DatabaseError>;

    /// Create a new A2A session
    async fn create_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String, DatabaseError>;

    /// Get A2A session by token
    async fn get_session(&self, token: &str) -> Result<Option<A2ASession>, DatabaseError>;

    /// Update A2A session activity timestamp
    async fn update_session_activity(&self, token: &str) -> Result<(), DatabaseError>;

    /// Get active sessions for a specific client
    async fn get_active_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>, DatabaseError>;

    /// Create a new A2A task
    async fn create_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String, DatabaseError>;

    /// Get A2A task by ID
    async fn get_task(&self, id: &str) -> Result<Option<A2ATask>, DatabaseError>;

    /// List A2A tasks for a client with optional filtering
    async fn list_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>, DatabaseError>;

    /// Update A2A task status
    async fn update_task_status(
        &self,
        id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<(), DatabaseError>;

    /// Record A2A usage for analytics
    async fn record_usage(&self, usage: &A2AUsage) -> Result<(), DatabaseError>;

    /// Get current A2A usage count for a client
    async fn get_client_current_usage(&self, client_id: &str) -> Result<u32, DatabaseError>;

    /// Get A2A usage statistics for a client
    async fn get_usage_stats(
        &self,
        client_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<A2AUsageStats, DatabaseError>;

    /// Get A2A client usage history
    async fn get_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>, DatabaseError>;
}

/// User profiles, goals, and configuration repository
#[async_trait]
pub trait ProfileRepository: Send + Sync {
    /// Upsert user profile data
    async fn upsert_profile(&self, user_id: Uuid, data: Value) -> Result<(), DatabaseError>;

    /// Get user profile data
    async fn get_profile(&self, user_id: Uuid) -> Result<Option<Value>, DatabaseError>;

    /// Create a new goal for a user
    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String, DatabaseError>;

    /// Get all goals for a user
    async fn list_goals(&self, user_id: Uuid) -> Result<Vec<Value>, DatabaseError>;

    /// Update progress on a goal, scoped to the owning user
    async fn update_goal_progress(
        &self,
        goal_id: &str,
        user_id: Uuid,
        current_value: f64,
    ) -> Result<(), DatabaseError>;

    /// Get user configuration data
    async fn get_config(&self, user_id: &str) -> Result<Option<String>, DatabaseError>;

    /// Save user configuration data
    async fn save_config(&self, user_id: &str, config_json: &str) -> Result<(), DatabaseError>;
}

/// AI-generated insights storage repository
#[async_trait]
pub trait InsightRepository: Send + Sync {
    /// Store an AI-generated insight
    async fn store(&self, user_id: Uuid, insight_data: Value) -> Result<String, DatabaseError>;

    /// Get insights for a user
    async fn list(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>, DatabaseError>;
}

/// Admin token management repository
#[async_trait]
pub trait AdminRepository: Send + Sync {
    /// Create a new admin token
    async fn create_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken, DatabaseError>;

    /// Get admin token by ID
    async fn get_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>, DatabaseError>;

    /// Get admin token by prefix for fast lookup
    async fn get_token_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>, DatabaseError>;

    /// List all admin tokens (super admin only)
    async fn list_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>, DatabaseError>;

    /// Deactivate admin token
    async fn deactivate_token(&self, token_id: &str) -> Result<(), DatabaseError>;

    /// Update admin token last used timestamp
    async fn update_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<(), DatabaseError>;

    /// Record admin token usage for audit trail
    async fn record_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<(), DatabaseError>;

    /// Get admin token usage history
    async fn get_usage_history(
        &self,
        token_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>, DatabaseError>;

    /// Record API key provisioning by admin
    async fn record_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<(), DatabaseError>;

    /// Get admin provisioned keys history
    async fn get_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Value>, DatabaseError>;
}

/// Multi-tenant management repository
#[async_trait]
pub trait TenantRepository: Send + Sync {
    /// Create a new tenant
    async fn create(&self, tenant: &crate::models::Tenant) -> Result<(), DatabaseError>;

    /// Get tenant by ID
    async fn get_by_id(&self, id: Uuid) -> Result<crate::models::Tenant, DatabaseError>;

    /// Get tenant by slug
    async fn get_by_slug(&self, slug: &str) -> Result<crate::models::Tenant, DatabaseError>;

    /// List tenants for a user
    async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Tenant>, DatabaseError>;

    /// Get all tenants
    async fn list_all(&self) -> Result<Vec<crate::models::Tenant>, DatabaseError>;

    /// Store tenant OAuth credentials
    async fn store_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<(), DatabaseError>;

    /// Get tenant OAuth providers
    async fn get_oauth_providers(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>, DatabaseError>;

    /// Get tenant OAuth credentials for specific provider
    async fn get_oauth_credentials(
        &self,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>, DatabaseError>;

    /// Get user role for a specific tenant
    async fn get_user_role(
        &self,
        user_id: &str,
        tenant_id: TenantId,
    ) -> Result<Option<String>, DatabaseError>;

    /// Create OAuth application for MCP clients
    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<(), DatabaseError>;

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<crate::models::OAuthApp, DatabaseError>;

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>, DatabaseError>;
}

/// OAuth 2.0 server repository (RFC 7591)
#[async_trait]
pub trait OAuth2ServerRepository: Send + Sync {
    /// Store OAuth 2.0 client registration
    async fn store_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<(), DatabaseError>;

    /// Get OAuth 2.0 client by `client_id`
    async fn get_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>, DatabaseError>;

    /// Store OAuth 2.0 authorization code
    async fn store_auth_code(
        &self,
        code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<(), DatabaseError>;

    /// Get OAuth 2.0 authorization code
    async fn get_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>, DatabaseError>;

    /// Update OAuth 2.0 authorization code (mark as used)
    async fn update_auth_code(
        &self,
        code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<(), DatabaseError>;

    /// Atomically consume OAuth 2.0 authorization code
    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>, DatabaseError>;

    /// Store OAuth 2.0 refresh token
    async fn store_refresh_token(
        &self,
        token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<(), DatabaseError>;

    /// Get OAuth 2.0 refresh token
    async fn get_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>, DatabaseError>;

    /// Get refresh token by value (without `client_id` constraint)
    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>, DatabaseError>;

    /// Atomically consume OAuth 2.0 refresh token
    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>, DatabaseError>;

    /// Revoke OAuth 2.0 refresh token
    async fn revoke_refresh_token(&self, token: &str) -> Result<(), DatabaseError>;

    /// Store authorization code
    async fn store_authorization_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<(), DatabaseError>;

    /// Get authorization code data
    async fn get_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<crate::oauth2_server::models::OAuth2AuthCode, DatabaseError>;

    /// Delete authorization code (after use)
    async fn delete_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<(), DatabaseError>;

    /// Store `OAuth2` state for CSRF protection
    async fn store_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<(), DatabaseError>;

    /// Consume `OAuth2` state (atomically check and mark as used)
    async fn consume_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>, DatabaseError>;
}

/// Security, key rotation, and audit repository
#[async_trait]
pub trait SecurityRepository: Send + Sync {
    /// Save RSA keypair to database for persistence across restarts
    async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
        key_size_bits: usize,
    ) -> Result<(), DatabaseError>;

    /// Load all RSA keypairs from database
    async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>, bool)>, DatabaseError>;

    /// Update active status of RSA keypair
    async fn update_rsa_keypair_status(
        &self,
        kid: &str,
        is_active: bool,
    ) -> Result<(), DatabaseError>;

    /// Store key version metadata
    async fn store_key_version(
        &self,
        tenant_id: Option<Uuid>,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<(), DatabaseError>;

    /// Get all key versions for a tenant
    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>, DatabaseError>;

    /// Get current active key version for a tenant
    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>, DatabaseError>;

    /// Update key version status (activate/deactivate)
    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<(), DatabaseError>;

    /// Delete old key versions
    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64, DatabaseError>;

    /// Store audit event
    async fn store_audit_event(
        &self,
        tenant_id: Option<Uuid>,
        event: &crate::security::audit::AuditEvent,
    ) -> Result<(), DatabaseError>;

    /// Get audit events with filters
    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>, DatabaseError>;

    /// Get or create system secret (generates if not exists)
    async fn get_or_create_system_secret(&self, secret_type: &str)
        -> Result<String, DatabaseError>;

    /// Get existing system secret
    async fn get_system_secret(&self, secret_type: &str) -> Result<String, DatabaseError>;

    /// Update system secret (for rotation)
    async fn update_system_secret(
        &self,
        secret_type: &str,
        new_value: &str,
    ) -> Result<(), DatabaseError>;
}

/// OAuth notification repository
#[async_trait]
pub trait NotificationRepository: Send + Sync {
    /// Store OAuth completion notification for MCP resource delivery
    async fn store(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String, DatabaseError>;

    /// Get unread OAuth notifications for a user
    async fn get_unread(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthNotification>, DatabaseError>;

    /// Mark OAuth notification as read
    async fn mark_read(&self, notification_id: &str, user_id: Uuid) -> Result<bool, DatabaseError>;

    /// Mark all OAuth notifications as read for a user
    async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, DatabaseError>;

    /// Get all OAuth notifications for a user (read and unread)
    async fn get_all(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::models::OAuthNotification>, DatabaseError>;
}

/// Fitness configuration management repository
#[async_trait]
pub trait FitnessConfigRepository: Send + Sync {
    /// Save tenant-level fitness configuration
    async fn save_tenant_config(
        &self,
        tenant_id: TenantId,
        configuration_name: &str,
        config: &crate::config::fitness::FitnessConfig,
    ) -> Result<String, DatabaseError>;

    /// Save user-specific fitness configuration
    async fn save_user_config(
        &self,
        tenant_id: TenantId,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness::FitnessConfig,
    ) -> Result<String, DatabaseError>;

    /// Get tenant-level fitness configuration
    async fn get_tenant_config(
        &self,
        tenant_id: TenantId,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness::FitnessConfig>, DatabaseError>;

    /// Get user-specific fitness configuration
    async fn get_user_config(
        &self,
        tenant_id: TenantId,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness::FitnessConfig>, DatabaseError>;

    /// List all tenant-level fitness configuration names
    async fn list_tenant_configs(&self, tenant_id: TenantId) -> Result<Vec<String>, DatabaseError>;

    /// List all user-specific fitness configuration names
    async fn list_user_configs(
        &self,
        tenant_id: TenantId,
        user_id: &str,
    ) -> Result<Vec<String>, DatabaseError>;

    /// Delete fitness configuration (tenant or user-specific)
    async fn delete_config(
        &self,
        tenant_id: TenantId,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool, DatabaseError>;
}

/// Recipe storage and management repository (tenant-scoped)
#[async_trait]
pub trait RecipeRepository: Send + Sync {
    /// Create a new recipe for a user
    async fn create(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        recipe: &crate::intelligence::recipes::Recipe,
    ) -> Result<String, DatabaseError>;

    /// Get recipe by ID for a specific user
    async fn get_by_id(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Option<crate::intelligence::recipes::Recipe>, DatabaseError>;

    /// List recipes for a user with optional meal timing filter
    async fn list(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        meal_timing: Option<crate::intelligence::recipes::MealTiming>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<crate::intelligence::recipes::Recipe>, DatabaseError>;

    /// Update a recipe
    async fn update(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        recipe: &crate::intelligence::recipes::Recipe,
    ) -> Result<bool, DatabaseError>;

    /// Delete a recipe
    async fn delete(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<bool, DatabaseError>;

    /// Update cached nutrition for a recipe after USDA validation
    async fn update_nutrition_cache(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        nutrition: &crate::intelligence::recipes::ValidatedNutrition,
    ) -> Result<bool, DatabaseError>;

    /// Search recipes by name, tags, or description
    async fn search(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::intelligence::recipes::Recipe>, DatabaseError>;

    /// Count recipes for a user
    async fn count(&self, user_id: Uuid, tenant_id: TenantId) -> Result<u32, DatabaseError>;
}

/// Tool selection and per-tenant configuration repository
#[async_trait]
pub trait ToolSelectionRepository: Send + Sync {
    /// Get the complete tool catalog
    async fn get_tool_catalog(&self) -> Result<Vec<crate::models::ToolCatalogEntry>, DatabaseError>;

    /// Get a specific tool catalog entry by name
    async fn get_tool_catalog_entry(
        &self,
        tool_name: &str,
    ) -> Result<Option<crate::models::ToolCatalogEntry>, DatabaseError>;

    /// Get tools filtered by category
    async fn get_tools_by_category(
        &self,
        category: crate::models::ToolCategory,
    ) -> Result<Vec<crate::models::ToolCatalogEntry>, DatabaseError>;

    /// Get tools available for a specific plan level
    async fn get_tools_by_min_plan(
        &self,
        plan: crate::models::TenantPlan,
    ) -> Result<Vec<crate::models::ToolCatalogEntry>, DatabaseError>;

    /// Get all tool overrides for a tenant
    async fn get_tenant_tool_overrides(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<crate::models::TenantToolOverride>, DatabaseError>;

    /// Get a specific tool override for a tenant
    async fn get_tenant_tool_override(
        &self,
        tenant_id: TenantId,
        tool_name: &str,
    ) -> Result<Option<crate::models::TenantToolOverride>, DatabaseError>;

    /// Create or update a tool override for a tenant
    async fn upsert_tenant_tool_override(
        &self,
        tenant_id: TenantId,
        tool_name: &str,
        is_enabled: bool,
        enabled_by_user_id: Option<Uuid>,
        reason: Option<String>,
    ) -> Result<crate::models::TenantToolOverride, DatabaseError>;

    /// Delete a tool override (revert to catalog default)
    async fn delete_tenant_tool_override(
        &self,
        tenant_id: TenantId,
        tool_name: &str,
    ) -> Result<bool, DatabaseError>;

    /// Count enabled tools for a tenant (for summary)
    async fn count_enabled_tools(&self, tenant_id: TenantId) -> Result<usize, DatabaseError>;
}

/// Coaches (custom AI personas) storage and management repository (tenant-scoped)
#[async_trait]
pub trait CoachesRepository: Send + Sync {
    /// Create a new coach for a user
    async fn create(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &crate::database::coaches::CreateCoachRequest,
    ) -> Result<crate::database::coaches::Coach, DatabaseError>;

    /// Get coach by ID for a specific user
    async fn get_by_id(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Option<crate::database::coaches::Coach>, DatabaseError>;

    /// List coaches for a user with optional filtering
    async fn list(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        filter: &crate::database::coaches::ListCoachesFilter,
    ) -> Result<Vec<crate::database::coaches::Coach>, DatabaseError>;

    /// Update a coach
    async fn update(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &crate::database::coaches::UpdateCoachRequest,
    ) -> Result<Option<crate::database::coaches::Coach>, DatabaseError>;

    /// Delete a coach
    async fn delete(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<bool, DatabaseError>;

    /// Record coach usage (increment use_count and update last_used_at)
    async fn record_usage(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<bool, DatabaseError>;

    /// Toggle favorite status for a coach
    async fn toggle_favorite(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Option<bool>, DatabaseError>;

    /// Search coaches by title, description, or tags
    async fn search(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        query: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<crate::database::coaches::Coach>, DatabaseError>;

    /// Count coaches for a user
    async fn count(&self, user_id: Uuid, tenant_id: TenantId) -> Result<u32, DatabaseError>;
}

/// Mobility (stretching exercises and yoga poses) read-only repository
#[async_trait]
pub trait MobilityRepository: Send + Sync {
    /// Get a stretching exercise by ID
    async fn get_stretching_exercise(
        &self,
        id: &str,
    ) -> Result<Option<crate::database::mobility::StretchingExercise>, DatabaseError>;

    /// List stretching exercises with filtering
    async fn list_stretching_exercises(
        &self,
        filter: &crate::database::mobility::ListStretchingFilter,
    ) -> Result<Vec<crate::database::mobility::StretchingExercise>, DatabaseError>;

    /// Search stretching exercises by name or description
    async fn search_stretching_exercises(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::database::mobility::StretchingExercise>, DatabaseError>;

    /// Get stretches recommended for a specific activity
    async fn get_stretches_for_activity(
        &self,
        activity_type: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::database::mobility::StretchingExercise>, DatabaseError>;

    /// Get a yoga pose by ID
    async fn get_yoga_pose(
        &self,
        id: &str,
    ) -> Result<Option<crate::database::mobility::YogaPose>, DatabaseError>;

    /// List yoga poses with filtering
    async fn list_yoga_poses(
        &self,
        filter: &crate::database::mobility::ListYogaFilter,
    ) -> Result<Vec<crate::database::mobility::YogaPose>, DatabaseError>;

    /// Search yoga poses by name
    async fn search_yoga_poses(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::database::mobility::YogaPose>, DatabaseError>;

    /// Get poses recommended for a recovery context
    async fn get_poses_for_recovery(
        &self,
        recovery_context: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::database::mobility::YogaPose>, DatabaseError>;

    /// Get activity-muscle mapping
    async fn get_activity_muscle_mapping(
        &self,
        activity_type: &str,
    ) -> Result<Option<crate::database::mobility::ActivityMuscleMapping>, DatabaseError>;

    /// List all activity-muscle mappings
    async fn list_activity_muscle_mappings(
        &self,
    ) -> Result<Vec<crate::database::mobility::ActivityMuscleMapping>, DatabaseError>;
}

/// Social features repository for friend connections and shared insights
#[async_trait]
pub trait SocialRepository: Send + Sync {
    /// Create a new friend connection request
    async fn create_friend_connection(
        &self,
        connection: &crate::models::FriendConnection,
    ) -> Result<Uuid, DatabaseError>;

    /// Get a friend connection by ID
    async fn get_friend_connection(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::FriendConnection>, DatabaseError>;

    /// Get friend connection between two users
    async fn get_friend_connection_between(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Option<crate::models::FriendConnection>, DatabaseError>;

    /// Update friend connection status
    async fn update_friend_connection_status(
        &self,
        id: Uuid,
        user_id: Uuid,
        status: crate::models::FriendStatus,
    ) -> Result<(), DatabaseError>;

    /// Get all friends for a user (accepted connections)
    async fn get_friends(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::FriendConnection>, DatabaseError>;

    /// Get pending friend requests received by a user
    async fn get_pending_friend_requests(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::FriendConnection>, DatabaseError>;

    /// Get pending friend requests sent by a user
    async fn get_sent_friend_requests(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::FriendConnection>, DatabaseError>;

    /// Check if two users are friends
    async fn are_friends(&self, user_a: Uuid, user_b: Uuid) -> Result<bool, DatabaseError>;

    /// Delete a friend connection
    async fn delete_friend_connection(&self, id: Uuid, user_id: Uuid) -> Result<bool, DatabaseError>;

    /// Get or create social settings for a user
    async fn get_or_create_social_settings(
        &self,
        user_id: Uuid,
    ) -> Result<crate::models::UserSocialSettings, DatabaseError>;

    /// Get social settings for a user
    async fn get_social_settings(
        &self,
        user_id: Uuid,
    ) -> Result<Option<crate::models::UserSocialSettings>, DatabaseError>;

    /// Update social settings
    async fn upsert_social_settings(
        &self,
        settings: &crate::models::UserSocialSettings,
    ) -> Result<(), DatabaseError>;

    /// Create a shared insight
    async fn create_shared_insight(
        &self,
        insight: &crate::models::SharedInsight,
    ) -> Result<Uuid, DatabaseError>;

    /// Get a shared insight by ID
    async fn get_shared_insight(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<crate::models::SharedInsight>, DatabaseError>;

    /// Get friends' shared insights for feed
    async fn get_friends_feed(
        &self,
        user_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<crate::models::SharedInsight>, DatabaseError>;

    /// Get user's own shared insights
    async fn get_user_shared_insights(
        &self,
        user_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<crate::models::SharedInsight>, DatabaseError>;

    /// Delete a shared insight
    async fn delete_shared_insight(&self, id: Uuid, user_id: Uuid) -> Result<bool, DatabaseError>;

    /// Create or update a reaction
    async fn upsert_insight_reaction(
        &self,
        reaction: &crate::models::InsightReaction,
    ) -> Result<(), DatabaseError>;

    /// Get user's reaction to an insight
    async fn get_insight_reaction(
        &self,
        insight_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<crate::models::InsightReaction>, DatabaseError>;

    /// Delete a reaction
    async fn delete_insight_reaction(
        &self,
        insight_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, DatabaseError>;

    /// Get all reactions for an insight
    async fn get_insight_reactions(
        &self,
        insight_id: Uuid,
    ) -> Result<Vec<crate::models::InsightReaction>, DatabaseError>;

    /// Create an adapted insight
    async fn create_adapted_insight(
        &self,
        insight: &crate::models::AdaptedInsight,
    ) -> Result<Uuid, DatabaseError>;

    /// Get an adapted insight by ID
    async fn get_adapted_insight(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::AdaptedInsight>, DatabaseError>;

    /// Get user's adaptation of a specific source insight
    async fn get_user_adaptation(
        &self,
        source_insight_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<crate::models::AdaptedInsight>, DatabaseError>;

    /// Get user's adapted insights
    async fn get_user_adapted_insights(
        &self,
        user_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<crate::models::AdaptedInsight>, DatabaseError>;

    /// Update was_helpful for an adapted insight
    async fn update_adapted_insight_helpful(
        &self,
        id: Uuid,
        user_id: Uuid,
        was_helpful: bool,
    ) -> Result<bool, DatabaseError>;

    /// Search for discoverable users
    async fn search_discoverable_users(
        &self,
        query: &str,
        exclude_user_id: Uuid,
        limit: u32,
    ) -> Result<Vec<(Uuid, String, Option<String>)>, DatabaseError>;

    /// Get friend count for a user
    async fn get_friend_count(&self, user_id: Uuid) -> Result<i64, DatabaseError>;
}
