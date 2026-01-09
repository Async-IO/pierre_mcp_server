// ABOUTME: Data context for dependency injection of database, cache, and provider services
// ABOUTME: Contains database, cache, provider registry, and intelligence services for data operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::cache::factory::Cache;
use crate::database_plugins::factory::Database;
use crate::intelligence::ActivityIntelligence;
use crate::providers::ProviderRegistry;
use std::sync::Arc;

/// Data context containing database, cache, and provider dependencies
///
/// This context provides all data-related dependencies needed for
/// database operations, caching, provider integration, and activity intelligence.
///
/// # Dependencies
/// - `database`: Primary database interface for all persistence operations
/// - `cache`: Cache layer for performance optimization
/// - `provider_registry`: Registry of external service providers
/// - `activity_intelligence`: AI/ML services for activity analysis
#[derive(Clone)]
pub struct DataContext {
    database: Arc<Database>,
    cache: Arc<Cache>,
    provider_registry: Arc<ProviderRegistry>,
    activity_intelligence: Arc<ActivityIntelligence>,
}

impl DataContext {
    /// Create new data context
    #[must_use]
    pub const fn new(
        database: Arc<Database>,
        cache: Arc<Cache>,
        provider_registry: Arc<ProviderRegistry>,
        activity_intelligence: Arc<ActivityIntelligence>,
    ) -> Self {
        Self {
            database,
            cache,
            provider_registry,
            activity_intelligence,
        }
    }

    /// Get database for persistence operations
    #[must_use]
    pub const fn database(&self) -> &Arc<Database> {
        &self.database
    }

    /// Get cache for performance optimization
    #[must_use]
    pub const fn cache(&self) -> &Arc<Cache> {
        &self.cache
    }

    /// Get provider registry for external integrations
    #[must_use]
    pub const fn provider_registry(&self) -> &Arc<ProviderRegistry> {
        &self.provider_registry
    }

    /// Get activity intelligence for AI/ML operations
    #[must_use]
    pub const fn activity_intelligence(&self) -> &Arc<ActivityIntelligence> {
        &self.activity_intelligence
    }
}
