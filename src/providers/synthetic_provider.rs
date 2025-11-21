// ABOUTME: Production synthetic fitness provider for development and testing
// ABOUTME: Provides configurable activity data without requiring OAuth authentication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// Allow expect for RwLock poisoning - these are truly exceptional errors indicating
// serious program bugs (another thread panicked while holding lock). In such cases,
// propagating the panic is the correct behavior as the program state is compromised.
#![allow(clippy::expect_used)]
// Allow missing panics docs - all panics are from RwLock poisoning which is documented above
#![allow(clippy::missing_panics_doc)]
// Allow significant drop tightening - RwLock optimization suggestions for development provider
#![allow(clippy::significant_drop_tightening)]

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
use crate::models::{Activity, Athlete, PersonalRecord, PrMetric, Stats};
use crate::pagination::{Cursor, CursorPage, PaginationParams};
use crate::providers::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use crate::providers::errors::ProviderError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Synthetic fitness provider for development and testing
///
/// Provides pre-loaded activity data for automated testing, development,
/// and demonstration purposes without requiring real API connections or OAuth tokens.
///
/// # Examples
///
/// ```rust,no_run
/// use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
/// use pierre_mcp_server::models::Activity;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create provider with custom activities
/// let activities = vec![/* your activities */];
/// let provider = SyntheticProvider::with_activities(activities);
///
/// // Use like any other provider
/// let result = provider.get_activities(Some(10), None).await?;
/// # Ok(())
/// # }
/// ```
pub struct SyntheticProvider {
    /// Pre-loaded activities for testing
    activities: Arc<RwLock<Vec<Activity>>>,
    /// Activity lookup by ID for fast access
    activity_index: Arc<RwLock<HashMap<String, Activity>>>,
    /// Provider configuration
    config: ProviderConfig,
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
        // Build activity index for O(1) lookup by ID
        let mut index = HashMap::new();
        for activity in &activities {
            index.insert(activity.id.clone(), activity.clone());
        }

        Self {
            activities: Arc::new(RwLock::new(activities)),
            activity_index: Arc::new(RwLock::new(index)),
            config: ProviderConfig {
                name: oauth_providers::SYNTHETIC.to_owned(),
                auth_url: "http://localhost/synthetic/auth".to_owned(),
                token_url: "http://localhost/synthetic/token".to_owned(),
                api_base_url: "http://localhost/synthetic/api".to_owned(),
                revoke_url: None,
                default_scopes: vec!["activity:read_all".to_owned()],
            },
        }
    }

    /// Create an empty provider (no activities)
    ///
    /// Useful as a starting point where activities will be added dynamically.
    #[must_use]
    pub fn new() -> Self {
        Self::with_activities(Vec::new())
    }

    /// Add an activity to the provider dynamically
    ///
    /// # Arguments
    ///
    /// * `activity` - Activity to add
    ///
    /// # Panics
    ///
    /// Panics if the internal `RwLock` is poisoned (only occurs if another thread
    /// panicked while holding the lock, which indicates a serious program error).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
    /// # use pierre_mcp_server::models::Activity;
    /// let provider = SyntheticProvider::new();
    /// // provider.add_activity(activity);
    /// ```
    pub fn add_activity(&self, activity: Activity) {
        // Lock index first, then activities to prevent deadlock
        self.activity_index
            .write()
            .expect("Synthetic provider index RwLock poisoned")
            .insert(activity.id.clone(), activity.clone());

        self.activities
            .write()
            .expect("Synthetic provider activities RwLock poisoned")
            .push(activity);
    }

    /// Replace all activities with a new set
    ///
    /// # Arguments
    ///
    /// * `new_activities` - New activities to replace existing ones
    ///
    /// # Panics
    ///
    /// Panics if the internal `RwLock` is poisoned (only occurs if another thread
    /// panicked while holding the lock, which indicates a serious program error).
    pub fn set_activities(&self, new_activities: Vec<Activity>) {
        // Lock index first, then activities to prevent deadlock
        let mut index = self
            .activity_index
            .write()
            .expect("Synthetic provider index RwLock poisoned");

        index.clear();
        for activity in &new_activities {
            index.insert(activity.id.clone(), activity.clone());
        }
        drop(index); // Release index lock before acquiring activities lock

        let mut activities = self
            .activities
            .write()
            .expect("Synthetic provider activities RwLock poisoned");

        *activities = new_activities;
    }

    /// Get total number of activities
    ///
    /// # Returns
    ///
    /// Number of activities currently loaded in the provider
    #[must_use]
    pub fn activity_count(&self) -> usize {
        self.activities
            .read()
            .expect("Synthetic provider activities RwLock poisoned")
            .len()
    }

    /// Calculate aggregate statistics from loaded activities
    fn calculate_stats(&self) -> Stats {
        let activities = self
            .activities
            .read()
            .expect("Synthetic provider activities RwLock poisoned");

        // Activity count is bounded by memory, safe truncation to u64
        #[allow(clippy::cast_possible_truncation)]
        let total_activities = activities.len() as u64;

        let total_distance = activities.iter().filter_map(|a| a.distance_meters).sum();
        let total_duration = activities.iter().map(|a| a.duration_seconds).sum();
        let total_elevation_gain = activities.iter().filter_map(|a| a.elevation_gain).sum();

        Stats {
            total_activities,
            total_distance,
            total_duration,
            total_elevation_gain,
        }
    }

    /// Extract personal records from activities
    fn extract_personal_records(&self) -> Vec<PersonalRecord> {
        let activities = self
            .activities
            .read()
            .expect("Synthetic provider activities RwLock poisoned");

        let mut records: HashMap<PrMetric, PersonalRecord> = HashMap::new();

        for activity in activities.iter() {
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

        records.into_values().collect()
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
        oauth_providers::SYNTHETIC
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

    async fn get_athlete(&self) -> AppResult<Athlete> {
        // Return consistent synthetic athlete
        Ok(Athlete {
            id: "synthetic_athlete_001".to_owned(),
            username: "synthetic_athlete".to_owned(),
            firstname: Some("Synthetic".to_owned()),
            lastname: Some("Athlete".to_owned()),
            profile_picture: None,
            provider: oauth_providers::SYNTHETIC.to_owned(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
        let activities = self
            .activities
            .read()
            .expect("Synthetic provider activities RwLock poisoned");

        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(30);

        // Sort by start_date descending (most recent first)
        let mut sorted = activities.clone();
        sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date));

        Ok(sorted.into_iter().skip(offset).take(limit).collect())
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        let activities = self
            .activities
            .read()
            .expect("Synthetic provider activities RwLock poisoned");

        // Sort by start_date descending (most recent first)
        let mut sorted = activities.clone();
        let activities_len = activities.len();
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

    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let index = self
            .activity_index
            .read()
            .expect("Synthetic provider index RwLock poisoned");

        index.get(id).cloned().ok_or_else(|| {
            ProviderError::NotFound {
                provider: oauth_providers::SYNTHETIC.to_owned(),
                resource_type: "Activity".to_owned(),
                resource_id: id.to_owned(),
            }
            .into()
        })
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        Ok(self.calculate_stats())
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        Ok(self.extract_personal_records())
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
        Box::new(SyntheticProvider::with_activities(Vec::new()))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::SYNTHETIC]
    }
}
