// ABOUTME: Demonstrates A2A agent card discovery and capability negotiation
// ABOUTME: Shows how agents discover each other's capabilities before collaboration
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Agent Discovery Example
//!
//! This example demonstrates the A2A protocol's agent card discovery mechanism,
//! showing how agents:
//! 1. Fetch agent cards to discover capabilities
//! 2. Parse and validate agent capabilities
//! 3. Negotiate authentication methods
//! 4. Make informed decisions about which agent to use

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tracing::{info, warn};

/// Agent Card structure per A2A specification
#[derive(Debug, Deserialize, Serialize)]
struct AgentCard {
    /// Agent name
    name: String,
    /// Human-readable description
    description: String,
    /// Agent version
    version: String,
    /// High-level capabilities
    capabilities: Vec<String>,
    /// Authentication information
    authentication: AuthenticationInfo,
    /// Available tools
    tools: Vec<ToolDefinition>,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
}

/// Authentication information from agent card
#[derive(Debug, Deserialize, Serialize)]
struct AuthenticationInfo {
    /// Supported authentication schemes
    schemes: Vec<String>,
    /// `OAuth2` configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    oauth2: Option<OAuth2Info>,
    /// API key configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<ApiKeyInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OAuth2Info {
    authorization_url: String,
    token_url: String,
    scopes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ApiKeyInfo {
    header_name: String,
    prefix: Option<String>,
    registration_url: String,
}

/// Tool definition from agent card
#[derive(Debug, Deserialize, Serialize)]
struct ToolDefinition {
    name: String,
    description: String,
    input_schema: Value,
    output_schema: Value,
}

/// Agent Discovery Client
struct AgentDiscovery {
    http_client: Client,
    server_url: String,
}

impl AgentDiscovery {
    /// Create a new agent discovery client
    fn new(server_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("AgentDiscoveryExample/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            server_url,
        }
    }

    /// Fetch agent card from server
    async fn fetch_agent_card(&self) -> Result<AgentCard> {
        info!("📡 Fetching agent card from: {}", self.server_url);

        let url = format!("{}/a2a/agent-card", self.server_url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch agent card")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch agent card: HTTP {}", response.status());
        }

        let agent_card: AgentCard = response
            .json()
            .await
            .context("Failed to parse agent card JSON")?;

        info!("✅ Successfully fetched agent card for: {}", agent_card.name);
        Ok(agent_card)
    }

    /// Analyze agent capabilities
    fn analyze_capabilities(card: &AgentCard) {
        info!("\n📊 Agent Capability Analysis:");
        info!("   Agent: {} v{}", card.name, card.version);
        info!("   Description: {}", card.description);

        info!("\n🔧 Available Capabilities ({}):", card.capabilities.len());
        for capability in &card.capabilities {
            info!("   • {}", capability);
        }

        info!("\n🛠️  Available Tools ({}):", card.tools.len());
        for tool in &card.tools {
            info!("   • {} - {}", tool.name, tool.description);
        }

        info!("\n🔐 Authentication Methods:");
        for scheme in &card.authentication.schemes {
            info!("   • {}", scheme);
        }

        if let Some(oauth2) = &card.authentication.oauth2 {
            info!("\n   OAuth2 Configuration:");
            info!("      Authorization URL: {}", oauth2.authorization_url);
            info!("      Token URL: {}", oauth2.token_url);
            info!("      Scopes: {}", oauth2.scopes.join(", "));
        }
    }

    /// Check if agent has required capability
    fn has_capability(card: &AgentCard, required_capability: &str) -> bool {
        card.capabilities
            .iter()
            .any(|cap| cap.contains(required_capability))
    }

    /// Find tools matching a pattern
    fn find_tools<'a>(card: &'a AgentCard, pattern: &str) -> Vec<&'a ToolDefinition> {
        card.tools
            .iter()
            .filter(|tool| {
                tool.name.contains(pattern) || tool.description.to_lowercase().contains(pattern)
            })
            .collect()
    }

    /// Recommend best authentication method
    fn recommend_auth_method(card: &AgentCard) -> String {
        if card.authentication.schemes.contains(&"oauth2".to_string()) {
            info!("💡 Recommendation: Use OAuth2 for secure user-delegated access");
            "oauth2".to_string()
        } else if card.authentication.schemes.contains(&"api-key".to_string()) {
            info!("💡 Recommendation: Use API Key for service-to-service communication");
            "api-key".to_string()
        } else {
            warn!("⚠️  No standard authentication method found");
            "unknown".to_string()
        }
    }

    /// Demonstrate capability negotiation
    async fn demonstrate_capability_negotiation(&self) -> Result<()> {
        info!("\n🤝 Starting A2A Capability Negotiation Demo\n");

        // Step 1: Fetch agent card
        let agent_card = self.fetch_agent_card().await?;

        // Step 2: Analyze capabilities
        Self::analyze_capabilities(&agent_card);

        // Step 3: Check for specific capabilities
        info!("\n🔍 Capability Check:");
        let required_capabilities = vec![
            "fitness-data-analysis",
            "activity-intelligence",
            "performance-prediction",
        ];

        for capability in required_capabilities {
            let has_it = Self::has_capability(&agent_card, capability);
            if has_it {
                info!("   ✅ Has capability: {}", capability);
            } else {
                info!("   ❌ Missing capability: {}", capability);
            }
        }

        // Step 4: Find relevant tools
        info!("\n🔎 Finding fitness-related tools:");
        let fitness_tools = Self::find_tools(&agent_card, "activit");
        for tool in &fitness_tools {
            info!("   • {} - {}", tool.name, tool.description);
        }

        // Step 5: Recommend authentication
        info!("\n🔐 Authentication Method Recommendation:");
        let _auth_method = Self::recommend_auth_method(&agent_card);

        // Step 6: Demonstrate decision making
        info!("\n✅ Agent Suitability Assessment:");
        let is_suitable = Self::has_capability(&agent_card, "fitness");
        if is_suitable {
            info!("   ✅ This agent is suitable for fitness data analysis tasks");
            info!("   ✅ Supports {} tools for fitness analysis", fitness_tools.len());
            info!("   ✅ Recommended for integration");
        } else {
            info!("   ❌ This agent may not be suitable for fitness tasks");
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("agent_discovery_example=info")
        .init();

    info!("🚀 A2A Agent Discovery Example");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Get server URL from environment or use default
    let server_url = std::env::var("PIERRE_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string());

    // Create discovery client
    let discovery = AgentDiscovery::new(server_url);

    // Run the demonstration
    match discovery.demonstrate_capability_negotiation().await {
        Ok(()) => {
            info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            info!("✅ Agent Discovery Demo Completed Successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            tracing::error!("❌ Agent Discovery Demo Failed: {}", e);
            tracing::error!("   Make sure Pierre server is running: cargo run --bin pierre-mcp-server");
            Err(e)
        }
    }
}
