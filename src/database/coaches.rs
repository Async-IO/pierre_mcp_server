// ABOUTME: Database operations for user-created Coaches (custom AI personas)
// ABOUTME: Handles CRUD operations for coaches with tenant isolation and token counting
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::coaches::CoachPrerequisites;
use crate::errors::{AppError, AppResult};
use crate::pagination::{Cursor, CursorPage, StoreCursor, StoreSortOrder};
use chrono::{DateTime, Utc};
use pierre_core::models::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::collections::HashMap;
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

/// Coach publish status for Store workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PublishStatus {
    /// Not submitted for review (default)
    #[default]
    Draft,
    /// Submitted and waiting for admin approval
    PendingReview,
    /// Approved and visible in Store
    Published,
    /// Rejected by admin (reason provided)
    Rejected,
}

impl PublishStatus {
    /// Convert to database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Published => "published",
            Self::Rejected => "rejected",
        }
    }

    /// Parse from database string representation
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "pending_review" => Self::PendingReview,
            "published" => Self::Published,
            "rejected" => Self::Rejected,
            _ => Self::Draft,
        }
    }

    /// Check if coach is visible in the Store
    #[must_use]
    pub const fn is_published(&self) -> bool {
        matches!(self, Self::Published)
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
    /// Mobility, stretching, and yoga focused coaches
    Mobility,
    /// Analysis and insights focused coaches
    Analysis,
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
            Self::Mobility => "mobility",
            Self::Analysis => "analysis",
            Self::Custom => "custom",
        }
    }

    /// Parse from database string representation (case-insensitive)
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "training" => Self::Training,
            "nutrition" => Self::Nutrition,
            "recovery" => Self::Recovery,
            "recipes" => Self::Recipes,
            "mobility" => Self::Mobility,
            "analysis" => Self::Analysis,
            _ => Self::Custom,
        }
    }

    /// Human-readable display name for UI
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Training => "Training",
            Self::Nutrition => "Nutrition",
            Self::Recovery => "Recovery",
            Self::Recipes => "Recipes",
            Self::Mobility => "Mobility",
            Self::Analysis => "Analysis",
            Self::Custom => "Custom",
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
    /// Prerequisites required to use this coach (providers, activities, etc.)
    #[serde(default)]
    pub prerequisites: CoachPrerequisites,
    /// ID of the coach this was forked from (None for original coaches)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
    /// Publishing status for Store workflow
    #[serde(default)]
    pub publish_status: PublishStatus,
    /// When the coach was published to the store
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<DateTime<Utc>>,
    /// When the coach was submitted for review
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_submitted_at: Option<DateTime<Utc>>,
    /// When admin made the review decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_decision_at: Option<DateTime<Utc>>,
    /// Admin user who made the review decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_decision_by: Option<String>,
    /// Reason for rejection (if rejected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    /// Number of Store installs (denormalized for performance)
    #[serde(default)]
    pub install_count: u32,
    /// URL to coach icon/avatar for Store display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    /// Author profile ID (for published coaches)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
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
        tenant_id: TenantId,
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
                last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
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
        .bind(Option::<String>::None) // prerequisites (user-created coaches don't have prerequisites)
        .bind(Option::<String>::None) // forked_from (not a fork)
        .bind(PublishStatus::Draft.as_str()) // publish_status (default to draft)
        .bind(Option::<String>::None) // published_at
        .bind(Option::<String>::None) // review_submitted_at
        .bind(Option::<String>::None) // review_decision_at
        .bind(Option::<String>::None) // review_decision_by
        .bind(Option::<String>::None) // rejection_reason
        .bind(0i64) // install_count
        .bind(Option::<String>::None) // icon_url
        .bind(Option::<String>::None) // author_id
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create coach: {e}")))?;

        Ok(Coach {
            id,
            user_id,
            tenant_id: tenant_id.to_string(),
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
            prerequisites: CoachPrerequisites::default(),
            forked_from: None,
            publish_status: PublishStatus::Draft,
            published_at: None,
            review_submitted_at: None,
            review_decision_at: None,
            review_decision_by: None,
            rejection_reason: None,
            install_count: 0,
            icon_url: None,
            author_id: None,
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
        tenant_id: TenantId,
    ) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
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
        tenant_id: TenantId,
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
        // System coaches (is_system=1) are always visible to all users
        // regardless of their visibility setting - they're platform-wide resources
        let system_condition = if filter.include_system {
            "OR c.is_system = 1"
        } else {
            ""
        };

        // Build the unified query
        // Uses a subquery to identify assigned coaches for the is_assigned flag
        let query = format!(
            r"
            SELECT c.id, c.user_id, c.tenant_id, c.title, c.description, c.system_prompt,
                   c.category, c.tags, c.sample_prompts, c.token_count, c.is_favorite, c.is_active, c.use_count,
                   c.last_used_at, c.created_at, c.updated_at, c.is_system, c.visibility, c.prerequisites, c.forked_from,
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
            .bind(tenant_id.to_string())
            .bind(limit_val)
            .bind(offset_val)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to list coaches: {e}")))?;

        rows.iter().map(row_to_coach_list_item).collect()
    }

    /// Update an existing coach
    ///
    /// Automatically creates a version snapshot before applying changes.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or coach not found
    pub async fn update(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &UpdateCoachRequest,
    ) -> AppResult<Option<Coach>> {
        self.update_with_summary(coach_id, user_id, tenant_id, request, None)
            .await
    }

    /// Update an existing coach with a change summary
    ///
    /// Automatically creates a version snapshot before applying changes.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or coach not found
    pub async fn update_with_summary(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &UpdateCoachRequest,
        change_summary: Option<&str>,
    ) -> AppResult<Option<Coach>> {
        // First get the existing coach
        let existing = self.get(coach_id, user_id, tenant_id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        // Create a version snapshot BEFORE applying changes
        self.create_version(coach_id, user_id, change_summary)
            .await?;

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
        .bind(tenant_id.to_string())
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
    pub async fn delete(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM coaches
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete coach: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Fork a system coach to create a user-owned copy
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Source coach is not found
    /// - Source coach is not a system coach
    /// - Database operation fails
    pub async fn fork_coach(
        &self,
        source_coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<Coach> {
        // Get the source coach (must be a system coach)
        // System coaches are platform-wide, so no tenant filter — any user can fork them
        let source = self
            .get_system_coach_any_tenant(source_coach_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {source_coach_id}")))?;

        if !source.is_system {
            return Err(AppError::invalid_input(
                "Only system coaches can be forked. Use duplicate for personal coaches.",
            ));
        }

        let now = Utc::now();
        let id = Uuid::new_v4();
        let tags_json = serde_json::to_string(&source.tags)?;
        let sample_prompts_json = serde_json::to_string(&source.sample_prompts)?;
        let prerequisites_json = serde_json::to_string(&source.prerequisites)?;

        sqlx::query(
            r"
            INSERT INTO coaches (
                id, user_id, tenant_id, title, description, system_prompt,
                category, tags, sample_prompts, token_count, is_favorite, use_count,
                last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(&source.title)
        .bind(&source.description)
        .bind(&source.system_prompt)
        .bind(source.category.as_str())
        .bind(&tags_json)
        .bind(&sample_prompts_json)
        .bind(i64::from(source.token_count))
        .bind(false) // is_favorite
        .bind(0i64) // use_count
        .bind(Option::<String>::None) // last_used_at
        .bind(now.to_rfc3339())
        .bind(0i64) // is_system = false (user's copy)
        .bind(CoachVisibility::Private.as_str()) // visibility = private
        .bind(&prerequisites_json) // prerequisites
        .bind(source_coach_id) // forked_from
        .bind(PublishStatus::Draft.as_str()) // publish_status (forked coaches start as draft)
        .bind(Option::<String>::None) // published_at
        .bind(Option::<String>::None) // review_submitted_at
        .bind(Option::<String>::None) // review_decision_at
        .bind(Option::<String>::None) // review_decision_by
        .bind(Option::<String>::None) // rejection_reason
        .bind(0i64) // install_count
        .bind(Option::<String>::None) // icon_url
        .bind(Option::<String>::None) // author_id
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fork coach: {e}")))?;

        Ok(Coach {
            id,
            user_id,
            tenant_id: tenant_id.to_string(),
            title: source.title,
            description: source.description,
            system_prompt: source.system_prompt,
            category: source.category,
            tags: source.tags,
            sample_prompts: source.sample_prompts,
            token_count: source.token_count,
            is_favorite: false,
            is_active: false,
            use_count: 0,
            last_used_at: None,
            created_at: now,
            updated_at: now,
            is_system: false,
            visibility: CoachVisibility::Private,
            prerequisites: source.prerequisites,
            forked_from: Some(source_coach_id.to_owned()),
            publish_status: PublishStatus::Draft,
            published_at: None,
            review_submitted_at: None,
            review_decision_at: None,
            review_decision_by: None,
            rejection_reason: None,
            install_count: 0,
            icon_url: None,
            author_id: None,
        })
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
        tenant_id: TenantId,
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
        .bind(tenant_id.to_string())
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
        tenant_id: TenantId,
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
        .bind(tenant_id.to_string())
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
        .bind(tenant_id.to_string())
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
    pub async fn count(&self, user_id: Uuid, tenant_id: TenantId) -> AppResult<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count FROM coaches
            WHERE user_id = $1 AND tenant_id = $2
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
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
        tenant_id: TenantId,
        query: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<Coach>> {
        let limit_val = i32::try_from(limit.unwrap_or(20)).unwrap_or(20);
        let offset_val = i32::try_from(offset.unwrap_or(0)).unwrap_or(0);
        let search_pattern = format!("%{query}%");

        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE user_id = $1 AND tenant_id = $2 AND (
                title LIKE $3 OR description LIKE $3 OR tags LIKE $3
            )
            ORDER BY updated_at DESC
            LIMIT $4 OFFSET $5
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(&search_pattern)
        .bind(limit_val)
        .bind(offset_val)
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
        tenant_id: TenantId,
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
        .bind(tenant_id.to_string())
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
        .bind(tenant_id.to_string())
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
    pub async fn deactivate_coach(&self, user_id: Uuid, tenant_id: TenantId) -> AppResult<bool> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r"
            UPDATE coaches SET is_active = 0, updated_at = $1
            WHERE user_id = $2 AND tenant_id = $3 AND is_active = 1
            ",
        )
        .bind(&now)
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
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
        tenant_id: TenantId,
    ) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE user_id = $1 AND tenant_id = $2 AND is_active = 1
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
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
        tenant_id: TenantId,
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
                last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)
            ",
        )
        .bind(id.to_string())
        .bind(admin_user_id.to_string())
        .bind(tenant_id.to_string())
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
        .bind(Option::<String>::None) // prerequisites (system coaches may have this set later)
        .bind(Option::<String>::None) // forked_from (system coaches are originals)
        .bind(PublishStatus::Draft.as_str()) // publish_status (system coaches start as draft)
        .bind(Option::<String>::None) // published_at
        .bind(Option::<String>::None) // review_submitted_at
        .bind(Option::<String>::None) // review_decision_at
        .bind(Option::<String>::None) // review_decision_by
        .bind(Option::<String>::None) // rejection_reason
        .bind(0i64) // install_count
        .bind(Option::<String>::None) // icon_url
        .bind(Option::<String>::None) // author_id
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create system coach: {e}")))?;

        Ok(Coach {
            id,
            user_id: admin_user_id,
            tenant_id: tenant_id.to_string(),
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
            prerequisites: CoachPrerequisites::default(),
            forked_from: None,
            publish_status: PublishStatus::Draft,
            published_at: None,
            review_submitted_at: None,
            review_decision_at: None,
            review_decision_by: None,
            rejection_reason: None,
            install_count: 0,
            icon_url: None,
            author_id: None,
        })
    }

    /// List all system coaches in a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_system_coaches(&self, tenant_id: TenantId) -> AppResult<Vec<Coach>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE tenant_id = $1 AND is_system = 1
            ORDER BY created_at DESC
            ",
        )
        .bind(tenant_id.to_string())
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
        tenant_id: TenantId,
    ) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE id = $1 AND tenant_id = $2 AND is_system = 1
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get system coach: {e}")))?;

        row.map(|r| row_to_coach(&r)).transpose()
    }

    /// Get a system coach by ID without tenant filtering
    ///
    /// System coaches are platform-wide resources visible to all users.
    /// Used by `fork_coach` where any user can fork any system coach.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_system_coach_any_tenant(&self, coach_id: &str) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE id = $1 AND is_system = 1
            ",
        )
        .bind(coach_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get system coach: {e}")))?;

        row.map(|r| row_to_coach(&r)).transpose()
    }

    /// Update a system coach
    ///
    /// Automatically creates a version snapshot before applying changes.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_system_coach(
        &self,
        coach_id: &str,
        tenant_id: TenantId,
        request: &UpdateCoachRequest,
    ) -> AppResult<Option<Coach>> {
        self.update_system_coach_with_summary(coach_id, tenant_id, request, None)
            .await
    }

    /// Update a system coach with a change summary
    ///
    /// Automatically creates a version snapshot before applying changes.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_system_coach_with_summary(
        &self,
        coach_id: &str,
        tenant_id: TenantId,
        request: &UpdateCoachRequest,
        change_summary: Option<&str>,
    ) -> AppResult<Option<Coach>> {
        // First get the existing coach
        let existing = self.get_system_coach(coach_id, tenant_id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        // Create a version snapshot BEFORE applying changes
        // Use the existing coach's user_id (admin who created it) for the version record
        self.create_version(coach_id, existing.user_id, change_summary)
            .await?;

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
        .bind(tenant_id.to_string())
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
    pub async fn delete_system_coach(
        &self,
        coach_id: &str,
        tenant_id: TenantId,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM coaches
            WHERE id = $1 AND tenant_id = $2 AND is_system = 1
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
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

    /// List all assignments for a coach (no tenant filtering).
    ///
    /// Used by tests where `tenant_users` table may not be set up.
    /// Production code should use `list_assignments_for_tenant` instead.
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

    /// List assignments for a coach, scoped to a specific tenant.
    ///
    /// Only returns assignments where the assigned user belongs to the given tenant,
    /// preventing cross-tenant data leakage.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_assignments_for_tenant(
        &self,
        coach_id: &str,
        tenant_id: TenantId,
    ) -> AppResult<Vec<CoachAssignment>> {
        let rows = sqlx::query(
            r"
            SELECT ca.user_id, ca.created_at, ca.assigned_by, u.email
            FROM coach_assignments ca
            LEFT JOIN users u ON ca.user_id = u.id
            INNER JOIN tenant_users tu ON ca.user_id = tu.user_id AND tu.tenant_id = $2
            WHERE ca.coach_id = $1
            ORDER BY ca.created_at DESC
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
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
    pub async fn hide_coach(&self, coach_id: &str, user_id: Uuid) -> AppResult<bool> {
        // Check if the coach is hideable (must be system or assigned, not personal)
        if !self.is_coach_hideable(coach_id, user_id).await? {
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
        tenant_id: TenantId,
    ) -> AppResult<Vec<Coach>> {
        let rows = sqlx::query(
            r"
            SELECT c.id, c.user_id, c.tenant_id, c.title, c.description, c.system_prompt,
                   c.category, c.tags, c.sample_prompts, c.token_count, c.is_favorite, c.is_active, c.use_count,
                   c.last_used_at, c.created_at, c.updated_at, c.is_system, c.visibility, c.prerequisites, c.forked_from
            FROM coaches c
            INNER JOIN user_coach_preferences ucp ON c.id = ucp.coach_id
            WHERE ucp.user_id = $1 AND ucp.is_hidden = 1 AND c.tenant_id = $2
            ORDER BY c.title
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list hidden coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Check if a coach can be hidden by a user
    ///
    /// A coach is hideable if it's a system coach or assigned to the user,
    /// but NOT if it's a personal coach created by the user.
    async fn is_coach_hideable(&self, coach_id: &str, user_id: Uuid) -> AppResult<bool> {
        // Check if it's a system coach
        // System coaches are visible across all tenants, so no tenant_id restriction here
        let is_system = sqlx::query(
            r"
            SELECT 1 FROM coaches
            WHERE id = $1 AND is_system = 1
            ",
        )
        .bind(coach_id)
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

    // ============================================
    // Coach Version History Methods (ASY-153)
    // ============================================

    /// Create a new version snapshot for a coach
    ///
    /// This is called automatically when a coach is updated to track version history.
    /// Returns the new version number.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn create_version(
        &self,
        coach_id: &str,
        user_id: Uuid,
        change_summary: Option<&str>,
    ) -> AppResult<i32> {
        // Get the current coach to snapshot
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches WHERE id = $1
            ",
        )
        .bind(coach_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get coach for versioning: {e}")))?
        .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        let coach = row_to_coach(&row)?;

        // Get the next version number
        let version_row = sqlx::query(
            r"
            SELECT COALESCE(MAX(version), 0) as max_version
            FROM coach_versions WHERE coach_id = $1
            ",
        )
        .bind(coach_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get max version: {e}")))?;

        let max_version: i32 = version_row.get("max_version");
        let new_version = max_version + 1;

        // Create content snapshot as JSON
        let content_snapshot = serde_json::json!({
            "title": coach.title,
            "description": coach.description,
            "system_prompt": coach.system_prompt,
            "category": coach.category.as_str(),
            "tags": coach.tags,
            "sample_prompts": coach.sample_prompts,
            "token_count": coach.token_count,
            "visibility": coach.visibility.as_str(),
            "prerequisites": coach.prerequisites,
        });

        // Compute content hash
        let content_hash = compute_content_hash(&content_snapshot);

        // Insert the version record
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r"
            INSERT INTO coach_versions (
                id, coach_id, version, content_hash, content_snapshot,
                change_summary, created_at, created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ",
        )
        .bind(id.to_string())
        .bind(coach_id)
        .bind(new_version)
        .bind(&content_hash)
        .bind(content_snapshot.to_string())
        .bind(change_summary)
        .bind(now.to_rfc3339())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create version: {e}")))?;

        Ok(new_version)
    }

    /// Get version history for a coach
    ///
    /// Returns versions in descending order (newest first).
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_versions(
        &self,
        coach_id: &str,
        tenant_id: TenantId,
        limit: u32,
    ) -> AppResult<Vec<CoachVersion>> {
        // Verify the coach exists and belongs to the tenant
        let exists = sqlx::query(
            r"
            SELECT 1 FROM coaches WHERE id = $1 AND tenant_id = $2
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to verify coach: {e}")))?;

        if exists.is_none() {
            return Err(AppError::not_found(format!("Coach {coach_id}")));
        }

        let limit_val = i32::try_from(limit).unwrap_or(50);

        let rows = sqlx::query(
            r"
            SELECT cv.id, cv.coach_id, cv.version, cv.content_hash, cv.content_snapshot,
                   cv.change_summary, cv.created_at, cv.created_by
            FROM coach_versions cv
            WHERE cv.coach_id = $1
            ORDER BY cv.version DESC
            LIMIT $2
            ",
        )
        .bind(coach_id)
        .bind(limit_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get versions: {e}")))?;

        rows.iter().map(row_to_coach_version).collect()
    }

    /// Get a specific version of a coach
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or version not found
    pub async fn get_version(
        &self,
        coach_id: &str,
        version: i32,
        tenant_id: TenantId,
    ) -> AppResult<Option<CoachVersion>> {
        // Verify the coach exists and belongs to the tenant
        let exists = sqlx::query(
            r"
            SELECT 1 FROM coaches WHERE id = $1 AND tenant_id = $2
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to verify coach: {e}")))?;

        if exists.is_none() {
            return Err(AppError::not_found(format!("Coach {coach_id}")));
        }

        let row = sqlx::query(
            r"
            SELECT id, coach_id, version, content_hash, content_snapshot,
                   change_summary, created_at, created_by
            FROM coach_versions
            WHERE coach_id = $1 AND version = $2
            ",
        )
        .bind(coach_id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get version: {e}")))?;

        row.map(|r| row_to_coach_version(&r)).transpose()
    }

    /// Revert a coach to a previous version
    ///
    /// This creates a NEW version with the content from the specified version,
    /// preserving the complete history (doesn't delete any versions).
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or version not found
    pub async fn revert_to_version(
        &self,
        coach_id: &str,
        version: i32,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<Coach> {
        // Get the target version
        let target_version = self
            .get_version(coach_id, version, tenant_id)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("Version {version} for coach {coach_id}"))
            })?;

        // Extract fields from the snapshot
        let snapshot = &target_version.content_snapshot;

        let title = snapshot
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::internal("Missing title in version snapshot"))?;

        let description = snapshot
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let system_prompt = snapshot
            .get("system_prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::internal("Missing system_prompt in version snapshot"))?;

        let category_str = snapshot
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("custom");

        let tags: Vec<String> = snapshot
            .get("tags")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let sample_prompts: Vec<String> = snapshot
            .get("sample_prompts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let now = Utc::now();
        let tags_json = serde_json::to_string(&tags)?;
        let sample_prompts_json = serde_json::to_string(&sample_prompts)?;
        let token_count = Self::estimate_tokens(system_prompt);

        // Update the coach with the reverted content
        let result = sqlx::query(
            r"
            UPDATE coaches SET
                title = $1, description = $2, system_prompt = $3,
                category = $4, tags = $5, sample_prompts = $6, token_count = $7, updated_at = $8
            WHERE id = $9 AND tenant_id = $10
            ",
        )
        .bind(title)
        .bind(&description)
        .bind(system_prompt)
        .bind(category_str)
        .bind(&tags_json)
        .bind(&sample_prompts_json)
        .bind(i64::from(token_count))
        .bind(now.to_rfc3339())
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to revert coach: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("Coach {coach_id}")));
        }

        // Create a new version recording this revert
        let change_summary = format!("Reverted to version {version}");
        self.create_version(coach_id, user_id, Some(&change_summary))
            .await?;

        // Return the updated coach
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches WHERE id = $1 AND tenant_id = $2
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get reverted coach: {e}")))?
        .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        row_to_coach(&row)
    }

    /// Get the current version number for a coach
    ///
    /// Returns 0 if no versions exist yet.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_current_version(&self, coach_id: &str) -> AppResult<i32> {
        let row = sqlx::query(
            r"
            SELECT COALESCE(MAX(version), 0) as current_version
            FROM coach_versions WHERE coach_id = $1
            ",
        )
        .bind(coach_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get current version: {e}")))?;

        Ok(row.get("current_version"))
    }

    // ============================================
    // Store Methods (Publishing and Discovery)
    // ============================================

    /// Submit a coach for admin review
    ///
    /// Changes `publish_status` from `draft` to `pending_review`.
    /// Only the coach owner can submit their coach.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Coach not found or user doesn't own it
    /// - Coach is not in draft status
    /// - Database operation fails
    pub async fn submit_for_review(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<Coach> {
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE coaches SET
                publish_status = $1,
                review_submitted_at = $2,
                updated_at = $2
            WHERE id = $3 AND user_id = $4 AND tenant_id = $5 AND publish_status = 'draft'
            ",
        )
        .bind(PublishStatus::PendingReview.as_str())
        .bind(now.to_rfc3339())
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to submit coach for review: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::invalid_input(
                "Coach not found, not owned by you, or not in draft status",
            ));
        }

        self.get(coach_id, user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))
    }

    /// Get coaches pending admin review
    ///
    /// Returns coaches with `publish_status = 'pending_review'` ordered by submission time.
    /// This is an admin-only operation.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_pending_review_coaches(
        &self,
        tenant_id: TenantId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<Coach>> {
        let limit_val = i64::from(limit.unwrap_or(50).min(100));
        let offset_val = i64::from(offset.unwrap_or(0));

        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE tenant_id = $1 AND publish_status = 'pending_review'
            ORDER BY review_submitted_at ASC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(tenant_id.to_string())
        .bind(limit_val)
        .bind(offset_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get pending review coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Approve a coach and publish to the Store
    ///
    /// Changes `publish_status` from `pending_review` to `published`.
    /// This is an admin-only operation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Coach not found or not pending review
    /// - Database operation fails
    pub async fn approve_coach(
        &self,
        coach_id: &str,
        tenant_id: TenantId,
        admin_user_id: impl Into<Option<Uuid>>,
    ) -> AppResult<Coach> {
        let admin_user_id = admin_user_id.into();
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE coaches SET
                publish_status = $1,
                published_at = $2,
                review_decision_at = $2,
                review_decision_by = $3,
                rejection_reason = NULL,
                updated_at = $2
            WHERE id = $4 AND tenant_id = $5 AND publish_status = 'pending_review'
            ",
        )
        .bind(PublishStatus::Published.as_str())
        .bind(now.to_rfc3339())
        .bind(admin_user_id.map(|id| id.to_string()))
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to approve coach: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::invalid_input(
                "Coach not found or not pending review",
            ));
        }

        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches WHERE id = $1 AND tenant_id = $2
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get approved coach: {e}")))?;

        row_to_coach(&row)
    }

    /// Reject a coach with a reason
    ///
    /// Changes `publish_status` from `pending_review` to `rejected`.
    /// This is an admin-only operation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Coach not found or not pending review
    /// - Database operation fails
    pub async fn reject_coach(
        &self,
        coach_id: &str,
        tenant_id: TenantId,
        admin_user_id: impl Into<Option<Uuid>>,
        reason: &str,
    ) -> AppResult<Coach> {
        let admin_user_id = admin_user_id.into();
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE coaches SET
                publish_status = $1,
                review_decision_at = $2,
                review_decision_by = $3,
                rejection_reason = $4,
                updated_at = $2
            WHERE id = $5 AND tenant_id = $6 AND publish_status = 'pending_review'
            ",
        )
        .bind(PublishStatus::Rejected.as_str())
        .bind(now.to_rfc3339())
        .bind(admin_user_id.map(|id| id.to_string()))
        .bind(reason)
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to reject coach: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::invalid_input(
                "Coach not found or not pending review",
            ));
        }

        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches WHERE id = $1 AND tenant_id = $2
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get rejected coach: {e}")))?;

        row_to_coach(&row)
    }

    /// Get coaches that have been rejected with reason
    ///
    /// Returns coaches with `publish_status = 'rejected'` for admin review history.
    /// This is an admin-only operation.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_rejected_coaches(
        &self,
        tenant_id: TenantId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<Coach>> {
        let limit_val = i64::from(limit.unwrap_or(50).min(100));
        let offset_val = i64::from(offset.unwrap_or(0));

        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE tenant_id = $1 AND publish_status = 'rejected'
            ORDER BY review_decision_at DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(tenant_id.to_string())
        .bind(limit_val)
        .bind(offset_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get rejected coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Unpublish a coach (revert from published to draft)
    ///
    /// Changes `publish_status` from `published` to `draft` and clears publish date.
    /// This is an admin-only operation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Coach not found or not published
    /// - Database operation fails
    pub async fn unpublish_coach(&self, coach_id: &str, tenant_id: TenantId) -> AppResult<Coach> {
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE coaches SET
                publish_status = $1,
                published_at = NULL,
                updated_at = $2
            WHERE id = $3 AND tenant_id = $4 AND publish_status = 'published'
            ",
        )
        .bind(PublishStatus::Draft.as_str())
        .bind(now.to_rfc3339())
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to unpublish coach: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::invalid_input("Coach not found or not published"));
        }

        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches WHERE id = $1 AND tenant_id = $2
            ",
        )
        .bind(coach_id)
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get unpublished coach: {e}")))?;

        row_to_coach(&row)
    }

    /// Get store admin statistics
    ///
    /// Returns counts for pending, published, rejected coaches and total installs.
    /// This is an admin-only operation.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_store_admin_stats(&self, tenant_id: TenantId) -> AppResult<StoreAdminStats> {
        let row = sqlx::query(
            r"
            SELECT
                COUNT(CASE WHEN publish_status = 'pending_review' THEN 1 END) as pending_count,
                COUNT(CASE WHEN publish_status = 'published' THEN 1 END) as published_count,
                COUNT(CASE WHEN publish_status = 'rejected' THEN 1 END) as rejected_count,
                COALESCE(SUM(CASE WHEN publish_status = 'published' THEN install_count ELSE 0 END), 0) as total_installs
            FROM coaches
            WHERE tenant_id = $1
            ",
        )
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get store stats: {e}")))?;

        let pending_count: i64 = row.get("pending_count");
        let published_count: i64 = row.get("published_count");
        let rejected_count: i64 = row.get("rejected_count");
        let total_installs: i64 = row.get("total_installs");

        // Calculate rejection rate
        // Values are always non-negative counts from DB, precision loss acceptable for percentage
        let total_decided = published_count + rejected_count;
        #[allow(clippy::cast_precision_loss)]
        let rejection_rate = if total_decided > 0 {
            (rejected_count as f64 / total_decided as f64) * 100.0
        } else {
            0.0
        };

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(StoreAdminStats {
            pending_count: pending_count as u32,
            published_count: published_count as u32,
            rejected_count: rejected_count as u32,
            total_installs: total_installs as u32,
            rejection_rate,
        })
    }

    /// Get author email for a coach by looking up the user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_author_email(&self, user_id: Uuid) -> AppResult<Option<String>> {
        let row = sqlx::query(
            r"
            SELECT email FROM users WHERE id = $1
            ",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get author email: {e}")))?;

        Ok(row.map(|r| r.get("email")))
    }

    /// Get published coaches for Store browsing
    ///
    /// Returns coaches with `publish_status = 'published'` sorted by various criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    /// Get published coaches for the Store (cross-tenant)
    ///
    /// Published coaches are visible to ALL users regardless of tenant.
    /// This enables the Store to be a global marketplace.
    pub async fn get_published_coaches(
        &self,
        category: Option<CoachCategory>,
        sort_by: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<Coach>> {
        let limit_val = i64::from(limit.unwrap_or(50).min(100));
        let offset_val = i64::from(offset.unwrap_or(0));

        let order_clause = match sort_by {
            Some("popular") => "install_count DESC, published_at DESC",
            Some("title") => "title ASC",
            // "newest" is the default, so handle all other cases the same way
            _ => "published_at DESC",
        };

        let category_filter = category.map_or_else(String::new, |cat| {
            format!("AND category = '{}'", cat.as_str())
        });

        let query = format!(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE publish_status = 'published' {category_filter}
            ORDER BY {order_clause}
            LIMIT $1 OFFSET $2
            "
        );

        let rows = sqlx::query(&query)
            .bind(limit_val)
            .bind(offset_val)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get published coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Get published coaches with cursor-based pagination for Store browsing
    ///
    /// Returns coaches with `publish_status = 'published'` using cursor-based
    /// pagination for efficient infinite scrolling. Supports multiple sort orders.
    ///
    /// # Arguments
    ///
    /// * `category` - Optional category filter
    /// * `sort_by` - Sort order (newest, popular, title)
    /// * `limit` - Maximum number of items to return (default 20, max 100)
    /// * `cursor` - Optional cursor from previous page
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or cursor is invalid
    pub async fn get_published_coaches_cursor(
        &self,
        category: Option<CoachCategory>,
        sort_by: StoreSortOrder,
        limit: u32,
        cursor: Option<&str>,
    ) -> AppResult<CursorPage<Coach>> {
        let limit_val = limit.min(100);
        // Fetch one extra to determine if there are more pages
        let fetch_limit = i64::from(limit_val) + 1;

        // Decode cursor if provided
        let decoded_cursor = if let Some(cursor_str) = cursor {
            let cursor_obj = Cursor::from_string(cursor_str.to_owned());
            let decoded = StoreCursor::decode(&cursor_obj, sort_by)
                .ok_or_else(|| AppError::invalid_input("Invalid cursor for current sort order"))?;
            Some(decoded)
        } else {
            None
        };

        let category_filter = category.map_or_else(String::new, |cat| {
            format!("AND category = '{}'", cat.as_str())
        });

        // Build query based on sort order and cursor presence
        let rows = match sort_by {
            StoreSortOrder::Newest => {
                self.query_newest_sort(&category_filter, decoded_cursor.as_ref(), fetch_limit)
                    .await?
            }
            StoreSortOrder::Popular => {
                self.query_popular_sort(&category_filter, decoded_cursor.as_ref(), fetch_limit)
                    .await?
            }
            StoreSortOrder::Title => {
                self.query_title_sort(&category_filter, decoded_cursor.as_ref(), fetch_limit)
                    .await?
            }
        };

        // Convert rows to coaches
        let mut all_coaches: Vec<Coach> = Vec::new();
        for row in rows {
            all_coaches.push(row_to_coach(&row)?);
        }

        // Check if we fetched more than requested (indicates more pages)
        let has_more = all_coaches.len() > limit_val as usize;

        // Trim to requested limit
        let coaches: Vec<Coach> = all_coaches.into_iter().take(limit_val as usize).collect();

        // Generate next cursor from last item
        let next_cursor = if has_more {
            coaches.last().map(|coach| {
                let store_cursor = match sort_by {
                    StoreSortOrder::Newest => {
                        StoreCursor::newest(coach.id.to_string(), coach.published_at)
                    }
                    StoreSortOrder::Popular => StoreCursor::popular(
                        coach.id.to_string(),
                        coach.install_count,
                        coach.published_at,
                    ),
                    StoreSortOrder::Title => {
                        StoreCursor::title(coach.id.to_string(), coach.title.clone())
                    }
                };
                store_cursor.encode()
            })
        } else {
            None
        };

        Ok(CursorPage::new(coaches, next_cursor, None, has_more))
    }

    /// Query for newest sort order (`published_at` DESC, id DESC)
    async fn query_newest_sort(
        &self,
        category_filter: &str,
        cursor: Option<&StoreCursor>,
        fetch_limit: i64,
    ) -> AppResult<Vec<SqliteRow>> {
        if let Some(c) = cursor {
            let ts = c.published_at.map_or(0, |dt| dt.timestamp_millis());
            let query = format!(
                r"
                SELECT id, user_id, tenant_id, title, description, system_prompt,
                       category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                       last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                       publish_status, published_at, review_submitted_at, review_decision_at,
                       review_decision_by, rejection_reason, install_count, icon_url, author_id
                FROM coaches
                WHERE publish_status = 'published' {category_filter}
                  AND (
                    (CAST(strftime('%s', published_at) AS INTEGER) * 1000 +
                     CAST(strftime('%f', published_at) * 1000 AS INTEGER) % 1000) < $1
                    OR (
                      (CAST(strftime('%s', published_at) AS INTEGER) * 1000 +
                       CAST(strftime('%f', published_at) * 1000 AS INTEGER) % 1000) = $1
                      AND id < $2
                    )
                  )
                ORDER BY published_at DESC, id DESC
                LIMIT $3
                "
            );
            sqlx::query(&query)
                .bind(ts)
                .bind(&c.id)
                .bind(fetch_limit)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::database(format!("Failed to query coaches (newest): {e}")))
        } else {
            let query = format!(
                r"
                SELECT id, user_id, tenant_id, title, description, system_prompt,
                       category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                       last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                       publish_status, published_at, review_submitted_at, review_decision_at,
                       review_decision_by, rejection_reason, install_count, icon_url, author_id
                FROM coaches
                WHERE publish_status = 'published' {category_filter}
                ORDER BY published_at DESC, id DESC
                LIMIT $1
                "
            );
            sqlx::query(&query)
                .bind(fetch_limit)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to query coaches (newest first): {e}"))
                })
        }
    }

    /// Query for popular sort order (`install_count` DESC, `published_at` DESC, id DESC)
    async fn query_popular_sort(
        &self,
        category_filter: &str,
        cursor: Option<&StoreCursor>,
        fetch_limit: i64,
    ) -> AppResult<Vec<SqliteRow>> {
        if let Some(c) = cursor {
            let count = c.install_count.unwrap_or(0);
            let ts = c.published_at.map_or(0, |dt| dt.timestamp_millis());
            let query = format!(
                r"
                SELECT id, user_id, tenant_id, title, description, system_prompt,
                       category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                       last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                       publish_status, published_at, review_submitted_at, review_decision_at,
                       review_decision_by, rejection_reason, install_count, icon_url, author_id
                FROM coaches
                WHERE publish_status = 'published' {category_filter}
                  AND (
                    install_count < $1
                    OR (
                      install_count = $1
                      AND (CAST(strftime('%s', published_at) AS INTEGER) * 1000 +
                           CAST(strftime('%f', published_at) * 1000 AS INTEGER) % 1000) < $2
                    )
                    OR (
                      install_count = $1
                      AND (CAST(strftime('%s', published_at) AS INTEGER) * 1000 +
                           CAST(strftime('%f', published_at) * 1000 AS INTEGER) % 1000) = $2
                      AND id < $3
                    )
                  )
                ORDER BY install_count DESC, published_at DESC, id DESC
                LIMIT $4
                "
            );
            sqlx::query(&query)
                .bind(count)
                .bind(ts)
                .bind(&c.id)
                .bind(fetch_limit)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::database(format!("Failed to query coaches (popular): {e}")))
        } else {
            let query = format!(
                r"
                SELECT id, user_id, tenant_id, title, description, system_prompt,
                       category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                       last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                       publish_status, published_at, review_submitted_at, review_decision_at,
                       review_decision_by, rejection_reason, install_count, icon_url, author_id
                FROM coaches
                WHERE publish_status = 'published' {category_filter}
                ORDER BY install_count DESC, published_at DESC, id DESC
                LIMIT $1
                "
            );
            sqlx::query(&query)
                .bind(fetch_limit)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to query coaches (popular first): {e}"))
                })
        }
    }

    /// Query for title sort order (title ASC, id ASC)
    async fn query_title_sort(
        &self,
        category_filter: &str,
        cursor: Option<&StoreCursor>,
        fetch_limit: i64,
    ) -> AppResult<Vec<SqliteRow>> {
        if let Some(c) = cursor {
            let title = c.title.as_deref().unwrap_or("");
            let query = format!(
                r"
                SELECT id, user_id, tenant_id, title, description, system_prompt,
                       category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                       last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                       publish_status, published_at, review_submitted_at, review_decision_at,
                       review_decision_by, rejection_reason, install_count, icon_url, author_id
                FROM coaches
                WHERE publish_status = 'published' {category_filter}
                  AND (
                    title > $1
                    OR (title = $1 AND id > $2)
                  )
                ORDER BY title ASC, id ASC
                LIMIT $3
                "
            );
            sqlx::query(&query)
                .bind(title)
                .bind(&c.id)
                .bind(fetch_limit)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::database(format!("Failed to query coaches (title): {e}")))
        } else {
            let query = format!(
                r"
                SELECT id, user_id, tenant_id, title, description, system_prompt,
                       category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                       last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                       publish_status, published_at, review_submitted_at, review_decision_at,
                       review_decision_by, rejection_reason, install_count, icon_url, author_id
                FROM coaches
                WHERE publish_status = 'published' {category_filter}
                ORDER BY title ASC, id ASC
                LIMIT $1
                "
            );
            sqlx::query(&query)
                .bind(fetch_limit)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to query coaches (title first): {e}"))
                })
        }
    }

    /// Get category counts in a single efficient query
    ///
    /// Returns a map of category to count for published coaches.
    /// This replaces 7 separate queries with 1 GROUP BY query.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_category_counts(&self) -> AppResult<HashMap<CoachCategory, i64>> {
        let rows = sqlx::query(
            r"
            SELECT category, COUNT(*) as count
            FROM coaches
            WHERE publish_status = 'published'
            GROUP BY category
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get category counts: {e}")))?;

        let mut counts = HashMap::new();
        for row in rows {
            let category_str: String = row.get("category");
            let count: i64 = row.get("count");
            let category = CoachCategory::parse(&category_str);
            counts.insert(category, count);
        }

        Ok(counts)
    }

    /// Search published coaches in the Store (cross-tenant)
    ///
    /// Searches title, description, and tags of published coaches.
    /// Published coaches are visible to ALL users regardless of tenant.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn search_published_coaches(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> AppResult<Vec<Coach>> {
        let limit_val = i64::from(limit.unwrap_or(20).min(100));
        let search_pattern = format!("%{query}%");

        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE publish_status = 'published'
                AND (title LIKE $1 OR description LIKE $1 OR tags LIKE $1)
            ORDER BY install_count DESC, published_at DESC
            LIMIT $2
            ",
        )
        .bind(&search_pattern)
        .bind(limit_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to search published coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Get a published coach by ID (for Store viewing, cross-tenant)
    ///
    /// Returns a published coach regardless of ownership or tenant.
    /// Used for Store detail page. Published coaches are visible to ALL users.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_published_coach(&self, coach_id: &str) -> AppResult<Option<Coach>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE id = $1 AND publish_status = 'published'
            ",
        )
        .bind(coach_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get published coach: {e}")))?;

        row.map(|r| row_to_coach(&r)).transpose()
    }

    /// Increment install count for a coach
    ///
    /// Called when a user installs a coach from the Store.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn increment_install_count(&self, coach_id: &str) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE coaches SET install_count = install_count + 1
            WHERE id = $1 AND publish_status = 'published'
            ",
        )
        .bind(coach_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to increment install count: {e}")))?;

        Ok(())
    }

    /// Decrement install count for a coach
    ///
    /// Called when a user uninstalls a coach.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn decrement_install_count(&self, coach_id: &str) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE coaches SET install_count = MAX(0, install_count - 1)
            WHERE id = $1
            ",
        )
        .bind(coach_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to decrement install count: {e}")))?;

        Ok(())
    }

    /// Install a coach from the Store
    ///
    /// Creates a personal copy of a published coach for the user.
    /// Increments the source coach's install count.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Source coach is not found or not published
    /// - User has already installed this coach
    /// - Database operation fails
    pub async fn install_from_store(
        &self,
        source_coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<Coach> {
        // Get the source coach (must be published, cross-tenant lookup)
        let source = self
            .get_published_coach(source_coach_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Published coach {source_coach_id}")))?;

        // Check if user already has this coach installed
        self.check_not_already_installed(user_id, tenant_id, source_coach_id, &source.title)
            .await?;

        // Create the user's copy
        let id = self
            .create_installed_copy(&source, user_id, tenant_id, source_coach_id)
            .await?;

        // Increment install count on the source coach
        self.increment_install_count(source_coach_id).await?;

        // Fetch and return the created coach
        self.get(&id.to_string(), user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to fetch installed coach"))
    }

    /// Check if user has already installed a coach
    async fn check_not_already_installed(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        source_coach_id: &str,
        title: &str,
    ) -> AppResult<()> {
        let existing = sqlx::query(
            "SELECT id FROM coaches WHERE user_id = $1 AND tenant_id = $2 AND forked_from = $3",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(source_coach_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to check existing installation: {e}")))?;

        if existing.is_some() {
            return Err(AppError::invalid_input(format!(
                "Coach {title} is already installed"
            )));
        }
        Ok(())
    }

    /// Create a copy of a coach for the user (store installation)
    async fn create_installed_copy(
        &self,
        source: &Coach,
        user_id: Uuid,
        tenant_id: TenantId,
        source_coach_id: &str,
    ) -> AppResult<Uuid> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let tags_json = serde_json::to_string(&source.tags)?;
        let sample_prompts_json = serde_json::to_string(&source.sample_prompts)?;
        let prerequisites_json = serde_json::to_string(&source.prerequisites)?;

        sqlx::query(
            r"
            INSERT INTO coaches (
                id, user_id, tenant_id, title, description, system_prompt, category, tags,
                sample_prompts, token_count, is_favorite, use_count, last_used_at,
                created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, icon_url
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0, 0, NULL, $11, $11, 0, $12, $13, $14, $15, $16)
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(&source.title)
        .bind(&source.description)
        .bind(&source.system_prompt)
        .bind(source.category.as_str())
        .bind(&tags_json)
        .bind(&sample_prompts_json)
        .bind(i64::from(source.token_count))
        .bind(now.to_rfc3339())
        .bind(CoachVisibility::Private.as_str())
        .bind(&prerequisites_json)
        .bind(source_coach_id)
        .bind(PublishStatus::Draft.as_str())
        .bind(&source.icon_url)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to install coach: {e}")))?;

        Ok(id)
    }

    /// Get user's installed coaches from the Store
    ///
    /// Returns coaches where `forked_from` points to a published coach.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_installed_coaches(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<Vec<Coach>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, description, system_prompt,
                   category, tags, sample_prompts, token_count, is_favorite, is_active, use_count,
                   last_used_at, created_at, updated_at, is_system, visibility, prerequisites, forked_from,
                publish_status, published_at, review_submitted_at, review_decision_at,
                review_decision_by, rejection_reason, install_count, icon_url, author_id
            FROM coaches
            WHERE user_id = $1 AND tenant_id = $2 AND forked_from IS NOT NULL
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get installed coaches: {e}")))?;

        rows.iter().map(row_to_coach).collect()
    }

    /// Uninstall a coach (delete user's installed copy)
    ///
    /// Also decrements the source coach's install count.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Coach is not found
    /// - Coach was not installed from Store (no `forked_from`)
    /// - Database operation fails
    pub async fn uninstall_coach(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<String> {
        // Get the coach to verify ownership and get forked_from
        let coach = self
            .get(coach_id, user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        let source_id = coach.forked_from.ok_or_else(|| {
            AppError::invalid_input("This coach was not installed from the Store")
        })?;

        // Delete the user's copy
        sqlx::query(
            r"
            DELETE FROM coaches
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(coach_id)
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to uninstall coach: {e}")))?;

        // Decrement install count on the source coach
        self.decrement_install_count(&source_id).await?;

        Ok(source_id)
    }

    // ============================================
    // Startup Query Methods
    // ============================================

    /// Get the startup query for a coach by matching its system prompt.
    ///
    /// This is used to automatically inject a startup query when a user
    /// starts a conversation with a coach that has one configured.
    /// The `system_prompt` is stored in conversations when a coach is selected.
    ///
    /// # Arguments
    ///
    /// * `system_prompt` - The system prompt to match against coach instructions
    /// * `tenant_id` - The tenant ID to scope the search
    ///
    /// # Returns
    ///
    /// The `startup_query` if found, None otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_startup_query_by_system_prompt(
        &self,
        system_prompt: &str,
        tenant_id: TenantId,
    ) -> AppResult<Option<String>> {
        // First try to find a coach matching the tenant (custom coaches)
        // Then fall back to system coaches (is_system = 1) which are shared across tenants
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r"
            SELECT startup_query
            FROM coaches
            WHERE system_prompt = $1
              AND startup_query IS NOT NULL
              AND (tenant_id = $2 OR is_system = 1)
            LIMIT 1
            ",
        )
        .bind(system_prompt)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get startup query: {e}")))?;

        Ok(row.and_then(|(q,)| q))
    }
}

/// Compute SHA-256 hash of content for version tracking
fn compute_content_hash(content: &serde_json::Value) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Convert a database row to a `CoachVersion` struct
fn row_to_coach_version(row: &SqliteRow) -> AppResult<CoachVersion> {
    let id: String = row.get("id");
    let coach_id: String = row.get("coach_id");
    let version: i32 = row.get("version");
    let content_hash: String = row.get("content_hash");
    let content_snapshot_str: String = row.get("content_snapshot");
    let change_summary: Option<String> = row.get("change_summary");
    let created_at_str: String = row.get("created_at");
    let created_by_str: Option<String> = row.get("created_by");

    let content_snapshot: serde_json::Value = serde_json::from_str(&content_snapshot_str)
        .map_err(|e| AppError::internal(format!("Invalid JSON in version snapshot: {e}")))?;

    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map_err(|e| AppError::internal(format!("Invalid datetime: {e}")))?
        .with_timezone(&Utc);

    let created_by = created_by_str
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| AppError::internal(format!("Invalid UUID: {e}")))?;

    Ok(CoachVersion {
        id,
        coach_id,
        version,
        content_hash,
        content_snapshot,
        change_summary,
        created_at,
        created_by,
    })
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

/// Store admin statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreAdminStats {
    /// Number of coaches pending review
    pub pending_count: u32,
    /// Number of published coaches
    pub published_count: u32,
    /// Number of rejected coaches
    pub rejected_count: u32,
    /// Total installs across all published coaches
    pub total_installs: u32,
    /// Rejection rate as percentage
    pub rejection_rate: f64,
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

/// A snapshot of a coach at a specific version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachVersion {
    /// Unique identifier for this version
    pub id: String,
    /// Reference to the coach
    pub coach_id: String,
    /// Version number (incremented on each update)
    pub version: i32,
    /// Content hash at this version (SHA-256 of serialized content)
    pub content_hash: String,
    /// Full content snapshot as JSON
    pub content_snapshot: serde_json::Value,
    /// Summary of what changed in this version
    pub change_summary: Option<String>,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// User who created this version
    pub created_by: Option<Uuid>,
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

    // Fields with defaults when columns are null or missing
    let is_system: i64 = row.try_get("is_system").unwrap_or(0);
    let visibility_str: String = row
        .try_get("visibility")
        .unwrap_or_else(|_| "private".to_owned());
    let sample_prompts_json: String = row
        .try_get("sample_prompts")
        .unwrap_or_else(|_| "[]".to_owned());
    let prerequisites_json: String = row
        .try_get("prerequisites")
        .unwrap_or_else(|_| "{}".to_owned());
    let forked_from: Option<String> = row.try_get("forked_from").ok();

    let tags: Vec<String> = serde_json::from_str(&tags_json)?;
    let sample_prompts: Vec<String> = serde_json::from_str(&sample_prompts_json)?;
    let prerequisites: CoachPrerequisites =
        serde_json::from_str(&prerequisites_json).unwrap_or_default();

    // Store-related fields with defaults when columns are null or missing
    let publish_status_str: String = row
        .try_get("publish_status")
        .unwrap_or_else(|_| "draft".to_owned());
    let published_at_str: Option<String> = row.try_get("published_at").ok().flatten();
    let review_submitted_at_str: Option<String> = row.try_get("review_submitted_at").ok().flatten();
    let review_decision_at_str: Option<String> = row.try_get("review_decision_at").ok().flatten();
    let review_decision_by: Option<String> = row.try_get("review_decision_by").ok().flatten();
    let rejection_reason: Option<String> = row.try_get("rejection_reason").ok().flatten();
    let install_count: i64 = row.try_get("install_count").unwrap_or(0);
    let icon_url: Option<String> = row.try_get("icon_url").ok().flatten();
    let author_id: Option<String> = row.try_get("author_id").ok().flatten();

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
        prerequisites,
        forked_from,
        publish_status: PublishStatus::parse(&publish_status_str),
        published_at: published_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
        review_submitted_at: review_submitted_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
        review_decision_at: review_decision_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
        review_decision_by,
        rejection_reason,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        install_count: install_count as u32,
        icon_url,
        author_id,
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
