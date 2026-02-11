// ABOUTME: USDA FoodData Central API client for nutritional data retrieval
// ABOUTME: Implements food search, detail retrieval, caching, and rate limiting

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! USDA `FoodData` Central API Client
//!
//! This module provides a client for the USDA `FoodData` Central API, which offers
//! comprehensive nutritional information for foods. The API is free and requires
//! no authentication beyond an API key.
//!
//! # Features
//! - Food search with pagination
//! - Detailed food information retrieval
//! - 24-hour caching to minimize API calls
//! - Rate limiting (30 requests per minute)
//!
//! # API Reference
//! USDA `FoodData` Central API: <https://fdc.nal.usda.gov/api-guide.html>
//!
//! # Example
//! ```rust,no_run
//! use pierre_mcp_server::external::usda_client::{UsdaClient, UsdaClientConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = UsdaClientConfig {
//!     api_key: "your_api_key".to_owned(),
//!     base_url: "https://api.nal.usda.gov/fdc/v1".to_owned(),
//!     cache_ttl_secs: 86400, // 24 hours
//!     rate_limit_per_minute: 30,
//! };
//!
//! let client = UsdaClient::new(config);
//! let results = client.search_foods("apple", 10, 1).await?;
//! # Ok(())
//! # }
//! ```

use crate::errors::AppError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::{sync::RwLock, time::sleep};

/// USDA API client configuration
#[derive(Debug, Clone)]
pub struct UsdaClientConfig {
    /// USDA API key (free from <https://fdc.nal.usda.gov/api-key-signup.html>)
    pub api_key: String,
    /// Base URL for USDA API (default: <https://api.nal.usda.gov/fdc/v1>)
    pub base_url: String,
    /// Cache TTL in seconds (default: 86400 = 24 hours)
    pub cache_ttl_secs: u64,
    /// Rate limit per minute (default: 30)
    pub rate_limit_per_minute: u32,
}

impl Default for UsdaClientConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.nal.usda.gov/fdc/v1".to_owned(),
            cache_ttl_secs: 86400, // 24 hours
            rate_limit_per_minute: 30,
        }
    }
}

/// USDA Food Search Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodSearchResult {
    /// `FoodData` Central ID
    #[serde(rename = "fdcId")]
    pub fdc_id: u64,
    /// Food description
    pub description: String,
    /// Data type (e.g., "Survey (FNDDS)", "Foundation", "SR Legacy")
    #[serde(rename = "dataType")]
    pub data_type: String,
    /// Publication date
    #[serde(rename = "publishedDate", skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<String>,
    /// Brand owner (for branded foods)
    #[serde(rename = "brandOwner", skip_serializing_if = "Option::is_none")]
    pub brand_owner: Option<String>,
}

/// USDA Food Nutrient
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodNutrient {
    /// Nutrient ID
    pub nutrient_id: u32,
    /// Nutrient name (e.g., "Protein", "Energy")
    pub nutrient_name: String,
    /// Nutrient unit (e.g., "g", "kcal", "mg")
    pub unit_name: String,
    /// Amount per 100g
    pub amount: f64,
}

/// Detailed USDA Food Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodDetails {
    /// `FoodData` Central ID
    pub fdc_id: u64,
    /// Food description
    pub description: String,
    /// Data type
    pub data_type: String,
    /// List of nutrients with amounts
    pub food_nutrients: Vec<FoodNutrient>,
    /// Portion information (serving size)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_size: Option<f64>,
    /// Unit of measurement for serving size (e.g., "g", "cup")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_size_unit: Option<String>,
}

/// Public search response with pagination metadata
#[derive(Debug, Clone, Serialize)]
pub struct FoodSearchPaginatedResponse {
    /// List of matching foods
    pub foods: Vec<FoodSearchResult>,
    /// Total number of matching foods in database
    pub total_hits: u32,
    /// Current page number (1-indexed)
    pub current_page: u32,
    /// Total number of pages available
    pub total_pages: u32,
}

/// USDA API search response (internal)
#[derive(Debug, Deserialize)]
struct SearchResponse {
    foods: Vec<FoodSearchResult>,
    #[serde(rename = "totalHits")]
    total_hits: Option<u32>,
    #[serde(rename = "currentPage")]
    current_page: Option<u32>,
    #[serde(rename = "totalPages")]
    total_pages: Option<u32>,
}

/// USDA API food details response
#[derive(Debug, Deserialize)]
struct FoodDetailsResponse {
    #[serde(rename = "fdcId")]
    fdc_id: u64,
    description: String,
    #[serde(rename = "dataType")]
    data_type: String,
    #[serde(rename = "foodNutrients")]
    food_nutrients: Vec<FoodNutrientResponse>,
    #[serde(rename = "servingSize")]
    serving_size: Option<f64>,
    #[serde(rename = "servingSizeUnit")]
    serving_size_unit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FoodNutrientResponse {
    nutrient: Option<NutrientInfo>,
    amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct NutrientInfo {
    id: u32,
    name: String,
    #[serde(rename = "unitName")]
    unit_name: String,
}

/// Cache entry with expiration
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

/// Rate limiter for API requests
#[derive(Debug)]
struct RateLimiter {
    requests: Vec<Instant>,
    limit: u32,
    window: Duration,
}

impl RateLimiter {
    const fn new(limit: u32, window: Duration) -> Self {
        Self {
            requests: Vec::new(),
            limit,
            window,
        }
    }

    /// Check if a request can be made, removing expired entries
    fn can_request(&mut self) -> bool {
        let now = Instant::now();
        self.requests
            .retain(|&t| now.duration_since(t) < self.window);
        self.requests.len() < self.limit as usize
    }

    /// Record a new request
    fn record_request(&mut self) {
        self.requests.push(Instant::now());
    }

    /// Wait until a request can be made
    async fn wait_if_needed(&mut self) {
        while !self.can_request() {
            // Sleep for 1 second and check again
            sleep(Duration::from_secs(1)).await;
        }
    }
}

/// USDA `FoodData` Central API Client
pub struct UsdaClient {
    config: UsdaClientConfig,
    http_client: reqwest::Client,
    search_cache: Arc<RwLock<HashMap<String, CacheEntry<FoodSearchPaginatedResponse>>>>,
    details_cache: Arc<RwLock<HashMap<u64, CacheEntry<FoodDetails>>>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl UsdaClient {
    /// Create a new USDA API client
    #[must_use]
    pub fn new(config: UsdaClientConfig) -> Self {
        let rate_limiter = RateLimiter::new(config.rate_limit_per_minute, Duration::from_secs(60));

        Self {
            config,
            http_client: Client::new(),
            search_cache: Arc::new(RwLock::new(HashMap::new())),
            details_cache: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
        }
    }

    /// Search for foods by query string with pagination
    ///
    /// # Arguments
    /// * `query` - Search query (e.g., "apple", "chicken breast")
    /// * `page_size` - Number of results per page (1-200)
    /// * `page_number` - Page number to retrieve (1-indexed, default: 1)
    ///
    /// # Returns
    /// Paginated response with foods and pagination metadata
    ///
    /// # Errors
    /// Returns error if API request fails or rate limit is exceeded
    pub async fn search_foods(
        &self,
        query: &str,
        page_size: u32,
        page_number: u32,
    ) -> Result<FoodSearchPaginatedResponse, AppError> {
        if query.is_empty() {
            return Err(AppError::invalid_input("Search query cannot be empty"));
        }

        if page_size == 0 || page_size > 200 {
            return Err(AppError::invalid_input(
                "Page size must be between 1 and 200",
            ));
        }

        if page_number == 0 {
            return Err(AppError::invalid_input(
                "Page number must be at least 1 (1-indexed)",
            ));
        }

        let page_num = page_number;

        // Check cache first
        let cache_key = format!("{query}:{page_size}:{page_num}");
        {
            let cache = self.search_cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                if Instant::now() < entry.expires_at {
                    return Ok(entry.data.clone());
                }
            }
        }

        // Wait for rate limit if needed
        {
            let mut limiter = self.rate_limiter.write().await;
            limiter.wait_if_needed().await;
            limiter.record_request();
        }

        // Make API request
        let url = format!("{}/foods/search", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .query(&[
                ("query", query),
                ("pageSize", &page_size.to_string()),
                ("pageNumber", &page_num.to_string()),
                ("api_key", &self.config.api_key),
            ])
            .send()
            .await
            .map_err(|e| AppError::external_service("USDA API", e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::external_service(
                "USDA API",
                format!("Food search request failed with HTTP {status}"),
            ));
        }

        let search_response: SearchResponse = response.json().await.map_err(|e| {
            AppError::external_service("USDA API", format!("JSON parse error: {e}"))
        })?;

        // Build paginated response
        let paginated_response = FoodSearchPaginatedResponse {
            foods: search_response.foods,
            total_hits: search_response.total_hits.unwrap_or(0),
            current_page: search_response.current_page.unwrap_or(page_num),
            total_pages: search_response.total_pages.unwrap_or(1),
        };

        // Cache the results
        {
            let mut cache = self.search_cache.write().await;
            cache.insert(
                cache_key,
                CacheEntry {
                    data: paginated_response.clone(),
                    expires_at: Instant::now() + Duration::from_secs(self.config.cache_ttl_secs),
                },
            );
        }

        Ok(paginated_response)
    }

    /// Get detailed information for a specific food by FDC ID
    ///
    /// # Arguments
    /// * `fdc_id` - `FoodData` Central ID
    ///
    /// # Returns
    /// Detailed food information including all nutrients
    ///
    /// # Errors
    /// Returns error if API request fails or food not found
    pub async fn get_food_details(&self, fdc_id: u64) -> Result<FoodDetails, AppError> {
        // Check cache first
        {
            let cache = self.details_cache.read().await;
            if let Some(entry) = cache.get(&fdc_id) {
                if Instant::now() < entry.expires_at {
                    return Ok(entry.data.clone());
                }
            }
        }

        // Wait for rate limit if needed
        {
            let mut limiter = self.rate_limiter.write().await;
            limiter.wait_if_needed().await;
            limiter.record_request();
        }

        // Make API request
        let url = format!("{}/food/{fdc_id}", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .query(&[("api_key", &self.config.api_key)])
            .send()
            .await
            .map_err(|e| AppError::external_service("USDA API", e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::external_service(
                "USDA API",
                format!("Food details request failed with HTTP {status}"),
            ));
        }

        let details_response: FoodDetailsResponse = response.json().await.map_err(|e| {
            AppError::external_service("USDA API", format!("JSON parse error: {e}"))
        })?;

        // Convert response to our format
        let food_nutrients: Vec<FoodNutrient> = details_response
            .food_nutrients
            .into_iter()
            .filter_map(|n| {
                let nutrient = n.nutrient?;
                Some(FoodNutrient {
                    nutrient_id: nutrient.id,
                    nutrient_name: nutrient.name,
                    unit_name: nutrient.unit_name,
                    amount: n.amount.unwrap_or(0.0),
                })
            })
            .collect();

        let food_details = FoodDetails {
            fdc_id: details_response.fdc_id,
            description: details_response.description,
            data_type: details_response.data_type,
            food_nutrients,
            serving_size: details_response.serving_size,
            serving_size_unit: details_response.serving_size_unit,
        };

        // Cache the results
        {
            let mut cache = self.details_cache.write().await;
            cache.insert(
                fdc_id,
                CacheEntry {
                    data: food_details.clone(),
                    expires_at: Instant::now() + Duration::from_secs(self.config.cache_ttl_secs),
                },
            );
        }

        Ok(food_details)
    }

    /// Clear all caches (useful for testing)
    pub async fn clear_caches(&self) {
        self.search_cache.write().await.clear();
        self.details_cache.write().await.clear();
    }

    /// Get cache statistics (useful for monitoring)
    pub async fn cache_stats(&self) -> (usize, usize) {
        let search_count = self.search_cache.read().await.len();
        let details_count = self.details_cache.read().await.len();
        (search_count, details_count)
    }
}
