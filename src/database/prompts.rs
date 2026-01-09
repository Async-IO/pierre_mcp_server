// ABOUTME: Database operations for prompt suggestions with tenant isolation
// ABOUTME: Handles CRUD operations for AI chat prompts and welcome messages
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::{AppError, AppResult, ErrorCode};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

/// Pillar types for visual categorization of prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pillar {
    /// Activity pillar (Emerald gradient)
    Activity,
    /// Nutrition pillar (Amber gradient)
    Nutrition,
    /// Recovery pillar (Indigo gradient)
    Recovery,
}

impl Pillar {
    /// Convert from database string representation
    fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "activity" => Ok(Self::Activity),
            "nutrition" => Ok(Self::Nutrition),
            "recovery" => Ok(Self::Recovery),
            other => Err(AppError::invalid_input(format!("Invalid pillar: {other}"))),
        }
    }

    /// Convert to database string representation
    const fn as_str(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::Nutrition => "nutrition",
            Self::Recovery => "recovery",
        }
    }
}

/// A prompt suggestion category with its prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCategory {
    /// Unique ID
    pub id: Uuid,
    /// Tenant this category belongs to
    pub tenant_id: String,
    /// Unique key for this category within the tenant (e.g., "training", "nutrition")
    pub category_key: String,
    /// Display title for the category
    pub category_title: String,
    /// Emoji icon for the category
    pub category_icon: String,
    /// Visual pillar classification
    pub pillar: Pillar,
    /// List of prompt suggestions in this category
    pub prompts: Vec<String>,
    /// Display order (lower numbers shown first)
    pub display_order: i32,
    /// Whether this category is active
    pub is_active: bool,
}

/// API response format for prompt categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCategoryResponse {
    /// Unique key for this category
    pub category_key: String,
    /// Display title
    pub category_title: String,
    /// Emoji icon
    pub category_icon: String,
    /// Visual pillar classification
    pub pillar: Pillar,
    /// List of prompts
    pub prompts: Vec<String>,
}

impl From<PromptCategory> for PromptCategoryResponse {
    fn from(cat: PromptCategory) -> Self {
        Self {
            category_key: cat.category_key,
            category_title: cat.category_title,
            category_icon: cat.category_icon,
            pillar: cat.pillar,
            prompts: cat.prompts,
        }
    }
}

/// A welcome prompt for first-time connected users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomePrompt {
    /// Unique ID
    pub id: Uuid,
    /// Tenant this prompt belongs to
    pub tenant_id: String,
    /// The welcome prompt text
    pub prompt_text: String,
    /// Whether this prompt is active
    pub is_active: bool,
}

/// The system prompt (instructions) for the LLM assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPrompt {
    /// Unique ID
    pub id: Uuid,
    /// Tenant this prompt belongs to
    pub tenant_id: String,
    /// The system prompt text (markdown format)
    pub prompt_text: String,
    /// Whether this prompt is active
    pub is_active: bool,
}

/// Request to create a new prompt category
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePromptCategoryRequest {
    /// Unique key for this category
    pub category_key: String,
    /// Display title
    pub category_title: String,
    /// Emoji icon
    pub category_icon: String,
    /// Visual pillar classification
    pub pillar: Pillar,
    /// List of prompts
    pub prompts: Vec<String>,
    /// Display order (optional, defaults to 0)
    pub display_order: Option<i32>,
}

/// Request to update an existing prompt category
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePromptCategoryRequest {
    /// Display title (optional)
    pub category_title: Option<String>,
    /// Emoji icon (optional)
    pub category_icon: Option<String>,
    /// Visual pillar classification (optional)
    pub pillar: Option<Pillar>,
    /// List of prompts (optional)
    pub prompts: Option<Vec<String>>,
    /// Display order (optional)
    pub display_order: Option<i32>,
    /// Whether this category is active (optional)
    pub is_active: Option<bool>,
}

/// Default prompt categories JSON loaded from file (single source of truth)
const DEFAULT_PROMPT_CATEGORIES_JSON: &str = include_str!("../llm/prompts/prompt_categories.json");

/// Parsed prompt category from JSON
#[derive(Debug, Clone, Deserialize)]
struct DefaultPromptCategory {
    key: String,
    title: String,
    icon: String,
    pillar: String,
    prompts: Vec<String>,
}

/// Parse default prompt categories from JSON file
fn parse_default_prompt_categories() -> Vec<DefaultPromptCategory> {
    serde_json::from_str(DEFAULT_PROMPT_CATEGORIES_JSON).unwrap_or_else(|e| {
        tracing::error!("Failed to parse default prompt categories JSON: {e}");
        Vec::new()
    })
}

/// Default welcome prompt loaded from file (single source of truth)
pub const DEFAULT_WELCOME_PROMPT: &str = include_str!("../llm/prompts/welcome_prompt.md");

/// Default system prompt for the LLM assistant
/// This provides instructions for the AI's role, communication style, and available tools
pub const DEFAULT_SYSTEM_PROMPT: &str = include_str!("../llm/prompts/pierre_system.md");

/// Prompt suggestions database operations manager
pub struct PromptManager {
    pool: SqlitePool,
}

impl PromptManager {
    /// Create a new prompt manager
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get all active prompt categories for a tenant
    ///
    /// If no categories exist, seeds the default categories first.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_prompt_categories(&self, tenant_id: &str) -> AppResult<Vec<PromptCategory>> {
        let categories = self.fetch_prompt_categories(tenant_id).await?;

        // If no categories exist, seed defaults
        if categories.is_empty() {
            self.seed_default_prompts(tenant_id).await?;
            return self.fetch_prompt_categories(tenant_id).await;
        }

        Ok(categories)
    }

    /// Fetch prompt categories from database (internal helper)
    async fn fetch_prompt_categories(&self, tenant_id: &str) -> AppResult<Vec<PromptCategory>> {
        let rows = sqlx::query(
            r"
            SELECT id, tenant_id, category_key, category_title, category_icon,
                   pillar, prompts, display_order, is_active
            FROM prompt_suggestions
            WHERE tenant_id = $1 AND is_active = 1
            ORDER BY display_order ASC, category_key ASC
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch prompt categories: {e}")))?;

        let mut categories = Vec::with_capacity(rows.len());
        for row in rows {
            categories.push(Self::row_to_prompt_category(&row)?);
        }

        Ok(categories)
    }

    /// Get all prompt categories for a tenant (including inactive) - for admin
    ///
    /// If no categories exist, seeds the default categories first.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_all_prompt_categories(
        &self,
        tenant_id: &str,
    ) -> AppResult<Vec<PromptCategory>> {
        let categories = self.fetch_all_prompt_categories(tenant_id).await?;

        // If no categories exist, seed defaults
        if categories.is_empty() {
            self.seed_default_prompts(tenant_id).await?;
            return self.fetch_all_prompt_categories(tenant_id).await;
        }

        Ok(categories)
    }

    /// Fetch all prompt categories from database (internal helper)
    async fn fetch_all_prompt_categories(&self, tenant_id: &str) -> AppResult<Vec<PromptCategory>> {
        let rows = sqlx::query(
            r"
            SELECT id, tenant_id, category_key, category_title, category_icon,
                   pillar, prompts, display_order, is_active
            FROM prompt_suggestions
            WHERE tenant_id = $1
            ORDER BY display_order ASC, category_key ASC
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch prompt categories: {e}")))?;

        let mut categories = Vec::with_capacity(rows.len());
        for row in rows {
            categories.push(Self::row_to_prompt_category(&row)?);
        }

        Ok(categories)
    }

    /// Get a single prompt category by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or category not found
    pub async fn get_prompt_category(
        &self,
        tenant_id: &str,
        category_id: &str,
    ) -> AppResult<PromptCategory> {
        let row = sqlx::query(
            r"
            SELECT id, tenant_id, category_key, category_title, category_icon,
                   pillar, prompts, display_order, is_active
            FROM prompt_suggestions
            WHERE tenant_id = $1 AND id = $2
            ",
        )
        .bind(tenant_id)
        .bind(category_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch prompt category: {e}")))?
        .ok_or_else(|| AppError::not_found(format!("Prompt category {category_id}")))?;

        Self::row_to_prompt_category(&row)
    }

    /// Create a new prompt category
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or `category_key` already exists
    pub async fn create_prompt_category(
        &self,
        tenant_id: &str,
        request: &CreatePromptCategoryRequest,
    ) -> AppResult<PromptCategory> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let prompts_json = serde_json::to_string(&request.prompts)?;
        let display_order = request.display_order.unwrap_or(0);

        sqlx::query(
            r"
            INSERT INTO prompt_suggestions (
                id, tenant_id, category_key, category_title, category_icon,
                pillar, prompts, display_order, is_active, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, $9, $9)
            ",
        )
        .bind(id.to_string())
        .bind(tenant_id)
        .bind(&request.category_key)
        .bind(&request.category_title)
        .bind(&request.category_icon)
        .bind(request.pillar.as_str())
        .bind(&prompts_json)
        .bind(display_order)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                AppError::new(
                    ErrorCode::ResourceAlreadyExists,
                    format!(
                        "Category with key '{}' already exists",
                        request.category_key
                    ),
                )
            } else {
                AppError::database(format!("Failed to create prompt category: {e}"))
            }
        })?;

        Ok(PromptCategory {
            id,
            tenant_id: tenant_id.to_owned(),
            category_key: request.category_key.clone(),
            category_title: request.category_title.clone(),
            category_icon: request.category_icon.clone(),
            pillar: request.pillar,
            prompts: request.prompts.clone(),
            display_order,
            is_active: true,
        })
    }

    /// Update an existing prompt category
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or category not found
    pub async fn update_prompt_category(
        &self,
        tenant_id: &str,
        category_id: &str,
        request: &UpdatePromptCategoryRequest,
    ) -> AppResult<PromptCategory> {
        // First fetch existing category to apply partial updates
        let mut category = self.get_prompt_category(tenant_id, category_id).await?;

        // Apply updates
        if let Some(title) = &request.category_title {
            category.category_title.clone_from(title);
        }
        if let Some(icon) = &request.category_icon {
            category.category_icon.clone_from(icon);
        }
        if let Some(pillar) = request.pillar {
            category.pillar = pillar;
        }
        if let Some(prompts) = &request.prompts {
            category.prompts.clone_from(prompts);
        }
        if let Some(order) = request.display_order {
            category.display_order = order;
        }
        if let Some(active) = request.is_active {
            category.is_active = active;
        }

        let now = Utc::now().to_rfc3339();
        let prompts_json = serde_json::to_string(&category.prompts)?;

        sqlx::query(
            r"
            UPDATE prompt_suggestions
            SET category_title = $1, category_icon = $2, pillar = $3,
                prompts = $4, display_order = $5, is_active = $6, updated_at = $7
            WHERE tenant_id = $8 AND id = $9
            ",
        )
        .bind(&category.category_title)
        .bind(&category.category_icon)
        .bind(category.pillar.as_str())
        .bind(&prompts_json)
        .bind(category.display_order)
        .bind(category.is_active)
        .bind(&now)
        .bind(tenant_id)
        .bind(category_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update prompt category: {e}")))?;

        Ok(category)
    }

    /// Delete a prompt category
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn delete_prompt_category(
        &self,
        tenant_id: &str,
        category_id: &str,
    ) -> AppResult<()> {
        let result = sqlx::query(
            r"
            DELETE FROM prompt_suggestions
            WHERE tenant_id = $1 AND id = $2
            ",
        )
        .bind(tenant_id)
        .bind(category_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete prompt category: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!(
                "Prompt category {category_id}"
            )));
        }

        Ok(())
    }

    /// Get the welcome prompt for a tenant
    ///
    /// If no welcome prompt exists, seeds the default first.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_welcome_prompt(&self, tenant_id: &str) -> AppResult<WelcomePrompt> {
        let row = sqlx::query(
            r"
            SELECT id, tenant_id, prompt_text, is_active
            FROM welcome_prompts
            WHERE tenant_id = $1 AND is_active = 1
            ",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch welcome prompt: {e}")))?;

        if let Some(row) = row {
            return Self::row_to_welcome_prompt(&row);
        }

        // Seed default and return it
        self.seed_welcome_prompt(tenant_id).await?;
        let row = sqlx::query(
            r"
            SELECT id, tenant_id, prompt_text, is_active
            FROM welcome_prompts
            WHERE tenant_id = $1 AND is_active = 1
            ",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch seeded welcome prompt: {e}")))?;
        Self::row_to_welcome_prompt(&row)
    }

    /// Update the welcome prompt for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_welcome_prompt(
        &self,
        tenant_id: &str,
        prompt_text: &str,
    ) -> AppResult<WelcomePrompt> {
        let now = Utc::now().to_rfc3339();

        // Try to update existing, or insert if not exists
        let result = sqlx::query(
            r"
            UPDATE welcome_prompts
            SET prompt_text = $1, updated_at = $2
            WHERE tenant_id = $3
            ",
        )
        .bind(prompt_text)
        .bind(&now)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update welcome prompt: {e}")))?;

        if result.rows_affected() == 0 {
            // Insert new
            let id = Uuid::new_v4();
            sqlx::query(
                r"
                INSERT INTO welcome_prompts (id, tenant_id, prompt_text, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, 1, $4, $4)
                ",
            )
            .bind(id.to_string())
            .bind(tenant_id)
            .bind(prompt_text)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create welcome prompt: {e}")))?;

            return Ok(WelcomePrompt {
                id,
                tenant_id: tenant_id.to_owned(),
                prompt_text: prompt_text.to_owned(),
                is_active: true,
            });
        }

        self.get_welcome_prompt(tenant_id).await
    }

    /// Get the system prompt for a tenant
    ///
    /// If no system prompt exists, seeds the default first.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_system_prompt(&self, tenant_id: &str) -> AppResult<SystemPrompt> {
        let row = sqlx::query(
            r"
            SELECT id, tenant_id, prompt_text, is_active
            FROM system_prompts
            WHERE tenant_id = $1 AND is_active = 1
            ",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch system prompt: {e}")))?;

        if let Some(row) = row {
            return Self::row_to_system_prompt(&row);
        }

        // Seed default and return it
        self.seed_system_prompt(tenant_id).await?;
        let row = sqlx::query(
            r"
            SELECT id, tenant_id, prompt_text, is_active
            FROM system_prompts
            WHERE tenant_id = $1 AND is_active = 1
            ",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch seeded system prompt: {e}")))?;
        Self::row_to_system_prompt(&row)
    }

    /// Update the system prompt for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_system_prompt(
        &self,
        tenant_id: &str,
        prompt_text: &str,
    ) -> AppResult<SystemPrompt> {
        let now = Utc::now().to_rfc3339();

        // Try to update existing, or insert if not exists
        let result = sqlx::query(
            r"
            UPDATE system_prompts
            SET prompt_text = $1, updated_at = $2
            WHERE tenant_id = $3
            ",
        )
        .bind(prompt_text)
        .bind(&now)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update system prompt: {e}")))?;

        if result.rows_affected() == 0 {
            // Insert new
            let id = Uuid::new_v4();
            sqlx::query(
                r"
                INSERT INTO system_prompts (id, tenant_id, prompt_text, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, 1, $4, $4)
                ",
            )
            .bind(id.to_string())
            .bind(tenant_id)
            .bind(prompt_text)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create system prompt: {e}")))?;

            return Ok(SystemPrompt {
                id,
                tenant_id: tenant_id.to_owned(),
                prompt_text: prompt_text.to_owned(),
                is_active: true,
            });
        }

        self.get_system_prompt(tenant_id).await
    }

    /// Reset all prompts to defaults for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn reset_to_defaults(&self, tenant_id: &str) -> AppResult<()> {
        // Delete all existing categories, welcome prompt, and system prompt
        sqlx::query("DELETE FROM prompt_suggestions WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to delete prompt categories: {e}")))?;

        sqlx::query("DELETE FROM welcome_prompts WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to delete welcome prompt: {e}")))?;

        sqlx::query("DELETE FROM system_prompts WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to delete system prompt: {e}")))?;

        // Re-seed defaults
        self.seed_default_prompts(tenant_id).await?;
        self.seed_welcome_prompt(tenant_id).await?;
        self.seed_system_prompt(tenant_id).await?;

        Ok(())
    }

    /// Seed default prompt categories for a tenant
    ///
    /// Categories are loaded from `src/llm/prompts/prompt_categories.json`
    async fn seed_default_prompts(&self, tenant_id: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        let categories = parse_default_prompt_categories();

        for (order, category) in categories.iter().enumerate() {
            let id = Uuid::new_v4();
            let prompts_json = serde_json::to_string(&category.prompts)?;

            // Map icon names to emoji
            let icon_emoji = match category.icon.as_str() {
                "runner" => "🏃",
                "salad" => "🥗",
                "sleep" => "😴",
                "cooking" => "🍳",
                other => other, // Allow direct emoji in JSON
            };

            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let order_i32 = order as i32;

            sqlx::query(
                r"
                INSERT INTO prompt_suggestions (
                    id, tenant_id, category_key, category_title, category_icon,
                    pillar, prompts, display_order, is_active, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, $9, $9)
                ON CONFLICT(tenant_id, category_key) DO NOTHING
                ",
            )
            .bind(id.to_string())
            .bind(tenant_id)
            .bind(&category.key)
            .bind(&category.title)
            .bind(icon_emoji)
            .bind(&category.pillar)
            .bind(&prompts_json)
            .bind(order_i32)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to seed prompt category: {e}")))?;
        }

        Ok(())
    }

    /// Seed default welcome prompt for a tenant
    async fn seed_welcome_prompt(&self, tenant_id: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();

        sqlx::query(
            r"
            INSERT INTO welcome_prompts (id, tenant_id, prompt_text, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, 1, $4, $4)
            ON CONFLICT(tenant_id) DO NOTHING
            ",
        )
        .bind(id.to_string())
        .bind(tenant_id)
        .bind(DEFAULT_WELCOME_PROMPT)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to seed welcome prompt: {e}")))?;

        Ok(())
    }

    /// Seed default system prompt for a tenant
    async fn seed_system_prompt(&self, tenant_id: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();

        sqlx::query(
            r"
            INSERT INTO system_prompts (id, tenant_id, prompt_text, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, 1, $4, $4)
            ON CONFLICT(tenant_id) DO NOTHING
            ",
        )
        .bind(id.to_string())
        .bind(tenant_id)
        .bind(DEFAULT_SYSTEM_PROMPT)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to seed system prompt: {e}")))?;

        Ok(())
    }

    /// Convert a database row to a `PromptCategory`
    fn row_to_prompt_category(row: &SqliteRow) -> AppResult<PromptCategory> {
        let id_str: String = row
            .try_get("id")
            .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?;
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?;

        let tenant_id: String = row
            .try_get("tenant_id")
            .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?;

        let category_key: String = row
            .try_get("category_key")
            .map_err(|e| AppError::database(format!("Failed to get category_key: {e}")))?;

        let category_title: String = row
            .try_get("category_title")
            .map_err(|e| AppError::database(format!("Failed to get category_title: {e}")))?;

        let category_icon: String = row
            .try_get("category_icon")
            .map_err(|e| AppError::database(format!("Failed to get category_icon: {e}")))?;

        let pillar_str: String = row
            .try_get("pillar")
            .map_err(|e| AppError::database(format!("Failed to get pillar: {e}")))?;
        let pillar = Pillar::from_str(&pillar_str)?;

        let prompts_json: String = row
            .try_get("prompts")
            .map_err(|e| AppError::database(format!("Failed to get prompts: {e}")))?;
        let prompts: Vec<String> = serde_json::from_str(&prompts_json)?;

        let display_order: i32 = row
            .try_get("display_order")
            .map_err(|e| AppError::database(format!("Failed to get display_order: {e}")))?;

        let is_active: bool = row
            .try_get("is_active")
            .map_err(|e| AppError::database(format!("Failed to get is_active: {e}")))?;

        Ok(PromptCategory {
            id,
            tenant_id,
            category_key,
            category_title,
            category_icon,
            pillar,
            prompts,
            display_order,
            is_active,
        })
    }

    /// Convert a database row to a `WelcomePrompt`
    fn row_to_welcome_prompt(row: &SqliteRow) -> AppResult<WelcomePrompt> {
        let id_str: String = row
            .try_get("id")
            .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?;
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?;

        let tenant_id: String = row
            .try_get("tenant_id")
            .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?;

        let prompt_text: String = row
            .try_get("prompt_text")
            .map_err(|e| AppError::database(format!("Failed to get prompt_text: {e}")))?;

        let is_active: bool = row
            .try_get("is_active")
            .map_err(|e| AppError::database(format!("Failed to get is_active: {e}")))?;

        Ok(WelcomePrompt {
            id,
            tenant_id,
            prompt_text,
            is_active,
        })
    }

    /// Convert a database row to a `SystemPrompt`
    fn row_to_system_prompt(row: &SqliteRow) -> AppResult<SystemPrompt> {
        let id_str: String = row
            .try_get("id")
            .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?;
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?;

        let tenant_id: String = row
            .try_get("tenant_id")
            .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?;

        let prompt_text: String = row
            .try_get("prompt_text")
            .map_err(|e| AppError::database(format!("Failed to get prompt_text: {e}")))?;

        let is_active: bool = row
            .try_get("is_active")
            .map_err(|e| AppError::database(format!("Failed to get is_active: {e}")))?;

        Ok(SystemPrompt {
            id,
            tenant_id,
            prompt_text,
            is_active,
        })
    }
}
