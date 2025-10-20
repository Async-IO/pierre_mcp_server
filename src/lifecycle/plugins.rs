// ABOUTME: Core system plugin adapters for lifecycle management
// ABOUTME: Wraps database, cache, and auth systems with Plugin trait for deterministic initialization
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

//! Core system plugin adapters
//!
//! This module provides `Plugin` trait implementations for core server systems:
//! - Database plugin
//! - Cache plugin
//! - Authentication plugin
//!
//! These adapters enable deterministic initialization order and health monitoring.

use super::{Plugin, PluginHealth, PluginState};
use crate::{
    auth::AuthManager,
    cache::factory::Cache,
    database_plugins::{factory::Database, DatabaseProvider},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

/// Database plugin adapter
pub struct DatabasePlugin {
    name: String,
    state: Arc<RwLock<PluginState>>,
    database: Option<Database>,
    connection_string: String,
    encryption_key: Vec<u8>,
}

impl DatabasePlugin {
    /// Create new database plugin
    #[must_use]
    pub fn new(connection_string: String, encryption_key: Vec<u8>) -> Self {
        Self {
            name: "database".to_string(),
            state: Arc::new(RwLock::new(PluginState::Uninitialized)),
            database: None,
            connection_string,
            encryption_key,
        }
    }

    /// Get initialized database instance
    ///
    /// # Errors
    /// Returns error if database is not initialized
    pub fn get_database(&self) -> Result<&Database> {
        self.database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))
    }

    /// Take ownership of database instance
    ///
    /// # Errors
    /// Returns error if database is not initialized
    pub fn take_database(mut self) -> Result<Database> {
        self.database
            .take()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))
    }
}

#[async_trait]
impl Plugin for DatabasePlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u8 {
        10 // High priority - database needed first
    }

    async fn initialize(&mut self) -> Result<()> {
        *self.state.write().expect("Lock poisoned") = PluginState::Initializing;

        self.database =
            Some(Database::new(&self.connection_string, self.encryption_key.clone()).await?);
        *self.state.write().expect("Lock poisoned") = PluginState::Ready;

        Ok(())
    }

    async fn health_check(&self) -> Result<PluginHealth> {
        let state = *self.state.read().expect("Lock poisoned");

        let healthy = if let Some(db) = &self.database {
            // Perform basic health check by querying user count
            db.get_user_count().await.is_ok()
        } else {
            false
        };

        Ok(PluginHealth {
            name: self.name.clone(),
            state,
            healthy,
            message: if healthy {
                Some(format!(
                    "Database operational: {}",
                    self.database
                        .as_ref()
                        .map_or("unknown", Database::backend_info)
                ))
            } else {
                Some("Database not responding".to_string())
            },
            last_check: chrono::Utc::now(),
        })
    }

    async fn shutdown(&mut self) -> Result<()> {
        *self.state.write().expect("Lock poisoned") = PluginState::ShuttingDown;

        // Database cleanup - just drop the connection
        self.database = None;

        *self.state.write().expect("Lock poisoned") = PluginState::Shutdown;
        Ok(())
    }

    fn state(&self) -> PluginState {
        *self.state.read().expect("Lock poisoned")
    }

    fn is_required(&self) -> bool {
        true // Database is required for server operation
    }
}

/// Cache plugin adapter
pub struct CachePlugin {
    name: String,
    state: Arc<RwLock<PluginState>>,
    cache: Option<Cache>,
}

impl CachePlugin {
    /// Create new cache plugin
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "cache".to_string(),
            state: Arc::new(RwLock::new(PluginState::Uninitialized)),
            cache: None,
        }
    }

    /// Get initialized cache instance
    ///
    /// # Errors
    /// Returns error if cache is not initialized
    pub fn get_cache(&self) -> Result<&Cache> {
        self.cache
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cache not initialized"))
    }

    /// Take ownership of cache instance
    ///
    /// # Errors
    /// Returns error if cache is not initialized
    pub fn take_cache(mut self) -> Result<Cache> {
        self.cache
            .take()
            .ok_or_else(|| anyhow::anyhow!("Cache not initialized"))
    }
}

impl Default for CachePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for CachePlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u8 {
        20 // Medium-high priority - cache needed early
    }

    async fn initialize(&mut self) -> Result<()> {
        *self.state.write().expect("Lock poisoned") = PluginState::Initializing;

        self.cache = Some(Cache::from_env().await?);
        *self.state.write().expect("Lock poisoned") = PluginState::Ready;

        Ok(())
    }

    async fn health_check(&self) -> Result<PluginHealth> {
        let state = *self.state.read().expect("Lock poisoned");

        let healthy = self.cache.is_some();

        Ok(PluginHealth {
            name: self.name.clone(),
            state,
            healthy,
            message: if healthy {
                Some("Cache operational".to_string())
            } else {
                Some("Cache not initialized".to_string())
            },
            last_check: chrono::Utc::now(),
        })
    }

    async fn shutdown(&mut self) -> Result<()> {
        *self.state.write().expect("Lock poisoned") = PluginState::ShuttingDown;

        self.cache = None;

        *self.state.write().expect("Lock poisoned") = PluginState::Shutdown;
        Ok(())
    }

    fn state(&self) -> PluginState {
        *self.state.read().expect("Lock poisoned")
    }

    fn is_required(&self) -> bool {
        true // Cache is required for server operation
    }
}

/// Authentication manager plugin adapter
pub struct AuthPlugin {
    name: String,
    state: Arc<RwLock<PluginState>>,
    auth_manager: Option<AuthManager>,
    jwt_expiry_hours: i64,
}

impl AuthPlugin {
    /// Create new auth plugin
    #[must_use]
    pub fn new(jwt_expiry_hours: i64) -> Self {
        Self {
            name: "auth".to_string(),
            state: Arc::new(RwLock::new(PluginState::Uninitialized)),
            auth_manager: None,
            jwt_expiry_hours,
        }
    }

    /// Get initialized auth manager
    ///
    /// # Errors
    /// Returns error if auth manager is not initialized
    pub fn get_auth_manager(&self) -> Result<&AuthManager> {
        self.auth_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Auth manager not initialized"))
    }

    /// Take ownership of auth manager instance
    ///
    /// # Errors
    /// Returns error if auth manager is not initialized
    pub fn take_auth_manager(mut self) -> Result<AuthManager> {
        self.auth_manager
            .take()
            .ok_or_else(|| anyhow::anyhow!("Auth manager not initialized"))
    }
}

#[async_trait]
impl Plugin for AuthPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u8 {
        30 // Medium priority - auth depends on database
    }

    async fn initialize(&mut self) -> Result<()> {
        *self.state.write().expect("Lock poisoned") = PluginState::Initializing;

        self.auth_manager = Some(AuthManager::new(self.jwt_expiry_hours));
        *self.state.write().expect("Lock poisoned") = PluginState::Ready;

        Ok(())
    }

    async fn health_check(&self) -> Result<PluginHealth> {
        let state = *self.state.read().expect("Lock poisoned");

        let healthy = self.auth_manager.is_some();

        Ok(PluginHealth {
            name: self.name.clone(),
            state,
            healthy,
            message: if healthy {
                Some("Auth manager operational".to_string())
            } else {
                Some("Auth manager not initialized".to_string())
            },
            last_check: chrono::Utc::now(),
        })
    }

    async fn shutdown(&mut self) -> Result<()> {
        *self.state.write().expect("Lock poisoned") = PluginState::ShuttingDown;

        self.auth_manager = None;

        *self.state.write().expect("Lock poisoned") = PluginState::Shutdown;
        Ok(())
    }

    fn state(&self) -> PluginState {
        *self.state.read().expect("Lock poisoned")
    }

    fn is_required(&self) -> bool {
        true // Auth is required for server operation
    }
}
