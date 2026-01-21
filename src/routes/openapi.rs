// ABOUTME: OpenAPI documentation endpoint with Swagger UI for Pierre Fitness API
// ABOUTME: Provides machine-readable API spec at /api-docs/openapi.json and interactive docs at /swagger-ui
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! `OpenAPI` documentation routes
//!
//! This module provides `OpenAPI` 3.0 specification generation and Swagger UI
//! for exploring and testing the Pierre Fitness API.

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::routes::coaches::{
    CoachResponse, CoachesMetadata, CreateCoachBody, HideCoachResponse, ListCoachesQuery,
    ListCoachesResponse, RecordUsageResponse, SearchCoachesQuery, ToggleFavoriteResponse,
    UpdateCoachBody,
};

/// `OpenAPI` documentation for Pierre Fitness API
///
/// This struct provides the `OpenAPI` 3.0 specification with schema definitions
/// for API contract validation. Path annotations require standalone functions
/// (not impl methods), so only schemas are currently generated.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Pierre Fitness API",
        version = "0.2.0",
        description = "Multi-protocol fitness data API for LLMs (MCP + A2A)",
        license(name = "MIT OR Apache-2.0"),
        contact(
            name = "Pierre Fitness Intelligence",
            url = "https://github.com/Async-IO/pierre_mcp_server"
        )
    ),
    tags(
        (name = "coaches", description = "Custom AI personas management")
    ),
    components(
        schemas(
            CoachResponse,
            ListCoachesResponse,
            CoachesMetadata,
            ListCoachesQuery,
            SearchCoachesQuery,
            ToggleFavoriteResponse,
            RecordUsageResponse,
            HideCoachResponse,
            CreateCoachBody,
            UpdateCoachBody,
        )
    ),
    servers(
        (url = "http://localhost:8081", description = "Local development server")
    )
)]
pub struct ApiDoc;

/// `OpenAPI` routes provider
pub struct OpenApiRoutes;

impl OpenApiRoutes {
    /// Create `OpenAPI` documentation routes
    ///
    /// Provides:
    /// - `/swagger-ui` - Interactive Swagger UI documentation
    /// - `/api-docs/openapi.json` - Raw `OpenAPI` 3.0 JSON specification
    pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
        Router::new()
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    }
}
