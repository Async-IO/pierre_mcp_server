// ABOUTME: Database operations for user-created Coaches (custom AI personas)
// ABOUTME: Handles CRUD operations for coaches with tenant isolation and token counting
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

/// Token estimation constant: average characters per token for system prompts
const CHARS_PER_TOKEN: usize = 4;

/// Coach category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CoachCategory {
    /// Training and workout focused coaches
    Training,
    /// Nutrition and diet focused coaches
    Nutrition,
    /// Recovery and rest focused coaches
    Recovery,
    /// Recipe and meal planning focused coaches
    Recipes,
    /// User-defined custom category
    #[default]
    Custom,
}

impl CoachCategory {
    /// Convert to database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Nutrition => "nutrition",
            Self::Recovery => "recovery",
            Self::Recipes => "recipes",
            Self::Custom => "custom",
        }
    }

    /// Parse from database string representation
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "training" => Self::Training,
            "nutrition" => Self::Nutrition,
            "recovery" => Self::Recovery,
            "recipes" => Self::Recipes,
            _ => Self::Custom,
        }
    }
}

/// A Coach is a custom AI persona with a system prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coach {
    /// Unique identifier
    pub id: Uuid,
    /// User who created the coach
    pub user_id: Uuid,
    /// Tenant for multi-tenancy isolation
    pub tenant_id: String,
    /// Display title for the coach
    pub title: String,
    /// Optional description explaining the coach's purpose
    pub description: Option<String>,
    /// System prompt that shapes AI responses
    pub system_prompt: String,
    /// Category for organization
    pub category: CoachCategory,
    /// Tags for filtering and search (stored as JSON array)
    pub tags: Vec<String>,
    /// Estimated token count of system prompt
    pub token_count: u32,
    /// Whether this coach is marked as favorite
    pub is_favorite: bool,
    /// Whether this coach is currently active for the user
    pub is_active: bool,
    /// Number of times this coach has been used
    pub use_count: u32,
    /// Last time this coach was used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new coach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCoachRequest {
    /// Display title for the coach
    pub title: String,
    /// Optional description explaining the coach's purpose
    pub description: Option<String>,
    /// System prompt that shapes AI responses
    pub system_prompt: String,
    /// Category for organization
    #[serde(default)]
    pub category: CoachCategory,
    /// Tags for filtering and search
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request to update an existing coach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCoachRequest {
    /// New display title (if provided)
    pub title: Option<String>,
    /// New description (if provided)
    pub description: Option<String>,
    /// New system prompt (if provided)
    pub system_prompt: Option<String>,
    /// New category (if provided)
    pub category: Option<CoachCategory>,
    /// New tags (if provided)
    pub tags: Option<Vec<String>>,
}

/// Filter options for listing coaches
#[derive(Debug, Clone, Default)]
pub struct ListCoachesFilter {
    /// Filter by category
    pub category: Option<CoachCategory>,
    /// Filter to favorites only
    pub favorites_only: bool,
    /// Maximum number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Coach database operations manager
pub struct CoachesManager {
    pool: SqlitePool,
}

impl CoachesManager {
    /// Create a new coaches manager
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Estimate token count for a system prompt
    ///
    /// Uses conservative estimate of ~4 characters per token
    #[allow(clippy::cast_possible_truncation)]
    const fn estimate_tokens(text: &str) -> u32 {
        let char_count = text.len();
        let tokens = char_count / CHARS_PER_TOKEN;
        // Token count bounded by reasonable system prompt size (< 100K chars = < 25K tokens)
        tokens as u32
    }

    /// Create a new coach in the database
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn create(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        request: &CreateCoachRequest,
    ) -> AppResult<Coach> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let tags_json = serde_json::to_string(&request.tags)?;
        let token_count = Self::estimate_tokens(&request.system_prompt);

        sqlx::query(
            r"
            INSERT INTO coaches (
                id, user_id, tenant_id, title, description, system_prompt,
                category, tags, token_count, is_favorite, use_count,
                last_used_at, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(tenant_id)
        .bind(&request.title)
        .bind(&request.description)
        .bind(&request.system_prompt)
        .bind(request.category.as_str())
        .bind(&tags_json)
        .bind(i64::from(token_count))
        .bind(false) // is_favorite
        .bind(0i64) // use_count
        .bind(Option::<String>::None) // last_used_at
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create coach: {e}")))?;

        Ok(Coach {
            id,
            user_id,
            tenant_id: tenant_id.to_owned(),
            title: request.title.clone(),
            description: request.description.clone(),
            system_prompt: request.system_prompt.clone(),
            category: request.category,
            tags: request.tags.clone(),
            token_count,
            is_favorite: false,
            is_active: false,
            use_count: 0,
            last_used_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get a coach by ID for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at
            FROM coaches
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get coach: {e}")))?;

        row.map(|r| row_to_coach(&r)).transpose()
    }

    /// List coaches for a user with optional filtering
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        filter: &ListCoachesFilter,
    ) -> AppResult<Vec<Coach>> {
        let limit_val = i32::try_from(filter.limit.unwrap_or(50)).unwrap_or(50);
        let offset_val = i32::try_from(filter.offset.unwrap_or(0)).unwrap_or(0);

        // Build query based on filters
        let rows = match (filter.category, filter.favorites_only) {
            (Some(category), true) => {
                sqlx::query(
                    r"
                    SELECT id, user_id, tenant_id, title, description, system_prompt,
                           category, tags, token_count, is_favorite, is_active, use_count,
                           last_used_at, created_at, updated_at
                    FROM coaches
                    WHERE user_id = $1 AND tenant_id = $2 AND category = $3 AND is_favorite = 1
                    ORDER BY updated_at DESC
                    LIMIT $4 OFFSET $5
                    ",
                )
                .bind(user_id.to_string())
                .bind(tenant_id)
                .bind(category.as_str())
                .bind(limit_val)
                .bind(offset_val)
                .fetch_all(&self.pool)
                .await
            }
            (Some(category), false) => {
                sqlx::query(
                    r"
                    SELECT id, user_id, tenant_id, title, description, system_prompt,
                           category, tags, token_count, is_favorite, is_active, use_count,
                           last_used_at, created_at, updated_at
                    FROM coaches
                    WHERE user_id = $1 AND tenant_id = $2 AND category = $3
                    ORDER BY updated_at DESC
                    LIMIT $4 OFFSET $5
                    ",
                )
                .bind(user_id.to_string())
                .bind(tenant_id)
                .bind(category.as_str())
                .bind(limit_val)
                .bind(offset_val)
                .fetch_all(&self.pool)
                .await
            }
            (None, true) => {
                sqlx::query(
                    r"
                    SELECT id, user_id, tenant_id, title, description, system_prompt,
                           category, tags, token_count, is_favorite, is_active, use_count,
                           last_used_at, created_at, updated_at
                    FROM coaches
                    WHERE user_id = $1 AND tenant_id = $2 AND is_favorite = 1
                    ORDER BY updated_at DESC
                    LIMIT $3 OFFSET $4
                    ",
                )
                .bind(user_id.to_string())
                .bind(tenant_id)
                .bind(limit_val)
                .bind(offset_val)
                .fetch_all(&self.pool)
                .await
            }
            (None, false) => {
                sqlx::query(
                    r"
                    SELECT id, user_id, tenant_id, title, description, system_prompt,
                           category, tags, token_count, is_favorite, is_active, use_count,
                           last_used_at, created_at, updated_at
                    FROM coaches
                    WHERE user_id = $1 AND tenant_id = $2
                    ORDER BY updated_at DESC
                    LIMIT $3 OFFSET $4
                    ",
                )
                .bind(user_id.to_string())
                .bind(tenant_id)
                .bind(limit_val)
                .bind(offset_val)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| AppError::database(format!("Failed to list coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Update an existing coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or coach not found
    pub async fn update(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: &str,
        request: &UpdateCoachRequest,
    ) -> AppResult<Option<Coach>> {
        // First get the existing coach
        let existing = self.get(coach_id, user_id, tenant_id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let now = Utc::now();
        let title = request.title.as_ref().unwrap_or(&existing.title);
        let description = request.description.clone().or(existing.description);
        let system_prompt = request
            .system_prompt
            .as_ref()
            .unwrap_or(&existing.system_prompt);
        let category = request.category.unwrap_or(existing.category);
        let tags = request.tags.as_ref().unwrap_or(&existing.tags);
        let tags_json = serde_json::to_string(tags)?;
        let token_count = Self::estimate_tokens(system_prompt);

        let result = sqlx::query(
            r"
            UPDATE coaches SET
                title = $1, description = $2, system_prompt = $3,
                category = $4, tags = $5, token_count = $6, updated_at = $7
            WHERE id = $8 AND user_id = $9 AND tenant_id = $10
            ",
        )
        .bind(title)
        .bind(&description)
        .bind(system_prompt)
        .bind(category.as_str())
        .bind(&tags_json)
        .bind(i64::from(token_count))
        .bind(now.to_rfc3339())
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update coach: {e}")))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        // Return updated coach
        self.get(coach_id, user_id, tenant_id).await
    }

    /// Delete a coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn delete(&self, coach_id: &str, user_id: Uuid, tenant_id: &str) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM coaches
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete coach: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Record coach usage (increment `use_count` and update `last_used_at`)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn record_usage(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<bool> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            r"
            UPDATE coaches SET
                use_count = use_count + 1,
                last_used_at = $1,
                updated_at = $1
            WHERE id = $2 AND user_id = $3 AND tenant_id = $4
            ",
        )
        .bind(&now)
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to record coach usage: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Toggle favorite status for a coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn toggle_favorite(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<Option<bool>> {
        // Get current favorite status
        let row = sqlx::query(
            r"
            SELECT is_favorite FROM coaches
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get coach: {e}")))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let current: i64 = row.get("is_favorite");
        let new_value = i64::from(current != 1);
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r"
            UPDATE coaches SET is_favorite = $1, updated_at = $2
            WHERE id = $3 AND user_id = $4 AND tenant_id = $5
            ",
        )
        .bind(new_value)
        .bind(&now)
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to toggle favorite: {e}")))?;

        Ok(Some(new_value == 1))
    }

    /// Count coaches for a user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn count(&self, user_id: Uuid, tenant_id: &str) -> AppResult<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count FROM coaches
            WHERE user_id = $1 AND tenant_id = $2
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to count coaches: {e}")))?;

        let count: i64 = row.get("count");
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(count as u32)
    }

    /// Search coaches by title, description, or tags
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn search(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        query: &str,
        limit: Option<u32>,
    ) -> AppResult<Vec<Coach>> {
        let limit_val = i32::try_from(limit.unwrap_or(20)).unwrap_or(20);
        let search_pattern = format!("%{query}%");

        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at
            FROM coaches
            WHERE user_id = $1 AND tenant_id = $2 AND (
                title LIKE $3 OR description LIKE $3 OR tags LIKE $3
            )
            ORDER BY updated_at DESC
            LIMIT $4
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id)
        .bind(&search_pattern)
        .bind(limit_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to search coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Activate a coach (deactivates all other coaches for the user first)
    ///
    /// Only one coach can be active per user at a time. This method
    /// deactivates any currently active coach before activating the new one.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn activate_coach(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<Option<Coach>> {
        let now = Utc::now().to_rfc3339();

        // First deactivate all coaches for this user
        sqlx::query(
            r"
            UPDATE coaches SET is_active = 0, updated_at = $1
            WHERE user_id = $2 AND tenant_id = $3 AND is_active = 1
            ",
        )
        .bind(&now)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to deactivate coaches: {e}")))?;

        // Now activate the specified coach
        let result = sqlx::query(
            r"
            UPDATE coaches SET is_active = 1, updated_at = $1
            WHERE id = $2 AND user_id = $3 AND tenant_id = $4
            ",
        )
        .bind(&now)
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to activate coach: {e}")))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        // Return the activated coach
        self.get(coach_id, user_id, tenant_id).await
    }

    /// Deactivate the currently active coach for a user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn deactivate_coach(&self, user_id: Uuid, tenant_id: &str) -> AppResult<bool> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r"
            UPDATE coaches SET is_active = 0, updated_at = $1
            WHERE user_id = $2 AND tenant_id = $3 AND is_active = 1
            ",
        )
        .bind(&now)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to deactivate coach: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get the currently active coach for a user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_active_coach(
        &self,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at
            FROM coaches
            WHERE user_id = $1 AND tenant_id = $2 AND is_active = 1
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get active coach: {e}")))?;

        row.map(|r| row_to_coach(&r)).transpose()
    }
}

/// Convert a database row to a Coach struct
fn row_to_coach(row: &SqliteRow) -> AppResult<Coach> {
    let id_str: String = row.get("id");
    let user_id_str: String = row.get("user_id");
    let category_str: String = row.get("category");
    let tags_json: String = row.get("tags");
    let created_at_str: String = row.get("created_at");
    let updated_at_str: String = row.get("updated_at");
    let last_used_at_str: Option<String> = row.get("last_used_at");
    let token_count: i64 = row.get("token_count");
    let is_favorite: i64 = row.get("is_favorite");
    let is_active: i64 = row.get("is_active");
    let use_count: i64 = row.get("use_count");

    let tags: Vec<String> = serde_json::from_str(&tags_json)?;

    Ok(Coach {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| AppError::internal(format!("Invalid UUID: {e}")))?,
        user_id: Uuid::parse_str(&user_id_str)
            .map_err(|e| AppError::internal(format!("Invalid UUID: {e}")))?,
        tenant_id: row.get("tenant_id"),
        title: row.get("title"),
        description: row.get("description"),
        system_prompt: row.get("system_prompt"),
        category: CoachCategory::parse(&category_str),
        tags,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        token_count: token_count as u32,
        is_favorite: is_favorite == 1,
        is_active: is_active == 1,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        use_count: use_count as u32,
        last_used_at: last_used_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| AppError::internal(format!("Invalid datetime: {e}")))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| AppError::internal(format!("Invalid datetime: {e}")))?
            .with_timezone(&Utc),
    })
}
