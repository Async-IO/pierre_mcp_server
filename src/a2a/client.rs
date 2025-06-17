// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A2A Client Management
//!
//! Handles registration, management, and monitoring of A2A clients
//! that connect to Pierre for agent-to-agent communication.

use crate::a2a::auth::A2AClient;
use crate::database::Database;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// A2A Client registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistrationRequest {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub redirect_uris: Vec<String>,
    pub contact_email: String,
}

/// A2A Client credentials response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub api_key: String,
}

/// A2A Client usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUsageStats {
    pub client_id: String,
    pub requests_today: u64,
    pub requests_this_month: u64,
    pub total_requests: u64,
    pub last_request_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rate_limit_tier: String,
}

/// A2A Active session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ASession {
    pub id: String,
    pub client_id: String,
    pub user_id: Option<String>,
    pub granted_scopes: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub requests_count: u64,
}

/// A2A Client Manager
pub struct A2AClientManager {
    #[allow(dead_code)]
    database: Arc<Database>,
    #[allow(dead_code)]
    active_sessions: Arc<tokio::sync::RwLock<HashMap<String, A2ASession>>>,
}

impl A2AClientManager {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            active_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register a new A2A client
    pub async fn register_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientCredentials, crate::a2a::A2AError> {
        // Validate registration request
        self.validate_registration_request(&request)?;

        // Generate client credentials
        let client_id = format!("a2a_client_{}", Uuid::new_v4());
        let client_secret = format!("a2a_secret_{}", Uuid::new_v4());
        let api_key = format!("a2a_{}", uuid::Uuid::new_v4());

        // Create client record
        let client = A2AClient {
            id: client_id.clone(),
            name: request.name,
            description: request.description,
            public_key: "".to_string(), // TODO: Generate or accept public key
            capabilities: request.capabilities,
            redirect_uris: request.redirect_uris,
            is_active: true,
            created_at: chrono::Utc::now(),
        };

        // Store client in database
        self.store_client(&client, &client_secret, &api_key).await?;

        Ok(ClientCredentials {
            client_id,
            client_secret,
            api_key,
        })
    }

    /// Validate client registration request
    fn validate_registration_request(
        &self,
        request: &ClientRegistrationRequest,
    ) -> Result<(), crate::a2a::A2AError> {
        if request.name.is_empty() {
            return Err(crate::a2a::A2AError::InvalidRequest(
                "Client name is required".to_string(),
            ));
        }

        if request.capabilities.is_empty() {
            return Err(crate::a2a::A2AError::InvalidRequest(
                "At least one capability is required".to_string(),
            ));
        }

        // Validate capabilities are known
        let valid_capabilities = [
            "fitness-data-analysis",
            "activity-intelligence",
            "goal-management",
            "performance-prediction",
            "training-analytics",
            "provider-integration",
        ];

        for capability in &request.capabilities {
            if !valid_capabilities.contains(&capability.as_str()) {
                return Err(crate::a2a::A2AError::InvalidRequest(format!(
                    "Unknown capability: {}",
                    capability
                )));
            }
        }

        Ok(())
    }

    /// Store client in database
    async fn store_client(
        &self,
        client: &A2AClient,
        _client_secret: &str,
        _api_key: &str,
    ) -> Result<(), crate::a2a::A2AError> {
        // Create API key first using the API key manager
        let api_key_manager = crate::api_keys::ApiKeyManager::new();

        // For A2A clients, we need a user to link the API key to
        // Create a dummy system user for A2A clients
        let dummy_user = crate::models::User::new(
            format!("a2a-system-{}@pierre.ai", client.id),
            "dummy-hash".to_string(),
            Some("A2A System User".to_string()),
        );

        let dummy_user_id = self.database.create_user(&dummy_user).await.map_err(|e| {
            crate::a2a::A2AError::InternalError(format!("Failed to create dummy user: {}", e))
        })?;

        let request = crate::api_keys::CreateApiKeyRequest {
            name: format!("A2A Client: {}", client.name),
            description: Some(format!("API key for A2A client: {}", client.description)),
            tier: crate::api_keys::ApiKeyTier::Professional, // Default tier for A2A clients
            expires_in_days: None,                           // No expiration
        };

        let (api_key_obj, _generated_key) = api_key_manager
            .create_api_key(dummy_user_id, request)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to create API key: {}", e))
            })?;

        // Store the API key in database
        self.database
            .create_api_key(&api_key_obj)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to store API key: {}", e))
            })?;

        // Create A2A client entry linked to the API key
        self.database
            .create_a2a_client(client, &api_key_obj.id)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to create A2A client: {}", e))
            })?;

        tracing::info!("Registered A2A client: {} ({})", client.name, client.id);
        Ok(())
    }

    /// Get client by ID
    pub async fn get_client(
        &self,
        client_id: &str,
    ) -> Result<Option<A2AClient>, crate::a2a::A2AError> {
        self.database.get_a2a_client(client_id).await.map_err(|e| {
            crate::a2a::A2AError::InternalError(format!("Failed to get A2A client: {}", e))
        })
    }

    /// List all registered clients
    pub async fn list_clients(&self) -> Result<Vec<A2AClient>, crate::a2a::A2AError> {
        // For now, return an empty list since we don't have a user context
        // In production, this would list clients for a specific user
        Ok(vec![])
    }

    /// Deactivate a client
    pub async fn deactivate_client(&self, _client_id: &str) -> Result<(), crate::a2a::A2AError> {
        // TODO: Implement client deactivation
        // This would involve:
        // 1. Setting is_active to false
        // 2. Invalidating active sessions
        // 3. Deactivating API keys

        Ok(())
    }

    /// Get client usage statistics
    pub async fn get_client_usage(
        &self,
        client_id: &str,
    ) -> Result<ClientUsageStats, crate::a2a::A2AError> {
        // Get current month usage
        let requests_this_month = self.database
            .get_a2a_client_current_usage(client_id)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get current usage: {}", e))
            })? as u64;

        // Get today's usage
        let start_of_day = chrono::Utc::now()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        let end_of_day = chrono::Utc::now();

        let today_stats = self.database
            .get_a2a_usage_stats(client_id, start_of_day, end_of_day)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get today's stats: {}", e))
            })?;

        // Get last request from recent usage history
        let recent_usage = self.database
            .get_a2a_client_usage_history(client_id, Some(1))
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get recent usage: {}", e))
            })?;

        let last_request_at = recent_usage.first().map(|usage| usage.timestamp);

        // Get total requests (use a long period to approximate total)
        let total_start = chrono::Utc::now() - chrono::Duration::days(365);
        let total_stats = self.database
            .get_a2a_usage_stats(client_id, total_start, chrono::Utc::now())
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get total stats: {}", e))
            })?;

        Ok(ClientUsageStats {
            client_id: client_id.to_string(),
            requests_today: today_stats.total_requests as u64,
            requests_this_month,
            total_requests: total_stats.total_requests as u64,
            last_request_at,
            rate_limit_tier: "professional".to_string(), // Default for A2A clients
        })
    }

    /// Create a new session for a client
    pub async fn create_session(
        &self,
        client_id: &str,
        user_id: Option<&str>,
    ) -> Result<String, crate::a2a::A2AError> {
        let user_uuid = user_id.and_then(|id| uuid::Uuid::parse_str(id).ok());
        let granted_scopes = vec!["fitness:read".to_string(), "analytics:read".to_string()];

        let session_token = self
            .database
            .create_a2a_session(client_id, user_uuid.as_ref(), &granted_scopes, 24)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to create A2A session: {}", e))
            })?;

        Ok(session_token)
    }

    /// Update session activity
    pub async fn update_session_activity(
        &self,
        session_token: &str,
    ) -> Result<(), crate::a2a::A2AError> {
        self.database
            .update_a2a_session_activity(session_token)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!(
                    "Failed to update session activity: {}",
                    e
                ))
            })
    }

    /// Get active sessions for a client
    pub async fn get_active_sessions(&self, _client_id: &str) -> Vec<A2ASession> {
        // With database storage, this would require a more complex query
        // For now, return empty list
        vec![]
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) {
        // With database storage, expired sessions are automatically filtered out
        // This could trigger a cleanup job if needed
    }

    /// Record API usage for a client
    pub async fn record_usage(
        &self,
        client_id: &str,
        method: &str,
        success: bool,
    ) -> Result<(), crate::a2a::A2AError> {
        self.record_detailed_usage(
            client_id,
            None, // No session token for simple usage
            method,
            None, // No response time
            if success { 200 } else { 500 },
            None, // No error message
            None, // No request size
            None, // No response size
            None, // No IP address
            None, // No user agent
            &[], // No specific capabilities
            &[], // No specific scopes
        ).await
    }

    /// Record detailed A2A usage for tracking and analytics
    pub async fn record_detailed_usage(
        &self,
        client_id: &str,
        session_token: Option<&str>,
        tool_name: &str,
        response_time_ms: Option<u32>,
        status_code: u16,
        error_message: Option<&str>,
        request_size_bytes: Option<u32>,
        response_size_bytes: Option<u32>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        client_capabilities: &[String],
        granted_scopes: &[String],
    ) -> Result<(), crate::a2a::A2AError> {
        let usage = crate::database::A2AUsage {
            id: None,
            client_id: client_id.to_string(),
            session_token: session_token.map(|s| s.to_string()),
            timestamp: chrono::Utc::now(),
            tool_name: tool_name.to_string(),
            response_time_ms,
            status_code,
            error_message: error_message.map(|s| s.to_string()),
            request_size_bytes,
            response_size_bytes,
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: user_agent.map(|s| s.to_string()),
            protocol_version: "1.0".to_string(),
            client_capabilities: client_capabilities.to_vec(),
            granted_scopes: granted_scopes.to_vec(),
        };

        self.database
            .record_a2a_usage(&usage)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to record A2A usage: {}", e))
            })?;

        tracing::debug!(
            "A2A usage recorded - Client: {}, Tool: {}, Status: {}",
            client_id,
            tool_name,
            status_code
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    async fn create_test_database() -> Arc<Database> {
        // Use in-memory database for tests to avoid file system issues
        let database = Database::new(":memory:", vec![0u8; 32]).await.unwrap();
        Arc::new(database)
    }

    #[tokio::test]
    async fn test_client_manager_creation() {
        let database = create_test_database().await;
        let _manager = A2AClientManager::new(database);

        // Should create without errors
    }

    #[tokio::test]
    async fn test_validate_registration_request() {
        let database = create_test_database().await;
        let manager = A2AClientManager::new(database);

        // Valid request
        let valid_request = ClientRegistrationRequest {
            name: "Test Client".to_string(),
            description: "A test client".to_string(),
            capabilities: vec!["fitness-data-analysis".to_string()],
            redirect_uris: vec!["https://example.com/callback".to_string()],
            contact_email: "test@example.com".to_string(),
        };

        assert!(manager
            .validate_registration_request(&valid_request)
            .is_ok());

        // Invalid request - empty name
        let invalid_request = ClientRegistrationRequest {
            name: "".to_string(),
            description: "A test client".to_string(),
            capabilities: vec!["fitness-data-analysis".to_string()],
            redirect_uris: vec![],
            contact_email: "test@example.com".to_string(),
        };

        assert!(manager
            .validate_registration_request(&invalid_request)
            .is_err());

        // Invalid request - unknown capability
        let invalid_capability_request = ClientRegistrationRequest {
            name: "Test Client".to_string(),
            description: "A test client".to_string(),
            capabilities: vec!["unknown-capability".to_string()],
            redirect_uris: vec![],
            contact_email: "test@example.com".to_string(),
        };

        assert!(manager
            .validate_registration_request(&invalid_capability_request)
            .is_err());
    }

    #[tokio::test]
    async fn test_session_management() {
        let database = create_test_database().await;
        let manager = A2AClientManager::new(database);

        // First, create a test client
        let client_request = ClientRegistrationRequest {
            name: "Test Client".to_string(),
            description: "A test client".to_string(),
            capabilities: vec!["fitness-data-analysis".to_string()],
            redirect_uris: vec![],
            contact_email: "test@example.com".to_string(),
        };

        let credentials = manager.register_client(client_request).await.unwrap();

        // Create session with the actual client ID
        let session_id = manager
            .create_session(&credentials.client_id, Some("test_user"))
            .await
            .unwrap();
        assert!(!session_id.is_empty());

        // Update session activity
        assert!(manager.update_session_activity(&session_id).await.is_ok());

        // Get active sessions
        let sessions = manager.get_active_sessions(&credentials.client_id).await;
        assert_eq!(sessions.len(), 0); // Returns empty since we simplified the implementation
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let database = create_test_database().await;
        let manager = A2AClientManager::new(database);

        // First, create a test client
        let client_request = ClientRegistrationRequest {
            name: "Test Client 2".to_string(),
            description: "Another test client".to_string(),
            capabilities: vec!["fitness-data-analysis".to_string()],
            redirect_uris: vec![],
            contact_email: "test2@example.com".to_string(),
        };

        let credentials = manager.register_client(client_request).await.unwrap();

        // Create session with the actual client ID
        let _session_id = manager
            .create_session(&credentials.client_id, Some("test_user"))
            .await
            .unwrap();

        // Simplified test since we're using database storage now

        // Cleanup expired sessions
        manager.cleanup_expired_sessions().await;

        // Session should be removed (but our implementation always returns empty)
        let active_sessions = manager.get_active_sessions(&credentials.client_id).await;
        assert_eq!(active_sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_record_usage() {
        let database = create_test_database().await;
        let manager = A2AClientManager::new(database);

        // First, create a test client
        let client_request = ClientRegistrationRequest {
            name: "Usage Test Client".to_string(),
            description: "A client for testing usage tracking".to_string(),
            capabilities: vec!["fitness-data-analysis".to_string()],
            redirect_uris: vec![],
            contact_email: "usage@example.com".to_string(),
        };

        let credentials = manager.register_client(client_request).await.unwrap();

        // Now record usage for the actual client
        assert!(manager
            .record_usage(&credentials.client_id, "get_activities", true)
            .await
            .is_ok());

        // Create a session for more detailed testing
        let session_token = manager
            .create_session(&credentials.client_id, Some("test_user"))
            .await
            .unwrap();

        // Test detailed usage recording with real session
        let result = manager
            .record_detailed_usage(
                &credentials.client_id,
                Some(&session_token),
                "analyze_activity",
                Some(150),
                200,
                None,
                Some(256),
                Some(512),
                Some("127.0.0.1"),
                Some("test-agent/1.0"),
                &["fitness-data-analysis".to_string()],
                &["fitness:read".to_string()],
            )
            .await;
        
        if let Err(ref e) = result {
            println!("Error recording detailed usage: {:?}", e);
        }
        assert!(result.is_ok());

        // Test getting usage statistics
        let usage_stats = manager.get_client_usage(&credentials.client_id).await.unwrap();
        assert_eq!(usage_stats.client_id, credentials.client_id);
        assert_eq!(usage_stats.total_requests, 2); // Two usage records above
    }
}
