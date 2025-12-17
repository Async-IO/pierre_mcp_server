// ABOUTME: Production synthetic fitness provider for development and testing
// ABOUTME: Provides configurable activity data without requiring OAuth authentication
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// RwLock poisoning errors are converted to ProviderError::ConfigurationError
// for proper error propagation through the application

//! # Synthetic Fitness Provider
//!
//! A production-ready synthetic provider for development, testing, and demonstration purposes.
//! Unlike real fitness providers (Strava, Garmin, Fitbit), the synthetic provider:
//!
//! - Requires no OAuth authentication
//! - Supports dynamic activity injection
//! - Provides deterministic data for testing
//! - Can be used as a default fallback provider
//!
//! ## Use Cases
//!
//! - **Development**: Test features without external API dependencies
//! - **CI/CD**: Run integration tests without OAuth credentials
//! - **Demonstrations**: Show platform capabilities with pre-loaded data
//! - **Fallback**: Provide basic functionality when providers are unavailable
//!
//! ## Thread Safety
//!
//! All data access is protected by `RwLock` for thread-safe concurrent operations.
//! Multiple requests can safely access the same provider instance.

use crate::constants::oauth_providers;
use crate::errors::AppResult;
use crate::models::{
    Activity, Athlete, PersonalRecord, PrMetric, SleepSession, SleepStage, SleepStageType,
    SportType, Stats,
};
use crate::pagination::{Cursor, CursorPage, PaginationParams};
use crate::providers::core::{
    ActivityQueryParams, FitnessProvider, OAuth2Credentials, ProviderConfig,
};
use crate::providers::errors::ProviderError;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tracing::instrument;

/// Global test seed for synthetic provider factory
///
/// When set to a non-zero value, the `SyntheticProviderFactory` will create
/// providers with deterministic seeded data instead of empty providers.
static TEST_SEED: AtomicU64 = AtomicU64::new(0);

/// Set the test seed for synthetic provider creation
///
/// When a non-zero seed is set, all synthetic providers created by the factory
/// will use `SyntheticProvider::from_seed()` to generate deterministic test data.
///
/// # Arguments
/// * `seed` - The seed value (0 disables seeded mode)
///
/// # Example
/// ```no_run
/// use pierre_mcp_server::providers::synthetic_provider::set_synthetic_test_seed;
///
/// // In test setup
/// set_synthetic_test_seed(12345);
/// // ... run tests ...
/// // In test teardown
/// set_synthetic_test_seed(0);
/// ```
pub fn set_synthetic_test_seed(seed: u64) {
    TEST_SEED.store(seed, Ordering::SeqCst);
}

/// Get the current test seed (0 if not in test mode)
pub fn get_synthetic_test_seed() -> u64 {
    TEST_SEED.load(Ordering::SeqCst)
}

/// Synthetic fitness provider for development and testing
///
/// Provides pre-loaded activity data for automated testing, development,
/// and demonstration purposes without requiring real API connections or OAuth tokens.
///
/// # Examples
///
/// ```rust,no_run
/// use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
/// use pierre_mcp_server::providers::core::FitnessProvider;  // Import trait for methods
/// use pierre_mcp_server::models::Activity;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create provider with custom activities
/// let activities = vec![/* your activities */];
/// let provider = SyntheticProvider::with_activities(activities);
///
/// // Use like any other provider (FitnessProvider trait must be in scope)
/// let result = provider.get_activities(Some(10), None).await?;
/// # Ok(())
/// # }
/// ```
pub struct SyntheticProvider {
    /// Pre-loaded activities for testing
    activities: Arc<RwLock<Vec<Activity>>>,
    /// Activity lookup by ID for fast access
    activity_index: Arc<RwLock<HashMap<String, Activity>>>,
    /// Pre-loaded sleep sessions for testing
    sleep_sessions: Arc<RwLock<Vec<SleepSession>>>,
    /// Provider configuration
    config: ProviderConfig,
    /// Provider name as static string (allows different instances with different names)
    /// NOTE: This uses `Box::leak` for dynamic names - acceptable for test providers
    /// which are typically created once and live for the program duration.
    provider_name: &'static str,
}

impl SyntheticProvider {
    /// Create a new synthetic provider with given activities
    ///
    /// # Arguments
    ///
    /// * `activities` - Vector of activities to pre-load
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
    /// # use pierre_mcp_server::models::Activity;
    /// let activities = vec![/* your activities */];
    /// let provider = SyntheticProvider::with_activities(activities);
    /// ```
    #[must_use]
    pub fn with_activities(activities: Vec<Activity>) -> Self {
        Self::with_activities_and_name(activities, oauth_providers::SYNTHETIC)
    }

    /// Create a new synthetic provider with given activities and custom name
    ///
    /// Allows creating multiple synthetic providers with different names for
    /// cross-provider testing scenarios (e.g., one for activities, one for sleep).
    ///
    /// # Arguments
    ///
    /// * `activities` - Vector of activities to pre-load
    /// * `name` - Provider name constant (e.g., `oauth_providers::SYNTHETIC_SLEEP`)
    #[must_use]
    pub fn with_activities_and_name(activities: Vec<Activity>, name: &'static str) -> Self {
        // Build activity index for O(1) lookup by ID
        let mut index = HashMap::new();
        for activity in &activities {
            index.insert(activity.id.clone(), activity.clone());
        }

        Self {
            activities: Arc::new(RwLock::new(activities)),
            activity_index: Arc::new(RwLock::new(index)),
            sleep_sessions: Arc::new(RwLock::new(Vec::new())),
            config: ProviderConfig {
                name: name.to_owned(),
                auth_url: format!("http://localhost/{name}/auth"),
                token_url: format!("http://localhost/{name}/token"),
                api_base_url: format!("http://localhost/{name}/api"),
                revoke_url: None,
                default_scopes: vec!["activity:read_all".to_owned(), "sleep:read".to_owned()],
            },
            provider_name: name,
        }
    }

    /// Create an empty provider (no activities)
    ///
    /// Useful as a starting point where activities will be added dynamically.
    #[must_use]
    pub fn new() -> Self {
        Self::with_activities(Vec::new())
    }

    /// Create a new synthetic provider with a custom name
    ///
    /// # Arguments
    ///
    /// * `name` - Provider name constant (e.g., `oauth_providers::SYNTHETIC_SLEEP`)
    #[must_use]
    pub fn with_name(name: &'static str) -> Self {
        Self::with_activities_and_name(Vec::new(), name)
    }

    /// Add a sleep session to the provider dynamically
    ///
    /// # Arguments
    ///
    /// * `session` - Sleep session to add
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::ConfigurationError` if the internal `RwLock` is poisoned.
    pub fn add_sleep_session(&self, session: SleepSession) -> Result<(), ProviderError> {
        self.sleep_sessions
            .write()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: self.provider_name.to_owned(),
                details: "RwLock poisoned: sleep_sessions lock".to_owned(),
            })?
            .push(session);

        Ok(())
    }

    /// Replace all sleep sessions with a new set
    ///
    /// # Arguments
    ///
    /// * `new_sessions` - New sleep sessions to replace existing ones
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::ConfigurationError` if the internal `RwLock` is poisoned.
    pub fn set_sleep_sessions(&self, new_sessions: Vec<SleepSession>) -> Result<(), ProviderError> {
        // Assign and drop lock immediately - minimal lock holding time
        *self
            .sleep_sessions
            .write()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: self.provider_name.to_owned(),
                details: "RwLock poisoned: sleep_sessions lock".to_owned(),
            })? = new_sessions;
        Ok(())
    }

    /// Get total number of sleep sessions
    ///
    /// # Returns
    ///
    /// Number of sleep sessions currently loaded in the provider
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::ConfigurationError` if the internal `RwLock` is poisoned.
    pub fn sleep_session_count(&self) -> Result<usize, ProviderError> {
        Ok(self
            .sleep_sessions
            .read()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: self.provider_name.to_owned(),
                details: "RwLock poisoned: sleep_sessions lock".to_owned(),
            })?
            .len())
    }

    /// Generate synthetic sleep sessions for testing
    ///
    /// Creates realistic sleep sessions with sleep stages for the specified number of nights.
    /// Useful for testing sleep analysis tools without requiring real provider data.
    ///
    /// # Arguments
    ///
    /// * `nights` - Number of nights of sleep data to generate
    /// * `base_date` - Starting date for sleep sessions (works backward from this date)
    #[must_use]
    pub fn generate_sleep_sessions(nights: u32, base_date: DateTime<Utc>) -> Vec<SleepSession> {
        let mut sessions = Vec::with_capacity(nights as usize);

        for i in 0..nights {
            // Sleep session starts around 10-11 PM previous day
            let end_date = base_date - Duration::days(i64::from(i));
            let sleep_start = end_date - Duration::hours(8) - Duration::minutes(30);
            let sleep_end = end_date - Duration::minutes(30);

            // Generate sleep stages (approximately 90-minute cycles)
            let stages = Self::generate_sleep_stages(sleep_start);

            // Calculate stage totals for scoring
            let deep_minutes: u32 = stages
                .iter()
                .filter(|s| matches!(s.stage_type, SleepStageType::Deep))
                .map(|s| s.duration_minutes)
                .sum();
            let rem_minutes: u32 = stages
                .iter()
                .filter(|s| matches!(s.stage_type, SleepStageType::Rem))
                .map(|s| s.duration_minutes)
                .sum();

            let total_sleep_time = 7 * 60 + 30 + (i % 3) * 15; // 7.5-8 hours varying
            let time_in_bed = total_sleep_time + 20 + (i % 5) * 5; // Add some time awake

            // Sleep efficiency varies between 85-95%
            #[allow(clippy::cast_precision_loss)]
            let sleep_efficiency = total_sleep_time as f32 / time_in_bed as f32 * 100.0;

            // Sleep score based on duration and stages (70-95 range)
            #[allow(clippy::cast_precision_loss)]
            let sleep_score = 70.0
                + (deep_minutes as f32 / 60.0 * 5.0).min(15.0)
                + (rem_minutes as f32 / 60.0 * 5.0).min(10.0);

            sessions.push(SleepSession {
                id: format!("sleep_{i}_{}", base_date.timestamp()),
                start_time: sleep_start,
                end_time: sleep_end,
                time_in_bed,
                total_sleep_time,
                sleep_efficiency,
                sleep_score: Some(sleep_score.min(95.0)),
                stages,
                hrv_during_sleep: Some(45.0 + f64::from(i % 20)), // 45-65ms HRV
                respiratory_rate: Some(
                    f32::from(u8::try_from(i % 4).unwrap_or(0)).mul_add(0.5, 14.5),
                ), // 14.5-16 breaths/min
                temperature_variation: Some(
                    f32::from(u8::try_from(i % 5).unwrap_or(0)).mul_add(0.1, -0.3),
                ), // -0.3 to +0.1°C
                wake_count: Some(1 + (i % 3)),
                sleep_onset_latency: Some(10 + (i % 3) * 5), // 10-20 minutes
                provider: oauth_providers::SYNTHETIC.to_owned(),
            });
        }

        sessions
    }

    /// Generate realistic sleep stages for a single night
    fn generate_sleep_stages(sleep_start: DateTime<Utc>) -> Vec<SleepStage> {
        let mut stages = Vec::with_capacity(20);
        let mut current_time = sleep_start;

        // Typical sleep cycle pattern: Light -> Deep -> Light -> REM, repeated 4-5 times
        let cycle_patterns = [
            // Cycle 1: More deep sleep
            (SleepStageType::Light, 15),
            (SleepStageType::Deep, 45),
            (SleepStageType::Light, 10),
            (SleepStageType::Rem, 10),
            // Cycle 2
            (SleepStageType::Light, 20),
            (SleepStageType::Deep, 35),
            (SleepStageType::Light, 15),
            (SleepStageType::Rem, 20),
            // Cycle 3: Less deep, more REM
            (SleepStageType::Light, 25),
            (SleepStageType::Deep, 20),
            (SleepStageType::Light, 15),
            (SleepStageType::Rem, 30),
            // Cycle 4: Mostly light and REM
            (SleepStageType::Light, 30),
            (SleepStageType::Deep, 10),
            (SleepStageType::Light, 20),
            (SleepStageType::Rem, 35),
            // Brief awakening
            (SleepStageType::Awake, 5),
            // Final cycle
            (SleepStageType::Light, 25),
            (SleepStageType::Rem, 25),
            (SleepStageType::Light, 15),
        ];

        for (stage_type, duration) in cycle_patterns {
            stages.push(SleepStage {
                stage_type,
                start_time: current_time,
                duration_minutes: duration,
            });
            current_time += Duration::minutes(i64::from(duration));
        }

        stages
    }

    /// Create a synthetic provider with deterministically generated data from a seed
    ///
    /// This method generates realistic activity and sleep data using a seeded random
    /// number generator, ensuring reproducible test data across test runs.
    ///
    /// # Arguments
    ///
    /// * `seed` - Seed value for the random number generator
    /// * `activity_count` - Number of activities to generate
    /// * `sleep_nights` - Number of sleep sessions to generate
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
    ///
    /// fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Create provider with seed 12345, 30 activities, 14 nights of sleep
    ///     let provider = SyntheticProvider::from_seed(12345, 30, 14)?;
    ///
    ///     // Same seed always produces same data
    ///     let provider2 = SyntheticProvider::from_seed(12345, 30, 14)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::ConfigurationError` if internal lock is poisoned
    /// (should never happen on freshly created provider).
    pub fn from_seed(
        seed: u64,
        activity_count: u32,
        sleep_nights: u32,
    ) -> Result<Self, ProviderError> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let base_date = Utc::now();

        let activities = Self::generate_activities_from_rng(&mut rng, activity_count, base_date);
        let sleep_sessions =
            Self::generate_sleep_sessions_from_rng(&mut rng, sleep_nights, base_date);

        let provider = Self::with_activities(activities);
        provider.set_sleep_sessions(sleep_sessions)?;
        Ok(provider)
    }

    /// Generate activities using a seeded random number generator
    ///
    /// Creates realistic activity data with varied sport types, durations,
    /// distances, and performance metrics.
    // Long function: Generates complete Activity struct with all fields populated
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
    fn generate_activities_from_rng(
        rng: &mut ChaCha8Rng,
        count: u32,
        base_date: DateTime<Utc>,
    ) -> Vec<Activity> {
        let sport_types = [
            SportType::Run,
            SportType::Ride,
            SportType::Swim,
            SportType::Walk,
            SportType::Hike,
            SportType::Workout,
        ];

        let activity_names = [
            "Morning Run",
            "Evening Ride",
            "Lunch Swim",
            "Recovery Walk",
            "Trail Hike",
            "Strength Training",
            "Interval Session",
            "Long Run",
            "Easy Spin",
            "Tempo Run",
        ];

        let mut activities = Vec::with_capacity(count as usize);

        for i in 0..count {
            let sport_type = sport_types[rng.gen_range(0..sport_types.len())].clone();
            let name = activity_names[rng.gen_range(0..activity_names.len())];

            // Activity spread over the last N days (1-2 activities per day roughly)
            let days_ago = i64::from(i / 2);
            let hour_offset = rng.gen_range(6..20); // Between 6 AM and 8 PM
            let start_date =
                base_date - Duration::days(days_ago) - Duration::hours(24 - hour_offset);

            // Duration varies by sport type (in seconds)
            let duration_seconds = match sport_type {
                SportType::Run => rng.gen_range(1200..7200), // 20 min - 2 hours
                SportType::Ride => rng.gen_range(1800..14400), // 30 min - 4 hours
                SportType::Swim => rng.gen_range(900..3600), // 15 min - 1 hour
                SportType::Walk => rng.gen_range(1200..5400), // 20 min - 1.5 hours
                SportType::Hike => rng.gen_range(3600..18000), // 1 - 5 hours
                SportType::Workout => rng.gen_range(1800..5400), // 30 min - 1.5 hours
                _ => rng.gen_range(1800..3600),
            };

            // Distance varies by sport type (in meters)
            let distance_meters = match sport_type {
                SportType::Run => Some(rng.gen_range(3000.0..21_000.0)),
                SportType::Ride => Some(rng.gen_range(10_000.0..100_000.0)),
                SportType::Swim => Some(rng.gen_range(500.0..4000.0)),
                SportType::Walk => Some(rng.gen_range(2000.0..8000.0)),
                SportType::Hike => Some(rng.gen_range(5000.0..25_000.0)),
                _ => None,
            };

            // Calculate speed from distance and duration
            let average_speed = distance_meters.map(|d| d / duration_seconds as f64);
            let max_speed = average_speed.map(|s| s * rng.gen_range(1.2..1.5));

            // Heart rate data
            let average_heart_rate = Some(rng.gen_range(120..165));
            let max_heart_rate = average_heart_rate.map(|hr| hr + rng.gen_range(15..35));

            // Elevation for outdoor activities
            let elevation_gain = match sport_type {
                SportType::Run | SportType::Ride | SportType::Hike | SportType::Walk => {
                    Some(rng.gen_range(50.0..500.0))
                }
                _ => None,
            };

            // Calories estimation
            let calories = Some(rng.gen_range(150..800));

            activities.push(Activity {
                id: format!("synthetic_activity_{}_{}", seed_fingerprint(rng), i),
                name: format!("{} #{}", name, i + 1),
                sport_type,
                start_date,
                duration_seconds,
                distance_meters,
                elevation_gain,
                average_heart_rate,
                max_heart_rate,
                average_speed,
                max_speed,
                calories,
                steps: None,
                heart_rate_zones: None,
                average_power: None,
                max_power: None,
                normalized_power: None,
                power_zones: None,
                ftp: None,
                average_cadence: None,
                max_cadence: None,
                hrv_score: None,
                recovery_heart_rate: None,
                temperature: Some(rng.gen_range(10.0..30.0)),
                humidity: Some(rng.gen_range(30.0..80.0)),
                average_altitude: None,
                wind_speed: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                running_power: None,
                breathing_rate: None,
                spo2: None,
                training_stress_score: None,
                intensity_factor: None,
                suffer_score: None,
                time_series_data: None,
                start_latitude: Some(45.5017 + rng.gen_range(-0.1..0.1)),
                start_longitude: Some(-73.5673 + rng.gen_range(-0.1..0.1)),
                city: Some("Montreal".to_owned()),
                region: Some("Quebec".to_owned()),
                country: Some("Canada".to_owned()),
                trail_name: None,
                provider: oauth_providers::SYNTHETIC.to_owned(),
                workout_type: None,
                sport_type_detail: None,
                segment_efforts: None,
            });
        }

        activities
    }

    /// Generate sleep sessions using a seeded random number generator
    #[allow(clippy::cast_precision_loss)]
    fn generate_sleep_sessions_from_rng(
        rng: &mut ChaCha8Rng,
        nights: u32,
        base_date: DateTime<Utc>,
    ) -> Vec<SleepSession> {
        let mut sessions = Vec::with_capacity(nights as usize);

        for i in 0..nights {
            let end_date = base_date - Duration::days(i64::from(i));

            // Vary bedtime (10 PM - midnight) and wake time
            let bedtime_offset = rng.gen_range(22..24);
            let sleep_duration_hours: u32 = rng.gen_range(6..9);
            let sleep_start = end_date - Duration::hours(bedtime_offset);
            let sleep_end = sleep_start + Duration::hours(i64::from(sleep_duration_hours));

            let stages = Self::generate_sleep_stages(sleep_start);

            let deep_minutes: u32 = stages
                .iter()
                .filter(|s| matches!(s.stage_type, SleepStageType::Deep))
                .map(|s| s.duration_minutes)
                .sum();
            let rem_minutes: u32 = stages
                .iter()
                .filter(|s| matches!(s.stage_type, SleepStageType::Rem))
                .map(|s| s.duration_minutes)
                .sum();

            let total_sleep_time = sleep_duration_hours * 60 - rng.gen_range(10..40);
            let time_in_bed = total_sleep_time + rng.gen_range(15..45);
            let sleep_efficiency = total_sleep_time as f32 / time_in_bed as f32 * 100.0;

            let sleep_score = 70.0
                + (deep_minutes as f32 / 60.0 * 5.0).min(15.0)
                + (rem_minutes as f32 / 60.0 * 5.0).min(10.0);

            sessions.push(SleepSession {
                id: format!("synthetic_sleep_{}_{}", seed_fingerprint(rng), i),
                start_time: sleep_start,
                end_time: sleep_end,
                time_in_bed,
                total_sleep_time,
                sleep_efficiency,
                sleep_score: Some(sleep_score.min(95.0)),
                stages,
                hrv_during_sleep: Some(45.0 + rng.gen_range(0.0..25.0)),
                respiratory_rate: Some(14.5 + rng.gen_range(0.0..2.0)),
                temperature_variation: Some(rng.gen_range(-0.3..0.2)),
                wake_count: Some(rng.gen_range(0..4)),
                sleep_onset_latency: Some(rng.gen_range(5..25)),
                provider: oauth_providers::SYNTHETIC.to_owned(),
            });
        }

        sessions
    }

    /// Add an activity to the provider dynamically
    ///
    /// # Arguments
    ///
    /// * `activity` - Activity to add
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::ConfigurationError` if the internal `RwLock` is poisoned.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
    /// # use pierre_mcp_server::models::Activity;
    /// let provider = SyntheticProvider::new();
    /// // provider.add_activity(activity);
    /// ```
    pub fn add_activity(&self, activity: Activity) -> Result<(), ProviderError> {
        // Lock index first, then activities to prevent deadlock
        self.activity_index
            .write()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: oauth_providers::SYNTHETIC.to_owned(),
                details: "RwLock poisoned: index lock".to_owned(),
            })?
            .insert(activity.id.clone(), activity.clone());

        self.activities
            .write()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: oauth_providers::SYNTHETIC.to_owned(),
                details: "RwLock poisoned: activities lock".to_owned(),
            })?
            .push(activity);

        Ok(())
    }

    /// Replace all activities with a new set
    ///
    /// # Arguments
    ///
    /// * `new_activities` - New activities to replace existing ones
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::ConfigurationError` if the internal `RwLock` is poisoned.
    pub fn set_activities(&self, new_activities: Vec<Activity>) -> Result<(), ProviderError> {
        // Lock index first, then activities to prevent deadlock
        {
            let mut index =
                self.activity_index
                    .write()
                    .map_err(|_| ProviderError::ConfigurationError {
                        provider: oauth_providers::SYNTHETIC.to_owned(),
                        details: "RwLock poisoned: index lock".to_owned(),
                    })?;

            index.clear();
            for activity in &new_activities {
                index.insert(activity.id.clone(), activity.clone());
            }
        } // Drop index lock here

        {
            let mut activities =
                self.activities
                    .write()
                    .map_err(|_| ProviderError::ConfigurationError {
                        provider: oauth_providers::SYNTHETIC.to_owned(),
                        details: "RwLock poisoned: activities lock".to_owned(),
                    })?;

            *activities = new_activities;
        } // Drop activities lock here

        Ok(())
    }

    /// Get total number of activities
    ///
    /// # Returns
    ///
    /// Number of activities currently loaded in the provider
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::ConfigurationError` if the internal `RwLock` is poisoned.
    pub fn activity_count(&self) -> Result<usize, ProviderError> {
        Ok(self
            .activities
            .read()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: oauth_providers::SYNTHETIC.to_owned(),
                details: "RwLock poisoned: activities lock".to_owned(),
            })?
            .len())
    }

    /// Calculate aggregate statistics from loaded activities
    fn calculate_stats(&self) -> Result<Stats, ProviderError> {
        let (total_activities, total_distance, total_duration, total_elevation_gain) = {
            let activities =
                self.activities
                    .read()
                    .map_err(|_| ProviderError::ConfigurationError {
                        provider: oauth_providers::SYNTHETIC.to_owned(),
                        details: "RwLock poisoned: activities lock".to_owned(),
                    })?;

            // Activity count is bounded by memory, safe truncation to u64
            #[allow(clippy::cast_possible_truncation)]
            let total_activities = activities.len() as u64;

            let total_distance = activities.iter().filter_map(|a| a.distance_meters).sum();
            let total_duration = activities.iter().map(|a| a.duration_seconds).sum();
            let total_elevation_gain = activities.iter().filter_map(|a| a.elevation_gain).sum();
            drop(activities);

            (
                total_activities,
                total_distance,
                total_duration,
                total_elevation_gain,
            )
        }; // Drop activities lock here

        Ok(Stats {
            total_activities,
            total_distance,
            total_duration,
            total_elevation_gain,
        })
    }

    /// Extract personal records from activities
    fn extract_personal_records(&self) -> Result<Vec<PersonalRecord>, ProviderError> {
        let activities_snapshot = {
            let activities =
                self.activities
                    .read()
                    .map_err(|_| ProviderError::ConfigurationError {
                        provider: oauth_providers::SYNTHETIC.to_owned(),
                        details: "RwLock poisoned: activities lock".to_owned(),
                    })?;

            activities.clone()
        }; // Drop activities lock here

        let mut records: HashMap<PrMetric, PersonalRecord> = HashMap::new();

        for activity in &activities_snapshot {
            // Fastest pace (lowest seconds per meter)
            if let (Some(distance), true) =
                (activity.distance_meters, activity.duration_seconds > 0)
            {
                if distance > 0.0 {
                    // Duration in seconds, precision loss acceptable for pace calculation
                    #[allow(clippy::cast_precision_loss)]
                    let pace_sec_per_meter = activity.duration_seconds as f64 / distance;

                    let entry =
                        records
                            .entry(PrMetric::FastestPace)
                            .or_insert_with(|| PersonalRecord {
                                activity_id: activity.id.clone(),
                                metric: PrMetric::FastestPace,
                                value: pace_sec_per_meter,
                                date: activity.start_date,
                            });

                    if pace_sec_per_meter < entry.value {
                        *entry = PersonalRecord {
                            activity_id: activity.id.clone(),
                            metric: PrMetric::FastestPace,
                            value: pace_sec_per_meter,
                            date: activity.start_date,
                        };
                    }
                }
            }

            // Longest distance
            if let Some(distance) = activity.distance_meters {
                let entry =
                    records
                        .entry(PrMetric::LongestDistance)
                        .or_insert_with(|| PersonalRecord {
                            activity_id: activity.id.clone(),
                            metric: PrMetric::LongestDistance,
                            value: distance,
                            date: activity.start_date,
                        });

                if distance > entry.value {
                    *entry = PersonalRecord {
                        activity_id: activity.id.clone(),
                        metric: PrMetric::LongestDistance,
                        value: distance,
                        date: activity.start_date,
                    };
                }
            }

            // Highest elevation
            if let Some(elevation) = activity.elevation_gain {
                let entry = records
                    .entry(PrMetric::HighestElevation)
                    .or_insert_with(|| PersonalRecord {
                        activity_id: activity.id.clone(),
                        metric: PrMetric::HighestElevation,
                        value: elevation,
                        date: activity.start_date,
                    });

                if elevation > entry.value {
                    *entry = PersonalRecord {
                        activity_id: activity.id.clone(),
                        metric: PrMetric::HighestElevation,
                        value: elevation,
                        date: activity.start_date,
                    };
                }
            }

            // Fastest time (for any activity)
            if activity.duration_seconds > 0 {
                // Duration in seconds, precision loss acceptable for time comparison
                #[allow(clippy::cast_precision_loss)]
                let duration = activity.duration_seconds as f64;

                let entry =
                    records
                        .entry(PrMetric::FastestTime)
                        .or_insert_with(|| PersonalRecord {
                            activity_id: activity.id.clone(),
                            metric: PrMetric::FastestTime,
                            value: duration,
                            date: activity.start_date,
                        });

                if duration < entry.value {
                    *entry = PersonalRecord {
                        activity_id: activity.id.clone(),
                        metric: PrMetric::FastestTime,
                        value: duration,
                        date: activity.start_date,
                    };
                }
            }
        }

        Ok(records.into_values().collect())
    }
}

impl Default for SyntheticProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FitnessProvider for SyntheticProvider {
    fn name(&self) -> &'static str {
        self.provider_name
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, _credentials: OAuth2Credentials) -> AppResult<()> {
        // No-op: synthetic provider doesn't need credentials
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        // Always authenticated for testing
        true
    }

    async fn refresh_token_if_needed(&self) -> AppResult<()> {
        // No-op: no tokens to refresh
        Ok(())
    }

    #[instrument(skip(self), fields(provider = "synthetic", api_call = "get_athlete"))]
    async fn get_athlete(&self) -> AppResult<Athlete> {
        // Return consistent synthetic athlete
        Ok(Athlete {
            id: format!("{}_athlete_001", self.provider_name),
            username: "test_athlete".to_owned(),
            firstname: Some("Synthetic".to_owned()),
            lastname: Some("Athlete".to_owned()),
            profile_picture: None,
            provider: self.provider_name.to_owned(),
        })
    }

    #[instrument(
        skip(self, params),
        fields(
            provider = "synthetic",
            api_call = "get_activities",
            limit = ?params.limit,
            offset = ?params.offset,
        )
    )]
    async fn get_activities_with_params(
        &self,
        params: &ActivityQueryParams,
    ) -> AppResult<Vec<Activity>> {
        let mut sorted = {
            let activities =
                self.activities
                    .read()
                    .map_err(|_| ProviderError::ConfigurationError {
                        provider: oauth_providers::SYNTHETIC.to_owned(),
                        details: "RwLock poisoned: activities lock".to_owned(),
                    })?;

            activities.clone()
        }; // Drop activities lock here

        let offset = params.offset.unwrap_or(0);
        let limit = params.limit.unwrap_or(30);

        // Sort by start_date descending (most recent first)
        sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date));

        // Apply time filtering if before/after specified
        if let Some(after_ts) = params.after {
            if let Some(after_dt) = chrono::DateTime::from_timestamp(after_ts, 0) {
                sorted.retain(|a| a.start_date >= after_dt);
            }
        }
        if let Some(before_ts) = params.before {
            if let Some(before_dt) = chrono::DateTime::from_timestamp(before_ts, 0) {
                sorted.retain(|a| a.start_date < before_dt);
            }
        }

        Ok(sorted.into_iter().skip(offset).take(limit).collect())
    }

    #[instrument(
        skip(self, params),
        fields(
            provider = "synthetic",
            api_call = "get_activities_cursor",
            limit = params.limit,
        )
    )]
    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        let (mut sorted, activities_len) = {
            let activities =
                self.activities
                    .read()
                    .map_err(|_| ProviderError::ConfigurationError {
                        provider: oauth_providers::SYNTHETIC.to_owned(),
                        details: "RwLock poisoned: activities lock".to_owned(),
                    })?;

            let activities_len = activities.len();
            let sorted = activities.clone();
            drop(activities);

            (sorted, activities_len)
        }; // Drop activities lock here

        // Sort by start_date descending (most recent first)
        sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date));

        // Find starting position based on cursor
        let start_index = params.cursor.as_ref().map_or(0, |cursor| {
            cursor.decode().map_or(0, |(_timestamp, id)| {
                sorted
                    .iter()
                    .position(|a| a.id == id)
                    .map_or(0, |pos| pos + 1)
            })
        });

        let limit = params.limit.min(100); // Cap at 100
        let items: Vec<Activity> = sorted
            .iter()
            .skip(start_index)
            .take(limit)
            .cloned()
            .collect();

        let has_more = start_index + items.len() < activities_len;

        // Create next cursor using the last item's timestamp and ID
        let next_cursor = if has_more && !items.is_empty() {
            let last_item = &items[items.len() - 1];
            Some(Cursor::new(last_item.start_date, &last_item.id))
        } else {
            None
        };

        Ok(CursorPage::new(
            items,
            next_cursor,
            None, // prev_cursor not needed for synthetic provider
            has_more,
        ))
    }

    #[instrument(
        skip(self),
        fields(provider = "synthetic", api_call = "get_activity", activity_id = %id)
    )]
    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let index = self
            .activity_index
            .read()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: self.provider_name.to_owned(),
                details: "RwLock poisoned: index lock".to_owned(),
            })?;

        index.get(id).cloned().ok_or_else(|| {
            ProviderError::NotFound {
                provider: self.provider_name.to_owned(),
                resource_type: "Activity".to_owned(),
                resource_id: id.to_owned(),
            }
            .into()
        })
    }

    #[instrument(skip(self), fields(provider = "synthetic", api_call = "get_stats"))]
    async fn get_stats(&self) -> AppResult<Stats> {
        Ok(self.calculate_stats()?)
    }

    #[instrument(
        skip(self),
        fields(provider = "synthetic", api_call = "get_personal_records")
    )]
    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        Ok(self.extract_personal_records()?)
    }

    #[instrument(
        skip(self),
        fields(provider = "synthetic", api_call = "get_sleep_sessions")
    )]
    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        // Filter and clone sessions within the lock scope, then release lock immediately
        let filtered: Vec<SleepSession> = self
            .sleep_sessions
            .read()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: self.provider_name.to_owned(),
                details: "RwLock poisoned: sleep_sessions lock".to_owned(),
            })?
            .iter()
            .filter(|s| s.end_time >= start_date && s.start_time <= end_date)
            .cloned()
            .collect();

        Ok(filtered)
    }

    #[instrument(
        skip(self),
        fields(provider = "synthetic", api_call = "get_latest_sleep_session")
    )]
    async fn get_latest_sleep_session(&self) -> Result<SleepSession, ProviderError> {
        // Find latest session within lock scope, clone and release lock immediately
        self.sleep_sessions
            .read()
            .map_err(|_| ProviderError::ConfigurationError {
                provider: self.provider_name.to_owned(),
                details: "RwLock poisoned: sleep_sessions lock".to_owned(),
            })?
            .iter()
            .max_by_key(|s| s.end_time)
            .cloned()
            .ok_or_else(|| ProviderError::NotFound {
                provider: self.provider_name.to_owned(),
                resource_type: "SleepSession".to_owned(),
                resource_id: "latest".to_owned(),
            })
    }

    async fn disconnect(&self) -> AppResult<()> {
        // No-op: nothing to disconnect
        Ok(())
    }
}

// ============================================================================
// Provider Factory
// ============================================================================

use crate::providers::core::ProviderFactory;

/// Factory for creating Synthetic provider instances
pub struct SyntheticProviderFactory;

impl ProviderFactory for SyntheticProviderFactory {
    fn create(&self, _config: ProviderConfig) -> Box<dyn FitnessProvider> {
        let seed = get_synthetic_test_seed();
        if seed != 0 {
            // Test mode: create provider with deterministic seeded data
            // Default: 30 activities, 14 nights of sleep
            SyntheticProvider::from_seed(seed, 30, 14).map_or_else(
                |_| {
                    Box::new(SyntheticProvider::with_activities(Vec::new()))
                        as Box<dyn FitnessProvider>
                },
                |provider| Box::new(provider) as Box<dyn FitnessProvider>,
            )
        } else {
            // Production mode: empty provider (data injected via other means)
            Box::new(SyntheticProvider::with_activities(Vec::new()))
        }
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::SYNTHETIC]
    }
}

/// Factory for creating Synthetic Sleep provider instances
///
/// Creates a synthetic provider configured for sleep data, allowing
/// cross-provider testing scenarios where activities come from one
/// provider and sleep data from another.
pub struct SyntheticSleepProviderFactory;

impl ProviderFactory for SyntheticSleepProviderFactory {
    fn create(&self, _config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(SyntheticProvider::with_name(
            oauth_providers::SYNTHETIC_SLEEP,
        ))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::SYNTHETIC_SLEEP]
    }
}

/// Generate a short fingerprint from RNG state for unique IDs
fn seed_fingerprint(rng: &mut ChaCha8Rng) -> String {
    format!("{:08x}", rng.gen::<u32>())
}
