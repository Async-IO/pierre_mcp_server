// ABOUTME: Fitness provider API handlers for universal protocol
// ABOUTME: Provider-agnostic single responsibility handlers that delegate auth to AuthService
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::cache::{factory::Cache, CacheKey, CacheResource};
use crate::intelligence::physiological_constants::api_limits::{
    DEFAULT_ACTIVITY_LIMIT, DEFAULT_ACTIVITY_LIMIT_U32, MAX_ACTIVITY_LIMIT, QUICK_ACTIVITY_LIMIT,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use crate::providers::core::FitnessProvider;
use crate::utils::uuid::parse_user_id_for_protocol;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Create metadata for activity analysis responses
fn create_activity_metadata(
    activity_id: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&String>,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut map = std::collections::HashMap::new();
    map.insert(
        "activity_id".to_owned(),
        serde_json::Value::String(activity_id.to_owned()),
    );
    map.insert(
        "user_id".to_owned(),
        serde_json::Value::String(user_uuid.to_string()),
    );
    map.insert(
        "tenant_id".to_owned(),
        tenant_id.map_or(serde_json::Value::Null, |id| {
            serde_json::Value::String(id.clone()) // Safe: String ownership for JSON value
        }),
    );
    map
}

/// Try to get activities from cache
async fn try_get_cached_activities(
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&String>,
    limit: usize,
    provider_name: &str,
) -> Option<UniversalResponse> {
    if let Ok(Some(cached_activities)) = cache.get::<Vec<crate::models::Activity>>(cache_key).await
    {
        info!("Cache hit for activities (limit={})", limit);
        return Some(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "activities": cached_activities,
                "provider": provider_name,
                "count": cached_activities.len()
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "total_activities".to_owned(),
                    serde_json::Value::Number(cached_activities.len().into()),
                );
                map.insert(
                    "user_id".to_owned(),
                    serde_json::Value::String(user_uuid.to_string()),
                );
                map.insert(
                    "tenant_id".to_owned(),
                    tenant_id.map_or(serde_json::Value::Null, |id| {
                        serde_json::Value::String(id.clone())
                    }),
                );
                map.insert("cached".to_owned(), serde_json::Value::Bool(true));
                map
            }),
        });
    }
    info!("Cache miss for activities (limit={})", limit);
    None
}

/// Cache activities after fetching from API
async fn cache_activities_result(
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    activities: &Vec<crate::models::Activity>,
    per_page: u32,
) {
    let ttl = CacheResource::ActivityList { page: 1, per_page }.recommended_ttl();
    if let Err(e) = cache.set(cache_key, activities, ttl).await {
        warn!("Failed to cache activities: {}", e);
    } else {
        info!("Cached {} activities with TTL {:?}", activities.len(), ttl);
    }
}

/// Build success response for activities
fn build_activities_success_response(
    activities: &[crate::models::Activity],
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
    provider_name: &str,
) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "activities": activities,
            "provider": provider_name,
            "count": activities.len()
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "total_activities".to_owned(),
                serde_json::Value::Number(activities.len().into()),
            );
            map.insert(
                "user_id".to_owned(),
                serde_json::Value::String(user_uuid.to_string()),
            );
            map.insert(
                "tenant_id".to_owned(),
                tenant_id.map_or(serde_json::Value::Null, serde_json::Value::String),
            );
            map.insert("cached".to_owned(), serde_json::Value::Bool(false));
            map
        }),
    }
}

/// Try to get athlete from cache
async fn try_get_cached_athlete(
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&String>,
) -> Result<Option<UniversalResponse>, ProtocolError> {
    if let Ok(Some(cached_athlete)) = cache.get::<crate::models::Athlete>(cache_key).await {
        info!("Cache hit for athlete profile");
        return Ok(Some(UniversalResponse {
            success: true,
            result: Some(serde_json::to_value(&cached_athlete).map_err(|e| {
                ProtocolError::SerializationError(format!(
                    "Failed to serialize cached athlete: {e}"
                ))
            })?),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "user_id".to_owned(),
                    serde_json::Value::String(user_uuid.to_string()),
                );
                map.insert(
                    "tenant_id".to_owned(),
                    tenant_id.map_or(serde_json::Value::Null, |id| {
                        serde_json::Value::String(id.clone())
                    }),
                );
                map.insert("cached".to_owned(), serde_json::Value::Bool(true));
                map
            }),
        }));
    }
    info!("Cache miss for athlete profile");
    Ok(None)
}

/// Cache athlete profile after fetching from API
async fn cache_athlete_result(
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    athlete: &crate::models::Athlete,
) {
    let ttl = CacheResource::AthleteProfile.recommended_ttl();
    if let Err(e) = cache.set(cache_key, athlete, ttl).await {
        warn!("Failed to cache athlete profile: {}", e);
    } else {
        info!("Cached athlete profile with TTL {:?}", ttl);
    }
}

/// Fetch athlete from API and cache result
async fn fetch_and_cache_athlete(
    provider: &dyn FitnessProvider,
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
) -> Result<UniversalResponse, ProtocolError> {
    match provider.get_athlete().await {
        Ok(athlete) => {
            cache_athlete_result(cache, cache_key, &athlete).await;

            Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::to_value(&athlete).map_err(|e| {
                    ProtocolError::SerializationError(format!("Failed to serialize athlete: {e}"))
                })?),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "tenant_id".to_owned(),
                        tenant_id.map_or(serde_json::Value::Null, serde_json::Value::String),
                    );
                    map.insert("cached".to_owned(), serde_json::Value::Bool(false));
                    map
                }),
            })
        }
        Err(e) => Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Failed to fetch athlete profile: {e}")),
            metadata: None,
        }),
    }
}

/// Process activity analysis when activity is found
async fn process_activity_analysis(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
    activity_id: &str,
    user_uuid: uuid::Uuid,
) -> Result<UniversalResponse, ProtocolError> {
    let analysis_response =
        super::intelligence::handle_get_activity_intelligence(executor, request).await?;
    let analysis = analysis_response
        .result
        .unwrap_or_else(|| serde_json::json!({}));

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::to_value(analysis).map_err(|e| {
            ProtocolError::SerializationError(format!("Failed to serialize analysis: {e}"))
        })?),
        error: None,
        metadata: Some(create_activity_metadata(
            activity_id,
            user_uuid,
            analysis_response
                .metadata
                .as_ref()
                .and_then(|m| {
                    m.get("tenant_id")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                })
                .as_ref(),
        )),
    })
}

/// Handle `get_activities` tool - retrieve user's fitness activities
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_get_activities(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_activities cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(crate::config::environment::default_provider, String::from);

        // Extract limit parameter with bounds checking
        let requested_limit = request
            .parameters
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(QUICK_ACTIVITY_LIMIT)
            .try_into()
            .unwrap_or(DEFAULT_ACTIVITY_LIMIT);

        let limit = requested_limit.min(MAX_ACTIVITY_LIMIT);

        // Create cache key for activities
        let tenant_uuid = request
            .tenant_id
            .as_ref()
            .and_then(|t| {
                uuid::Uuid::parse_str(t)
                    .inspect_err(|e| {
                        debug!(
                            tenant_id_str = %t,
                            error = %e,
                            "Failed to parse tenant ID for activities cache key - using nil UUID"
                        );
                    })
                    .ok()
            })
            .unwrap_or_else(uuid::Uuid::nil);

        // For caching activities, use page=1 and per_page=limit
        // Safe: limit is bounded by MAX_ACTIVITY_LIMIT which fits in u32
        let per_page = u32::try_from(limit).unwrap_or(DEFAULT_ACTIVITY_LIMIT_U32);
        let cache_key = CacheKey::new(
            tenant_uuid,
            user_uuid,
            provider_name.clone(),
            CacheResource::ActivityList { page: 1, per_page },
        );

        // Try to get from cache first
        if let Some(cached_response) = try_get_cached_activities(
            &executor.resources.cache,
            &cache_key,
            user_uuid,
            request.tenant_id.as_ref(),
            limit,
            &provider_name,
        )
        .await
        {
            // Report completion if we got from cache
            if let Some(reporter) = &request.progress_reporter {
                reporter.report(
                    100.0,
                    Some(100.0),
                    Some("Activities loaded from cache".to_owned()),
                );
            }
            return Ok(cached_response);
        }

        // Report progress after cache miss
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before expensive auth operation
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_activities cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after successful auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some(format!(
                            "Authenticated - fetching activities from {provider_name}..."
                        )),
                    );
                }

                // Check cancellation before API call
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "get_activities cancelled before API call".to_owned(),
                        ));
                    }
                }

                // Get activities from provider
                match provider.get_activities(Some(limit), None).await {
                    Ok(activities) => {
                        // Report completion
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                100.0,
                                Some(100.0),
                                Some(format!(
                                    "Successfully fetched {} activities",
                                    activities.len()
                                )),
                            );
                        }

                        cache_activities_result(
                            &executor.resources.cache,
                            &cache_key,
                            &activities,
                            per_page,
                        )
                        .await;
                        Ok(build_activities_success_response(
                            &activities,
                            user_uuid,
                            request.tenant_id,
                            &provider_name,
                        ))
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `get_athlete` tool - retrieve user's athlete profile
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_get_athlete(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_athlete cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(crate::config::environment::default_provider, String::from);

        // Create cache key for athlete profile
        let tenant_uuid = request
            .tenant_id
            .as_ref()
            .and_then(|t| {
                uuid::Uuid::parse_str(t)
                    .inspect_err(|e| {
                        debug!(
                            tenant_id_str = %t,
                            error = %e,
                            "Failed to parse tenant ID for cache key - using nil UUID"
                        );
                    })
                    .ok()
            })
            .unwrap_or_else(uuid::Uuid::nil);

        let cache_key = CacheKey::new(
            tenant_uuid,
            user_uuid,
            provider_name.clone(),
            CacheResource::AthleteProfile,
        );

        // Try to get from cache first
        if let Some(cached_response) = try_get_cached_athlete(
            &executor.resources.cache,
            &cache_key,
            user_uuid,
            request.tenant_id.as_ref(),
        )
        .await?
        {
            // Report completion if loaded from cache
            if let Some(reporter) = &request.progress_reporter {
                reporter.report(
                    100.0,
                    Some(100.0),
                    Some("Athlete profile loaded from cache".to_owned()),
                );
            }
            return Ok(cached_response);
        }

        // Report progress after cache miss
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_athlete cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some("Authenticated - fetching athlete profile...".to_owned()),
                    );
                }

                // Check cancellation before fetch
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "get_athlete cancelled before fetch".to_owned(),
                        ));
                    }
                }

                let result = fetch_and_cache_athlete(
                    provider.as_ref(),
                    &executor.resources.cache,
                    &cache_key,
                    user_uuid,
                    request.tenant_id,
                )
                .await;

                // Report completion on success
                if result.is_ok() {
                    if let Some(reporter) = &request.progress_reporter {
                        reporter.report(
                            100.0,
                            Some(100.0),
                            Some("Athlete profile fetched successfully".to_owned()),
                        );
                    }
                }

                result
            }
            Err(response) => Ok(response),
        }
    })
}

/// Try to get athlete ID from cached athlete profile
async fn try_get_athlete_id_from_cache(
    cache: &Arc<Cache>,
    athlete_cache_key: &CacheKey,
) -> Option<u64> {
    if let Ok(Some(athlete)) = cache.get::<crate::models::Athlete>(athlete_cache_key).await {
        return athlete
            .id
            .parse::<u64>()
            .inspect_err(|e| {
                debug!(
                    athlete_id_str = %athlete.id,
                    error = %e,
                    "Failed to parse athlete ID from cache as u64"
                );
            })
            .ok();
    }
    None
}

/// Try to get stats from cache
async fn try_get_cached_stats(
    cache: &Arc<Cache>,
    stats_cache_key: &CacheKey,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&String>,
) -> Result<Option<UniversalResponse>, ProtocolError> {
    if let Ok(Some(cached_stats)) = cache.get::<crate::models::Stats>(stats_cache_key).await {
        info!("Cache hit for stats");
        return Ok(Some(UniversalResponse {
            success: true,
            result: Some(serde_json::to_value(&cached_stats).map_err(|e| {
                ProtocolError::SerializationError(format!("Failed to serialize cached stats: {e}"))
            })?),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "user_id".to_owned(),
                    serde_json::Value::String(user_uuid.to_string()),
                );
                map.insert(
                    "tenant_id".to_owned(),
                    tenant_id.map_or(serde_json::Value::Null, |id| {
                        serde_json::Value::String(id.clone())
                    }),
                );
                map.insert("cached".to_owned(), serde_json::Value::Bool(true));
                map
            }),
        }));
    }
    info!("Cache miss for stats");
    Ok(None)
}

/// Create metadata for stats responses
fn create_stats_metadata(
    user_uuid: uuid::Uuid,
    tenant_uuid: uuid::Uuid,
    cached: bool,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut map = std::collections::HashMap::new();
    map.insert(
        "user_id".to_owned(),
        serde_json::Value::String(user_uuid.to_string()),
    );
    map.insert(
        "tenant_id".to_owned(),
        serde_json::Value::String(tenant_uuid.to_string()),
    );
    map.insert("cached".to_owned(), serde_json::Value::Bool(cached));
    map
}

/// Cache a single item with TTL, logging errors
async fn cache_item<T: serde::Serialize + Send + Sync>(
    cache: &Arc<Cache>,
    key: &CacheKey,
    item: &T,
    ttl: std::time::Duration,
    item_name: &str,
) {
    if let Err(e) = cache.set(key, item, ttl).await {
        warn!("Failed to cache {}: {}", item_name, e);
    }
}

/// Cache athlete and stats data
async fn cache_athlete_and_stats(
    cache: &Arc<Cache>,
    athlete_cache_key: &CacheKey,
    athlete: &crate::models::Athlete,
    stats: &crate::models::Stats,
    tenant_uuid: uuid::Uuid,
    user_uuid: uuid::Uuid,
    provider_name: &str,
) {
    let Some(athlete_id) = athlete
        .id
        .parse::<u64>()
        .inspect_err(|e| debug!("Failed to parse athlete ID: {e}"))
        .ok()
    else {
        return;
    };

    // Cache athlete
    let athlete_ttl = CacheResource::AthleteProfile.recommended_ttl();
    cache_item(cache, athlete_cache_key, athlete, athlete_ttl, "athlete").await;

    // Cache stats
    let stats_cache_key = CacheKey::new(
        tenant_uuid,
        user_uuid,
        provider_name.to_owned(),
        CacheResource::Stats { athlete_id },
    );
    let stats_ttl = CacheResource::Stats { athlete_id }.recommended_ttl();
    cache_item(cache, &stats_cache_key, stats, stats_ttl, "stats").await;
    info!("Cached stats with TTL {:?}", stats_ttl);
}

/// Fetch stats from API and cache both athlete and stats
async fn fetch_and_cache_stats(
    provider: &dyn FitnessProvider,
    cache: &Arc<Cache>,
    athlete_cache_key: &CacheKey,
    tenant_uuid: uuid::Uuid,
    user_uuid: uuid::Uuid,
    provider_name: &str,
) -> Result<UniversalResponse, ProtocolError> {
    let stats = match provider.get_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to fetch stats: {e}")),
                metadata: None,
            });
        }
    };

    // Get athlete to extract athlete_id for caching
    if let Ok(athlete) = provider.get_athlete().await {
        cache_athlete_and_stats(
            cache,
            athlete_cache_key,
            &athlete,
            &stats,
            tenant_uuid,
            user_uuid,
            provider_name,
        )
        .await;
    }

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::to_value(&stats).map_err(|e| {
            ProtocolError::SerializationError(format!("Failed to serialize stats: {e}"))
        })?),
        error: None,
        metadata: Some(create_stats_metadata(user_uuid, tenant_uuid, false)),
    })
}

/// Handle `get_stats` tool - retrieve user's activity statistics
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_get_stats(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_stats cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(crate::config::environment::default_provider, String::from);

        // Create cache key for stats (need athlete_id from athlete profile)
        let tenant_uuid = request
            .tenant_id
            .as_ref()
            .and_then(|t| {
                uuid::Uuid::parse_str(t)
                    .inspect_err(|e| {
                        debug!(
                            tenant_id_str = %t,
                            error = %e,
                            "Failed to parse tenant ID for cache key - using nil UUID"
                        );
                    })
                    .ok()
            })
            .unwrap_or_else(uuid::Uuid::nil);

        let athlete_cache_key = CacheKey::new(
            tenant_uuid,
            user_uuid,
            provider_name.clone(),
            CacheResource::AthleteProfile,
        );

        // Try to get athlete_id from cache and then stats
        if let Some(athlete_id) =
            try_get_athlete_id_from_cache(&executor.resources.cache, &athlete_cache_key).await
        {
            let stats_cache_key = CacheKey::new(
                tenant_uuid,
                user_uuid,
                provider_name.clone(),
                CacheResource::Stats { athlete_id },
            );

            if let Some(cached_response) = try_get_cached_stats(
                &executor.resources.cache,
                &stats_cache_key,
                user_uuid,
                request.tenant_id.as_ref(),
            )
            .await?
            {
                // Report completion if loaded from cache
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        100.0,
                        Some(100.0),
                        Some("Stats loaded from cache".to_owned()),
                    );
                }
                return Ok(cached_response);
            }
        }

        // Report progress after cache miss
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_stats cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some("Authenticated - fetching stats...".to_owned()),
                    );
                }

                // Check cancellation before fetch
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "get_stats cancelled before fetch".to_owned(),
                        ));
                    }
                }

                let result = fetch_and_cache_stats(
                    provider.as_ref(),
                    &executor.resources.cache,
                    &athlete_cache_key,
                    tenant_uuid,
                    user_uuid,
                    &provider_name,
                )
                .await;

                // Report completion on success
                if result.is_ok() {
                    if let Some(reporter) = &request.progress_reporter {
                        reporter.report(
                            100.0,
                            Some(100.0),
                            Some("Stats fetched successfully".to_owned()),
                        );
                    }
                }

                result
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `analyze_activity` tool - analyze specific activity with intelligence
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_analyze_activity(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_activity cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID and extract activity ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(crate::config::environment::default_provider, String::from);

        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned) // Safe: String ownership needed to avoid borrowing issues
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("activity_id parameter required".to_owned())
            })?;

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                20.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_activity cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        40.0,
                        Some(100.0),
                        Some("Authenticated - fetching activity...".to_owned()),
                    );
                }

                // Check cancellation before provider creation
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "analyze_activity cancelled before fetch".to_owned(),
                        ));
                    }
                }

                // Fetch the specific activity directly - efficient single API call
                match provider.get_activity(&activity_id).await {
                    Ok(_activity) => {
                        // Report progress before analysis
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                60.0,
                                Some(100.0),
                                Some("Activity retrieved - analyzing...".to_owned()),
                            );
                        }

                        // Activity found - process analysis
                        // Note: process_activity_analysis takes ownership of request
                        process_activity_analysis(executor, request, &activity_id, user_uuid).await
                    }
                    Err(e) => {
                        // Activity not found or API error
                        Ok(UniversalResponse {
                            success: false,
                            result: None,
                            error: Some(format!("Activity {activity_id} not found: {e}")),
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "activity_id".to_owned(),
                                    serde_json::Value::String(activity_id.clone()),
                                );
                                map.insert(
                                    "provider".to_owned(),
                                    serde_json::Value::String(provider_name.clone()),
                                );
                                map
                            }),
                        })
                    }
                }
            }
            Err(response) => Ok(response),
        }
    })
}
