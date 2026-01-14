// ABOUTME: Route handlers for Coaches REST API (custom AI personas)
// ABOUTME: Provides REST endpoints for CRUD operations on user-created coaches
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Coaches routes
//!
//! This module handles coach endpoints for custom AI personas.
//! All endpoints require JWT authentication to identify the user and tenant.

use crate::{
    auth::AuthResult,
    database::coaches::{
        Coach, CoachCategory, CoachesManager, CreateCoachRequest, ListCoachesFilter,
        UpdateCoachRequest,
    },
    database_plugins::DatabaseProvider,
    errors::AppError,
    mcp::resources::ServerResources,
    security::cookies::get_cookie_value,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Response for a coach
#[derive(Debug, Serialize, Deserialize)]
pub struct CoachResponse {
    /// Unique identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// System prompt that shapes AI responses
    pub system_prompt: String,
    /// Category for organization
    pub category: String,
    /// Tags for filtering
    pub tags: Vec<String>,
    /// Estimated token count
    pub token_count: u32,
    /// Whether marked as favorite
    pub is_favorite: bool,
    /// Number of times used
    pub use_count: u32,
    /// Last time used
    pub last_used_at: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

impl From<Coach> for CoachResponse {
    fn from(coach: Coach) -> Self {
        Self {
            id: coach.id.to_string(),
            title: coach.title,
            description: coach.description,
            system_prompt: coach.system_prompt,
            category: coach.category.as_str().to_owned(),
            tags: coach.tags,
            token_count: coach.token_count,
            is_favorite: coach.is_favorite,
            use_count: coach.use_count,
            last_used_at: coach.last_used_at.map(|dt| dt.to_rfc3339()),
            created_at: coach.created_at.to_rfc3339(),
            updated_at: coach.updated_at.to_rfc3339(),
        }
    }
}

/// Response for listing coaches
#[derive(Debug, Serialize, Deserialize)]
pub struct ListCoachesResponse {
    /// List of coaches
    pub coaches: Vec<CoachResponse>,
    /// Total count of coaches matching the filter
    pub total: u32,
    /// Metadata
    pub metadata: CoachesMetadata,
}

/// Metadata for coaches response
#[derive(Debug, Serialize, Deserialize)]
pub struct CoachesMetadata {
    /// Response timestamp
    pub timestamp: String,
    /// API version
    pub api_version: String,
}

/// Query parameters for listing coaches
#[derive(Debug, Deserialize, Default)]
pub struct ListCoachesQuery {
    /// Filter by category
    pub category: Option<String>,
    /// Filter to favorites only
    pub favorites_only: Option<bool>,
    /// Maximum results to return
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Query parameters for searching coaches
#[derive(Debug, Deserialize)]
pub struct SearchCoachesQuery {
    /// Search query string
    pub q: String,
    /// Maximum results to return
    pub limit: Option<u32>,
}

/// Response for toggle favorite
#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleFavoriteResponse {
    /// New favorite status
    pub is_favorite: bool,
}

/// Response for record usage
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordUsageResponse {
    /// Whether the usage was recorded
    pub success: bool,
}

/// Request body for creating a coach (mirrors `CreateCoachRequest` with serde derives)
#[derive(Debug, Deserialize)]
pub struct CreateCoachBody {
    /// Display title for the coach
    pub title: String,
    /// Optional description explaining the coach's purpose
    pub description: Option<String>,
    /// System prompt that shapes AI responses
    pub system_prompt: String,
    /// Category for organization
    pub category: Option<String>,
    /// Tags for filtering and search
    #[serde(default)]
    pub tags: Vec<String>,
}

impl From<CreateCoachBody> for CreateCoachRequest {
    fn from(body: CreateCoachBody) -> Self {
        Self {
            title: body.title,
            description: body.description,
            system_prompt: body.system_prompt,
            category: body
                .category
                .map(|c| CoachCategory::parse(&c))
                .unwrap_or_default(),
            tags: body.tags,
        }
    }
}

/// Request body for updating a coach
#[derive(Debug, Deserialize)]
pub struct UpdateCoachBody {
    /// New title (if provided)
    pub title: Option<String>,
    /// New description (if provided)
    pub description: Option<String>,
    /// New system prompt (if provided)
    pub system_prompt: Option<String>,
    /// New category (if provided)
    pub category: Option<String>,
    /// New tags (if provided)
    pub tags: Option<Vec<String>>,
}

impl From<UpdateCoachBody> for UpdateCoachRequest {
    fn from(body: UpdateCoachBody) -> Self {
        Self {
            title: body.title,
            description: body.description,
            system_prompt: body.system_prompt,
            category: body.category.map(|c| CoachCategory::parse(&c)),
            tags: body.tags,
        }
    }
}

/// Coaches routes handler
pub struct CoachesRoutes;

impl CoachesRoutes {
    /// Create all coaches routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/api/coaches", get(Self::handle_list))
            .route("/api/coaches", post(Self::handle_create))
            .route("/api/coaches/search", get(Self::handle_search))
            .route("/api/coaches/:id", get(Self::handle_get))
            .route("/api/coaches/:id", put(Self::handle_update))
            .route("/api/coaches/:id", delete(Self::handle_delete))
            .route(
                "/api/coaches/:id/favorite",
                post(Self::handle_toggle_favorite),
            )
            .route("/api/coaches/:id/usage", post(Self::handle_record_usage))
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

    /// Get coaches manager from the `SQLite` pool
    fn get_coaches_manager(resources: &Arc<ServerResources>) -> Result<CoachesManager, AppError> {
        let pool = resources
            .database
            .sqlite_pool()
            .ok_or_else(|| AppError::internal("SQLite database required for coaches"))?;
        Ok(CoachesManager::new(pool.clone()))
    }

    /// Build metadata for responses
    fn build_metadata() -> CoachesMetadata {
        CoachesMetadata {
            timestamp: Utc::now().to_rfc3339(),
            api_version: "1.0".to_owned(),
        }
    }

    /// Handle GET /api/coaches - List coaches for a user
    async fn handle_list(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<ListCoachesQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        let filter = ListCoachesFilter {
            category: query.category.map(|c| CoachCategory::parse(&c)),
            favorites_only: query.favorites_only.unwrap_or(false),
            limit: query.limit,
            offset: query.offset,
        };

        let coaches = manager.list(auth.user_id, &tenant_id, &filter).await?;
        let total = manager.count(auth.user_id, &tenant_id).await?;

        let response = ListCoachesResponse {
            coaches: coaches.into_iter().map(Into::into).collect(),
            total,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/coaches - Create a new coach
    async fn handle_create(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<CreateCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let request: CreateCoachRequest = body.into();
        let coach = manager.create(auth.user_id, &tenant_id, &request).await?;

        let response: CoachResponse = coach.into();
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /api/coaches/search - Search coaches
    async fn handle_search(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<SearchCoachesQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager
            .search(auth.user_id, &tenant_id, &query.q, query.limit)
            .await?;

        let response = ListCoachesResponse {
            total: u32::try_from(coaches.len()).unwrap_or(0),
            coaches: coaches.into_iter().map(Into::into).collect(),
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/coaches/:id - Get a specific coach
    async fn handle_get(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .get(&id, auth.user_id, &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {id}")))?;

        let response: CoachResponse = coach.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle PUT /api/coaches/:id - Update a coach
    async fn handle_update(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<UpdateCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let request: UpdateCoachRequest = body.into();
        let coach = manager
            .update(&id, auth.user_id, &tenant_id, &request)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {id}")))?;

        let response: CoachResponse = coach.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle DELETE /api/coaches/:id - Delete a coach
    async fn handle_delete(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let deleted = manager.delete(&id, auth.user_id, &tenant_id).await?;

        if !deleted {
            return Err(AppError::not_found(format!("Coach {id}")));
        }

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    /// Handle POST /api/coaches/:id/favorite - Toggle favorite status
    async fn handle_toggle_favorite(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let is_favorite = manager
            .toggle_favorite(&id, auth.user_id, &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {id}")))?;

        let response = ToggleFavoriteResponse { is_favorite };
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/coaches/:id/usage - Record coach usage
    async fn handle_record_usage(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let success = manager.record_usage(&id, auth.user_id, &tenant_id).await?;

        if !success {
            return Err(AppError::not_found(format!("Coach {id}")));
        }

        let response = RecordUsageResponse { success };
        Ok((StatusCode::OK, Json(response)).into_response())
    }
}
