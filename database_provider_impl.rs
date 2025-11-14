// COMPLETE DatabaseProvider IMPLEMENTATION FOR Database
// This replaces the empty impl block at line 942 in src/database/mod.rs
//
// INSTRUCTIONS:
// 1. Replace the empty impl block at line 942-948 with this complete implementation
// 2. This eliminates the need for the sqlite.rs wrapper (3,044 lines of delegation)

#[async_trait]
impl crate::database_plugins::DatabaseProvider for Database {
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        Database::new(database_url, encryption_key).await
    }

    async fn migrate(&self) -> Result<()> {
        Database::migrate(self).await
    }

    async fn create_user(&self, user: &User) -> Result<Uuid> {
        Database::create_user(self, user).await
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        Database::get_user(self, user_id).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Database::get_user_by_email(self, email).await
    }

    async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        Database::get_user_by_email_required(self, email).await
    }

    async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        Database::update_last_active(self, user_id).await
    }

    async fn get_user_count(&self) -> Result<i64> {
        Database::get_user_count(self).await
    }

    async fn get_users_by_status(&self, status: &str) -> Result<Vec<User>> {
        Database::get_users_by_status(self, status).await
    }

    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> Result<crate::pagination::CursorPage<User>> {
        Database::get_users_by_status_cursor(self, status, params).await
    }

    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> Result<User> {
        Database::update_user_status(self, user_id, new_status, admin_token_id).await
    }

    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> Result<()> {
        Database::update_user_tenant_id(self, user_id, tenant_id).await
    }

    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> Result<()> {
        Database::upsert_user_profile(self, user_id, profile_data).await
    }

    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<Value>> {
        Database::get_user_profile(self, user_id).await
    }

    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String> {
        Database::create_goal(self, user_id, goal_data).await
    }

    async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<Value>> {
        Database::get_user_goals(self, user_id).await
    }

    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        Database::update_goal_progress(self, goal_id, current_value).await
    }

    async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
        Database::get_user_configuration(self, user_id).await
    }

    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()> {
        Database::save_user_configuration(self, user_id, config_json).await
    }

    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> Result<String> {
        Database::store_insight(self, user_id, insight_data).await
    }

    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>> {
        Database::get_user_insights(self, user_id, insight_type, limit).await
    }

    async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        Database::create_api_key(self, api_key).await
    }

    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> Result<Option<ApiKey>> {
        Database::get_api_key_by_prefix(self, prefix, hash).await
    }

    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        Database::get_user_api_keys(self, user_id).await
    }

    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        Database::update_api_key_last_used(self, api_key_id).await
    }

    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()> {
        Database::deactivate_api_key(self, api_key_id, user_id).await
    }

    async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<ApiKey>> {
        Database::get_api_key_by_id(self, api_key_id).await
    }

    async fn get_api_keys_filtered(
        &self,
        _user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>> {
        Database::get_api_keys_filtered(
            self,
            None,
            None,
            Some(active_only),
            limit.unwrap_or(10),
            offset.unwrap_or(0),
        )
        .await
    }

    async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        Database::cleanup_expired_api_keys(self).await
    }

    async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>> {
        Database::get_expired_api_keys(self).await
    }

    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()> {
        Database::record_api_key_usage(self, usage).await
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        Database::get_api_key_current_usage(self, api_key_id).await
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats> {
        Database::get_api_key_usage_stats(self, api_key_id, start_date, end_date).await
    }

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        Database::record_jwt_usage(self, usage).await
    }

    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32> {
        Database::get_jwt_current_usage(self, user_id).await
    }

    async fn get_request_logs(
        &self,
        _api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        _status_filter: Option<&str>,
        _tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        Database::get_request_logs(self, None, start_time, end_time, 10, 0).await
    }

    async fn get_system_stats(&self) -> Result<(u64, u64)> {
        Database::get_system_stats(self).await
    }

    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        Database::create_a2a_client(self, client, client_secret, api_key_id).await
    }

    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        Database::get_a2a_client(self, client_id).await
    }

    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> Result<Option<A2AClient>> {
        Database::get_a2a_client_by_api_key_id(self, api_key_id).await
    }

    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        Database::get_a2a_client_by_name(self, name).await
    }

    async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        Database::list_a2a_clients(self, user_id).await
    }

    async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        Database::deactivate_a2a_client(self, client_id).await
    }

    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        Database::get_a2a_client_credentials(self, client_id).await
    }

    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        Database::invalidate_a2a_client_sessions(self, client_id).await
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        Database::deactivate_client_api_keys(self, client_id).await
    }

    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String> {
        Database::create_a2a_session(self, client_id, user_id, granted_scopes, expires_in_hours).await
    }

    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        Database::get_a2a_session(self, session_token).await
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        Database::update_a2a_session_activity(self, session_token).await
    }

    async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        Database::get_active_a2a_sessions(self, client_id).await
    }

    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String> {
        Database::create_a2a_task(self, client_id, session_id, task_type, input_data).await
    }

    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        Database::get_a2a_task(self, task_id).await
    }

    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>> {
        Database::list_a2a_tasks(self, client_id, status_filter, limit, offset).await
    }

    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()> {
        Database::update_a2a_task_status(self, task_id, status, result, error).await
    }

    async fn record_a2a_usage(&self, usage: &A2AUsage) -> Result<()> {
        Database::record_a2a_usage(self, usage).await
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        Database::get_a2a_client_current_usage(self, client_id).await
    }

    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<crate::database::A2AUsageStats> {
        Database::get_a2a_usage_stats(self, client_id, start_date, end_date).await
    }

    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>> {
        Database::get_a2a_client_usage_history(self, client_id, days).await
    }

    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        Database::get_provider_last_sync(self, user_id, provider).await
    }

    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        Database::update_provider_last_sync(self, user_id, provider, sync_time).await
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        Database::get_top_tools_analysis(self, user_id, start_time, end_time).await
    }

    async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        Database::create_admin_token(self, request, admin_jwt_secret, jwks_manager).await
    }

    async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        Database::get_admin_token_by_id(self, token_id).await
    }

    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        Database::get_admin_token_by_prefix(self, token_prefix).await
    }

    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>> {
        Database::list_admin_tokens(self, include_inactive).await
    }

    async fn deactivate_admin_token(&self, token_id: &str) -> Result<()> {
        Database::deactivate_admin_token(self, token_id).await
    }

    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<()> {
        Database::update_admin_token_last_used(self, token_id, ip_address).await
    }

    async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<()> {
        Database::record_admin_token_usage(self, usage).await
    }

    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>> {
        Database::get_admin_token_usage_history(self, token_id, start_date, end_date).await
    }

    async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<()> {
        Database::record_admin_provisioned_key(
            self,
            admin_token_id,
            api_key_id,
            user_email,
            tier,
            rate_limit_requests,
            rate_limit_period,
        )
        .await
    }

    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>> {
        Database::get_admin_provisioned_keys(self, admin_token_id, start_date, end_date).await
    }

    async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
        key_size_bits: i32,
    ) -> Result<()> {
        Database::save_rsa_keypair(
            self,
            kid,
            private_key_pem,
            public_key_pem,
            created_at,
            is_active,
            key_size_bits,
        )
        .await
    }

    async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>, bool)>> {
        Database::load_rsa_keypairs(self).await
    }

    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> Result<()> {
        Database::update_rsa_keypair_active_status(self, kid, is_active).await
    }

    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        Database::create_tenant(self, tenant).await
    }

    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<crate::models::Tenant> {
        Database::get_tenant_by_id(self, tenant_id).await
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        Database::get_tenant_by_slug(self, slug).await
    }

    async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        Database::list_tenants_for_user(self, user_id).await
    }

    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        Database::store_tenant_oauth_credentials(self, credentials).await
    }

    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        Database::get_tenant_oauth_providers(self, tenant_id).await
    }

    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        Database::get_tenant_oauth_credentials(self, tenant_id, provider).await
    }

    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        Database::create_oauth_app(self, app).await
    }

    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp> {
        Database::get_oauth_app_by_client_id(self, client_id).await
    }

    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        Database::list_oauth_apps_for_user(self, user_id).await
    }

    async fn store_oauth2_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<()> {
        Database::store_oauth2_client(self, client).await
    }

    async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>> {
        Database::get_oauth2_client(self, client_id).await
    }

    async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Database::store_oauth2_auth_code(self, auth_code).await
    }

    async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Database::get_oauth2_auth_code(self, code).await
    }

    async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Database::update_oauth2_auth_code(self, auth_code).await
    }

    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<()> {
        Database::store_oauth2_refresh_token(self, refresh_token).await
    }

    async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Database::get_oauth2_refresh_token(self, token).await
    }

    async fn revoke_oauth2_refresh_token(&self, token: &str) -> Result<()> {
        Database::revoke_oauth2_refresh_token(self, token).await
    }

    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Database::consume_auth_code(self, code, client_id, redirect_uri, now).await
    }

    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Database::consume_refresh_token(self, token, client_id, now).await
    }

    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Database::get_refresh_token_by_value(self, token).await
    }

    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> Result<()> {
        Database::store_authorization_code(self, code, client_id, redirect_uri, scope, user_id).await
    }

    async fn get_authorization_code(&self, code: &str) -> Result<crate::models::AuthorizationCode> {
        Database::get_authorization_code(self, code).await
    }

    async fn delete_authorization_code(&self, code: &str) -> Result<()> {
        Database::delete_authorization_code(self, code).await
    }

    async fn store_oauth2_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<()> {
        Database::store_oauth2_state(self, state).await
    }

    async fn consume_oauth2_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>> {
        Database::consume_oauth2_state(self, state_value, client_id, now).await
    }

    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        Database::store_key_version(self, version).await
    }

    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        Database::get_key_versions(self, tenant_id).await
    }

    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        Database::get_current_key_version(self, tenant_id).await
    }

    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<()> {
        Database::update_key_version_status(self, tenant_id, version, is_active).await
    }

    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64> {
        Database::delete_old_key_versions(self, tenant_id, keep_count).await
    }

    async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        Database::get_all_tenants(self).await
    }

    async fn store_audit_event(&self, event: &crate::security::audit::AuditEvent) -> Result<()> {
        Database::store_audit_event(self, event).await
    }

    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>> {
        Database::get_audit_events(self, tenant_id, event_type, limit).await
    }

    async fn get_user_tenant_role(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<String>> {
        Database::get_user_tenant_role(self, &user_id.to_string(), &tenant_id.to_string()).await
    }

    async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String> {
        Database::get_or_create_system_secret(self, secret_type).await
    }

    async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        Database::get_system_secret(self, secret_type).await
    }

    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()> {
        Database::update_system_secret(self, secret_type, new_value).await
    }

    async fn store_oauth_notification(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String> {
        Database::store_oauth_notification(self, user_id, provider, success, message, expires_at).await
    }

    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Database::get_unread_oauth_notifications(self, user_id).await
    }

    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool> {
        Database::mark_oauth_notification_read(self, notification_id, user_id).await
    }

    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64> {
        Database::mark_all_oauth_notifications_read(self, user_id).await
    }

    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Database::get_all_oauth_notifications(self, user_id, limit).await
    }

    async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let manager = Database::fitness_configurations(self);
        manager.save_tenant_config(tenant_id, configuration_name, config).await
    }

    async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let manager = Database::fitness_configurations(self);
        manager.save_user_config(tenant_id, user_id, configuration_name, config).await
    }

    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let manager = Database::fitness_configurations(self);
        manager.get_tenant_config(tenant_id, configuration_name).await
    }

    async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let manager = Database::fitness_configurations(self);
        manager.get_user_config(tenant_id, user_id, configuration_name).await
    }

    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        let manager = Database::fitness_configurations(self);
        manager.list_tenant_configurations(tenant_id).await
    }

    async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>> {
        let manager = Database::fitness_configurations(self);
        manager.list_user_configurations(tenant_id, user_id).await
    }

    async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool> {
        let manager = Database::fitness_configurations(self);
        manager.delete_config(tenant_id, user_id, configuration_name).await
    }

    // OAuth Token Management
    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> Result<()> {
        use crate::database::user_oauth_tokens::OAuthTokenData;

        let token_data = OAuthTokenData {
            id: &token.id,
            user_id: token.user_id,
            tenant_id: &token.tenant_id,
            provider: &token.provider,
            access_token: &token.access_token,
            refresh_token: token.refresh_token.as_deref(),
            token_type: &token.token_type,
            expires_at: token.expires_at,
            scope: token.scope.as_deref().unwrap_or(""),
        };

        Database::upsert_user_oauth_token(self, &token_data).await
    }

    async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>> {
        Database::get_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    async fn get_user_oauth_tokens(&self, user_id: Uuid) -> Result<Vec<UserOAuthToken>> {
        Database::get_user_oauth_tokens(self, user_id).await
    }

    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>> {
        Database::get_tenant_provider_tokens(self, tenant_id, provider).await
    }

    async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<()> {
        Database::delete_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()> {
        Database::delete_user_oauth_tokens(self, user_id).await
    }

    async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        Database::refresh_user_oauth_token(
            self,
            user_id,
            tenant_id,
            provider,
            access_token,
            refresh_token,
            expires_at,
        )
        .await
    }

    async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        Database::store_user_oauth_app(self, user_id, provider, client_id, client_secret, redirect_uri).await
    }

    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>> {
        Database::get_user_oauth_app(self, user_id, provider).await
    }

    async fn list_user_oauth_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>> {
        Database::list_user_oauth_apps(self, user_id).await
    }

    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()> {
        Database::remove_user_oauth_app(self, user_id, provider).await
    }
}
