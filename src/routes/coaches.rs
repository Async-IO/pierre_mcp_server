// ABOUTME: Route handlers for Coaches REST API (custom AI personas)
// ABOUTME: Provides REST endpoints for CRUD operations on user-created coaches
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Coaches routes
//!
//! This module handles coach endpoints for custom AI personas.
//! All endpoints require JWT authentication to identify the user and tenant.

use std::collections::HashSet;

use crate::{
    auth::AuthResult,
    coaches::{
        parse_coach_content, to_markdown, CoachDefinition, CoachFrontmatter, CoachPrerequisites,
        CoachSections, CoachStartup,
    },
    database::{
        coaches::{
            Coach, CoachAssignment as DbCoachAssignment, CoachCategory, CoachListItem,
            CoachVersion, CoachVisibility, CoachesManager, CreateCoachRequest,
            CreateSystemCoachRequest as DbCreateSystemCoachRequest, ListCoachesFilter,
            UpdateCoachRequest,
        },
        ChatManager,
    },
    database_plugins::DatabaseProvider,
    errors::AppError,
    llm::{get_coach_generation_prompt, ChatMessage, ChatProvider, ChatRequest},
    mcp::resources::ServerResources,
    middleware::require_admin,
    models::TenantId,
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
#[cfg(feature = "openapi")]
use utoipa::ToSchema;
use uuid::Uuid;

/// Response for a coach
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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
    /// ID of the coach this was forked from (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
    /// Whether prerequisites are met (only present if `check_prerequisites=true`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisites_met: Option<bool>,
    /// List of missing prerequisites (only present if `check_prerequisites=true`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_prerequisites: Option<Vec<MissingPrerequisite>>,
}

/// A missing prerequisite for a coach
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct MissingPrerequisite {
    /// Type of prerequisite (provider, `activity_count`, `activity_type`)
    pub prerequisite_type: String,
    /// The specific requirement (e.g., "strava", "50 activities", "Run")
    pub requirement: String,
    /// Human-readable message explaining what's missing
    pub message: String,
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
            forked_from: coach.forked_from,
            prerequisites_met: None,
            missing_prerequisites: None,
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
            forked_from: item.coach.forked_from,
            prerequisites_met: None,
            missing_prerequisites: None,
        }
    }
}

/// Response for listing coaches
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CoachesMetadata {
    /// Response timestamp
    pub timestamp: String,
    /// API version
    pub api_version: String,
}

/// Query parameters for listing coaches
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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
    /// Check prerequisites against user's connected providers (default: false)
    pub check_prerequisites: Option<bool>,
}

/// Query parameters for searching coaches
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchCoachesQuery {
    /// Search query string
    pub q: String,
    /// Maximum results to return
    pub limit: Option<u32>,
    /// Pagination offset
    pub offset: Option<u32>,
}

/// Response for toggle favorite
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ToggleFavoriteResponse {
    /// New favorite status
    pub is_favorite: bool,
}

/// Response for record usage
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RecordUsageResponse {
    /// Whether the usage was recorded
    pub success: bool,
}

/// Response for hide/show coach operations
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct HideCoachResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Whether the coach is now hidden (true) or visible (false)
    pub is_hidden: bool,
}

/// Response for forking a coach
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ForkCoachResponse {
    /// The newly created forked coach
    pub coach: CoachResponse,
    /// The ID of the original coach that was forked
    pub source_coach_id: String,
}

// ============================================
// Version History Response Types (ASY-153)
// ============================================

/// Response for a coach version
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CoachVersionResponse {
    /// Version number
    pub version: i32,
    /// Full content snapshot
    pub content_snapshot: serde_json::Value,
    /// Summary of what changed
    pub change_summary: Option<String>,
    /// When this version was created
    pub created_at: String,
    /// Name of the user who created this version
    pub created_by_name: Option<String>,
}

impl From<CoachVersion> for CoachVersionResponse {
    fn from(v: CoachVersion) -> Self {
        Self {
            version: v.version,
            content_snapshot: v.content_snapshot,
            change_summary: v.change_summary,
            created_at: v.created_at.to_rfc3339(),
            created_by_name: None, // Populated separately with user lookup
        }
    }
}

/// Response for listing coach versions
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListVersionsResponse {
    /// List of versions
    pub versions: Vec<CoachVersionResponse>,
    /// Current version number
    pub current_version: i32,
    /// Total number of versions
    pub total: usize,
}

/// Response for reverting to a version
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RevertVersionResponse {
    /// The coach after reversion
    pub coach: CoachResponse,
    /// The version that was reverted to
    pub reverted_to_version: i32,
    /// The new version number (after revert)
    pub new_version: i32,
}

/// Response for comparing two versions
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CoachDiffResponse {
    /// Source version number
    pub from_version: i32,
    /// Target version number
    pub to_version: i32,
    /// List of field changes
    pub changes: Vec<FieldChange>,
}

/// A single field change between versions
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FieldChange {
    /// Name of the field that changed
    pub field: String,
    /// Old value (None if field was added)
    pub old_value: Option<serde_json::Value>,
    /// New value (None if field was removed)
    pub new_value: Option<serde_json::Value>,
}

/// Query parameters for listing versions
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListVersionsQuery {
    /// Maximum number of versions to return
    pub limit: Option<u32>,
}

/// Request body for creating a coach (mirrors `CreateCoachRequest` with serde derives)
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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

/// Request to generate a coach from a conversation
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GenerateCoachRequest {
    /// The conversation ID to analyze
    pub conversation_id: String,
    /// Maximum number of messages to analyze (default: 10)
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
}

const fn default_max_messages() -> usize {
    10
}

/// Response for coach generation
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GenerateCoachResponse {
    /// Generated title for the coach
    pub title: String,
    /// Generated description
    pub description: String,
    /// Generated system prompt
    pub system_prompt: String,
    /// Suggested category
    pub category: String,
    /// Suggested tags
    pub tags: Vec<String>,
    /// Number of messages analyzed
    pub messages_analyzed: usize,
    /// Total messages in the conversation
    pub total_messages: usize,
}

/// Internal struct for parsing LLM JSON response
#[derive(Debug, Deserialize)]
struct GeneratedCoachData {
    title: String,
    description: String,
    system_prompt: String,
    category: String,
    tags: Vec<String>,
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
            .route("/api/coaches/import", post(Self::handle_import))
            .route("/api/coaches/generate", post(Self::handle_generate))
            .route("/api/coaches/:id", get(Self::handle_get))
            .route("/api/coaches/:id", put(Self::handle_update))
            .route("/api/coaches/:id", delete(Self::handle_delete))
            .route("/api/coaches/:id/export", get(Self::handle_export))
            .route(
                "/api/coaches/:id/favorite",
                post(Self::handle_toggle_favorite),
            )
            .route("/api/coaches/:id/usage", post(Self::handle_record_usage))
            .route("/api/coaches/:id/hide", post(Self::handle_hide_coach))
            .route("/api/coaches/:id/hide", delete(Self::handle_show_coach))
            .route("/api/coaches/:id/fork", post(Self::handle_fork))
            // Version history routes (ASY-153)
            .route("/api/coaches/:id/versions", get(Self::handle_list_versions))
            .route(
                "/api/coaches/:id/versions/:version",
                get(Self::handle_get_version),
            )
            .route(
                "/api/coaches/:id/versions/:version/revert",
                post(Self::handle_revert_version),
            )
            .route(
                "/api/coaches/:id/versions/:v1/diff/:v2",
                get(Self::handle_diff_versions),
            )
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
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        let filter = ListCoachesFilter {
            category: query.category.map(|c| CoachCategory::parse(&c)),
            favorites_only: query.favorites_only.unwrap_or(false),
            limit: query.limit,
            offset: query.offset,
            include_system: query.include_system.unwrap_or(true),
            include_hidden: query.include_hidden.unwrap_or(false),
        };

        let coaches = manager.list(auth.user_id, tenant_id, &filter).await?;
        let total = manager.count(auth.user_id, tenant_id).await?;

        // Check prerequisites if requested
        let check_prereqs = query.check_prerequisites.unwrap_or(false);
        let user_providers = if check_prereqs {
            resources
                .database
                .get_user_oauth_tokens(auth.user_id, None)
                .await
                .map(|tokens| {
                    tokens
                        .iter()
                        .map(|t| t.provider.to_lowercase())
                        .collect::<HashSet<_>>()
                })
                .unwrap_or_default()
        } else {
            HashSet::new()
        };

        let coaches_with_prereqs: Vec<CoachResponse> = coaches
            .into_iter()
            .map(|item| {
                let mut response: CoachResponse = item.coach.clone().into();
                response.is_assigned = item.is_assigned;

                if check_prereqs {
                    let (met, missing) =
                        Self::check_prerequisites(&item.coach.prerequisites, &user_providers);
                    response.prerequisites_met = Some(met);
                    response.missing_prerequisites = if missing.is_empty() {
                        None
                    } else {
                        Some(missing)
                    };
                }

                response
            })
            .collect();

        let response = ListCoachesResponse {
            coaches: coaches_with_prereqs,
            total,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Check if prerequisites are met given user's connected providers
    fn check_prerequisites(
        prerequisites: &CoachPrerequisites,
        user_providers: &HashSet<String>,
    ) -> (bool, Vec<MissingPrerequisite>) {
        let mut missing = Vec::new();

        // Check required providers
        for provider in &prerequisites.providers {
            let provider_lower = provider.to_lowercase();
            if !user_providers.contains(&provider_lower) {
                missing.push(MissingPrerequisite {
                    prerequisite_type: "provider".to_owned(),
                    requirement: provider.clone(),
                    message: format!(
                        "Connect {} to unlock this coach",
                        capitalize_provider(provider)
                    ),
                });
            }
        }

        // Note: min_activities and activity_types checks would require
        // fetching activity data, which could be expensive. For now,
        // we only check providers. Activity-based checks can be added
        // in a future iteration when needed.

        let met = missing.is_empty();
        (met, missing)
    }

    /// Handle POST /api/coaches - Create a new coach
    async fn handle_create(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<CreateCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let request: CreateCoachRequest = body.into();
        let coach = manager.create(auth.user_id, tenant_id, &request).await?;

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
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager
            .search(auth.user_id, tenant_id, &query.q, query.limit, query.offset)
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
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .get(&id, auth.user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {id}")))?;

        let response: CoachResponse = coach.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/coaches/:id/export - Export coach as markdown
    async fn handle_export(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .get(&id, auth.user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {id}")))?;

        // Convert Coach to CoachDefinition for export
        let definition = coach_to_definition(&coach);
        let markdown = to_markdown(&definition);

        // Generate filename from coach name/title
        let filename = generate_coach_filename(&coach.title);

        Ok((
            StatusCode::OK,
            [
                ("content-type", "text/markdown; charset=utf-8"),
                (
                    "content-disposition",
                    &format!("attachment; filename=\"{filename}\""),
                ),
            ],
            markdown,
        )
            .into_response())
    }

    /// Handle POST /api/coaches/import - Import coach from markdown
    async fn handle_import(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        body: String,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        // Parse the markdown content
        let definition = parse_coach_content(&body, None)
            .map_err(|e| AppError::invalid_input(format!("Invalid markdown format: {e}")))?;

        // Create coach from the parsed definition
        let request = CreateCoachRequest {
            title: definition.frontmatter.title,
            description: Some(definition.sections.purpose.clone()),
            system_prompt: definition.sections.instructions,
            category: definition.frontmatter.category,
            tags: definition.frontmatter.tags,
            sample_prompts: definition
                .sections
                .example_inputs
                .map(|inputs| {
                    inputs
                        .lines()
                        .filter_map(|line| {
                            line.trim()
                                .strip_prefix('-')
                                .map(|s| s.trim().trim_matches('"').to_owned())
                        })
                        .collect()
                })
                .unwrap_or_default(),
        };

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager.create(auth.user_id, tenant_id, &request).await?;

        let response = ImportCoachResponse {
            coach: coach.into(),
            parsed_name: definition.frontmatter.name,
            token_count: definition.token_count,
        };
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle POST /api/coaches/generate - Generate coach from conversation
    ///
    /// Uses the LLM to analyze the last N messages of a conversation and
    /// generate a coach profile with title, description, system prompt, and tags.
    async fn handle_generate(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<GenerateCoachRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        // Get chat manager to fetch conversation messages
        let pool = resources
            .database
            .sqlite_pool()
            .ok_or_else(|| AppError::internal("SQLite database required for coach generation"))?
            .clone();
        let chat_manager = ChatManager::new(pool);

        // Verify user owns the conversation (get_conversation returns None if not found or not owned)
        chat_manager
            .get_conversation(&body.conversation_id, &auth.user_id.to_string(), tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Conversation"))?;

        // Get conversation messages
        let messages = chat_manager
            .get_messages(&body.conversation_id, &auth.user_id.to_string())
            .await?;
        let total_messages = messages.len();

        if messages.is_empty() {
            return Err(AppError::invalid_input(
                "Cannot generate coach from empty conversation",
            ));
        }

        // Take the last N messages (or all if fewer)
        let messages_to_analyze: Vec<_> = messages
            .iter()
            .rev()
            .take(body.max_messages)
            .rev()
            .collect();
        let messages_analyzed = messages_to_analyze.len();

        // Build the conversation text for LLM analysis
        let conversation_text = messages_to_analyze
            .iter()
            .map(|m| format!("[{}]: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Build LLM request with generation prompt
        let system_prompt = get_coach_generation_prompt();
        let user_prompt = format!(
            "Analyze this fitness conversation and create a specialized coach profile.\n\n\
            Conversation (last {messages_analyzed} of {total_messages} messages):\n\n\
            {conversation_text}"
        );

        let llm_messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(&user_prompt),
        ];

        // Get LLM provider and generate
        let provider = ChatProvider::from_env().await?;
        let request = ChatRequest::new(llm_messages);
        let response = provider.complete(&request).await?;

        if response.content.is_empty() {
            return Err(AppError::internal("LLM returned empty response"));
        }

        // Parse the JSON response from LLM
        let generated: GeneratedCoachData =
            serde_json::from_str(&response.content).map_err(|e| {
                AppError::internal(format!("Failed to parse LLM response as JSON: {e}"))
            })?;

        let response = GenerateCoachResponse {
            title: generated.title,
            description: generated.description,
            system_prompt: generated.system_prompt,
            category: generated.category,
            tags: generated.tags,
            messages_analyzed,
            total_messages,
        };

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
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let request: UpdateCoachRequest = body.into();
        let coach = manager
            .update(&id, auth.user_id, tenant_id, &request)
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
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let deleted = manager.delete(&id, auth.user_id, tenant_id).await?;

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
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let is_favorite = manager
            .toggle_favorite(&id, auth.user_id, tenant_id)
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
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let success = manager.record_usage(&id, auth.user_id, tenant_id).await?;

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

        let manager = Self::get_coaches_manager(&resources)?;
        let success = manager.hide_coach(&id, auth.user_id).await?;

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

    /// Handle POST /api/coaches/:id/fork - Fork a system coach to create a user copy
    async fn handle_fork(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let forked_coach = manager.fork_coach(&id, auth.user_id, tenant_id).await?;

        let response = ForkCoachResponse {
            coach: forked_coach.into(),
            source_coach_id: id,
        };
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /api/coaches/hidden - List hidden coaches for user
    async fn handle_list_hidden(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager.list_hidden_coaches(auth.user_id, tenant_id).await?;

        let response = ListCoachesResponse {
            total: u32::try_from(coaches.len()).unwrap_or(0),
            coaches: coaches.into_iter().map(Into::into).collect(),
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    // ============================================
    // Version History Routes (ASY-153)
    // ============================================

    /// Handle GET /api/coaches/:id/versions - List version history
    async fn handle_list_versions(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Query(query): Query<ListVersionsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let limit = query.limit.unwrap_or(50).clamp(1, 100);
        let versions = manager.get_versions(&id, tenant_id, limit).await?;
        let current_version = manager.get_current_version(&id).await?;

        let version_responses: Vec<CoachVersionResponse> =
            versions.into_iter().map(Into::into).collect();

        let response = ListVersionsResponse {
            total: version_responses.len(),
            versions: version_responses,
            current_version,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/coaches/:id/versions/:version - Get a specific version
    async fn handle_get_version(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path((id, version)): Path<(String, i32)>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let version_data = manager
            .get_version(&id, version, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Version {version} for coach {id}")))?;

        let response: CoachVersionResponse = version_data.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/coaches/:id/versions/:version/revert - Revert to a version
    async fn handle_revert_version(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path((id, version)): Path<(String, i32)>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .revert_to_version(&id, version, auth.user_id, tenant_id)
            .await?;

        let new_version = manager.get_current_version(&id).await?;

        let response = RevertVersionResponse {
            coach: coach.into(),
            reverted_to_version: version,
            new_version,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /api/coaches/:id/versions/:v1/diff/:v2 - Compare two versions
    async fn handle_diff_versions(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path((id, v1, v2)): Path<(String, i32, i32)>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        let version1 = manager
            .get_version(&id, v1, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Version {v1} for coach {id}")))?;

        let version2 = manager
            .get_version(&id, v2, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Version {v2} for coach {id}")))?;

        // Compare the content snapshots
        let changes = compute_diff(&version1.content_snapshot, &version2.content_snapshot);

        let response = CoachDiffResponse {
            from_version: v1,
            to_version: v2,
            changes,
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
            // Store management routes (ASY-228)
            .route("/store/stats", get(Self::handle_admin_store_stats))
            .route("/store/review-queue", get(Self::handle_admin_review_queue))
            .route("/store/published", get(Self::handle_admin_published))
            .route("/store/rejected", get(Self::handle_admin_rejected))
            .route(
                "/store/coaches/:id/approve",
                post(Self::handle_admin_approve),
            )
            .route("/store/coaches/:id/reject", post(Self::handle_admin_reject))
            .route(
                "/store/coaches/:id/unpublish",
                post(Self::handle_admin_unpublish),
            )
            .with_state(resources)
    }

    /// Handle GET /admin/coaches - List all system coaches in tenant
    async fn handle_admin_list(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager.list_system_coaches(tenant_id).await?;

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
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .create_system_coach(auth.user_id, tenant_id, &body.into())
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
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coach = manager
            .get_system_coach(&id, tenant_id)
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
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let request: UpdateCoachRequest = body.into();
        let coach = manager
            .update_system_coach(&id, tenant_id, &request)
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
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let deleted = manager.delete_system_coach(&id, tenant_id).await?;

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
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Verify the coach exists and is a system coach
        let coach = manager
            .get_system_coach(&id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        // Assign to each user after verifying tenant membership
        let mut assigned_count = 0;
        for user_id_str in &body.user_ids {
            let user_id = Uuid::parse_str(user_id_str)
                .map_err(|_| AppError::invalid_input(format!("Invalid user ID: {user_id_str}")))?;

            // Verify target user belongs to the same tenant
            let user_tenants = resources
                .database
                .list_tenants_for_user(user_id)
                .await
                .map_err(|e| {
                    AppError::database(format!(
                        "Failed to verify tenant membership for user {user_id}: {e}"
                    ))
                })?;
            if !user_tenants.iter().any(|t| t.id == tenant_id) {
                return Err(AppError::auth_invalid(format!(
                    "User {user_id} does not belong to this tenant"
                )));
            }

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
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Verify the coach exists
        manager
            .get_system_coach(&id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        // Unassign from each user after verifying tenant membership
        let mut removed_count = 0;
        for user_id_str in &body.user_ids {
            let user_id = Uuid::parse_str(user_id_str)
                .map_err(|_| AppError::invalid_input(format!("Invalid user ID: {user_id_str}")))?;

            // Verify target user belongs to the same tenant
            let user_tenants = resources
                .database
                .list_tenants_for_user(user_id)
                .await
                .map_err(|e| {
                    AppError::database(format!(
                        "Failed to verify tenant membership for user {user_id}: {e}"
                    ))
                })?;
            if !user_tenants.iter().any(|t| t.id == tenant_id) {
                return Err(AppError::auth_invalid(format!(
                    "User {user_id} does not belong to this tenant"
                )));
            }

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
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;

        // Verify the coach exists
        manager
            .get_system_coach(&id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {id}")))?;

        let db_assignments = manager.list_assignments_for_tenant(&id, tenant_id).await?;
        let assignments: Vec<CoachAssignment> =
            db_assignments.into_iter().map(Into::into).collect();

        let response = ListAssignmentsResponse {
            coach_id: id,
            assignments,
        };
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    // ============================================
    // Admin Store Management Routes (ASY-228)
    // ============================================

    /// Handle GET /admin/store/stats - Get store statistics
    async fn handle_admin_store_stats(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let stats = manager.get_store_admin_stats(tenant_id).await?;

        let response = StoreAdminStatsResponse {
            pending_count: stats.pending_count,
            published_count: stats.published_count,
            rejected_count: stats.rejected_count,
            total_installs: stats.total_installs,
            rejection_rate: stats.rejection_rate,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /admin/store/review-queue - Get pending review coaches
    async fn handle_admin_review_queue(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(params): Query<StoreListParams>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager
            .get_pending_review_coaches(tenant_id, params.limit, params.offset)
            .await?;

        let coaches_with_email = Self::enrich_coaches_with_email(&manager, coaches).await?;
        // Paginated results with limits - count never exceeds u32
        #[allow(clippy::cast_possible_truncation)]
        let total = coaches_with_email.len() as u32;

        let response = StoreCoachesResponse {
            coaches: coaches_with_email,
            total,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /admin/store/published - Get published coaches
    async fn handle_admin_published(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(params): Query<StoreListParams>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let sort_by = params.sort_by.as_deref();
        let coaches = manager
            .get_published_coaches(None, sort_by, params.limit, params.offset)
            .await?;

        let coaches_with_email = Self::enrich_coaches_with_email(&manager, coaches).await?;
        // Paginated results with limits - count never exceeds u32
        #[allow(clippy::cast_possible_truncation)]
        let total = coaches_with_email.len() as u32;

        let response = StoreCoachesResponse {
            coaches: coaches_with_email,
            total,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle GET /admin/store/rejected - Get rejected coaches
    async fn handle_admin_rejected(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(params): Query<StoreListParams>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        let coaches = manager
            .get_rejected_coaches(tenant_id, params.limit, params.offset)
            .await?;

        let coaches_with_email = Self::enrich_coaches_with_email(&manager, coaches).await?;
        // Paginated results with limits - count never exceeds u32
        #[allow(clippy::cast_possible_truncation)]
        let total = coaches_with_email.len() as u32;

        let response = StoreCoachesResponse {
            coaches: coaches_with_email,
            total,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /admin/store/coaches/:id/approve - Approve a coach
    async fn handle_admin_approve(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        manager.approve_coach(&id, tenant_id, auth.user_id).await?;

        let response = StoreActionResponse {
            success: true,
            message: "Coach approved and published".to_owned(),
            coach_id: id,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /admin/store/coaches/:id/reject - Reject a coach
    async fn handle_admin_reject(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<RejectCoachBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        // Combine reason with optional notes
        let rejection_reason = if let Some(notes) = &body.notes {
            if notes.trim().is_empty() {
                body.reason.clone()
            } else {
                format!("{}: {}", body.reason, notes.trim())
            }
        } else {
            body.reason.clone()
        };

        let manager = Self::get_coaches_manager(&resources)?;
        manager
            .reject_coach(&id, tenant_id, auth.user_id, &rejection_reason)
            .await?;

        let response = StoreActionResponse {
            success: true,
            message: "Coach rejected".to_owned(),
            coach_id: id,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /admin/store/coaches/:id/unpublish - Unpublish a coach
    async fn handle_admin_unpublish(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        require_admin(auth.user_id, &resources.database).await?;
        let tenant_id = Self::get_user_tenant(&auth, &resources).await?;

        let manager = Self::get_coaches_manager(&resources)?;
        manager.unpublish_coach(&id, tenant_id).await?;

        let response = StoreActionResponse {
            success: true,
            message: "Coach unpublished".to_owned(),
            coach_id: id,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Enrich coaches with author email information
    async fn enrich_coaches_with_email(
        manager: &CoachesManager,
        coaches: Vec<Coach>,
    ) -> Result<Vec<StoreCoachResponse>, AppError> {
        let mut result = Vec::with_capacity(coaches.len());

        for coach in coaches {
            let author_email = manager.get_author_email(coach.user_id).await?;
            result.push(StoreCoachResponse::from_coach(coach, author_email));
        }

        Ok(result)
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

/// Response for importing a coach from markdown
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ImportCoachResponse {
    /// The created coach
    pub coach: CoachResponse,
    /// The parsed name/slug from the markdown
    pub parsed_name: String,
    /// Estimated token count from the markdown
    pub token_count: u32,
}

// ============================================
// Helper Functions for Export/Import
// ============================================

/// Convert a Coach database model to `CoachDefinition` for export
fn coach_to_definition(coach: &Coach) -> CoachDefinition {
    let name = coach
        .title
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    CoachDefinition {
        frontmatter: CoachFrontmatter {
            name,
            title: coach.title.clone(),
            category: coach.category,
            tags: coach.tags.clone(),
            prerequisites: CoachPrerequisites::default(),
            visibility: coach.visibility,
            startup: CoachStartup::default(),
        },
        sections: CoachSections {
            purpose: coach.description.clone().unwrap_or_default(),
            when_to_use: None,
            instructions: coach.system_prompt.clone(),
            example_inputs: if coach.sample_prompts.is_empty() {
                None
            } else {
                Some(
                    coach
                        .sample_prompts
                        .iter()
                        .map(|p| format!("- {p}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            },
            example_outputs: None,
            success_criteria: None,
            related_coaches: Vec::new(),
        },
        source_file: format!("exported/{}.md", coach.id),
        content_hash: String::new(),
        token_count: coach.token_count,
    }
}

/// Generate a safe filename from coach title
fn generate_coach_filename(title: &str) -> String {
    let safe_name: String = title
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();

    format!("{safe_name}.md")
}

/// Capitalize provider name for user-friendly display
fn capitalize_provider(provider: &str) -> String {
    let provider_lower = provider.to_lowercase();
    match provider_lower.as_str() {
        "strava" => "Strava".to_owned(),
        "garmin" => "Garmin".to_owned(),
        "fitbit" => "Fitbit".to_owned(),
        "terra" => "Terra".to_owned(),
        _ => {
            // Capitalize first letter
            let mut chars = provider.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        }
    }
}

// ============================================
// Version Diff Helper (ASY-153)
// ============================================

/// Compute field-level differences between two JSON snapshots
fn compute_diff(from: &serde_json::Value, to: &serde_json::Value) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    // Fields we care about comparing
    let fields = [
        "title",
        "description",
        "system_prompt",
        "category",
        "tags",
        "sample_prompts",
        "visibility",
    ];

    for field in fields {
        let old_val = from.get(field);
        let new_val = to.get(field);

        match (old_val, new_val) {
            (Some(old), Some(new)) if old != new => {
                changes.push(FieldChange {
                    field: field.to_owned(),
                    old_value: Some(old.clone()),
                    new_value: Some(new.clone()),
                });
            }
            (None, Some(new)) => {
                changes.push(FieldChange {
                    field: field.to_owned(),
                    old_value: None,
                    new_value: Some(new.clone()),
                });
            }
            (Some(old), None) => {
                changes.push(FieldChange {
                    field: field.to_owned(),
                    old_value: Some(old.clone()),
                    new_value: None,
                });
            }
            _ => {}
        }
    }

    changes
}

// ============================================
// Store Admin Request/Response Types (ASY-228)
// ============================================

/// Query parameters for store listing endpoints
#[derive(Debug, Deserialize)]
pub struct StoreListParams {
    /// Maximum number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
    /// Sort by: "newest" or `most_installed`
    pub sort_by: Option<String>,
}

/// Store admin statistics response
#[derive(Debug, Serialize)]
pub struct StoreAdminStatsResponse {
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

/// Store coach response with author email
#[derive(Debug, Serialize)]
pub struct StoreCoachResponse {
    /// Coach ID
    pub id: String,
    /// Display title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// System prompt
    pub system_prompt: String,
    /// Category
    pub category: String,
    /// Tags
    pub tags: Vec<String>,
    /// Sample prompts
    pub sample_prompts: Vec<String>,
    /// Token count
    pub token_count: u32,
    /// Install count
    pub install_count: u32,
    /// Icon URL
    pub icon_url: Option<String>,
    /// Published timestamp
    pub published_at: Option<String>,
    /// When submitted for review
    pub submitted_at: Option<String>,
    /// When review decision was made
    pub rejected_at: Option<String>,
    /// Author user ID
    pub author_id: Option<String>,
    /// Author email (joined from users table)
    pub author_email: Option<String>,
    /// Rejection reason (if rejected)
    pub rejection_reason: Option<String>,
    /// Rejection notes (parsed from `rejection_reason`)
    pub rejection_notes: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Publish status
    pub publish_status: String,
}

impl StoreCoachResponse {
    /// Create from Coach with author email
    fn from_coach(coach: Coach, author_email: Option<String>) -> Self {
        // Parse rejection reason into reason code and notes
        let (rejection_reason, rejection_notes) =
            coach
                .rejection_reason
                .as_ref()
                .map_or((None, None), |reason| {
                    reason.find(": ").map_or_else(
                        || (Some(reason.clone()), None),
                        |colon_pos| {
                            let code = reason[..colon_pos].to_owned();
                            let notes = reason[colon_pos + 2..].to_owned();
                            (Some(code), Some(notes))
                        },
                    )
                });

        Self {
            id: coach.id.to_string(),
            title: coach.title,
            description: coach.description,
            system_prompt: coach.system_prompt,
            category: coach.category.as_str().to_owned(),
            tags: coach.tags,
            sample_prompts: coach.sample_prompts,
            token_count: coach.token_count,
            install_count: coach.install_count,
            icon_url: coach.icon_url,
            published_at: coach.published_at.map(|dt| dt.to_rfc3339()),
            submitted_at: coach.review_submitted_at.map(|dt| dt.to_rfc3339()),
            rejected_at: coach.review_decision_at.map(|dt| dt.to_rfc3339()),
            author_id: Some(coach.user_id.to_string()),
            author_email,
            rejection_reason,
            rejection_notes,
            created_at: coach.created_at.to_rfc3339(),
            publish_status: coach.publish_status.as_str().to_owned(),
        }
    }
}

/// Response for store coach listing
#[derive(Debug, Serialize)]
pub struct StoreCoachesResponse {
    /// List of coaches
    pub coaches: Vec<StoreCoachResponse>,
    /// Total count
    pub total: u32,
    /// Response metadata
    pub metadata: CoachesMetadata,
}

/// Store action response (approve/reject/unpublish)
#[derive(Debug, Serialize)]
pub struct StoreActionResponse {
    /// Whether the action was successful
    pub success: bool,
    /// Message describing the action
    pub message: String,
    /// Coach ID that was acted upon
    pub coach_id: String,
}

/// Request body for rejecting a coach
#[derive(Debug, Deserialize)]
pub struct RejectCoachBody {
    /// Rejection reason code
    pub reason: String,
    /// Optional additional notes
    pub notes: Option<String>,
}
