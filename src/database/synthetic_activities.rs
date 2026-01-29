// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Database operations for synthetic provider activities
// ABOUTME: Provides methods to query synthetic activities table for provider status

use crate::database::Database;
use crate::errors::AppResult;
use uuid::Uuid;

impl Database {
    /// Check if a user has any synthetic activities seeded
    ///
    /// This is used to determine if the synthetic provider should be shown
    /// as "connected" for a user in the providers list.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn user_has_synthetic_activities_impl(&self, user_id: Uuid) -> AppResult<bool> {
        let user_id_str = user_id.to_string();

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM synthetic_activities WHERE user_id = ? LIMIT 1",
        )
        .bind(&user_id_str)
        .fetch_one(self.pool())
        .await?;

        Ok(count > 0)
    }
}
