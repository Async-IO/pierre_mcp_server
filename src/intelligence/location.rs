// ABOUTME: Location and geographic intelligence for activity analysis and environmental context
// ABOUTME: Provides geocoding, elevation data, route analysis, and location-based insights
//
// NOTE: All remaining `.clone()` calls in this file are Safe - they are necessary for:
// - HTTP client Arc sharing for geocoding requests
// - Cache key and data ownership transfers for async operations
// - Address field Option chains for comprehensive location parsing
use crate::utils::http_client::shared_client;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationData {
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub trail_name: Option<String>,
    pub amenity: Option<String>,
    pub natural: Option<String>,
    pub tourism: Option<String>,
    pub leisure: Option<String>,
    pub display_name: String,
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

pub struct LocationService {
    client: Client,
    cache: HashMap<String, CacheEntry>,
    cache_duration: Duration,
    base_url: String,
    enabled: bool,
}

impl LocationService {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config("https://nominatim.openstreetmap.org".into(), true)
    }

    #[must_use]
    pub fn with_config(base_url: String, enabled: bool) -> Self {
        let client = Client::builder()
            .user_agent("Pierre MCP Server/0.1.0 (https://github.com/jfarcand/pierre_mcp_server)")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!(
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

    /// Get location information from GPS coordinates
    ///
    /// # Errors
    ///
    /// Returns an error if the reverse geocoding request fails or the response cannot be parsed
    pub async fn get_location_from_coordinates(
        &mut self,
        latitude: f64,
        longitude: f64,
    ) -> Result<LocationData> {
        // Check if service is enabled
        if !self.enabled {
            return Ok(LocationData {
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
            });
        }

        let cache_key = format!("{latitude:.6},{longitude:.6}");

        // Check cache first
        if let Some(entry) = self.cache.get(&cache_key) {
            if entry.timestamp.elapsed().unwrap_or(Duration::from_secs(0)) < self.cache_duration {
                debug!("Using cached location data for {}", cache_key);
                return Ok(entry.location.clone()); // Safe: LocationResult ownership from cache
            }
            debug!("Cache entry expired for {}", cache_key);
            self.cache.remove(&cache_key);
        }

        info!(
            "Fetching location data for coordinates: {}, {}",
            latitude, longitude
        );

        // Make request to configured geocoding API
        let url = format!(
            "{}/reverse?format=json&lat={}&lon={}&zoom=14&addressdetails=1",
            self.base_url, latitude, longitude
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send reverse geocoding request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Reverse geocoding API returned status: {}",
                response.status()
            ));
        }

        let nominatim_response: NominatimResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse reverse geocoding response: {}", e))?;

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
