// ABOUTME: OAuth 2.0 server repository implementation
// ABOUTME: Handles OAuth 2.0 client registration, auth codes, refresh tokens, and state
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::OAuth2ServerRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// SQLite/PostgreSQL implementation of `OAuth2ServerRepository`
pub struct OAuth2ServerRepositoryImpl {
    db: Database,
}

impl OAuth2ServerRepositoryImpl {
    /// Create a new `OAuth2ServerRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OAuth2ServerRepository for OAuth2ServerRepositoryImpl {
    async fn store_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_oauth2_client(client)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>, DatabaseError> {
        self.db
            .get_oauth2_client(client_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_auth_code(
        &self,
        code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_oauth2_auth_code(code)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>, DatabaseError> {
        self.db
            .get_oauth2_auth_code(code)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_auth_code(
        &self,
        code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_oauth2_auth_code(code)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>, DatabaseError> {
        self.db
            .consume_auth_code(code, client_id, redirect_uri, now)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_refresh_token(
        &self,
        token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_oauth2_refresh_token(token)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>, DatabaseError> {
        self.db
            .get_oauth2_refresh_token(token)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>, DatabaseError> {
        self.db
            .get_refresh_token_by_value(token)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>, DatabaseError> {
        self.db
            .consume_refresh_token(token, client_id, now)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn revoke_refresh_token(&self, token: &str) -> Result<(), DatabaseError> {
        self.db
            .revoke_oauth2_refresh_token(token)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_authorization_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_authorization_code(auth_code)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<crate::oauth2_server::models::OAuth2AuthCode, DatabaseError> {
        self.db
            .get_authorization_code(code, client_id, redirect_uri)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<(), DatabaseError> {
        self.db
            .delete_authorization_code(code, client_id, redirect_uri)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_oauth2_state(state)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn consume_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>, DatabaseError> {
        self.db
            .consume_oauth2_state(state_value, client_id, now)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
