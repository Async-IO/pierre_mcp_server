// ABOUTME: Route handlers for prompt suggestions API
// ABOUTME: Provides REST endpoints for fetching and managing AI chat prompts
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Prompt suggestions routes
//!
//! This module handles prompt suggestion endpoints for the AI chat interface.
//! Public endpoints require JWT authentication to identify the tenant.
//! Admin endpoints require admin role for CRUD operations.

use crate::{
    auth::AuthResult,
    database::{
        prompts::PromptManager, CreatePromptCategoryRequest, Pillar, PromptCategoryResponse,
        UpdatePromptCategoryRequest,
    },
    database_plugins::DatabaseProvider,
    errors::{AppError, ErrorCode},
    mcp::resources::ServerResources,
    security::cookies::get_cookie_value,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

/// Response for the public prompts endpoint
#[derive(Debug, Serialize)]
pub struct PromptsResponse {
    /// Prompt categories
    pub categories: Vec<PromptCategoryResponse>,
    /// Welcome prompt for first-time connected users
    pub welcome_prompt: String,
    /// Metadata about the response
    pub metadata: PromptsMetadata,
}

/// Metadata for the prompts response
#[derive(Debug, Serialize)]
pub struct PromptsMetadata {
    /// Timestamp of the response
    pub timestamp: String,
    /// API version
    pub api_version: String,
}

/// Response for admin prompt category operations
#[derive(Debug, Serialize)]
pub struct AdminPromptCategoryResponse {
    /// Category ID
    pub id: String,
    /// Unique key for this category
    pub category_key: String,
    /// Display title
    pub category_title: String,
    /// Emoji icon
    pub category_icon: String,
    /// Visual pillar classification
    pub pillar: String,
    /// List of prompts
    pub prompts: Vec<String>,
    /// Display order
    pub display_order: i32,
    /// Whether this category is active
    pub is_active: bool,
}

/// Response for admin welcome prompt operations
#[derive(Debug, Serialize)]
pub struct AdminWelcomePromptResponse {
    /// Welcome prompt text
    pub prompt_text: String,
}

/// Response for admin system prompt operations
#[derive(Debug, Serialize)]
pub struct AdminSystemPromptResponse {
    /// System prompt text (markdown format)
    pub prompt_text: String,
}

/// Request to update the welcome prompt
#[derive(Debug, serde::Deserialize)]
pub struct UpdateWelcomePromptRequest {
    /// New welcome prompt text
    pub prompt_text: String,
}

/// Request to update the system prompt
#[derive(Debug, serde::Deserialize)]
pub struct UpdateSystemPromptRequest {
    /// New system prompt text (markdown format)
    pub prompt_text: String,
}

/// Prompt routes handler
pub struct PromptRoutes;

impl PromptRoutes {
    /// Create all prompt routes (public endpoints)
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route(
                "/api/prompts/suggestions",
                get(Self::handle_get_suggestions),
            )
            .with_state(resources)
    }

    /// Create admin prompt routes
    pub fn admin_routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/prompts", get(Self::handle_admin_list_categories))
            .route("/prompts", post(Self::handle_admin_create_category))
            .route("/prompts/:id", get(Self::handle_admin_get_category))
            .route("/prompts/:id", put(Self::handle_admin_update_category))
            .route("/prompts/:id", delete(Self::handle_admin_delete_category))
            .route("/prompts/welcome", get(Self::handle_admin_get_welcome))
            .route("/prompts/welcome", put(Self::handle_admin_update_welcome))
            .route("/prompts/system", get(Self::handle_admin_get_system))
            .route("/prompts/system", put(Self::handle_admin_update_system))
            .route("/prompts/reset", post(Self::handle_admin_reset_defaults))
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        // Try Authorization header first, then fall back to auth_token cookie
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(headers, "auth_token") {
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Get tenant ID for an authenticated user
    async fn get_user_tenant(
        resources: &Arc<ServerResources>,
        user_id: Uuid,
    ) -> Result<String, AppError> {
        let user = resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user {user_id}: {e}")))?
            .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

        user.tenant_id.ok_or_else(|| {
            AppError::invalid_input(format!("User {user_id} has no tenant assigned"))
        })
    }

    /// Get prompt manager from the `SQLite` pool
    fn get_prompt_manager(resources: &Arc<ServerResources>) -> Result<PromptManager, AppError> {
        // Get the SQLite pool from the database factory
        let pool = resources
            .database
            .sqlite_pool()
            .ok_or_else(|| AppError::internal("SQLite database required for prompts"))?;
        Ok(PromptManager::new(pool.clone()))
    }

    /// Ensure the user has admin role
    async fn require_admin(
        resources: &Arc<ServerResources>,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let user = resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if !user.role.is_admin_or_higher() {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Admin privileges required",
            ));
        }
        Ok(())
    }

    /// Convert pillar enum to string
    fn pillar_to_string(pillar: Pillar) -> String {
        match pillar {
            Pillar::Activity => "activity".to_owned(),
            Pillar::Nutrition => "nutrition".to_owned(),
            Pillar::Recovery => "recovery".to_owned(),
        }
    }

    /// Handle GET /api/prompts/suggestions - Get prompt categories and welcome prompt
    async fn handle_get_suggestions(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;

        // Fetch categories and welcome prompt
        let categories = prompt_manager.get_prompt_categories(&tenant_id).await?;
        let welcome = prompt_manager.get_welcome_prompt(&tenant_id).await?;

        let response = PromptsResponse {
            categories: categories.into_iter().map(Into::into).collect(),
            welcome_prompt: welcome.prompt_text,
            metadata: PromptsMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                api_version: "1.0".to_owned(),
            },
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/admin/prompts - List all prompt categories (including inactive)
    async fn handle_admin_list_categories(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let categories = prompt_manager.get_all_prompt_categories(&tenant_id).await?;

        let response: Vec<AdminPromptCategoryResponse> = categories
            .into_iter()
            .map(|cat| AdminPromptCategoryResponse {
                id: cat.id.to_string(),
                category_key: cat.category_key,
                category_title: cat.category_title,
                category_icon: cat.category_icon,
                pillar: Self::pillar_to_string(cat.pillar),
                prompts: cat.prompts,
                display_order: cat.display_order,
                is_active: cat.is_active,
            })
            .collect();

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/admin/prompts - Create a new prompt category
    async fn handle_admin_create_category(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<CreatePromptCategoryRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let category = prompt_manager
            .create_prompt_category(&tenant_id, &request)
            .await?;

        let response = AdminPromptCategoryResponse {
            id: category.id.to_string(),
            category_key: category.category_key,
            category_title: category.category_title,
            category_icon: category.category_icon,
            pillar: Self::pillar_to_string(category.pillar),
            prompts: category.prompts,
            display_order: category.display_order,
            is_active: category.is_active,
        };

        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /api/admin/prompts/:id - Get a specific prompt category
    async fn handle_admin_get_category(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let category = prompt_manager.get_prompt_category(&tenant_id, &id).await?;

        let response = AdminPromptCategoryResponse {
            id: category.id.to_string(),
            category_key: category.category_key,
            category_title: category.category_title,
            category_icon: category.category_icon,
            pillar: Self::pillar_to_string(category.pillar),
            prompts: category.prompts,
            display_order: category.display_order,
            is_active: category.is_active,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle PUT /api/admin/prompts/:id - Update a prompt category
    async fn handle_admin_update_category(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(request): Json<UpdatePromptCategoryRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let category = prompt_manager
            .update_prompt_category(&tenant_id, &id, &request)
            .await?;

        let response = AdminPromptCategoryResponse {
            id: category.id.to_string(),
            category_key: category.category_key,
            category_title: category.category_title,
            category_icon: category.category_icon,
            pillar: Self::pillar_to_string(category.pillar),
            prompts: category.prompts,
            display_order: category.display_order,
            is_active: category.is_active,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle DELETE /api/admin/prompts/:id - Delete a prompt category
    async fn handle_admin_delete_category(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        prompt_manager
            .delete_prompt_category(&tenant_id, &id)
            .await?;

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    /// Handle GET /api/admin/prompts/welcome - Get the welcome prompt
    async fn handle_admin_get_welcome(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let welcome = prompt_manager.get_welcome_prompt(&tenant_id).await?;

        let response = AdminWelcomePromptResponse {
            prompt_text: welcome.prompt_text,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle PUT /api/admin/prompts/welcome - Update the welcome prompt
    async fn handle_admin_update_welcome(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<UpdateWelcomePromptRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let welcome = prompt_manager
            .update_welcome_prompt(&tenant_id, &request.prompt_text)
            .await?;

        let response = AdminWelcomePromptResponse {
            prompt_text: welcome.prompt_text,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/admin/prompts/system - Get the system prompt
    async fn handle_admin_get_system(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let system = prompt_manager.get_system_prompt(&tenant_id).await?;

        let response = AdminSystemPromptResponse {
            prompt_text: system.prompt_text,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle PUT /api/admin/prompts/system - Update the system prompt
    async fn handle_admin_update_system(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<UpdateSystemPromptRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        let system = prompt_manager
            .update_system_prompt(&tenant_id, &request.prompt_text)
            .await?;

        let response = AdminSystemPromptResponse {
            prompt_text: system.prompt_text,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/admin/prompts/reset - Reset prompts to defaults
    async fn handle_admin_reset_defaults(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let prompt_manager = Self::get_prompt_manager(&resources)?;
        prompt_manager.reset_to_defaults(&tenant_id).await?;

        Ok((StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response())
    }
}
