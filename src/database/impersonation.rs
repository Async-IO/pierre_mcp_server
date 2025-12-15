// ABOUTME: Impersonation session database operations for super admin user impersonation
// ABOUTME: Handles creation, retrieval, and termination of impersonation audit records
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::errors::{AppError, AppResult};
use crate::permissions::impersonation::ImpersonationSession;
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use uuid::Uuid;

impl Database {
    /// Create a new impersonation session
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Session already exists with same ID
    pub async fn create_impersonation_session(
        &self,
        session: &ImpersonationSession,
    ) -> AppResult<()> {
        let query = r"
            INSERT INTO impersonation_sessions (
                id, impersonator_id, target_user_id, reason,
                started_at, ended_at, is_active, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ";

        sqlx::query(query)
            .bind(&session.id)
            .bind(session.impersonator_id.to_string())
            .bind(session.target_user_id.to_string())
            .bind(&session.reason)
            .bind(session.started_at.to_rfc3339())
            .bind(session.ended_at.map(|dt| dt.to_rfc3339()))
            .bind(session.is_active)
            .bind(session.created_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to create impersonation session: {e}"))
            })?;

        Ok(())
    }

    /// Get impersonation session by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_impersonation_session(
        &self,
        session_id: &str,
    ) -> AppResult<Option<ImpersonationSession>> {
        let query = r"
            SELECT id, impersonator_id, target_user_id, reason,
                   started_at, ended_at, is_active, created_at
            FROM impersonation_sessions WHERE id = ?
        ";

        let row = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get impersonation session: {e}")))?;

        row.map(|r| Self::row_to_impersonation_session(&r))
            .transpose()
    }

    /// Get active impersonation session for impersonator
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_active_impersonation_session(
        &self,
        impersonator_id: Uuid,
    ) -> AppResult<Option<ImpersonationSession>> {
        let query = r"
            SELECT id, impersonator_id, target_user_id, reason,
                   started_at, ended_at, is_active, created_at
            FROM impersonation_sessions
            WHERE impersonator_id = ? AND is_active = 1
            ORDER BY started_at DESC LIMIT 1
        ";

        let row = sqlx::query(query)
            .bind(impersonator_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to get active impersonation session: {e}"))
            })?;

        row.map(|r| Self::row_to_impersonation_session(&r))
            .transpose()
    }

    /// End an impersonation session
    ///
    /// # Errors
    ///
    /// Returns an error if database update fails
    pub async fn end_impersonation_session(&self, session_id: &str) -> AppResult<()> {
        let query = r"
            UPDATE impersonation_sessions
            SET is_active = 0, ended_at = ?
            WHERE id = ?
        ";

        let ended_at = Utc::now().to_rfc3339();
        sqlx::query(query)
            .bind(&ended_at)
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to end impersonation session: {e}")))?;

        Ok(())
    }

    /// End all active impersonation sessions for an impersonator
    ///
    /// # Errors
    ///
    /// Returns an error if database update fails
    pub async fn end_all_impersonation_sessions(&self, impersonator_id: Uuid) -> AppResult<u64> {
        let query = r"
            UPDATE impersonation_sessions
            SET is_active = 0, ended_at = ?
            WHERE impersonator_id = ? AND is_active = 1
        ";

        let ended_at = Utc::now().to_rfc3339();
        let result = sqlx::query(query)
            .bind(&ended_at)
            .bind(impersonator_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to end impersonation sessions: {e}"))
            })?;

        Ok(result.rows_affected())
    }

    /// List impersonation sessions with optional filters
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn list_impersonation_sessions(
        &self,
        impersonator_id: Option<Uuid>,
        target_user_id: Option<Uuid>,
        active_only: bool,
        limit: u32,
    ) -> AppResult<Vec<ImpersonationSession>> {
        let mut query = String::from(
            r"
            SELECT id, impersonator_id, target_user_id, reason,
                   started_at, ended_at, is_active, created_at
            FROM impersonation_sessions WHERE 1=1
            ",
        );

        if impersonator_id.is_some() {
            query.push_str(" AND impersonator_id = ?");
        }
        if target_user_id.is_some() {
            query.push_str(" AND target_user_id = ?");
        }
        if active_only {
            query.push_str(" AND is_active = 1");
        }
        query.push_str(" ORDER BY started_at DESC LIMIT ?");

        let mut sql_query = sqlx::query(&query);

        if let Some(id) = impersonator_id {
            sql_query = sql_query.bind(id.to_string());
        }
        if let Some(id) = target_user_id {
            sql_query = sql_query.bind(id.to_string());
        }
        sql_query = sql_query.bind(limit);

        let rows = sql_query.fetch_all(&self.pool).await.map_err(|e| {
            AppError::database(format!("Failed to list impersonation sessions: {e}"))
        })?;

        rows.iter()
            .map(Self::row_to_impersonation_session)
            .collect()
    }

    /// Convert database row to `ImpersonationSession`
    fn row_to_impersonation_session(row: &SqliteRow) -> AppResult<ImpersonationSession> {
        let id: String = row.get("id");
        let impersonator_id: String = row.get("impersonator_id");
        let target_user_id: String = row.get("target_user_id");
        let reason: Option<String> = row.get("reason");
        let started_at: String = row.get("started_at");
        let ended_at: Option<String> = row.get("ended_at");
        let is_active: bool = row.get("is_active");
        let created_at: String = row.get("created_at");

        Ok(ImpersonationSession {
            id,
            impersonator_id: Uuid::parse_str(&impersonator_id)
                .map_err(|e| AppError::database(format!("Invalid impersonator_id UUID: {e}")))?,
            target_user_id: Uuid::parse_str(&target_user_id)
                .map_err(|e| AppError::database(format!("Invalid target_user_id UUID: {e}")))?,
            reason,
            started_at: DateTime::parse_from_rfc3339(&started_at)
                .map_err(|e| AppError::database(format!("Invalid started_at timestamp: {e}")))?
                .with_timezone(&Utc),
            ended_at: ended_at
                .map(|s| DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| AppError::database(format!("Invalid ended_at timestamp: {e}")))?
                .map(|dt| dt.with_timezone(&Utc)),
            is_active,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| AppError::database(format!("Invalid created_at timestamp: {e}")))?
                .with_timezone(&Utc),
        })
    }
}
