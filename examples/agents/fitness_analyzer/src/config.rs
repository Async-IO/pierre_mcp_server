// ABOUTME: Configuration management for the FitnessAnalysisAgent
// ABOUTME: Handles environment variables and configuration file parsing
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Pierre server base URL
    pub server_url: String,
    
    /// A2A client credentials
    pub client_id: String,
    pub client_secret: String,
    
    /// Analysis scheduling configuration
    pub analysis_interval_hours: u64,
    
    /// Agent behavior settings
    pub development_mode: bool,
    pub max_activities_per_analysis: u32,
    
    /// Reporting configuration
    pub generate_reports: bool,
    pub report_output_dir: String,
}

impl AgentConfig {
    /// Load configuration from environment variables and defaults
    pub fn load() -> Result<Self> {
        let config = Self {
            server_url: std::env::var("PIERRE_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            
            client_id: std::env::var("PIERRE_A2A_CLIENT_ID")
                .context("PIERRE_A2A_CLIENT_ID environment variable is required")?,
            
            client_secret: std::env::var("PIERRE_A2A_CLIENT_SECRET")
                .context("PIERRE_A2A_CLIENT_SECRET environment variable is required")?,
            
            analysis_interval_hours: std::env::var("ANALYSIS_INTERVAL_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .context("ANALYSIS_INTERVAL_HOURS must be a valid number")?,
            
            development_mode: std::env::var("DEVELOPMENT_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .context("DEVELOPMENT_MODE must be true or false")?,
            
            max_activities_per_analysis: std::env::var("MAX_ACTIVITIES_PER_ANALYSIS")
                .unwrap_or_else(|_| "200".to_string())
                .parse()
                .context("MAX_ACTIVITIES_PER_ANALYSIS must be a valid number")?,
            
            generate_reports: std::env::var("GENERATE_REPORTS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .context("GENERATE_REPORTS must be true or false")?,
            
            report_output_dir: std::env::var("REPORT_OUTPUT_DIR")
                .unwrap_or_else(|_| "/tmp/fitness_reports".to_string()),
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        if self.client_id.is_empty() {
            anyhow::bail!("Client ID cannot be empty");
        }
        
        if self.client_secret.is_empty() {
            anyhow::bail!("Client secret cannot be empty");
        }
        
        if self.analysis_interval_hours == 0 {
            anyhow::bail!("Analysis interval must be greater than 0");
        }
        
        if self.max_activities_per_analysis == 0 {
            anyhow::bail!("Max activities per analysis must be greater than 0");
        }
        
        Ok(())
    }

    /// Get analysis interval as Duration
    pub fn analysis_interval(&self) -> Duration {
        Duration::from_secs(self.analysis_interval_hours * 3600)
    }

    /// Get development mode analysis interval (shorter for testing)
    pub fn dev_analysis_interval(&self) -> Duration {
        if self.development_mode {
            Duration::from_secs(60) // 1 minute in dev mode
        } else {
            self.analysis_interval()
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8081".to_string(),
            client_id: "".to_string(),
            client_secret: "".to_string(),
            analysis_interval_hours: 24,
            development_mode: false,
            max_activities_per_analysis: 200,
            generate_reports: true,
            report_output_dir: "/tmp/fitness_reports".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = AgentConfig::default();
        
        // Should fail with empty client credentials
        assert!(config.validate().is_err());
        
        // Should pass with valid credentials
        config.client_id = "test_client".to_string();
        config.client_secret = "test_secret".to_string();
        assert!(config.validate().is_ok());
        
        // Should fail with zero interval
        config.analysis_interval_hours = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_development_mode_interval() {
        let mut config = AgentConfig::default();
        config.development_mode = false;
        config.analysis_interval_hours = 24;
        
        // Production mode should use full interval
        assert_eq!(config.dev_analysis_interval(), Duration::from_secs(24 * 3600));
        
        // Development mode should use 1 minute
        config.development_mode = true;
        assert_eq!(config.dev_analysis_interval(), Duration::from_secs(60));
    }
}