// ABOUTME: A2A protocol client implementation for direct JSON-RPC communication
// ABOUTME: Demonstrates raw A2A protocol usage without SDK abstractions
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// A2A authentication response
#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

/// JSON-RPC request structure
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: String,
}

/// JSON-RPC response structure
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: String,
}

/// JSON-RPC error structure
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Activity data structure returned by the API
#[derive(Debug, Deserialize, Clone)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub sport_type: String,
    pub distance_meters: Option<f64>,
    pub duration_seconds: Option<u32>,
    pub elevation_gain: Option<f64>,
    pub average_heart_rate: Option<u32>,
    pub max_heart_rate: Option<u32>,
    pub start_date: String,
    pub provider: String,
}

/// A2A client for direct protocol communication
pub struct A2AClient {
    http_client: Client,
    server_url: String,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
    token_expires_at: Option<Instant>,
}

impl A2AClient {
    /// Create a new A2A client
    pub fn new(server_url: String, client_id: String, client_secret: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("FitnessAnalysisAgent/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            server_url,
            client_id,
            client_secret,
            access_token: None,
            token_expires_at: None,
        }
    }

    /// Authenticate with A2A client credentials
    pub async fn authenticate(&mut self) -> Result<()> {
        info!("🔐 Authenticating via A2A protocol");
        debug!("Authenticating client_id: {}", self.client_id);

        let auth_payload = json!({
            "client_id": self.client_id,
            "client_secret": self.client_secret,
            "grant_type": "client_credentials",
            "scope": "read write"
        });

        let response = self
            .http_client
            .post(format!("{}/a2a/auth", self.server_url))
            .header("Content-Type", "application/json")
            .json(&auth_payload)
            .send()
            .await
            .context("Failed to send authentication request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Authentication failed: HTTP {} - {}", status, error_text);
        }

        let auth_response: AuthResponse = response
            .json()
            .await
            .context("Failed to parse authentication response")?;

        // Store token with expiration time
        self.access_token = Some(auth_response.access_token.clone());
        self.token_expires_at = Some(Instant::now() + Duration::from_secs(auth_response.expires_in));

        info!("✅ A2A authentication successful, token expires in {}s", auth_response.expires_in);
        debug!("Access token: {}...", &auth_response.access_token[..20]);

        Ok(())
    }

    /// Check if we need to refresh the access token
    async fn ensure_authenticated(&mut self) -> Result<()> {
        let needs_refresh = match (&self.access_token, self.token_expires_at) {
            (None, _) => true,
            (Some(_), None) => true,
            (Some(_), Some(expires_at)) => {
                // Refresh 5 minutes before expiration
                Instant::now() + Duration::from_secs(300) > expires_at
            }
        };

        if needs_refresh {
            info!("🔄 Refreshing A2A access token");
            self.authenticate().await?;
        }

        Ok(())
    }

    /// Execute a tool via A2A JSON-RPC protocol
    pub async fn execute_tool(&mut self, tool_name: &str, parameters: Value) -> Result<Value> {
        self.ensure_authenticated().await?;

        let request_id = Uuid::new_v4().to_string();
        
        // Construct JSON-RPC 2.0 request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: json!({
                "name": tool_name,
                "arguments": parameters
            }),
            id: request_id.clone(),
        };

        debug!("📤 Sending A2A request: {} with params: {}", tool_name, parameters);

        let access_token = self.access_token.as_ref()
            .context("No access token available")?;

        let response = self
            .http_client
            .post(format!("{}/a2a/execute", self.server_url))
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", access_token))
            .json(&request)
            .send()
            .await
            .context("Failed to send A2A request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("A2A request failed: HTTP {} - {}", status, error_text);
        }

        let json_rpc_response: JsonRpcResponse = response
            .json()
            .await
            .context("Failed to parse A2A response")?;

        // Check for JSON-RPC errors
        if let Some(error) = json_rpc_response.error {
            anyhow::bail!("A2A JSON-RPC error {}: {}", error.code, error.message);
        }

        // Verify request ID matches
        if json_rpc_response.id != request_id {
            warn!("Request ID mismatch: sent {}, received {}", request_id, json_rpc_response.id);
        }

        let result = json_rpc_response.result
            .context("A2A response missing result field")?;

        debug!("📥 A2A response received for {}: {}", tool_name, 
            serde_json::to_string(&result).unwrap_or_else(|_| "invalid json".to_string()));

        Ok(result)
    }

    /// Get activities from fitness providers via A2A
    pub async fn get_activities(&mut self, provider: &str, limit: u32) -> Result<Vec<Activity>> {
        info!("📊 Fetching {} activities from {} via A2A", limit, provider);

        let params = json!({
            "provider": provider,
            "limit": limit
        });

        let result = self.execute_tool("get_activities", params).await?;

        // Parse activities from result
        let activities: Vec<Activity> = if result.is_array() {
            serde_json::from_value(result)
                .context("Failed to parse activities array")?
        } else if let Some(activities_value) = result.get("activities") {
            serde_json::from_value(activities_value.clone())
                .context("Failed to parse activities from object")?
        } else {
            anyhow::bail!("Unexpected response format: activities not found");
        };

        info!("✅ Retrieved {} activities via A2A", activities.len());
        Ok(activities)
    }

    /// Get athlete profile information
    pub async fn get_athlete_profile(&mut self, provider: &str) -> Result<Value> {
        info!("👤 Fetching athlete profile from {} via A2A", provider);

        let params = json!({
            "provider": provider
        });

        let result = self.execute_tool("get_athlete", params).await?;
        info!("✅ Retrieved athlete profile via A2A");
        Ok(result)
    }

    /// Calculate fitness metrics via A2A
    pub async fn calculate_fitness_metrics(&mut self, provider: &str) -> Result<Value> {
        info!("🧮 Calculating fitness metrics for {} via A2A", provider);

        let params = json!({
            "provider": provider,
            "metrics": ["fitness_score", "training_load", "performance_trends"]
        });

        let result = self.execute_tool("calculate_metrics", params).await?;
        info!("✅ Calculated fitness metrics via A2A");
        Ok(result)
    }

    /// Generate training recommendations via A2A
    pub async fn generate_recommendations(&mut self, provider: &str) -> Result<Value> {
        info!("💡 Generating training recommendations for {} via A2A", provider);

        let params = json!({
            "provider": provider,
            "recommendation_types": ["training_plan", "recovery", "performance_optimization"]
        });

        let result = self.execute_tool("generate_recommendations", params).await?;
        info!("✅ Generated recommendations via A2A");
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = A2AClient::new(
            "http://localhost:8081".to_string(),
            "test_client".to_string(),
            "test_secret".to_string(),
        );

        assert_eq!(client.server_url, "http://localhost:8081");
        assert_eq!(client.client_id, "test_client");
        assert_eq!(client.client_secret, "test_secret");
        assert!(client.access_token.is_none());
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "get_activities".to_string(),
            params: json!({"provider": "strava", "limit": 10}),
            id: "test-123".to_string(),
        };

        let serialized = serde_json::to_value(&request).unwrap();
        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["method"], "get_activities");
        assert_eq!(serialized["id"], "test-123");
    }
}