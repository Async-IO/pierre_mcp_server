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

/// Coach visibility for access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CoachVisibility {
    /// Only visible to the owner
    #[default]
    Private,
    /// Visible to all users in the tenant
    Tenant,
    /// Visible across all tenants (super-admin only)
    Global,
}

impl CoachVisibility {
    /// Convert to database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Tenant => "tenant",
            Self::Global => "global",
        }
    }

    /// Parse from database string representation
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "tenant" => Self::Tenant,
            "global" => Self::Global,
            _ => Self::Private,
        }
    }
}

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
    /// User who created the coach (admin user for system coaches)
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
    /// Sample prompts for quick-start suggestions (stored as JSON array)
    #[serde(default)]
    pub sample_prompts: Vec<String>,
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
    /// Whether this is a system coach (admin-created)
    #[serde(default)]
    pub is_system: bool,
    /// Visibility level for the coach
    #[serde(default)]
    pub visibility: CoachVisibility,
}

/// Coach with computed context-dependent fields for list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachListItem {
    /// The coach data
    #[serde(flatten)]
    pub coach: Coach,
    /// Whether this coach is assigned to the current user (computed from query)
    pub is_assigned: bool,
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
    /// Sample prompts for quick-start suggestions
    #[serde(default)]
    pub sample_prompts: Vec<String>,
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
    /// New sample prompts (if provided)
    pub sample_prompts: Option<Vec<String>>,
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
    /// Include system coaches (default: true)
    pub include_system: bool,
    /// Include hidden coaches (default: false)
    pub include_hidden: bool,
}

impl ListCoachesFilter {
    /// Create a filter with sensible defaults (include system coaches, exclude hidden)
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            include_system: true,
            include_hidden: false,
            ..Default::default()
        }
    }
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
        let sample_prompts_json = serde_json::to_string(&request.sample_prompts)?;
        let token_count = Self::estimate_tokens(&request.system_prompt);

        sqlx::query(
            r"
            INSERT INTO coaches (
                id, user_id, tenant_id, title, description, system_prompt,
                category, tags, sample_prompts, token_count, is_favorite, use_count,
                last_used_at, created_at, updated_at, is_system, visibility
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14, $15, $16)
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
        .bind(&sample_prompts_json)
        .bind(i64::from(token_count))
        .bind(false) // is_favorite
        .bind(0i64) // use_count
        .bind(Option::<String>::None) // last_used_at
        .bind(now.to_rfc3339())
        .bind(0i64) // is_system (user-created coaches are not system)
        .bind(CoachVisibility::Private.as_str()) // visibility
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
            sample_prompts: request.sample_prompts.clone(),
            token_count,
            is_favorite: false,
            is_active: false,
            use_count: 0,
            last_used_at: None,
            created_at: now,
            updated_at: now,
            is_system: false,
            visibility: CoachVisibility::Private,
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
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility
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
    /// Returns coaches from three sources:
    /// 1. Personal coaches: created by the user (`is_system = 0`)
    /// 2. System coaches: visible to tenant (`is_system = 1 AND visibility = 'tenant'`)
    /// 3. Assigned coaches: explicitly assigned to the user via `coach_assignments`
    ///
    /// Hidden coaches are excluded unless `include_hidden` is true.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        filter: &ListCoachesFilter,
    ) -> AppResult<Vec<CoachListItem>> {
        let limit_val = i32::try_from(filter.limit.unwrap_or(50)).unwrap_or(50);
        let offset_val = i32::try_from(filter.offset.unwrap_or(0)).unwrap_or(0);
        let user_id_str = user_id.to_string();

        // Build dynamic query parts based on filters
        let category_filter = filter
            .category
            .as_ref()
            .map(|c| format!("AND c.category = '{}'", c.as_str()))
            .unwrap_or_default();
        let favorites_filter = if filter.favorites_only {
            "AND c.is_favorite = 1"
        } else {
            ""
        };
        let hidden_filter = if filter.include_hidden {
            ""
        } else {
            "AND c.id NOT IN (SELECT coach_id FROM user_coach_preferences WHERE user_id = $1 AND is_hidden = 1)"
        };

        // Build system coaches condition
        let system_condition = if filter.include_system {
            "OR (c.is_system = 1 AND c.visibility = 'tenant' AND c.tenant_id = $2)"
        } else {
            ""
        };

        // Build the unified query
        // Uses a subquery to identify assigned coaches for the is_assigned flag
        let query = format!(
            r"
            SELECT c.id, c.user_id, c.tenant_id, c.title, c.description, c.system_prompt,
                   c.category, c.tags, c.sample_prompts, c.token_count, c.is_favorite, c.is_active, c.use_count,
                   c.last_used_at, c.created_at, c.updated_at, c.is_system, c.visibility,
                   CASE WHEN ca.coach_id IS NOT NULL THEN 1 ELSE 0 END as is_assigned
            FROM coaches c
            LEFT JOIN coach_assignments ca ON c.id = ca.coach_id AND ca.user_id = $1
            WHERE (
                -- Personal coaches: owned by user
                (c.user_id = $1 AND c.is_system = 0 AND c.tenant_id = $2)
                -- System coaches visible to tenant
                {system_condition}
                -- Assigned coaches: explicitly assigned to user
                OR c.id IN (SELECT coach_id FROM coach_assignments WHERE user_id = $1)
            )
            {category_filter}
            {favorites_filter}
            {hidden_filter}
            ORDER BY c.updated_at DESC
            LIMIT $3 OFFSET $4
            "
        );

        let rows = sqlx::query(&query)
            .bind(&user_id_str)
            .bind(tenant_id)
            .bind(limit_val)
            .bind(offset_val)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to list coaches: {e}")))?;

        rows.iter().map(row_to_coach_list_item).collect()
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
        let sample_prompts = request
            .sample_prompts
            .as_ref()
            .unwrap_or(&existing.sample_prompts);
        let tags_json = serde_json::to_string(tags)?;
        let sample_prompts_json = serde_json::to_string(sample_prompts)?;
        let token_count = Self::estimate_tokens(system_prompt);

        let result = sqlx::query(
            r"
            UPDATE coaches SET
                title = $1, description = $2, system_prompt = $3,
                category = $4, tags = $5, sample_prompts = $6, token_count = $7, updated_at = $8
            WHERE id = $9 AND user_id = $10 AND tenant_id = $11
            ",
        )
        .bind(title)
        .bind(&description)
        .bind(system_prompt)
        .bind(category.as_str())
        .bind(&tags_json)
        .bind(&sample_prompts_json)
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
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility
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
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility
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

    // ============================================
    // System Coach Methods (Admin Operations)
    // ============================================

    /// Create a system coach (admin-created, visible to tenant users)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn create_system_coach(
        &self,
        admin_user_id: Uuid,
        tenant_id: &str,
        request: &CreateSystemCoachRequest,
    ) -> AppResult<Coach> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let tags_json = serde_json::to_string(&request.tags)?;
        let sample_prompts_json = serde_json::to_string(&request.sample_prompts)?;
        let token_count = Self::estimate_tokens(&request.system_prompt);

        sqlx::query(
            r"
            INSERT INTO coaches (
                id, user_id, tenant_id, title, description, system_prompt,
                category, tags, sample_prompts, token_count, is_favorite, use_count,
                last_used_at, created_at, updated_at, is_system, visibility
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14, $15, $16)
            ",
        )
        .bind(id.to_string())
        .bind(admin_user_id.to_string())
        .bind(tenant_id)
        .bind(&request.title)
        .bind(&request.description)
        .bind(&request.system_prompt)
        .bind(request.category.as_str())
        .bind(&tags_json)
        .bind(&sample_prompts_json)
        .bind(i64::from(token_count))
        .bind(false) // is_favorite
        .bind(0i64) // use_count
        .bind(Option::<String>::None) // last_used_at
        .bind(now.to_rfc3339())
        .bind(1i64) // is_system = true
        .bind(request.visibility.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create system coach: {e}")))?;

        Ok(Coach {
            id,
            user_id: admin_user_id,
            tenant_id: tenant_id.to_owned(),
            title: request.title.clone(),
            description: request.description.clone(),
            system_prompt: request.system_prompt.clone(),
            category: request.category,
            tags: request.tags.clone(),
            sample_prompts: request.sample_prompts.clone(),
            token_count,
            is_favorite: false,
            is_active: false,
            use_count: 0,
            last_used_at: None,
            created_at: now,
            updated_at: now,
            is_system: true,
            visibility: request.visibility,
        })
    }

    /// List all system coaches in a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_system_coaches(&self, tenant_id: &str) -> AppResult<Vec<Coach>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility
            FROM coaches
            WHERE tenant_id = $1 AND is_system = 1
            ORDER BY created_at DESC
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list system coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Get a system coach by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_system_coach(
        &self,
        coach_id: &str,
        tenant_id: &str,
    ) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility
            FROM coaches
            WHERE id = $1 AND tenant_id = $2 AND is_system = 1
            ",
        )
        .bind(coach_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get system coach: {e}")))?;

        row.map(|r| row_to_coach(&r)).transpose()
    }

    /// Update a system coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_system_coach(
        &self,
        coach_id: &str,
        tenant_id: &str,
        request: &UpdateCoachRequest,
    ) -> AppResult<Option<Coach>> {
        // First get the existing coach
        let existing = self.get_system_coach(coach_id, tenant_id).await?;
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
        let sample_prompts = request
            .sample_prompts
            .as_ref()
            .unwrap_or(&existing.sample_prompts);
        let tags_json = serde_json::to_string(tags)?;
        let sample_prompts_json = serde_json::to_string(sample_prompts)?;
        let token_count = Self::estimate_tokens(system_prompt);

        let result = sqlx::query(
            r"
            UPDATE coaches SET
                title = $1, description = $2, system_prompt = $3,
                category = $4, tags = $5, sample_prompts = $6, token_count = $7, updated_at = $8
            WHERE id = $9 AND tenant_id = $10 AND is_system = 1
            ",
        )
        .bind(title)
        .bind(&description)
        .bind(system_prompt)
        .bind(category.as_str())
        .bind(&tags_json)
        .bind(&sample_prompts_json)
        .bind(i64::from(token_count))
        .bind(now.to_rfc3339())
        .bind(coach_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update system coach: {e}")))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        // Return updated coach
        self.get_system_coach(coach_id, tenant_id).await
    }

    /// Delete a system coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn delete_system_coach(&self, coach_id: &str, tenant_id: &str) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM coaches
            WHERE id = $1 AND tenant_id = $2 AND is_system = 1
            ",
        )
        .bind(coach_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete system coach: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Assign a coach to a user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn assign_coach(
        &self,
        coach_id: &str,
        user_id: Uuid,
        assigned_by: Uuid,
    ) -> AppResult<bool> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        // Use INSERT OR IGNORE to handle duplicates gracefully
        let result = sqlx::query(
            r"
            INSERT OR IGNORE INTO coach_assignments (id, coach_id, user_id, assigned_by, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ",
        )
        .bind(id.to_string())
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(assigned_by.to_string())
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to assign coach: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Unassign a coach from a user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn unassign_coach(&self, coach_id: &str, user_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM coach_assignments
            WHERE coach_id = $1 AND user_id = $2
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to unassign coach: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// List all assignments for a coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_assignments(&self, coach_id: &str) -> AppResult<Vec<CoachAssignment>> {
        let rows = sqlx::query(
            r"
            SELECT ca.user_id, ca.created_at, ca.assigned_by, u.email
            FROM coach_assignments ca
            LEFT JOIN users u ON ca.user_id = u.id
            WHERE ca.coach_id = $1
            ORDER BY ca.created_at DESC
            ",
        )
        .bind(coach_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list assignments: {e}")))?;

        rows.iter()
            .map(|row| {
                let user_id: String = row.get("user_id");
                let created_at: String = row.get("created_at");
                let assigned_by: Option<String> = row.get("assigned_by");
                let user_email: Option<String> = row.get("email");

                Ok(CoachAssignment {
                    user_id,
                    user_email,
                    assigned_at: created_at,
                    assigned_by,
                })
            })
            .collect()
    }

    // ============================================
    // User Coach Preferences Methods
    // ============================================

    /// Hide a coach from a user's view
    ///
    /// Only system or assigned coaches can be hidden (not personal coaches).
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn hide_coach(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<bool> {
        // Check if the coach is hideable (must be system or assigned, not personal)
        if !self.is_coach_hideable(coach_id, user_id, tenant_id).await? {
            return Err(AppError::invalid_input(
                "Only system or assigned coaches can be hidden",
            ));
        }

        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        // Use INSERT OR REPLACE to update existing preference
        sqlx::query(
            r"
            INSERT INTO user_coach_preferences (id, user_id, coach_id, is_hidden, created_at)
            VALUES ($1, $2, $3, 1, $4)
            ON CONFLICT(user_id, coach_id) DO UPDATE SET is_hidden = 1
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(coach_id)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to hide coach: {e}")))?;

        Ok(true)
    }

    /// Show a previously hidden coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn show_coach(&self, coach_id: &str, user_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM user_coach_preferences
            WHERE coach_id = $1 AND user_id = $2 AND is_hidden = 1
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to show coach: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// List hidden coaches for a user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_hidden_coaches(
        &self,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<Vec<Coach>> {
        let rows = sqlx::query(
            r"
            SELECT c.id, c.user_id, c.tenant_id, c.title, c.description, c.system_prompt,
                   c.category, c.tags, c.sample_prompts, c.token_count, c.is_favorite, c.is_active, c.use_count,
                   c.last_used_at, c.created_at, c.updated_at, c.is_system, c.visibility
            FROM coaches c
            INNER JOIN user_coach_preferences ucp ON c.id = ucp.coach_id
            WHERE ucp.user_id = $1 AND ucp.is_hidden = 1 AND c.tenant_id = $2
            ORDER BY c.title
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list hidden coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Check if a coach can be hidden by a user
    ///
    /// A coach is hideable if it's a system coach or assigned to the user,
    /// but NOT if it's a personal coach created by the user.
    async fn is_coach_hideable(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: &str,
    ) -> AppResult<bool> {
        // Check if it's a system coach
        let is_system = sqlx::query(
            r"
            SELECT 1 FROM coaches
            WHERE id = $1 AND tenant_id = $2 AND is_system = 1
            ",
        )
        .bind(coach_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to check system coach: {e}")))?
        .is_some();

        if is_system {
            return Ok(true);
        }

        // Check if it's assigned to the user
        let is_assigned = sqlx::query(
            r"
            SELECT 1 FROM coach_assignments
            WHERE coach_id = $1 AND user_id = $2
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to check assignment: {e}")))?
        .is_some();

        Ok(is_assigned)
    }
}

/// Coach assignment info
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoachAssignment {
    /// User ID
    pub user_id: String,
    /// User email (for display)
    pub user_email: Option<String>,
    /// When assigned
    pub assigned_at: String,
    /// Who assigned
    pub assigned_by: Option<String>,
}

/// Request to create a system coach
pub struct CreateSystemCoachRequest {
    /// Display title
    pub title: String,
    /// Description
    pub description: Option<String>,
    /// System prompt
    pub system_prompt: String,
    /// Category
    pub category: CoachCategory,
    /// Tags
    pub tags: Vec<String>,
    /// Sample prompts for quick-start suggestions
    pub sample_prompts: Vec<String>,
    /// Visibility
    pub visibility: CoachVisibility,
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

    // Fields with defaults for backward compatibility
    let is_system: i64 = row.try_get("is_system").unwrap_or(0);
    let visibility_str: String = row
        .try_get("visibility")
        .unwrap_or_else(|_| "private".to_owned());
    let sample_prompts_json: String = row
        .try_get("sample_prompts")
        .unwrap_or_else(|_| "[]".to_owned());

    let tags: Vec<String> = serde_json::from_str(&tags_json)?;
    let sample_prompts: Vec<String> = serde_json::from_str(&sample_prompts_json)?;

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
        sample_prompts,
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
        is_system: is_system == 1,
        visibility: CoachVisibility::parse(&visibility_str),
    })
}

/// Convert a database row to a `CoachListItem` (with `is_assigned` column)
fn row_to_coach_list_item(row: &SqliteRow) -> AppResult<CoachListItem> {
    let coach = row_to_coach(row)?;
    let is_assigned: i64 = row.try_get("is_assigned").unwrap_or(0);
    Ok(CoachListItem {
        coach,
        is_assigned: is_assigned == 1,
    })
}
