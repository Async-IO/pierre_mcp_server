// ABOUTME: USDA FoodData Central API client for nutritional data retrieval
// ABOUTME: Implements food search, detail retrieval, caching, and rate limiting

// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
//! - Mock client for testing
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
//!     api_key: "your_api_key".to_string(),
//!     base_url: "https://api.nal.usda.gov/fdc/v1".to_string(),
//!     cache_ttl_secs: 86400, // 24 hours
//!     rate_limit_per_minute: 30,
//! };
//!
//! let client = UsdaClient::new(config);
//! let results = client.search_foods("apple", 10).await?;
//! # Ok(())
//! # }
//! ```

use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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
            base_url: "https://api.nal.usda.gov/fdc/v1".to_string(),
            cache_ttl_secs: 86400, // 24 hours
            rate_limit_per_minute: 30,
        }
    }
}

/// USDA Food Search Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodSearchResult {
    /// `FoodData` Central ID
    pub fdc_id: u64,
    /// Food description
    pub description: String,
    /// Data type (e.g., "Survey (FNDDS)", "Foundation", "SR Legacy")
    pub data_type: String,
    /// Publication date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<String>,
    /// Brand owner (for branded foods)
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_size_unit: Option<String>,
}

/// USDA API search response
#[derive(Debug, Deserialize)]
struct SearchResponse {
    foods: Vec<FoodSearchResult>,
    // Pagination fields not currently exposed but part of USDA API contract
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
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// USDA `FoodData` Central API Client
pub struct UsdaClient {
    config: UsdaClientConfig,
    http_client: reqwest::Client,
    search_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<FoodSearchResult>>>>>,
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
            http_client: reqwest::Client::new(),
            search_cache: Arc::new(RwLock::new(HashMap::new())),
            details_cache: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
        }
    }

    /// Search for foods by query string
    ///
    /// # Arguments
    /// * `query` - Search query (e.g., "apple", "chicken breast")
    /// * `page_size` - Number of results to return (1-200)
    ///
    /// # Returns
    /// List of matching foods with basic information
    ///
    /// # Errors
    /// Returns error if API request fails or rate limit is exceeded
    pub async fn search_foods(
        &self,
        query: &str,
        page_size: u32,
    ) -> Result<Vec<FoodSearchResult>, AppError> {
        if query.is_empty() {
            return Err(AppError::invalid_input("Search query cannot be empty"));
        }

        if page_size == 0 || page_size > 200 {
            return Err(AppError::invalid_input(
                "Page size must be between 1 and 200",
            ));
        }

        // Check cache first
        let cache_key = format!("{query}:{page_size}");
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
                ("api_key", &self.config.api_key),
            ])
            .send()
            .await
            .map_err(|e| AppError::external_service("USDA API", e.to_string()))?;

        if !response.status().is_success() {
            return Err(AppError::external_service(
                "USDA API",
                format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                ),
            ));
        }

        let search_response: SearchResponse = response.json().await.map_err(|e| {
            AppError::external_service("USDA API", format!("JSON parse error: {e}"))
        })?;

        // Cache the results
        {
            let mut cache = self.search_cache.write().await;
            cache.insert(
                cache_key,
                CacheEntry {
                    data: search_response.foods.clone(),
                    expires_at: Instant::now() + Duration::from_secs(self.config.cache_ttl_secs),
                },
            );
        }

        Ok(search_response.foods)
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
            return Err(AppError::external_service(
                "USDA API",
                format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                ),
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

/// Mock USDA client for testing (no API calls)
pub struct MockUsdaClient {
    mock_foods: HashMap<u64, FoodDetails>,
}

impl MockUsdaClient {
    /// Create a new mock client with predefined test data
    #[must_use]
    pub fn new() -> Self {
        let mut mock_foods = HashMap::new();

        // Mock food: Chicken breast (FDC ID: 171_477)
        mock_foods.insert(
            171_477,
            FoodDetails {
                fdc_id: 171_477,
                description: "Chicken, breast, meat only, cooked, roasted".to_string(),
                data_type: "SR Legacy".to_string(),
                food_nutrients: vec![
                    FoodNutrient {
                        nutrient_id: 1003,
                        nutrient_name: "Protein".to_string(),
                        unit_name: "g".to_string(),
                        amount: 31.02,
                    },
                    FoodNutrient {
                        nutrient_id: 1004,
                        nutrient_name: "Total lipid (fat)".to_string(),
                        unit_name: "g".to_string(),
                        amount: 3.57,
                    },
                    FoodNutrient {
                        nutrient_id: 1005,
                        nutrient_name: "Carbohydrate, by difference".to_string(),
                        unit_name: "g".to_string(),
                        amount: 0.0,
                    },
                    FoodNutrient {
                        nutrient_id: 1008,
                        nutrient_name: "Energy".to_string(),
                        unit_name: "kcal".to_string(),
                        amount: 165.0,
                    },
                ],
                serving_size: Some(100.0),
                serving_size_unit: Some("g".to_string()),
            },
        );

        // Mock food: Apple (FDC ID: 171_688)
        mock_foods.insert(
            171_688,
            FoodDetails {
                fdc_id: 171_688,
                description: "Apples, raw, with skin".to_string(),
                data_type: "SR Legacy".to_string(),
                food_nutrients: vec![
                    FoodNutrient {
                        nutrient_id: 1003,
                        nutrient_name: "Protein".to_string(),
                        unit_name: "g".to_string(),
                        amount: 0.26,
                    },
                    FoodNutrient {
                        nutrient_id: 1004,
                        nutrient_name: "Total lipid (fat)".to_string(),
                        unit_name: "g".to_string(),
                        amount: 0.17,
                    },
                    FoodNutrient {
                        nutrient_id: 1005,
                        nutrient_name: "Carbohydrate, by difference".to_string(),
                        unit_name: "g".to_string(),
                        amount: 13.81,
                    },
                    FoodNutrient {
                        nutrient_id: 1008,
                        nutrient_name: "Energy".to_string(),
                        unit_name: "kcal".to_string(),
                        amount: 52.0,
                    },
                ],
                serving_size: Some(182.0),
                serving_size_unit: Some("g".to_string()),
            },
        );

        Self { mock_foods }
    }

    /// Mock search implementation
    ///
    /// # Errors
    /// Returns `AppError::InvalidInput` if query is empty
    pub fn search_foods(
        &self,
        query: &str,
        _page_size: u32,
    ) -> Result<Vec<FoodSearchResult>, AppError> {
        if query.is_empty() {
            return Err(AppError::invalid_input("Search query cannot be empty"));
        }

        let query_lower = query.to_lowercase();
        let results: Vec<FoodSearchResult> = self
            .mock_foods
            .values()
            .filter(|food| food.description.to_lowercase().contains(&query_lower))
            .map(|food| FoodSearchResult {
                fdc_id: food.fdc_id,
                description: food.description.clone(),
                data_type: food.data_type.clone(),
                publication_date: None,
                brand_owner: None,
            })
            .collect();

        Ok(results)
    }

    /// Mock details implementation
    ///
    /// # Errors
    /// Returns `AppError::NotFound` if food with given FDC ID doesn't exist
    pub fn get_food_details(&self, fdc_id: u64) -> Result<FoodDetails, AppError> {
        self.mock_foods
            .get(&fdc_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("Food with FDC ID {fdc_id}")))
    }
}

impl Default for MockUsdaClient {
    fn default() -> Self {
        Self::new()
    }
}
