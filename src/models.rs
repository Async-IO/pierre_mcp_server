// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Data Models
//!
//! This module contains the core data structures used throughout the Pierre MCP Server.
//! These models provide a unified representation of fitness data from various providers
//! like Strava, Garmin Connect, and RunKeeper.
//!
//! ## Design Principles
//!
//! - **Provider Agnostic**: Models abstract away provider-specific differences
//! - **Extensible**: Optional fields accommodate different provider capabilities
//! - **Serializable**: All models support JSON serialization for MCP protocol
//! - **Type Safe**: Strong typing prevents common data handling errors
//!
//! ## Core Models
//!
//! - [`Activity`]: Represents a single fitness activity (run, ride, etc.)
//! - [`Athlete`]: User profile information
//! - [`Stats`]: Aggregated fitness statistics
//! - [`PersonalRecord`]: Individual performance records
//! - [`SportType`]: Enumeration of supported activity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a single fitness activity from any provider
///
/// An activity contains all the essential information about a workout,
/// including timing, distance, performance metrics, and metadata.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::{Activity, SportType};
/// use chrono::Utc;
///
/// let activity = Activity {
///     id: "12345".to_string(),
///     name: "Morning Run".to_string(),
///     sport_type: SportType::Run,
///     start_date: Utc::now(),
///     duration_seconds: 1800, // 30 minutes
///     distance_meters: Some(5000.0), // 5km
///     elevation_gain: Some(100.0),
///     average_heart_rate: Some(150),
///     max_heart_rate: Some(175),
///     average_speed: Some(2.78), // m/s
///     max_speed: Some(4.17), // m/s
///     calories: Some(300),
///     start_latitude: Some(45.5017), // Montreal
///     start_longitude: Some(-73.5673),
///     city: Some("Montreal".to_string()),
///     region: Some("Quebec".to_string()),
///     country: Some("Canada".to_string()),
///     trail_name: Some("Mount Royal Trail".to_string()),
///     provider: "strava".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Unique identifier for the activity (provider-specific)
    pub id: String,
    /// Human-readable name/title of the activity
    pub name: String,
    /// Type of sport/activity (run, ride, swim, etc.)
    pub sport_type: SportType,
    /// When the activity started (UTC)
    pub start_date: DateTime<Utc>,
    /// Total duration of the activity in seconds
    pub duration_seconds: u64,
    /// Total distance covered in meters (if applicable)
    pub distance_meters: Option<f64>,
    /// Total elevation gained in meters (if available)
    pub elevation_gain: Option<f64>,
    /// Average heart rate during the activity (BPM)
    pub average_heart_rate: Option<u32>,
    /// Maximum heart rate reached during the activity (BPM)
    pub max_heart_rate: Option<u32>,
    /// Average speed in meters per second
    pub average_speed: Option<f64>,
    /// Maximum speed reached in meters per second
    pub max_speed: Option<f64>,
    /// Estimated calories burned during the activity
    pub calories: Option<u32>,
    /// Starting latitude coordinate (if available)
    pub start_latitude: Option<f64>,
    /// Starting longitude coordinate (if available)
    pub start_longitude: Option<f64>,
    /// Location information extracted from GPS coordinates
    pub city: Option<String>,
    /// Region/state/province where the activity took place
    pub region: Option<String>,
    /// Country where the activity took place
    pub country: Option<String>,
    /// Trail or route name if available (e.g., "Saint-Hippolyte trail")
    pub trail_name: Option<String>,
    /// Source provider of this activity data
    pub provider: String,
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            id: "test_id".to_string(),
            name: "Test Activity".to_string(),
            sport_type: SportType::Run,
            start_date: chrono::Utc::now(),
            duration_seconds: 1800, // 30 minutes
            distance_meters: Some(5000.0), // 5km
            elevation_gain: Some(100.0),
            average_heart_rate: Some(150),
            max_heart_rate: Some(180),
            average_speed: Some(2.78), // ~10 km/h
            max_speed: Some(4.0),
            calories: Some(350),
            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            provider: "test".to_string(),
        }
    }
}

/// Enumeration of supported sport/activity types
///
/// This enum covers the most common fitness activities across all providers.
/// The `Other` variant handles provider-specific activity types that don't
/// map to the standard categories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SportType {
    /// Running activity
    Run,
    /// Cycling/biking activity
    Ride,
    /// Swimming activity
    Swim,
    /// Walking activity
    Walk,
    /// Hiking activity
    Hike,
    
    // Virtual/Indoor activities
    /// Indoor/trainer cycling activity
    VirtualRide,
    /// Treadmill running activity
    VirtualRun,
    /// Generic workout/exercise activity
    Workout,
    /// Yoga practice
    Yoga,
    
    // E-bike and specialty cycling
    /// Electric bike ride
    EbikeRide,
    /// Mountain biking activity
    MountainBike,
    /// Gravel cycling activity
    GravelRide,
    
    // Winter sports
    /// Cross-country skiing
    CrossCountrySkiing,
    /// Alpine/downhill skiing
    AlpineSkiing,
    /// Snowboarding activity
    Snowboarding,
    /// Snowshoeing activity
    Snowshoe,
    /// Ice skating activity
    IceSkating,
    /// Backcountry skiing
    BackcountrySkiing,
    
    // Water sports
    /// Kayaking activity
    Kayaking,
    /// Canoeing activity
    Canoeing,
    /// Rowing activity
    Rowing,
    /// Stand-up paddleboarding
    Paddleboarding,
    /// Surfing activity
    Surfing,
    /// Kitesurfing activity
    Kitesurfing,
    
    // Strength and fitness
    /// Weight/strength training
    StrengthTraining,
    /// CrossFit workout
    Crossfit,
    /// Pilates session
    Pilates,
    
    // Climbing and adventure
    /// Rock climbing activity
    RockClimbing,
    /// Trail running
    TrailRunning,
    
    // Team and racquet sports
    /// Soccer/football
    Soccer,
    /// Basketball
    Basketball,
    /// Tennis
    Tennis,
    /// Golf
    Golf,
    
    // Alternative transport
    /// Skateboarding
    Skateboarding,
    /// Inline skating
    InlineSkating,
    
    /// Other activity type not covered by standard categories
    Other(String),
}

impl SportType {
    /// Create SportType from provider string using configuration mapping
    pub fn from_provider_string(provider_sport: &str, fitness_config: &crate::config::FitnessConfig) -> Self {
        // First check if we have a configured mapping
        if let Some(internal_name) = fitness_config.map_sport_type(provider_sport) {
            return Self::from_internal_string(internal_name);
        }
        
        // Fall back to direct mapping for backward compatibility
        match provider_sport {
            "Run" => SportType::Run,
            "Ride" => SportType::Ride,
            "Swim" => SportType::Swim,
            "Walk" => SportType::Walk,
            "Hike" => SportType::Hike,
            "VirtualRide" => SportType::VirtualRide,
            "VirtualRun" => SportType::VirtualRun,
            "Workout" => SportType::Workout,
            "Yoga" => SportType::Yoga,
            "EBikeRide" => SportType::EbikeRide,
            "MountainBikeRide" => SportType::MountainBike,
            "GravelRide" => SportType::GravelRide,
            "CrossCountrySkiing" => SportType::CrossCountrySkiing,
            "AlpineSkiing" => SportType::AlpineSkiing,
            "Snowboarding" => SportType::Snowboarding,
            "Snowshoe" => SportType::Snowshoe,
            "IceSkate" => SportType::IceSkating,
            "BackcountrySki" => SportType::BackcountrySkiing,
            "Kayaking" => SportType::Kayaking,
            "Canoeing" => SportType::Canoeing,
            "Rowing" => SportType::Rowing,
            "StandUpPaddling" => SportType::Paddleboarding,
            "Surfing" => SportType::Surfing,
            "Kitesurf" => SportType::Kitesurfing,
            "WeightTraining" => SportType::StrengthTraining,
            "Crossfit" => SportType::Crossfit,
            "Pilates" => SportType::Pilates,
            "RockClimbing" => SportType::RockClimbing,
            "TrailRunning" => SportType::TrailRunning,
            "Soccer" => SportType::Soccer,
            "Basketball" => SportType::Basketball,
            "Tennis" => SportType::Tennis,
            "Golf" => SportType::Golf,
            "Skateboard" => SportType::Skateboarding,
            "InlineSkate" => SportType::InlineSkating,
            other => SportType::Other(other.to_string()),
        }
    }
    
    /// Create SportType from internal configuration string
    pub fn from_internal_string(internal_name: &str) -> Self {
        match internal_name {
            "run" => SportType::Run,
            "bike_ride" => SportType::Ride,
            "swim" => SportType::Swim,
            "walk" => SportType::Walk,
            "hike" => SportType::Hike,
            "virtual_ride" => SportType::VirtualRide,
            "virtual_run" => SportType::VirtualRun,
            "workout" => SportType::Workout,
            "yoga" => SportType::Yoga,
            "ebike_ride" => SportType::EbikeRide,
            "mountain_bike" => SportType::MountainBike,
            "gravel_ride" => SportType::GravelRide,
            "cross_country_skiing" => SportType::CrossCountrySkiing,
            "alpine_skiing" => SportType::AlpineSkiing,
            "snowboarding" => SportType::Snowboarding,
            "snowshoe" => SportType::Snowshoe,
            "ice_skating" => SportType::IceSkating,
            "backcountry_skiing" => SportType::BackcountrySkiing,
            "kayaking" => SportType::Kayaking,
            "canoeing" => SportType::Canoeing,
            "rowing" => SportType::Rowing,
            "paddleboarding" => SportType::Paddleboarding,
            "surfing" => SportType::Surfing,
            "kitesurfing" => SportType::Kitesurfing,
            "strength_training" => SportType::StrengthTraining,
            "crossfit" => SportType::Crossfit,
            "pilates" => SportType::Pilates,
            "rock_climbing" => SportType::RockClimbing,
            "trail_running" => SportType::TrailRunning,
            "soccer" => SportType::Soccer,
            "basketball" => SportType::Basketball,
            "tennis" => SportType::Tennis,
            "golf" => SportType::Golf,
            "skateboarding" => SportType::Skateboarding,
            "inline_skating" => SportType::InlineSkating,
            other => SportType::Other(other.to_string()),
        }
    }
    
    /// Get the human-readable name for this sport type
    pub fn display_name(&self) -> &'static str {
        match self {
            SportType::Run => "run",
            SportType::Ride => "bike ride",
            SportType::Swim => "swim",
            SportType::Walk => "walk",
            SportType::Hike => "hike",
            SportType::VirtualRide => "indoor bike ride",
            SportType::VirtualRun => "treadmill run",
            SportType::Workout => "workout",
            SportType::Yoga => "yoga session",
            SportType::EbikeRide => "e-bike ride",
            SportType::MountainBike => "mountain bike ride",
            SportType::GravelRide => "gravel ride",
            SportType::CrossCountrySkiing => "cross-country ski",
            SportType::AlpineSkiing => "alpine ski",
            SportType::Snowboarding => "snowboard session",
            SportType::Snowshoe => "snowshoe hike",
            SportType::IceSkating => "ice skating session",
            SportType::BackcountrySkiing => "backcountry ski",
            SportType::Kayaking => "kayak session",
            SportType::Canoeing => "canoe trip",
            SportType::Rowing => "rowing session",
            SportType::Paddleboarding => "paddleboard session",
            SportType::Surfing => "surf session",
            SportType::Kitesurfing => "kitesurf session",
            SportType::StrengthTraining => "strength training",
            SportType::Crossfit => "CrossFit workout",
            SportType::Pilates => "Pilates session",
            SportType::RockClimbing => "climbing session",
            SportType::TrailRunning => "trail run",
            SportType::Soccer => "soccer game",
            SportType::Basketball => "basketball game",
            SportType::Tennis => "tennis match",
            SportType::Golf => "golf round",
            SportType::Skateboarding => "skate session",
            SportType::InlineSkating => "inline skating",
            SportType::Other(_name) => "activity", // Could use name but keeping generic
        }
    }
}

/// Represents an athlete/user profile from any provider
///
/// Contains the essential profile information that's commonly available
/// across fitness platforms.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::Athlete;
///
/// let athlete = Athlete {
///     id: "12345".to_string(),
///     username: "runner123".to_string(),
///     firstname: Some("John".to_string()),
///     lastname: Some("Doe".to_string()),
///     profile_picture: Some("https://example.com/avatar.jpg".to_string()),
///     provider: "strava".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Athlete {
    /// Unique identifier for the athlete (provider-specific)
    pub id: String,
    /// Public username/handle
    pub username: String,
    /// First name (may not be public on some providers)
    pub firstname: Option<String>,
    /// Last name (may not be public on some providers)
    pub lastname: Option<String>,
    /// URL to profile picture/avatar
    pub profile_picture: Option<String>,
    /// Source provider of this athlete data
    pub provider: String,
}

/// Aggregated fitness statistics for an athlete
///
/// Contains summarized statistics across all activities for a given time period.
/// Values are typically calculated from the athlete's activity history.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::Stats;
///
/// let stats = Stats {
///     total_activities: 150,
///     total_distance: 1500000.0, // 1500 km in meters
///     total_duration: 540000, // 150 hours in seconds
///     total_elevation_gain: 25000.0, // 25km of elevation
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    /// Total number of recorded activities
    pub total_activities: u64,
    /// Total distance covered across all activities (meters)
    pub total_distance: f64,
    /// Total time spent in activities (seconds)
    pub total_duration: u64,
    /// Total elevation gained across all activities (meters)
    pub total_elevation_gain: f64,
}

/// Represents a personal record achievement
///
/// Tracks the athlete's best performance in various metrics.
/// Links back to the specific activity where the record was achieved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecord {
    /// ID of the activity where this record was achieved
    pub activity_id: String,
    /// Type of performance metric
    pub metric: PrMetric,
    /// Value of the record (units depend on metric type)
    pub value: f64,
    /// When the record was achieved
    pub date: DateTime<Utc>,
}

/// Types of personal record metrics tracked
///
/// Each metric represents a different aspect of athletic performance
/// that can be optimized and tracked over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrMetric {
    /// Fastest pace achieved (seconds per meter)
    FastestPace,
    /// Longest distance covered in a single activity (meters)
    LongestDistance,
    /// Highest elevation gained in a single activity (meters)
    HighestElevation,
    /// Fastest completion time for a standard distance (seconds)
    FastestTime,
}

// ================================================================================================
// Multi-Tenant Models
// ================================================================================================

/// Represents a user in the multi-tenant system
///
/// Users are authenticated through OAuth providers and have encrypted tokens
/// stored securely for accessing their fitness data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: Uuid,
    /// User email address (used for identification)
    pub email: String,
    /// Display name
    pub display_name: Option<String>,
    /// Hashed password for authentication
    pub password_hash: String,
    /// Encrypted Strava tokens
    pub strava_token: Option<EncryptedToken>,
    /// Encrypted Fitbit tokens
    pub fitbit_token: Option<EncryptedToken>,
    /// When the user account was created
    pub created_at: DateTime<Utc>,
    /// Last time user accessed the system
    pub last_active: DateTime<Utc>,
    /// Whether the user account is active
    pub is_active: bool,
}

/// Encrypted OAuth token storage
///
/// Tokens are encrypted at rest using AES-256-GCM encryption.
/// Only decrypted when needed for API calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedToken {
    /// Encrypted access token (base64 encoded)
    pub access_token: String,
    /// Encrypted refresh token (base64 encoded)
    pub refresh_token: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// Token scope permissions
    pub scope: String,
    /// Encryption nonce (unique per token)
    pub nonce: String,
}

/// Decrypted OAuth token for API calls
///
/// This is never stored - only exists in memory during API requests.
#[derive(Debug, Clone)]
pub struct DecryptedToken {
    /// Plain text access token
    pub access_token: String,
    /// Plain text refresh token
    pub refresh_token: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// Token scope permissions
    pub scope: String,
}

/// User session for MCP protocol authentication
///
/// Contains JWT token and user context for secure MCP communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// User ID this session belongs to
    pub user_id: Uuid,
    /// JWT token for authentication
    pub jwt_token: String,
    /// When the session expires
    pub expires_at: DateTime<Utc>,
    /// User's email for display
    pub email: String,
    /// Available fitness providers for this user
    pub available_providers: Vec<String>,
}

/// Authentication request for MCP protocol
///
/// Clients send this to authenticate with the MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// JWT token for authentication
    pub token: String,
}

/// Authentication response for MCP protocol
///
/// Server responds with user context and available capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Whether authentication was successful
    pub authenticated: bool,
    /// User ID if authenticated
    pub user_id: Option<Uuid>,
    /// Error message if authentication failed
    pub error: Option<String>,
    /// Available fitness providers for this user
    pub available_providers: Vec<String>,
}

impl User {
    /// Create a new user with the given email and password hash
    pub fn new(email: String, password_hash: String, display_name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            display_name,
            password_hash,
            strava_token: None,
            fitbit_token: None,
            created_at: now,
            last_active: now,
            is_active: true,
        }
    }

    /// Check if user has valid Strava token
    pub fn has_strava_access(&self) -> bool {
        self.strava_token.as_ref()
            .map(|token| token.expires_at > Utc::now())
            .unwrap_or(false)
    }

    /// Check if user has valid Fitbit token
    pub fn has_fitbit_access(&self) -> bool {
        self.fitbit_token.as_ref()
            .map(|token| token.expires_at > Utc::now())
            .unwrap_or(false)
    }

    /// Get list of available providers for this user
    pub fn available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.has_strava_access() {
            providers.push("strava".to_string());
        }
        if self.has_fitbit_access() {
            providers.push("fitbit".to_string());
        }
        providers
    }

    /// Update last active timestamp
    pub fn update_last_active(&mut self) {
        self.last_active = Utc::now();
    }
}

impl EncryptedToken {
    /// Create a new encrypted token
    pub fn new(
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
        encryption_key: &[u8],
    ) -> Result<Self, anyhow::Error> {
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();
        
        // Generate unique nonce
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        use base64::{Engine as _, engine::general_purpose};
        let _nonce_b64 = general_purpose::STANDARD.encode(&nonce_bytes);

        // Create encryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        // Encrypt access token
        let mut access_token_data = access_token.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut access_token_data)?;
        let encrypted_access = general_purpose::STANDARD.encode(&access_token_data);

        // Generate new nonce for refresh token
        let mut refresh_nonce_bytes = [0u8; 12];
        rng.fill(&mut refresh_nonce_bytes)?;
        let refresh_nonce = Nonce::assume_unique_for_key(refresh_nonce_bytes);

        // Encrypt refresh token
        let mut refresh_token_data = refresh_token.as_bytes().to_vec();
        let unbound_key2 = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key2 = LessSafeKey::new(unbound_key2);
        key2.seal_in_place_append_tag(refresh_nonce, Aad::empty(), &mut refresh_token_data)?;
        let encrypted_refresh = general_purpose::STANDARD.encode(&refresh_token_data);

        // Store both nonces (we'll use the first one as the main nonce, second is embedded in refresh token)
        let combined_nonce = general_purpose::STANDARD.encode(&[&nonce_bytes[..], &refresh_nonce_bytes[..]].concat());

        Ok(Self {
            access_token: encrypted_access,
            refresh_token: encrypted_refresh,
            expires_at,
            scope,
            nonce: combined_nonce,
        })
    }

    /// Decrypt the token for use
    pub fn decrypt(&self, encryption_key: &[u8]) -> Result<DecryptedToken, anyhow::Error> {
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use base64::{Engine as _, engine::general_purpose};

        // Decode combined nonce
        let nonce_data = general_purpose::STANDARD.decode(&self.nonce)?;
        if nonce_data.len() != 24 {
            return Err(anyhow::anyhow!("Invalid nonce length"));
        }
        
        let access_nonce = Nonce::try_assume_unique_for_key(&nonce_data[0..12])?;
        let refresh_nonce = Nonce::try_assume_unique_for_key(&nonce_data[12..24])?;

        // Decrypt access token
        let unbound_key = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key = LessSafeKey::new(unbound_key);
        
        let mut access_data = general_purpose::STANDARD.decode(&self.access_token)?;
        let access_plaintext = key.open_in_place(access_nonce, Aad::empty(), &mut access_data)?;
        let access_token = String::from_utf8(access_plaintext.to_vec())?;

        // Decrypt refresh token
        let unbound_key2 = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key2 = LessSafeKey::new(unbound_key2);
        
        let mut refresh_data = general_purpose::STANDARD.decode(&self.refresh_token)?;
        let refresh_plaintext = key2.open_in_place(refresh_nonce, Aad::empty(), &mut refresh_data)?;
        let refresh_token = String::from_utf8(refresh_plaintext.to_vec())?;

        Ok(DecryptedToken {
            access_token,
            refresh_token,
            expires_at: self.expires_at,
            scope: self.scope.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json;

    /// Test data for creating sample activities
    fn create_sample_activity() -> Activity {
        Activity {
            id: "12345".to_string(),
            name: "Morning Run".to_string(),
            sport_type: SportType::Run,
            start_date: Utc::now(),
            duration_seconds: 1800, // 30 minutes
            distance_meters: Some(5000.0), // 5km
            elevation_gain: Some(100.0),
            average_heart_rate: Some(150),
            max_heart_rate: Some(175),
            average_speed: Some(2.78), // ~10 km/h
            max_speed: Some(4.17), // ~15 km/h
            calories: Some(300),
            start_latitude: Some(45.5017), // Montreal
            start_longitude: Some(-73.5673),
            city: Some("Montreal".to_string()),
            region: Some("Quebec".to_string()),
            country: Some("Canada".to_string()),
            trail_name: Some("Mount Royal Trail".to_string()),
            provider: "strava".to_string(),
        }
    }

    /// Test data for creating sample athlete
    fn create_sample_athlete() -> Athlete {
        Athlete {
            id: "67890".to_string(),
            username: "runner123".to_string(),
            firstname: Some("John".to_string()),
            lastname: Some("Doe".to_string()),
            profile_picture: Some("https://example.com/avatar.jpg".to_string()),
            provider: "strava".to_string(),
        }
    }

    #[test]
    fn test_activity_creation() {
        let activity = create_sample_activity();
        assert_eq!(activity.id, "12345");
        assert_eq!(activity.name, "Morning Run");
        assert!(matches!(activity.sport_type, SportType::Run));
        assert_eq!(activity.duration_seconds, 1800);
        assert_eq!(activity.distance_meters, Some(5000.0));
        assert_eq!(activity.provider, "strava");
    }

    #[test]
    fn test_activity_serialization() {
        let activity = create_sample_activity();
        
        // Test JSON serialization
        let json = serde_json::to_string(&activity).expect("Failed to serialize activity");
        assert!(json.contains("Morning Run"));
        assert!(json.contains("run")); // sport_type should be snake_case
        
        // Test JSON deserialization
        let deserialized: Activity = serde_json::from_str(&json).expect("Failed to deserialize activity");
        assert_eq!(deserialized.id, activity.id);
        assert_eq!(deserialized.name, activity.name);
        assert!(matches!(deserialized.sport_type, SportType::Run));
    }

    #[test]
    fn test_sport_type_serialization() {
        // Test standard sport types
        assert_eq!(serde_json::to_string(&SportType::Run).unwrap(), "\"run\"");
        assert_eq!(serde_json::to_string(&SportType::Ride).unwrap(), "\"ride\"");
        assert_eq!(serde_json::to_string(&SportType::VirtualRun).unwrap(), "\"virtual_run\"");
        
        // Test Other variant
        let custom_sport = SportType::Other("CrossCountrySkiing".to_string());
        let json = serde_json::to_string(&custom_sport).unwrap();
        assert!(json.contains("CrossCountrySkiing"));
        
        // Test deserialization
        let sport: SportType = serde_json::from_str("\"run\"").unwrap();
        assert!(matches!(sport, SportType::Run));
    }

    #[test]
    fn test_athlete_creation() {
        let athlete = create_sample_athlete();
        assert_eq!(athlete.id, "67890");
        assert_eq!(athlete.username, "runner123");
        assert_eq!(athlete.firstname, Some("John".to_string()));
        assert_eq!(athlete.lastname, Some("Doe".to_string()));
        assert_eq!(athlete.provider, "strava");
    }

    #[test]
    fn test_athlete_serialization() {
        let athlete = create_sample_athlete();
        
        // Test JSON serialization
        let json = serde_json::to_string(&athlete).expect("Failed to serialize athlete");
        assert!(json.contains("runner123"));
        assert!(json.contains("John"));
        
        // Test JSON deserialization
        let deserialized: Athlete = serde_json::from_str(&json).expect("Failed to deserialize athlete");
        assert_eq!(deserialized.username, athlete.username);
        assert_eq!(deserialized.firstname, athlete.firstname);
    }

    #[test]
    fn test_stats_creation() {
        let stats = Stats {
            total_activities: 150,
            total_distance: 1500000.0, // 1500 km
            total_duration: 540000, // 150 hours
            total_elevation_gain: 25000.0,
        };
        
        assert_eq!(stats.total_activities, 150);
        assert_eq!(stats.total_distance, 1500000.0);
        assert_eq!(stats.total_duration, 540000);
        assert_eq!(stats.total_elevation_gain, 25000.0);
    }

    #[test]
    fn test_stats_serialization() {
        let stats = Stats {
            total_activities: 100,
            total_distance: 1000000.0,
            total_duration: 360000,
            total_elevation_gain: 15000.0,
        };
        
        let json = serde_json::to_string(&stats).expect("Failed to serialize stats");
        let deserialized: Stats = serde_json::from_str(&json).expect("Failed to deserialize stats");
        
        assert_eq!(deserialized.total_activities, stats.total_activities);
        assert_eq!(deserialized.total_distance, stats.total_distance);
    }

    #[test]
    fn test_personal_record_creation() {
        let pr = PersonalRecord {
            activity_id: "12345".to_string(),
            metric: PrMetric::LongestDistance,
            value: 42195.0, // Marathon distance in meters
            date: Utc::now(),
        };
        
        assert_eq!(pr.activity_id, "12345");
        assert!(matches!(pr.metric, PrMetric::LongestDistance));
        assert_eq!(pr.value, 42195.0);
    }

    #[test]
    fn test_pr_metric_serialization() {
        assert_eq!(serde_json::to_string(&PrMetric::FastestPace).unwrap(), "\"fastest_pace\"");
        assert_eq!(serde_json::to_string(&PrMetric::LongestDistance).unwrap(), "\"longest_distance\"");
        assert_eq!(serde_json::to_string(&PrMetric::HighestElevation).unwrap(), "\"highest_elevation\"");
        assert_eq!(serde_json::to_string(&PrMetric::FastestTime).unwrap(), "\"fastest_time\"");
        
        // Test deserialization
        let metric: PrMetric = serde_json::from_str("\"fastest_pace\"").unwrap();
        assert!(matches!(metric, PrMetric::FastestPace));
    }

    #[test]
    fn test_activity_optional_fields() {
        let minimal_activity = Activity {
            id: "123".to_string(),
            name: "Quick Walk".to_string(),
            sport_type: SportType::Walk,
            start_date: Utc::now(),
            duration_seconds: 600, // 10 minutes
            distance_meters: None, // No distance tracking
            elevation_gain: None,
            average_heart_rate: None,
            max_heart_rate: None,
            average_speed: None,
            max_speed: None,
            calories: None,
            start_latitude: Some(45.5017), // Montreal
            start_longitude: Some(-73.5673),
            city: None,
            region: None,
            country: None,
            trail_name: None,
            provider: "manual".to_string(),
        };
        
        // Should serialize and deserialize correctly even with None values
        let json = serde_json::to_string(&minimal_activity).unwrap();
        let deserialized: Activity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.distance_meters, None);
        assert_eq!(deserialized.calories, None);
        assert_eq!(deserialized.provider, "manual");
    }
}