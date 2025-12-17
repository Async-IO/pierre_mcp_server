// ABOUTME: Location and geographic intelligence for activity analysis and environmental context
// ABOUTME: Provides geocoding, elevation data, route analysis, and location-based insights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// NOTE: All remaining `.clone()` calls in this file are Safe - they are necessary for:
// - HTTP client Arc sharing for geocoding requests
// - Cache key and data ownership transfers for async operations
// - Address field Option chains for comprehensive location parsing
use crate::constants::project::user_agent;
use crate::errors::{AppError, AppResult};
use crate::utils::http_client::shared_client;
use crate::utils::route_timeout::geocoding_timeout_duration;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::{debug, info, instrument, warn};

/// Geographic location data with rich context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationData {
    /// City name if available
    pub city: Option<String>,
    /// Region/state name
    pub region: Option<String>,
    /// Country name
    pub country: Option<String>,
    /// Trail or path name for outdoor activities
    pub trail_name: Option<String>,
    /// Nearby amenity (park, restaurant, etc.)
    pub amenity: Option<String>,
    /// Natural feature (lake, mountain, forest, etc.)
    pub natural: Option<String>,
    /// Tourism destination name
    pub tourism: Option<String>,
    /// Leisure facility (sports center, gym, etc.)
    pub leisure: Option<String>,
    /// Human-readable location description
    pub display_name: String,
    /// Latitude and longitude coordinates
    pub coordinates: (f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NominatimResponse {
    place_id: u64,
    licence: String,
    osm_type: String,
    osm_id: u64,
    lat: String,
    lon: String,
    display_name: String,
    address: NominatimAddress,
    boundingbox: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NominatimAddress {
    house_number: Option<String>,
    road: Option<String>,
    suburb: Option<String>,
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    county: Option<String>,
    state: Option<String>,
    postcode: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    amenity: Option<String>,
    natural: Option<String>,
    tourism: Option<String>,
    leisure: Option<String>,
}

#[derive(Debug)]
struct CacheEntry {
    location: LocationData,
    timestamp: SystemTime,
}

/// Service for geocoding and location data enrichment
pub struct LocationService {
    client: Client,
    cache: HashMap<String, CacheEntry>,
    cache_duration: Duration,
    base_url: String,
    enabled: bool,
}

impl LocationService {
    /// Create a new location service with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config("https://nominatim.openstreetmap.org".into(), true)
    }

    /// Creates a location service with custom configuration
    #[must_use]
    pub fn with_config(base_url: String, enabled: bool) -> Self {
        let client = Client::builder()
            .user_agent(user_agent())
            .timeout(geocoding_timeout_duration())
            .build()
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to create HTTP client for location service: {}, using default",
                    e
                );
                shared_client().clone() // Safe: Arc clone for HTTP client sharing
            });

        Self {
            client,
            cache: HashMap::new(),
            cache_duration: Duration::from_secs(24 * 60 * 60), // 24 hours
            base_url,
            enabled,
        }
    }

    /// Create disabled location response
    fn disabled_location(latitude: f64, longitude: f64) -> LocationData {
        LocationData {
            city: None,
            country: None,
            region: None,
            trail_name: None,
            amenity: None,
            natural: None,
            tourism: None,
            leisure: None,
            display_name: "Location service disabled".into(),
            coordinates: (latitude, longitude),
        }
    }

    /// Check cache for existing location data
    fn check_cache(&mut self, cache_key: &str) -> Option<LocationData> {
        if let Some(entry) = self.cache.get(cache_key) {
            if entry.timestamp.elapsed().unwrap_or(Duration::from_secs(0)) < self.cache_duration {
                debug!("Using cached location data for {}", cache_key);
                return Some(entry.location.clone()); // Safe: LocationResult ownership from cache
            }
            debug!("Cache entry expired for {}", cache_key);
            self.cache.remove(cache_key);
        }
        None
    }

    /// Fetch location from geocoding API
    #[instrument(
        skip(self),
        fields(
            service = "nominatim",
            api_call = "reverse_geocode",
            lat = %latitude,
            lon = %longitude,
        )
    )]
    async fn fetch_from_api(&self, latitude: f64, longitude: f64) -> AppResult<NominatimResponse> {
        let url = format!(
            "{}/reverse?format=json&lat={}&lon={}&zoom=14&addressdetails=1",
            self.base_url, latitude, longitude
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            AppError::external_service(
                "Nominatim",
                format!("Failed to send reverse geocoding request: {e}"),
            )
        })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::external_service(
                "Nominatim",
                format!("Reverse geocoding API returned status: {status}"),
            ));
        }

        response.json().await.map_err(|e| {
            AppError::external_service(
                "Nominatim",
                format!("Failed to parse reverse geocoding response: {e}"),
            )
        })
    }

    /// Get location information from GPS coordinates
    ///
    /// # Errors
    ///
    /// Returns an error if the reverse geocoding request fails or the response cannot be parsed
    #[instrument(
        skip(self),
        fields(
            service = "location",
            operation = "get_location",
            lat = %latitude,
            lon = %longitude,
        )
    )]
    pub async fn get_location_from_coordinates(
        &mut self,
        latitude: f64,
        longitude: f64,
    ) -> AppResult<LocationData> {
        if !self.enabled {
            return Ok(Self::disabled_location(latitude, longitude));
        }

        let cache_key = format!("{latitude:.6},{longitude:.6}");

        if let Some(cached) = self.check_cache(&cache_key) {
            return Ok(cached);
        }

        info!(
            "Fetching location data for coordinates: {}, {}",
            latitude, longitude
        );

        let nominatim_response = self.fetch_from_api(latitude, longitude).await?;
        let location_data =
            Self::parse_nominatim_response(&nominatim_response, latitude, longitude);

        // Cache the result
        self.cache.insert(
            cache_key.clone(),
            CacheEntry {
                location: location_data.clone(),
                timestamp: SystemTime::now(),
            },
        );

        debug!(
            "Cached location data for {}: {:?}",
            cache_key, location_data
        );

        Ok(location_data)
    }

    fn parse_nominatim_response(
        response: &NominatimResponse,
        latitude: f64,
        longitude: f64,
    ) -> LocationData {
        let address = &response.address;

        // Determine city from various possible fields
        // Safe: String clones needed for Option ownership transfers in city resolution chain
        let city = address
            .city
            .clone()
            .or_else(|| address.town.clone())
            .or_else(|| address.village.clone())
            .or_else(|| address.suburb.clone());

        // Determine region (state/province)
        // Safe: String clones needed for Option ownership transfers
        let region = address.state.clone().or_else(|| address.county.clone());

        // Extract trail/route information from road or natural features
        let trail_name = address.road.as_ref().and_then(|road| {
            // Check if it's a trail, path, or route
            if road.to_lowercase().contains("trail") 
                || road.to_lowercase().contains("path")
                || road.to_lowercase().contains("route")
                || road.to_lowercase().contains("sentier") // French
                || road.to_lowercase().contains("chemin")
            // French
            {
                Some(road.clone())
            } else {
                None
            }
        });
        LocationData {
            city,
            region,
            country: address.country.clone(),
            trail_name,
            amenity: address.amenity.clone(),
            natural: address.natural.clone(),
            tourism: address.tourism.clone(),
            leisure: address.leisure.clone(),
            display_name: response.display_name.clone(),
            coordinates: (latitude, longitude),
        }
    }

    /// Returns cache statistics as `(total_entries, expired_entries)`
    #[must_use]
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let total_entries = self.cache.len();
        let expired_entries = self
            .cache
            .values()
            .filter(|entry| {
                entry.timestamp.elapsed().unwrap_or(Duration::from_secs(0)) >= self.cache_duration
            })
            .count();

        (total_entries, expired_entries)
    }

    /// Removes expired entries from the location cache
    pub fn clear_expired_cache(&mut self) {
        let now = SystemTime::now();
        self.cache.retain(|_, entry| {
            now.duration_since(entry.timestamp)
                .unwrap_or(Duration::from_secs(0))
                < self.cache_duration
        });
    }
}

impl Default for LocationService {
    fn default() -> Self {
        Self::new()
    }
}
