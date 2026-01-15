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
        Coach, CoachAssignment as DbCoachAssignment, CoachCategory, CoachListItem, CoachVisibility,
        CoachesManager, CreateCoachRequest, CreateSystemCoachRequest as DbCreateSystemCoachRequest,
        ListCoachesFilter, UpdateCoachRequest,
    },
    database_plugins::DatabaseProvider,
    errors::{AppError, ErrorCode},
    mcp::resources::ServerResources,
    permissions::UserRole,
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
    /// Whether this is a system coach (admin-created)
    pub is_system: bool,
    /// Visibility level
    pub visibility: String,
    /// Whether this coach is assigned to the current user
    pub is_assigned: bool,
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
            is_system: coach.is_system,
            visibility: coach.visibility.as_str().to_owned(),
            is_assigned: false, // Default for single coach responses
        }
    }
}

impl From<CoachListItem> for CoachResponse {
    fn from(item: CoachListItem) -> Self {
        Self {
            id: item.coach.id.to_string(),
            title: item.coach.title,
            description: item.coach.description,
            system_prompt: item.coach.system_prompt,
            category: item.coach.category.as_str().to_owned(),
            tags: item.coach.tags,
            token_count: item.coach.token_count,
            is_favorite: item.coach.is_favorite,
            use_count: item.coach.use_count,
            last_used_at: item.coach.last_used_at.map(|dt| dt.to_rfc3339()),
            created_at: item.coach.created_at.to_rfc3339(),
            updated_at: item.coach.updated_at.to_rfc3339(),
            is_system: item.coach.is_system,
            visibility: item.coach.visibility.as_str().to_owned(),
            is_assigned: item.is_assigned,
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
    /// Include system coaches (default: true)
    pub include_system: Option<bool>,
    /// Include hidden coaches (default: false)
    pub include_hidden: Option<bool>,
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

/// Response for hide/show coach operations
#[derive(Debug, Serialize, Deserialize)]
pub struct HideCoachResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Whether the coach is now hidden (true) or visible (false)
    pub is_hidden: bool,
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
    /// Sample prompts for quick-start suggestions
    #[serde(default)]
    pub sample_prompts: Vec<String>,
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
            sample_prompts: body.sample_prompts,
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
    /// New sample prompts (if provided)
    pub sample_prompts: Option<Vec<String>>,
}

impl From<UpdateCoachBody> for UpdateCoachRequest {
    fn from(body: UpdateCoachBody) -> Self {
        Self {
            title: body.title,
            description: body.description,
            system_prompt: body.system_prompt,
            category: body.category.map(|c| CoachCategory::parse(&c)),
            tags: body.tags,
            sample_prompts: body.sample_prompts,
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
            .route("/api/coaches/hidden", get(Self::handle_list_hidden))
            .route("/api/coaches/:id", get(Self::handle_get))
            .route("/api/coaches/:id", put(Self::handle_update))
            .route("/api/coaches/:id", delete(Self::handle_delete))
            .route(
                "/api/coaches/:id/favorite",
                post(Self::handle_toggle_favorite),
            )
            .route("/api/coaches/:id/usage", post(Self::handle_record_usage))
            .route("/api/coaches/:id/hide", post(Self::handle_hide_coach))
            .route("/api/coaches/:id/hide", delete(Self::handle_show_coach))
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
            include_system: query.include_system.unwrap_or(true),
            include_hidden: query.include_hidden.unwrap_or(false),
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

    /// Handle POST /api/coaches/:id/hide - Hide a coach from user's view
    async fn handle_hide_coach(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let success = manager.hide_coach(&id, auth.user_id, &tenant_id).await?;

        let response = HideCoachResponse {
            success,
            is_hidden: success,
        };
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle DELETE /api/coaches/:id/hide - Show (unhide) a coach
    async fn handle_show_coach(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let success = manager.show_coach(&id, auth.user_id).await?;

        let response = HideCoachResponse {
            success,
            is_hidden: false,
        };
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/coaches/hidden - List hidden coaches for user
    async fn handle_list_hidden(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager
            .list_hidden_coaches(auth.user_id, &tenant_id)
            .await?;

        let response = ListCoachesResponse {
            total: u32::try_from(coaches.len()).unwrap_or(0),
            coaches: coaches.into_iter().map(Into::into).collect(),
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    // ============================================
    // Admin Routes for System Coaches (ASY-59)
    // ============================================

    /// Create admin routes for system coaches management
    pub fn admin_routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/coaches", get(Self::handle_admin_list))
            .route("/coaches", post(Self::handle_admin_create))
            .route("/coaches/:id", get(Self::handle_admin_get))
            .route("/coaches/:id", put(Self::handle_admin_update))
            .route("/coaches/:id", delete(Self::handle_admin_delete))
            .route("/coaches/:id/assign", post(Self::handle_admin_assign))
            .route("/coaches/:id/assign", delete(Self::handle_admin_unassign))
            .route(
                "/coaches/:id/assignments",
                get(Self::handle_admin_list_assignments),
            )
            .with_state(resources)
    }

    /// Handle GET /admin/coaches - List all system coaches in tenant
    async fn handle_admin_list(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager.list_system_coaches(&tenant_id).await?;

        let response = ListCoachesResponse {
            total: u32::try_from(coaches.len()).unwrap_or(0),
            coaches: coaches.into_iter().map(Into::into).collect(),
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /admin/coaches - Create a system coach
    async fn handle_admin_create(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<AdminCreateCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .create_system_coach(auth.user_id, &tenant_id, &body.into())
            .await?;

        let response: CoachResponse = coach.into();
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /admin/coaches/:id - Get a system coach
    async fn handle_admin_get(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .get_system_coach(&id, &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        let response: CoachResponse = coach.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle PUT /admin/coaches/:id - Update a system coach
    async fn handle_admin_update(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<UpdateCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let request: UpdateCoachRequest = body.into();
        let coach = manager
            .update_system_coach(&id, &tenant_id, &request)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        let response: CoachResponse = coach.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle DELETE /admin/coaches/:id - Delete a system coach
    async fn handle_admin_delete(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let deleted = manager.delete_system_coach(&id, &tenant_id).await?;

        if !deleted {
            return Err(AppError::not_found(format!("System coach {id}")));
        }

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    /// Handle POST /admin/coaches/:id/assign - Assign coach to users
    async fn handle_admin_assign(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<AssignCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Verify the coach exists and is a system coach
        let coach = manager
            .get_system_coach(&id, &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        // Assign to each user
        let mut assigned_count = 0;
        for user_id_str in &body.user_ids {
            let user_id = Uuid::parse_str(user_id_str)
                .map_err(|_| AppError::invalid_input(format!("Invalid user ID: {user_id_str}")))?;
            if manager
                .assign_coach(&coach.id.to_string(), user_id, auth.user_id)
                .await?
            {
                assigned_count += 1;
            }
        }

        let response = AssignCoachResponse {
            coach_id: id,
            assigned_count,
            total_requested: body.user_ids.len(),
        };
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle DELETE /admin/coaches/:id/assign - Remove coach assignment from users
    async fn handle_admin_unassign(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<AssignCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Verify the coach exists
        manager
            .get_system_coach(&id, &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        // Unassign from each user
        let mut removed_count = 0;
        for user_id_str in &body.user_ids {
            let user_id = Uuid::parse_str(user_id_str)
                .map_err(|_| AppError::invalid_input(format!("Invalid user ID: {user_id_str}")))?;
            if manager.unassign_coach(&id, user_id).await? {
                removed_count += 1;
            }
        }

        let response = UnassignCoachResponse {
            coach_id: id,
            removed_count,
            total_requested: body.user_ids.len(),
        };
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /admin/coaches/:id/assignments - List users assigned to a coach
    async fn handle_admin_list_assignments(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        Self::require_admin(&resources, auth.user_id).await?;
        let tenant_id = Self::get_user_tenant(&resources, auth.user_id).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Verify the coach exists
        manager
            .get_system_coach(&id, &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        let db_assignments = manager.list_assignments(&id).await?;
        let assignments: Vec<CoachAssignment> =
            db_assignments.into_iter().map(Into::into).collect();

        let response = ListAssignmentsResponse {
            coach_id: id,
            assignments,
        };
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Check if user has admin role
    async fn require_admin(
        resources: &Arc<ServerResources>,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let user = resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

        // Check if user has admin role
        if !matches!(user.role, UserRole::Admin | UserRole::SuperAdmin) {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Admin role required for this operation",
            ));
        }

        Ok(())
    }
}

// ============================================
// Admin Request/Response Types
// ============================================

/// Request body for creating a system coach
#[derive(Debug, Deserialize)]
pub struct AdminCreateCoachBody {
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
    /// Sample prompts for quick-start suggestions
    #[serde(default)]
    pub sample_prompts: Vec<String>,
    /// Visibility level (tenant or global)
    pub visibility: Option<String>,
}

impl From<AdminCreateCoachBody> for DbCreateSystemCoachRequest {
    fn from(body: AdminCreateCoachBody) -> Self {
        Self {
            title: body.title,
            description: body.description,
            system_prompt: body.system_prompt,
            category: body
                .category
                .map(|c| CoachCategory::parse(&c))
                .unwrap_or_default(),
            tags: body.tags,
            sample_prompts: body.sample_prompts,
            visibility: body
                .visibility
                .map_or(CoachVisibility::Tenant, |v| CoachVisibility::parse(&v)),
        }
    }
}

/// Request body for assigning/unassigning coaches
#[derive(Debug, Deserialize)]
pub struct AssignCoachBody {
    /// User IDs to assign/unassign
    pub user_ids: Vec<String>,
}

/// Response for coach assignment
#[derive(Debug, Serialize)]
pub struct AssignCoachResponse {
    /// Coach ID
    pub coach_id: String,
    /// Number of users successfully assigned
    pub assigned_count: usize,
    /// Total number of users requested
    pub total_requested: usize,
}

/// Response for coach unassignment
#[derive(Debug, Serialize)]
pub struct UnassignCoachResponse {
    /// Coach ID
    pub coach_id: String,
    /// Number of users successfully unassigned
    pub removed_count: usize,
    /// Total number of users requested
    pub total_requested: usize,
}

/// Coach assignment info
#[derive(Debug, Serialize)]
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

impl From<DbCoachAssignment> for CoachAssignment {
    fn from(db: DbCoachAssignment) -> Self {
        Self {
            user_id: db.user_id,
            user_email: db.user_email,
            assigned_at: db.assigned_at,
            assigned_by: db.assigned_by,
        }
    }
}

/// Response for listing assignments
#[derive(Debug, Serialize)]
pub struct ListAssignmentsResponse {
    /// Coach ID
    pub coach_id: String,
    /// List of assignments
    pub assignments: Vec<CoachAssignment>,
}
