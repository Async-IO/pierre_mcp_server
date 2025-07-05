// ABOUTME: Location and geographic intelligence for activity analysis and environmental context
// ABOUTME: Provides geocoding, elevation data, route analysis, and location-based insights
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
                Client::new()
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
                return Ok(entry.location.clone());
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
        let city = address
            .city
            .clone()
            .or_else(|| address.town.clone())
            .or_else(|| address.village.clone())
            .or_else(|| address.suburb.clone());

        // Determine region (state/province)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_location_service_creation() {
        let service = LocationService::new();
        tracing::trace!(
            "Created location service for testing: {:?}",
            std::ptr::addr_of!(service)
        );
        assert_eq!(service.cache.len(), 0);
        assert_eq!(service.cache_duration, Duration::from_secs(24 * 60 * 60));
    }

    #[test]
    fn test_parse_nominatim_response_with_trail() {
        let service = LocationService::new();
        tracing::trace!(
            "Created location service for testing: {:?}",
            std::ptr::addr_of!(service)
        );
        let response = NominatimResponse {
            place_id: 12345,
            licence: "test".into(),
            osm_type: "way".into(),
            osm_id: 54321,
            lat: "45.5017".into(),
            lon: "-73.5673".into(),
            display_name: "Trail de la Montagne, Montreal, Quebec, Canada".into(),
            address: NominatimAddress {
                house_number: None,
                road: Some("Trail de la Montagne".into()),
                suburb: None,
                city: Some("Montreal".into()),
                town: None,
                village: None,
                county: None,
                state: Some("Quebec".into()),
                postcode: Some("H3A 0G4".into()),
                country: Some("Canada".into()),
                country_code: Some("ca".into()),
                amenity: None,
                natural: Some("peak".into()),
                tourism: None,
                leisure: None,
            },
            boundingbox: vec![
                "45.5000".into(),
                "45.5100".into(),
                "-73.5700".into(),
                "-73.5600".into(),
            ],
        };

        let location = LocationService::parse_nominatim_response(&response, 45.5017, -73.5673);

        assert_eq!(location.city, Some("Montreal".into()));
        assert_eq!(location.region, Some("Quebec".into()));
        assert_eq!(location.country, Some("Canada".into()));
        assert_eq!(location.trail_name, Some("Trail de la Montagne".into()));
        assert_eq!(location.natural, Some("peak".into()));
        assert_eq!(location.coordinates, (45.5017, -73.5673));
        assert_eq!(
            location.display_name,
            "Trail de la Montagne, Montreal, Quebec, Canada"
        );
    }

    #[test]
    fn test_parse_nominatim_response_saint_hippolyte() {
        let service = LocationService::new();
        tracing::trace!(
            "Created location service for testing: {:?}",
            std::ptr::addr_of!(service)
        );
        let response = NominatimResponse {
            place_id: 67890,
            licence: "test".into(),
            osm_type: "relation".into(),
            osm_id: 98765,
            lat: "45.9224".into(),
            lon: "-74.0679".into(),
            display_name: "Saint-Hippolyte, La Rivière-du-Nord, Laurentides, Québec, Canada"
                .to_string(),
            address: NominatimAddress {
                house_number: None,
                road: None,
                suburb: None,
                city: Some("Saint-Hippolyte".into()),
                town: None,
                village: None,
                county: Some("La Rivière-du-Nord".into()),
                state: Some("Québec".into()),
                postcode: None,
                country: Some("Canada".into()),
                country_code: Some("ca".into()),
                amenity: None,
                natural: None,
                tourism: None,
                leisure: None,
            },
            boundingbox: vec![
                "45.9000".into(),
                "45.9500".into(),
                "-74.1000".into(),
                "-74.0000".into(),
            ],
        };

        let location = LocationService::parse_nominatim_response(&response, 45.9224, -74.0679);

        assert_eq!(location.city, Some("Saint-Hippolyte".into()));
        assert_eq!(location.region, Some("Québec".into()));
        assert_eq!(location.country, Some("Canada".into()));
        assert_eq!(location.trail_name, None); // No trail in this response
        assert_eq!(location.coordinates, (45.9224, -74.0679));
        assert!(location.display_name.contains("Saint-Hippolyte"));
    }

    #[test]
    fn test_parse_nominatim_response_with_path() {
        let service = LocationService::new();
        tracing::trace!(
            "Created location service for testing: {:?}",
            std::ptr::addr_of!(service)
        );
        let response = NominatimResponse {
            place_id: 11111,
            licence: "test".into(),
            osm_type: "way".into(),
            osm_id: 22222,
            lat: "45.4000".into(),
            lon: "-73.6000".into(),
            display_name: "Sentier de la Nature, Parc du Mont-Royal, Montreal, Quebec, Canada"
                .to_string(),
            address: NominatimAddress {
                house_number: None,
                road: Some("Sentier de la Nature".into()), // French trail name
                suburb: Some("Parc du Mont-Royal".into()),
                city: Some("Montreal".into()),
                town: None,
                village: None,
                county: None,
                state: Some("Quebec".into()),
                postcode: None,
                country: Some("Canada".into()),
                country_code: Some("ca".into()),
                amenity: None,
                natural: Some("forest".into()),
                tourism: None,
                leisure: None,
            },
            boundingbox: vec![
                "45.3900".into(),
                "45.4100".into(),
                "-73.6100".into(),
                "-73.5900".into(),
            ],
        };

        let location = LocationService::parse_nominatim_response(&response, 45.4000, -73.6000);

        assert_eq!(location.city, Some("Montreal".into()));
        assert_eq!(location.region, Some("Quebec".into()));
        assert_eq!(location.country, Some("Canada".into()));
        assert_eq!(location.trail_name, Some("Sentier de la Nature".into())); // French "sentier" = trail
        assert_eq!(location.natural, Some("forest".into()));
    }

    #[test]
    fn test_parse_nominatim_response_city_fallback() {
        let service = LocationService::new();
        tracing::trace!(
            "Created location service for testing: {:?}",
            std::ptr::addr_of!(service)
        );
        let response = NominatimResponse {
            place_id: 33333,
            licence: "test".into(),
            osm_type: "node".into(),
            osm_id: 44444,
            lat: "45.5000".into(),
            lon: "-73.5500".into(),
            display_name: "Downtown, Montreal, Quebec, Canada".into(),
            address: NominatimAddress {
                house_number: None,
                road: None,
                suburb: Some("Downtown".into()),
                city: None,                    // No city field
                town: Some("Montreal".into()), // Use town instead
                village: None,
                county: None,
                state: Some("Quebec".into()),
                postcode: Some("H3B 4W5".into()),
                country: Some("Canada".into()),
                country_code: Some("ca".into()),
                amenity: None,
                natural: None,
                tourism: None,
                leisure: None,
            },
            boundingbox: vec![
                "45.4900".into(),
                "45.5100".into(),
                "-73.5600".into(),
                "-73.5400".into(),
            ],
        };

        let location = LocationService::parse_nominatim_response(&response, 45.5000, -73.5500);

        // Should fall back to town when city is not available
        assert_eq!(location.city, Some("Montreal".into()));
        assert_eq!(location.region, Some("Quebec".into()));
        assert_eq!(location.country, Some("Canada".into()));
        assert_eq!(location.trail_name, None);
    }

    #[test]
    fn test_cache_stats() {
        let service = LocationService::new();
        tracing::trace!(
            "Created location service for testing: {:?}",
            std::ptr::addr_of!(service)
        );
        let (total, expired) = service.get_cache_stats();
        assert_eq!(total, 0);
        assert_eq!(expired, 0);
    }

    #[test]
    fn test_cache_expiration_logic() {
        let mut service = LocationService::new();

        // Manually add a cache entry
        let location_data = LocationData {
            city: Some("Test City".into()),
            region: Some("Test Region".into()),
            country: Some("Test Country".into()),
            trail_name: None,
            amenity: None,
            natural: None,
            tourism: None,
            leisure: None,
            display_name: "Test Location".into(),
            coordinates: (45.0, -73.0),
        };

        let cache_entry = CacheEntry {
            location: location_data,
            timestamp: SystemTime::now(),
        };

        service
            .cache
            .insert("45.000000,-73.000000".into(), cache_entry);

        let (total, expired) = service.get_cache_stats();
        assert_eq!(total, 1);
        assert_eq!(expired, 0); // Should not be expired immediately
    }

    #[test]
    fn test_trail_name_detection() {
        let service = LocationService::new();
        tracing::trace!(
            "Created location service for testing: {:?}",
            std::ptr::addr_of!(service)
        );

        // Test various trail naming patterns
        let test_cases = vec![
            ("Mountain Trail", true),
            ("Forest Path", true),
            ("Hiking Route", true),
            ("Sentier du Lac", true),  // French
            ("Chemin des Bois", true), // French
            ("Main Street", false),
            ("Highway 401", false),
            ("Boulevard Saint-Laurent", false),
        ];

        for (road_name, should_be_trail) in test_cases {
            let response = NominatimResponse {
                place_id: 1,
                licence: "test".into(),
                osm_type: "way".into(),
                osm_id: 1,
                lat: "45.0".into(),
                lon: "-73.0".into(),
                display_name: format!("{road_name}, Test City, Test Region, Test Country"),
                address: NominatimAddress {
                    house_number: None,
                    road: Some(road_name.to_string()),
                    suburb: None,
                    city: Some("Test City".into()),
                    town: None,
                    village: None,
                    county: None,
                    state: Some("Test Region".into()),
                    postcode: None,
                    country: Some("Test Country".into()),
                    country_code: Some("tc".into()),
                    amenity: None,
                    natural: None,
                    tourism: None,
                    leisure: None,
                },
                boundingbox: vec!["44.9".into(), "45.1".into(), "-73.1".into(), "-72.9".into()],
            };

            let location = LocationService::parse_nominatim_response(&response, 45.0, -73.0);

            if should_be_trail {
                assert_eq!(
                    location.trail_name,
                    Some(road_name.to_string()),
                    "Expected '{road_name}' to be detected as a trail"
                );
            } else {
                assert_eq!(
                    location.trail_name, None,
                    "Expected '{road_name}' to NOT be detected as a trail"
                );
            }
        }
    }
}
