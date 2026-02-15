// ABOUTME: Route handlers for Coach Store REST API (browse, search, install coaches)
// ABOUTME: Provides REST endpoints for Store discovery and installation operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Coach Store routes
//!
//! This module handles Store endpoints for discovering and installing coaches.
//! All endpoints require JWT authentication to identify the user and tenant.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::{
    auth::AuthResult,
    database::{Coach, CoachCategory, CoachesManager, PublishStatus},
    database_plugins::DatabaseProvider,
    errors::AppError,
    mcp::resources::ServerResources,
    models::TenantId,
    pagination::StoreSortOrder,
    security::cookies::get_cookie_value,
};

/// Query parameters for browsing published coaches
#[derive(Debug, Deserialize)]
pub struct BrowseCoachesQuery {
    /// Filter by category
    pub category: Option<String>,
    /// Sort by: "newest" (default), "popular", "title"
    pub sort_by: Option<String>,
    /// Maximum number of results (default 20, max 100)
    pub limit: Option<u32>,
    /// Encoded cursor for pagination (replaces offset)
    pub cursor: Option<String>,
}

/// Query parameters for searching coaches
#[derive(Debug, Deserialize)]
pub struct SearchCoachesQuery {
    /// Search query string
    pub q: String,
    /// Maximum number of results (default 20, max 100)
    pub limit: Option<u32>,
}

/// A published coach for the Store API (subset of full Coach)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreCoach {
    /// Unique coach identifier
    pub id: Uuid,
    /// Coach title
    pub title: String,
    /// Coach description
    pub description: Option<String>,
    /// Category for organization
    pub category: CoachCategory,
    /// Tags for discovery
    pub tags: Vec<String>,
    /// Sample prompts showing usage
    pub sample_prompts: Vec<String>,
    /// Token count estimate
    pub token_count: u32,
    /// Number of installations
    pub install_count: u32,
    /// Optional icon URL
    pub icon_url: Option<String>,
    /// When published (ISO 8601 format)
    pub published_at: Option<String>,
    /// Author ID (optional - for author profile linking)
    pub author_id: Option<String>,
}

impl From<Coach> for StoreCoach {
    fn from(coach: Coach) -> Self {
        Self {
            id: coach.id,
            title: coach.title,
            description: coach.description,
            category: coach.category,
            tags: coach.tags,
            sample_prompts: coach.sample_prompts,
            token_count: coach.token_count,
            install_count: coach.install_count,
            icon_url: coach.icon_url,
            published_at: coach.published_at.map(|dt| dt.to_rfc3339()),
            author_id: coach.author_id,
        }
    }
}

/// Full coach details for the Store (includes system prompt)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreCoachDetail {
    /// Basic store coach info
    #[serde(flatten)]
    pub coach: StoreCoach,
    /// System prompt (shown on detail page)
    pub system_prompt: String,
    /// When the coach was created (ISO 8601 format)
    pub created_at: String,
    /// Publish status
    pub publish_status: PublishStatus,
}

impl From<Coach> for StoreCoachDetail {
    fn from(coach: Coach) -> Self {
        Self {
            system_prompt: coach.system_prompt.clone(),
            created_at: coach.created_at.to_rfc3339(),
            publish_status: coach.publish_status,
            coach: coach.into(),
        }
    }
}

/// Category with coach count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCount {
    /// Category identifier
    pub category: CoachCategory,
    /// Human-readable category name
    pub name: String,
    /// Number of published coaches in this category
    pub count: usize,
}

/// Response for browse endpoint with cursor-based pagination
#[derive(Debug, Serialize, Deserialize)]
pub struct BrowseCoachesResponse {
    /// List of coaches
    pub coaches: Vec<StoreCoach>,
    /// Cursor for fetching the next page (null if no more pages)
    pub next_cursor: Option<String>,
    /// Whether there are more items after this page
    pub has_more: bool,
    /// Response metadata
    pub metadata: StoreMetadata,
}

/// Response for search endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchCoachesResponse {
    /// Search results
    pub coaches: Vec<StoreCoach>,
    /// Search query that was used
    pub query: String,
    /// Response metadata
    pub metadata: StoreMetadata,
}

/// Response for categories endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoriesResponse {
    /// Categories with counts
    pub categories: Vec<CategoryCount>,
    /// Response metadata
    pub metadata: StoreMetadata,
}

/// Response metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreMetadata {
    /// Response timestamp
    pub timestamp: String,
    /// API version
    pub api_version: String,
}

/// Store routes configuration
pub struct StoreRoutes;

impl StoreRoutes {
    /// Create the store router
    ///
    /// # Endpoints
    ///
    /// - `GET /api/store/coaches` - Browse published coaches
    /// - `GET /api/store/coaches/:id` - Get coach details by ID
    /// - `GET /api/store/categories` - List categories with counts
    /// - `GET /api/store/search` - Search coaches
    /// - `POST /api/store/coaches/:id/install` - Install a coach (ASY-163)
    /// - `DELETE /api/store/coaches/:id/install` - Uninstall a coach (ASY-163)
    /// - `GET /api/store/installations` - List user's installed coaches (ASY-163)
    pub fn router(resources: &ServerResources) -> Router {
        Router::new()
            .route("/api/store/health", get(store_health))
            .route("/api/store/coaches", get(Self::handle_browse))
            .route("/api/store/coaches/:id", get(Self::handle_get_coach))
            .route(
                "/api/store/coaches/:id/install",
                post(Self::handle_install).delete(Self::handle_uninstall),
            )
            .route("/api/store/categories", get(Self::handle_categories))
            .route("/api/store/search", get(Self::handle_search))
            .route(
                "/api/store/installations",
                get(Self::handle_list_installations),
            )
            .with_state(Arc::new(resources.clone()))
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
    ///
    /// Uses `active_tenant_id` from JWT claims (user's selected tenant) when available,
    /// falling back to the user's first tenant for single-tenant users or tokens without `active_tenant_id`.
    async fn get_user_tenant(
        auth: &AuthResult,
        resources: &Arc<ServerResources>,
    ) -> Result<TenantId, AppError> {
        // Prefer active_tenant_id from JWT claims (user's selected tenant)
        if let Some(tenant_id) = auth.active_tenant_id {
            return Ok(TenantId::from(tenant_id));
        }
        // Fall back to user's first tenant (single-tenant users or tokens without active_tenant_id)
        let tenants = resources
            .database
            .list_tenants_for_user(auth.user_id)
            .await
            .map_err(|e| {
                AppError::database(format!(
                    "Failed to get tenants for user {}: {e}",
                    auth.user_id
                ))
            })?;

        tenants.first().map(|t| t.id).ok_or_else(|| {
            AppError::invalid_input(format!("User {} has no tenant assigned", auth.user_id))
        })
    }

    /// Get coaches manager from the `SQLite` pool
    fn get_coaches_manager(resources: &Arc<ServerResources>) -> Result<CoachesManager, AppError> {
        let pool = resources
            .database
            .sqlite_pool()
            .ok_or_else(|| AppError::internal("SQLite database required for store"))?;
        Ok(CoachesManager::new(pool.clone()))
    }

    /// Build response metadata
    fn build_metadata() -> StoreMetadata {
        StoreMetadata {
            timestamp: Utc::now().to_rfc3339(),
            api_version: "1.0".to_owned(),
        }
    }

    /// Handle GET /api/store/coaches - Browse published coaches with cursor pagination
    async fn handle_browse(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<BrowseCoachesQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        let category = query.category.as_ref().map(|c| CoachCategory::parse(c));
        let sort_by = query
            .sort_by
            .as_deref()
            .map_or(StoreSortOrder::Newest, StoreSortOrder::parse);
        let limit = query.limit.unwrap_or(20).clamp(1, 100);

        // Use cursor-based pagination for efficient infinite scrolling
        let page = manager
            .get_published_coaches_cursor(category, sort_by, limit, query.cursor.as_deref())
            .await?;

        let store_coaches: Vec<StoreCoach> = page.items.into_iter().map(StoreCoach::from).collect();

        info!(
            "User {} browsed store: {} coaches (category={:?}, sort={:?}, has_more={})",
            auth.user_id,
            store_coaches.len(),
            query.category,
            query.sort_by,
            page.has_more
        );

        let response = BrowseCoachesResponse {
            coaches: store_coaches,
            next_cursor: page.next_cursor.map(|c| c.to_string()),
            has_more: page.has_more,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/store/coaches/:id - Get coach details
    async fn handle_get_coach(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(coach_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Parse coach ID to validate format
        Uuid::parse_str(&coach_id)
            .map_err(|_| AppError::invalid_input(format!("Invalid coach ID: {coach_id}")))?;

        // Get the published coach (cross-tenant - any published coach is visible)
        let coach = manager
            .get_published_coach(&coach_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        info!(
            "User {} viewed store coach: {} ({})",
            auth.user_id, coach.title, coach_id
        );

        let detail: StoreCoachDetail = coach.into();
        Ok((StatusCode::OK, Json(detail)).into_response())
    }

    /// Handle GET /api/store/categories - List categories with counts
    async fn handle_categories(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Use optimized single-query category count (replaces 7 queries with 1)
        let counts = manager.get_category_counts().await?;

        // Build response with all categories that have coaches
        let all_categories = [
            CoachCategory::Training,
            CoachCategory::Nutrition,
            CoachCategory::Recovery,
            CoachCategory::Recipes,
            CoachCategory::Mobility,
            CoachCategory::Analysis,
            CoachCategory::Custom,
        ];

        let categories: Vec<CategoryCount> = all_categories
            .iter()
            .filter_map(|cat| {
                counts.get(cat).and_then(|&count| {
                    if count > 0 {
                        // Count from SQL COUNT is always non-negative and fits in usize
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let count_usize = count as usize;
                        Some(CategoryCount {
                            category: *cat,
                            name: cat.display_name().to_owned(),
                            count: count_usize,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        info!(
            "User {} fetched {} store categories",
            auth.user_id,
            categories.len()
        );

        let response = CategoriesResponse {
            categories,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/store/search - Search published coaches
    async fn handle_search(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<SearchCoachesQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        if query.q.trim().is_empty() {
            return Err(AppError::invalid_input("Search query cannot be empty"));
        }

        let manager = Self::get_coaches_manager(&resources)?;

        // Search across all tenants (global Store)
        let coaches = manager
            .search_published_coaches(&query.q, query.limit)
            .await?;

        let store_coaches: Vec<StoreCoach> = coaches.into_iter().map(StoreCoach::from).collect();

        info!(
            "User {} searched store for '{}': {} results",
            auth.user_id,
            query.q,
            store_coaches.len()
        );

        let response = SearchCoachesResponse {
            coaches: store_coaches,
            query: query.q,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/store/coaches/:id/install - Install a coach from the Store
    async fn handle_install(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(coach_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Validate coach ID format
        Uuid::parse_str(&coach_id)
            .map_err(|_| AppError::invalid_input(format!("Invalid coach ID: {coach_id}")))?;

        // Install the coach (creates user's copy)
        let installed = manager
            .install_from_store(&coach_id, auth.user_id, tenant_id)
            .await?;

        info!(
            "User {} installed coach '{}' ({}) from Store",
            auth.user_id, installed.title, coach_id
        );

        let response = InstallCoachResponse {
            message: format!("Successfully installed '{}'", installed.title),
            coach: installed.into(),
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle DELETE /api/store/coaches/:id/install - Uninstall a coach
    async fn handle_uninstall(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(coach_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Validate coach ID format
        Uuid::parse_str(&coach_id)
            .map_err(|_| AppError::invalid_input(format!("Invalid coach ID: {coach_id}")))?;

        // Uninstall the coach (deletes user's copy)
        let source_id = manager
            .uninstall_coach(&coach_id, auth.user_id, tenant_id)
            .await?;

        info!(
            "User {} uninstalled coach {} (source: {})",
            auth.user_id, coach_id, source_id
        );

        let response = UninstallCoachResponse {
            message: "Coach uninstalled successfully".to_owned(),
            source_coach_id: source_id,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/store/installations - List user's installed coaches
    async fn handle_list_installations(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        let coaches = manager
            .get_installed_coaches(auth.user_id, tenant_id)
            .await?;

        let store_coaches: Vec<StoreCoach> = coaches.into_iter().map(StoreCoach::from).collect();

        info!(
            "User {} listed {} installed coaches",
            auth.user_id,
            store_coaches.len()
        );

        let response = InstallationsResponse {
            coaches: store_coaches,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }
}

/// Response for install endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallCoachResponse {
    /// Success message
    pub message: String,
    /// The installed coach (user's copy)
    pub coach: StoreCoach,
    /// Response metadata
    pub metadata: StoreMetadata,
}

/// Response for uninstall endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct UninstallCoachResponse {
    /// Success message
    pub message: String,
    /// The source coach ID that was uninstalled
    pub source_coach_id: String,
    /// Response metadata
    pub metadata: StoreMetadata,
}

/// Response for installations list endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationsResponse {
    /// User's installed coaches
    pub coaches: Vec<StoreCoach>,
    /// Response metadata
    pub metadata: StoreMetadata,
}

/// Health check endpoint for store routes
async fn store_health() -> &'static str {
    "Store routes healthy"
}
