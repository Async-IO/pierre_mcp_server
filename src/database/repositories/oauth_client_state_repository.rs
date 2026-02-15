// ABOUTME: OAuth client-side state repository for CSRF protection during external OAuth flows
// ABOUTME: Delegates to DatabaseProvider for state storage and atomic consumption
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::OAuthClientStateRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::oauth2_client::OAuthClientState;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// SQLite/PostgreSQL implementation of `OAuthClientStateRepository`
pub struct OAuthClientStateRepositoryImpl {
    db: Database,
}

impl OAuthClientStateRepositoryImpl {
    /// Create a new `OAuthClientStateRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OAuthClientStateRepository for OAuthClientStateRepositoryImpl {
    async fn store(&self, state: &OAuthClientState) -> Result<(), DatabaseError> {
        self.db
            .store_oauth_client_state(state)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn consume(
        &self,
        state_value: &str,
        provider: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<OAuthClientState>, DatabaseError> {
        self.db
            .consume_oauth_client_state(state_value, provider, now)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
