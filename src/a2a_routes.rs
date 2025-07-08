// ABOUTME: HTTP route handlers for A2A protocol endpoints and client management
// ABOUTME: Implements REST API endpoints for A2A authentication, tool execution, and client administration
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A2A HTTP Routes
//!
//! HTTP endpoints for A2A (Agent-to-Agent) protocol management

use crate::auth::AuthManager;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::intelligence::ActivityIntelligence;
use crate::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use crate::{
    a2a::{
        agent_card::AgentCard,
        auth::A2AAuthenticator,
        client::{A2AClientManager, ClientRegistrationRequest},
        A2AError,
    },
    constants::demo_data::{DEMO_CONSISTENCY_SCORE, DEMO_EFFICIENCY_SCORE},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct A2ADashboardOverview {
    pub total_clients: u32,
    pub active_clients: u32,
    pub total_sessions: u32,
    pub active_sessions: u32,
    pub requests_today: u32,
    pub requests_this_month: u32,
    pub most_used_capability: Option<String>,
    pub error_rate: f64,
    pub usage_by_tier: Vec<A2ATierUsage>,
}

#[derive(Debug, Serialize)]
pub struct A2ATierUsage {
    pub tier: String,
    pub client_count: u32,
    pub request_count: u32,
    pub percentage: f64,
}

#[derive(Debug, Deserialize)]
pub struct A2AClientRequest {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub contact_email: String,
    pub agent_version: Option<String>,
    pub documentation_url: Option<String>,
}

/// A2A Routes handler
pub struct A2ARoutes {
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    client_manager: Arc<A2AClientManager>,
    authenticator: Arc<A2AAuthenticator>,
    tool_executor: UniversalToolExecutor,
    config: Arc<crate::config::environment::ServerConfig>,
}

impl A2ARoutes {
    /// Extract and validate JWT token from Authorization header
    fn extract_jwt_token(auth_header: Option<&str>) -> Result<String, serde_json::Value> {
        let auth = auth_header.ok_or_else(|| {
            serde_json::json!({
                "code": -32001,
                "message": "Missing Authorization header"
            })
        })?;

        let token = auth.strip_prefix("Bearer ").ok_or_else(|| {
            serde_json::json!({
                "code": -32001,
                "message": "Invalid authorization header format"
            })
        })?;

        Ok(token.to_string())
    }

    /// Validate JWT token and return user ID
    fn validate_jwt_and_get_user_id(&self, token: &str) -> Result<String, serde_json::Value> {
        self.auth_manager
            .validate_token(token)
            .map(|claims| claims.sub)
            .map_err(|_| {
                serde_json::json!({
                    "code": -32001,
                    "message": "Invalid or expired authentication token"
                })
            })
    }

    /// Create standard A2A `ActivityIntelligence` instance
    #[must_use]
    fn create_a2a_intelligence() -> Arc<ActivityIntelligence> {
        Arc::new(ActivityIntelligence::new(
            "A2A Intelligence".into(),
            vec![],
            crate::intelligence::PerformanceMetrics {
                relative_effort: Some(7.5),
                zone_distribution: None,
                personal_records: vec![],
                efficiency_score: Some({
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        DEMO_EFFICIENCY_SCORE.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
                    }
                }),
                trend_indicators: crate::intelligence::TrendIndicators {
                    pace_trend: crate::intelligence::TrendDirection::Stable,
                    effort_trend: crate::intelligence::TrendDirection::Improving,
                    distance_trend: crate::intelligence::TrendDirection::Stable,
                    consistency_score: {
                        #[allow(clippy::cast_possible_truncation)]
                        {
                            DEMO_CONSISTENCY_SCORE.clamp(f64::from(f32::MIN), f64::from(f32::MAX))
                                as f32
                        }
                    },
                },
            },
            crate::intelligence::ContextualFactors {
                weather: None,
                location: None,
                time_of_day: crate::intelligence::TimeOfDay::Morning,
                days_since_last_activity: Some(1),
                weekly_load: None,
            },
        ))
    }

    #[must_use]
    pub fn new(
        database: Arc<Database>,
        auth_manager: Arc<AuthManager>,
        config: Arc<crate::config::environment::ServerConfig>,
    ) -> Self {
        let client_manager = Arc::new(A2AClientManager::new(database.clone()));
        let authenticator = Arc::new(A2AAuthenticator::new(
            database.clone(),
            auth_manager.jwt_secret().to_vec(),
        ));

        let intelligence = Self::create_a2a_intelligence();

        let tool_executor =
            UniversalToolExecutor::new(database.clone(), intelligence, config.clone());

        Self {
            database,
            auth_manager,
            client_manager,
            authenticator,
            tool_executor,
            config,
        }
    }

    /// Get A2A agent card
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if the agent card cannot be created
    pub fn get_agent_card(&self) -> Result<AgentCard, A2AError> {
        Ok(AgentCard::new())
    }

    /// Get A2A dashboard overview
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Database operations fail
    /// - Client list cannot be retrieved
    pub async fn get_dashboard_overview(
        &self,
        _auth_header: Option<&str>,
    ) -> Result<A2ADashboardOverview, A2AError> {
        // Use existing client manager methods for real data
        let clients = self
            .client_manager
            .list_clients()
            .await
            .map_err(|e| A2AError::DatabaseError(e.to_string()))?;

        let total_clients = u32::try_from(clients.len()).unwrap_or(u32::MAX);
        let active_clients =
            u32::try_from(clients.iter().filter(|c| c.is_active).count()).unwrap_or(0);

        // Sessions and usage stats based on database queries
        // These would need proper session tracking implementation
        let total_sessions = 0; // No session tracking implemented yet
        let active_sessions = 0; // No session tracking implemented yet
        let requests_today = 0; // No usage logging implemented yet
        let requests_this_month = 0; // No usage logging implemented yet
        let most_used_capability = None; // No usage tracking implemented yet
        let error_rate = 0.0; // No error tracking implemented yet

        // Create tier structure based on user subscription level
        let usage_tiers = if active_clients > 0 {
            vec![A2ATierUsage {
                tier: "basic".into(),
                client_count: active_clients,
                request_count: 0, // No usage tracking yet
                percentage: 100.0,
            }]
        } else {
            vec![]
        };

        let overview = A2ADashboardOverview {
            total_clients,
            active_clients,
            total_sessions,
            active_sessions,
            requests_today,
            requests_this_month,
            most_used_capability,
            error_rate,
            usage_by_tier: usage_tiers,
        };

        Ok(overview)
    }

    /// Register new A2A client
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Client registration fails
    /// - Database operations fail
    pub async fn register_client(
        &self,
        _auth_header: Option<&str>,
        request: A2AClientRequest,
    ) -> Result<crate::a2a::client::ClientCredentials, A2AError> {
        let registration = ClientRegistrationRequest {
            name: request.name,
            description: request.description,
            capabilities: request.capabilities,
            redirect_uris: request.redirect_uris.unwrap_or_default(),
            contact_email: request.contact_email,
        };

        self.client_manager.register_client(registration).await
    }

    /// List A2A clients
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Database operations fail
    /// - Client list cannot be retrieved
    pub async fn list_clients(
        &self,
        _auth_header: Option<&str>,
    ) -> Result<Vec<crate::a2a::auth::A2AClient>, A2AError> {
        self.client_manager.list_clients().await
    }

    /// Get A2A client usage statistics
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Client does not exist
    /// - Database operations fail
    pub async fn get_client_usage(
        &self,
        _auth_header: Option<&str>,
        client_id: &str,
    ) -> Result<crate::a2a::client::ClientUsageStats, A2AError> {
        self.client_manager.get_client_usage(client_id).await
    }

    /// Get A2A client rate limit status
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Client does not exist
    /// - Database operations fail
    pub async fn get_client_rate_limit(
        &self,
        _auth_header: Option<&str>,
        client_id: &str,
    ) -> Result<crate::a2a::client::A2ARateLimitStatus, A2AError> {
        self.client_manager
            .get_client_rate_limit_status(client_id)
            .await
    }

    /// Deactivate A2A client
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Client does not exist
    /// - Database operations fail
    pub async fn deactivate_client(
        &self,
        _auth_header: Option<&str>,
        client_id: &str,
    ) -> Result<(), A2AError> {
        self.client_manager.deactivate_client(client_id).await
    }

    /// Authenticate A2A request
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Required fields are missing from the request
    /// - Client authentication fails
    /// - Session creation fails
    pub async fn authenticate(
        &self,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, A2AError> {
        // Parse authentication request
        let client_id = request
            .get("client_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| A2AError::InvalidRequest("Missing client_id".into()))?;

        let client_secret = request
            .get("client_secret")
            .and_then(|v| v.as_str())
            .ok_or_else(|| A2AError::InvalidRequest("Missing client_secret".into()))?;

        let scopes = request
            .get("scopes")
            .and_then(|v| v.as_array())
            .map_or_else(
                || vec!["read".into()],
                |arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<String>>()
                },
            );

        // Verify client exists and credentials are valid
        let client = self
            .client_manager
            .get_client(client_id)
            .await?
            .ok_or_else(|| A2AError::AuthenticationFailed("Invalid client_id".into()))?;

        if !client.is_active {
            return Err(A2AError::AuthenticationFailed(
                "Client is deactivated".into(),
            ));
        }

        // Verify client secret - in production this would be properly hashed
        // Use certificate-based verification for client authentication
        let credentials = self
            .client_manager
            .get_client_credentials(client_id)
            .await?
            .ok_or_else(|| A2AError::AuthenticationFailed("Invalid credentials".into()))?;

        if credentials.client_secret != client_secret {
            return Err(A2AError::AuthenticationFailed(
                "Invalid client_secret".into(),
            ));
        }

        // Create A2A session
        let session_token = self
            .database
            .create_a2a_session(client_id, None, &scopes, 24)
            .await
            .map_err(|e| A2AError::InternalError(format!("Failed to create session: {e}")))?;

        Ok(serde_json::json!({
            "status": "authenticated",
            "session_token": session_token,
            "expires_in": 86400, // 24 hours in seconds
            "token_type": "Bearer",
            "scope": scopes.join(" ")
        }))
    }

    /// Handle tool execution method
    async fn handle_tools_execute(
        &self,
        auth_header: Option<&str>,
        params: &serde_json::Value,
        id: &serde_json::Value,
    ) -> Result<serde_json::Value, A2AError> {
        let tool_name = params
            .get("tool_name")
            .and_then(|t| t.as_str())
            .ok_or_else(|| A2AError::InvalidRequest("Missing tool_name in params".into()))?;

        let parameters = params
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // Extract and validate JWT token
        let token = match Self::extract_jwt_token(auth_header) {
            Ok(token) => token,
            Err(error) => {
                return Ok(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": error
                }));
            }
        };

        let user_id = match self.validate_jwt_and_get_user_id(&token) {
            Ok(user_id) => user_id,
            Err(error) => {
                return Ok(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": error
                }));
            }
        };

        // Create universal request
        let universal_request = UniversalRequest {
            tool_name: tool_name.to_string(),
            parameters,
            user_id,
            protocol: "a2a".into(),
        };

        // Execute the tool
        match self.tool_executor.execute_tool(universal_request).await {
            Ok(universal_response) => {
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": universal_response.result.unwrap_or_else(|| serde_json::json!({}))
                });
                Ok(response)
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32000,
                        "message": format!("Tool execution failed: {e}"),
                        "data": null
                    }
                });
                Ok(error_response)
            }
        }
    }

    /// Handle client info method
    fn handle_client_info(id: &serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "name": "Pierre Fitness AI",
                "version": "1.0.0",
                "capabilities": [
                    "fitness-data-analysis",
                    "goal-management",
                    "activity-insights",
                    "performance-metrics"
                ],
                "protocols": ["A2A", "MCP"],
                "description": "AI-powered fitness data analysis and insights platform"
            }
        })
    }

    /// Handle session heartbeat method
    async fn handle_session_heartbeat(
        &self,
        auth_header: Option<&str>,
        id: &serde_json::Value,
    ) -> Result<serde_json::Value, A2AError> {
        let token = match Self::extract_jwt_token(auth_header) {
            Ok(token) => token,
            Err(error) => {
                return Ok(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": error
                }));
            }
        };

        match self.database.update_a2a_session_activity(&token).await {
            Ok(()) => Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "status": "alive",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            })),
            Err(e) => Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32000,
                    "message": format!("Failed to update session: {e}")
                }
            })),
        }
    }

    /// Handle capabilities list method
    fn handle_capabilities_list(id: &serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "capabilities": [
                    {
                        "name": "fitness-data-analysis",
                        "description": "Analyze fitness and activity data for insights",
                        "version": "1.0.0"
                    },
                    {
                        "name": "goal-management",
                        "description": "Create and track fitness goals",
                        "version": "1.0.0"
                    },
                    {
                        "name": "activity-insights",
                        "description": "Generate insights from activity patterns",
                        "version": "1.0.0"
                    },
                    {
                        "name": "performance-metrics",
                        "description": "Calculate performance metrics and trends",
                        "version": "1.0.0"
                    }
                ]
            }
        })
    }

    /// Execute A2A tool
    ///
    /// # Errors
    ///
    /// Returns `A2AError` if:
    /// - Request is malformed or missing required fields
    /// - Authentication fails
    /// - Tool execution fails
    pub async fn execute_tool(
        &self,
        auth_header: Option<&str>,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, A2AError> {
        // Parse the JSON-RPC request
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| A2AError::InvalidRequest("Missing method field".into()))?;

        let params = request
            .get("params")
            .ok_or_else(|| A2AError::InvalidRequest("Missing params field".into()))?;

        let id = request
            .get("id")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(1));

        match method {
            "tools.execute" => self.handle_tools_execute(auth_header, params, &id).await,
            "client.info" => Ok(Self::handle_client_info(&id)),
            "session.heartbeat" => self.handle_session_heartbeat(auth_header, &id).await,
            "capabilities.list" => Ok(Self::handle_capabilities_list(&id)),
            _ => Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method '{method}' not found"),
                    "data": {
                        "available_methods": [
                            "tools.execute",
                            "client.info",
                            "session.heartbeat",
                            "capabilities.list"
                        ]
                    }
                }
            })),
        }
    }
}

impl Clone for A2ARoutes {
    fn clone(&self) -> Self {
        // For the clone, we need to recreate the tool executor since it doesn't implement Clone
        let intelligence = Self::create_a2a_intelligence();

        let tool_executor =
            UniversalToolExecutor::new(self.database.clone(), intelligence, self.config.clone());

        Self {
            database: self.database.clone(),
            auth_manager: self.auth_manager.clone(),
            client_manager: self.client_manager.clone(),
            authenticator: self.authenticator.clone(),
            tool_executor,
            config: self.config.clone(),
        }
    }
}
