// ABOUTME: Data context for dependency injection of database and provider services
// ABOUTME: Contains database, provider registry, and intelligence services for data operations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::database_plugins::factory::Database;
// TODO(fitness-decoupling): Re-enable when plugin system ready
// TODO(fitness-decoupling): // use crate::intelligence::ActivityIntelligence;
use crate::providers::ProviderRegistry;
use std::sync::Arc;

/// Data context containing database and provider dependencies
///
/// This context provides all data-related dependencies needed for
/// database operations, provider integration, and activity intelligence.
///
/// # Dependencies
/// - `database`: Primary database interface for all persistence operations
/// - `provider_registry`: Registry of external service providers
/// - `activity_intelligence`: AI/ML services for activity analysis (moved to fitness-app)
#[derive(Clone)]
pub struct DataContext {
    database: Arc<Database>,
    provider_registry: Arc<ProviderRegistry>,
    // TODO(fitness-decoupling): ActivityIntelligence moved to pierre-fitness-app
    // activity_intelligence: Arc<ActivityIntelligence>,
}

impl DataContext {
    /// Create new data context
    #[must_use]
    pub const fn new(
        database: Arc<Database>,
        provider_registry: Arc<ProviderRegistry>,
        // TODO(fitness-decoupling): ActivityIntelligence parameter removed
        // activity_intelligence: Arc<ActivityIntelligence>,
    ) -> Self {
        Self {
            database,
            provider_registry,
            // TODO(fitness-decoupling): ActivityIntelligence field removed
            // activity_intelligence,
        }
    }

    /// Get database for persistence operations
    #[must_use]
    pub const fn database(&self) -> &Arc<Database> {
        &self.database
    }

    /// Get provider registry for external integrations
    #[must_use]
    pub const fn provider_registry(&self) -> &Arc<ProviderRegistry> {
        &self.provider_registry
    }

    // TODO(fitness-decoupling): ActivityIntelligence moved to pierre-fitness-app
    // Get activity intelligence for AI/ML operations
    // #[must_use]
    // pub const fn activity_intelligence(&self) -> &Arc<ActivityIntelligence> {
    //     &self.activity_intelligence
    // }
}
