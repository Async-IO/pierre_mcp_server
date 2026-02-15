// ABOUTME: Recipe repository implementation for user recipe storage
// ABOUTME: Provides trait-based abstraction for recipe CRUD operations with tenant isolation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::RecipeRepository;
use crate::database::recipes::RecipeManager;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::intelligence::recipes::{MealTiming, Recipe, ValidatedNutrition};
use async_trait::async_trait;
use pierre_core::models::TenantId;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `RecipeRepository`
pub struct RecipeRepositoryImpl {
    db: Database,
}

impl RecipeRepositoryImpl {
    /// Create a new `RecipeRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }

    fn get_manager(&self) -> Option<RecipeManager> {
        self.db
            .sqlite_pool()
            .map(|pool| RecipeManager::new(pool.clone()))
    }
}

#[async_trait]
impl RecipeRepository for RecipeRepositoryImpl {
    async fn create(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        recipe: &Recipe,
    ) -> Result<String, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .create_recipe(user_id, tenant_id, recipe)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_id(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Option<Recipe>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .get_recipe(recipe_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        meal_timing: Option<MealTiming>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Recipe>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .list_recipes(user_id, tenant_id, meal_timing, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        recipe: &Recipe,
    ) -> Result<bool, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .update_recipe(recipe_id, user_id, tenant_id, recipe)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<bool, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .delete_recipe(recipe_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_nutrition_cache(
        &self,
        recipe_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        nutrition: &ValidatedNutrition,
    ) -> Result<bool, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .update_nutrition_cache(recipe_id, user_id, tenant_id, nutrition)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn search(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Recipe>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .search_recipes(user_id, tenant_id, query, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn count(&self, user_id: Uuid, tenant_id: TenantId) -> Result<u32, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Recipe operations require SQLite backend (enable postgresql feature for PostgreSQL)".to_string(),
        })?;

        manager
            .count_recipes(user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
