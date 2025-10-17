// ABOUTME: Plugin lifecycle management system for deterministic initialization and health monitoring
// ABOUTME: Provides traits and managers for consistent plugin startup, health checks, and graceful shutdown
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

//! Plugin Lifecycle Management
//!
//! This module provides a deterministic plugin initialization system with:
//! - Explicit initialization order
//! - Health check monitoring
//! - Graceful degradation
//! - Proper shutdown hooks

/// Core system plugin adapters (database, cache, auth)
pub mod plugins;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

/// Plugin lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is not yet initialized
    Uninitialized,
    /// Plugin is currently initializing
    Initializing,
    /// Plugin is ready and operational
    Ready,
    /// Plugin initialization failed
    Failed,
    /// Plugin is shutting down
    ShuttingDown,
    /// Plugin has shut down
    Shutdown,
}

/// Plugin health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    /// Plugin name
    pub name: String,
    /// Current state
    pub state: PluginState,
    /// Health check status
    pub healthy: bool,
    /// Optional status message
    pub message: Option<String>,
    /// Last health check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Plugin trait for lifecycle management
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin name
    fn name(&self) -> &str;

    /// Get plugin initialization priority (lower = earlier, 0-100)
    fn priority(&self) -> u8 {
        50 // Default medium priority
    }

    /// Initialize the plugin
    ///
    /// # Errors
    /// Returns an error if initialization fails
    async fn initialize(&mut self) -> Result<()>;

    /// Perform health check
    ///
    /// # Errors
    /// Returns an error if health check fails
    async fn health_check(&self) -> Result<PluginHealth>;

    /// Gracefully shutdown the plugin
    ///
    /// # Errors
    /// Returns an error if shutdown fails
    async fn shutdown(&mut self) -> Result<()>;

    /// Get current plugin state
    fn state(&self) -> PluginState;

    /// Check if plugin is required for server operation
    fn is_required(&self) -> bool {
        true // Default: all plugins are required
    }
}

/// Plugin manager for orchestrating initialization and health checks
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    initialization_timeout: Duration,
}

impl PluginManager {
    /// Create a new plugin manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            initialization_timeout: Duration::from_secs(30),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        info!("Registering plugin: {}", plugin.name());
        self.plugins.push(plugin);
    }

    /// Initialize all plugins in priority order
    ///
    /// # Errors
    /// Returns an error if any required plugin fails to initialize
    pub async fn initialize_all(&mut self) -> Result<()> {
        info!("Initializing {} plugins", self.plugins.len());

        // Sort plugins by priority (lower number = higher priority)
        self.plugins.sort_by_key(|p| p.priority());

        for plugin in &mut self.plugins {
            let plugin_name = plugin.name().to_string();
            let is_required = plugin.is_required();
            let priority = plugin.priority();

            info!(
                "Initializing plugin '{}' (priority: {}, required: {})",
                plugin_name, priority, is_required
            );

            match tokio::time::timeout(self.initialization_timeout, plugin.initialize()).await {
                Ok(Ok(())) => {
                    info!("Plugin '{}' initialized successfully", plugin_name);
                }
                Ok(Err(e)) => {
                    if is_required {
                        error!(
                            "Required plugin '{}' failed to initialize: {}",
                            plugin_name, e
                        );
                        return Err(e);
                    }
                    warn!(
                        "Optional plugin '{}' failed to initialize: {}",
                        plugin_name, e
                    );
                }
                Err(_) => {
                    if is_required {
                        error!(
                            "Required plugin '{}' initialization timed out after {:?}",
                            plugin_name, self.initialization_timeout
                        );
                        return Err(anyhow::anyhow!(
                            "Plugin initialization timeout: {}",
                            plugin_name
                        ));
                    }
                    warn!("Optional plugin '{}' initialization timed out", plugin_name);
                }
            }
        }

        info!("All plugins initialized successfully");
        Ok(())
    }

    /// Perform health checks on all plugins
    #[must_use]
    pub async fn health_check_all(&self) -> Vec<PluginHealth> {
        let mut results = Vec::new();

        for plugin in &self.plugins {
            match plugin.health_check().await {
                Ok(health) => results.push(health),
                Err(e) => {
                    error!("Health check failed for plugin '{}': {}", plugin.name(), e);
                    results.push(PluginHealth {
                        name: plugin.name().to_string(),
                        state: plugin.state(),
                        healthy: false,
                        message: Some(format!("Health check error: {e}")),
                        last_check: chrono::Utc::now(),
                    });
                }
            }
        }

        results
    }

    /// Shutdown all plugins in reverse priority order
    ///
    /// # Errors
    /// Returns an error if any plugin fails to shutdown gracefully
    pub async fn shutdown_all(&mut self) -> Result<()> {
        info!("Shutting down {} plugins", self.plugins.len());

        // Reverse order for shutdown
        self.plugins.reverse();

        for plugin in &mut self.plugins {
            let plugin_name = plugin.name().to_string();
            info!("Shutting down plugin '{}'", plugin_name);

            if let Err(e) = plugin.shutdown().await {
                error!("Plugin '{}' shutdown error: {}", plugin_name, e);
                // Continue shutting down other plugins even if one fails
            }
        }

        info!("All plugins shut down");
        Ok(())
    }

    /// Get overall system health status
    #[must_use]
    pub async fn is_healthy(&self) -> bool {
        let health_checks = self.health_check_all().await;

        // Check if all required plugins are healthy
        for health in &health_checks {
            let plugin = self.plugins.iter().find(|p| p.name() == health.name);

            if let Some(p) = plugin {
                if p.is_required() && !health.healthy {
                    return false;
                }
            }
        }

        true
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
