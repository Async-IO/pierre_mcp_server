// ABOUTME: Synthetic fitness provider for automated testing without OAuth
// ABOUTME: Returns pre-configured activity data for intelligence algorithm validation

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use anyhow::Result;
use async_trait::async_trait;
use pierre_mcp_server::constants::oauth_providers;
use pierre_mcp_server::models::{Activity, Athlete, PersonalRecord, PrMetric, Stats};
use pierre_mcp_server::pagination::{Cursor, CursorPage, PaginationParams};
use pierre_mcp_server::providers::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use pierre_mcp_server::providers::errors::ProviderError;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Synthetic provider for testing intelligence algorithms without OAuth
///
/// Provides pre-loaded activity data for automated testing, allowing
/// validation of metrics calculations, trend analysis, and predictions
/// without requiring real API connections or OAuth tokens.
///
/// # Thread Safety
///
/// All data access is protected by `RwLock` for thread-safe concurrent access.
/// Multiple tests can safely use the same provider instance.
///
/// # Examples
///
/// ```rust,no_run
/// // Note: This is test-only code in tests/helpers/
/// # mod helpers { pub mod synthetic_provider { pub struct SyntheticProvider; } }
/// # use helpers::synthetic_provider::SyntheticProvider;
/// use pierre_mcp_server::models::Activity;
///
/// # async fn example() -> anyhow::Result<()> {
/// let mut activities = Vec::new();
/// // ... build activities ...
///
/// let provider = SyntheticProvider::with_activities(activities);
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
                name: "synthetic".to_owned(),
                auth_url: "http://localhost/synthetic/auth".to_owned(),
                token_url: "http://localhost/synthetic/token".to_owned(),
                api_base_url: "http://localhost/synthetic/api".to_owned(),
                revoke_url: None,
                default_scopes: vec!["activity:read_all".to_owned()],
            },
        }
    }

    /// Create an empty provider (no activities)
    #[must_use]
    pub fn new() -> Self {
        Self::with_activities(Vec::new())
    }

    /// Add an activity to the provider dynamically
    pub fn add_activity(&self, activity: Activity) {
        {
            let mut activities = self
                .activities
                .write()
                .expect("Synthetic provider activities RwLock poisoned");

            {
                let mut index = self
                    .activity_index
                    .write()
                    .expect("Synthetic provider index RwLock poisoned");
                index.insert(activity.id.clone(), activity.clone());
            } // Drop index early

            activities.push(activity);
        } // RwLock guards dropped here
    }

    /// Replace all activities with a new set
    /// Reserved for future dynamic activity replacement tests
    #[allow(dead_code)]
    pub fn set_activities(&self, new_activities: Vec<Activity>) {
        {
            let mut activities = self
                .activities
                .write()
                .expect("Synthetic provider activities RwLock poisoned");

            {
                let mut index = self
                    .activity_index
                    .write()
                    .expect("Synthetic provider index RwLock poisoned");

                index.clear();
                for activity in &new_activities {
                    index.insert(activity.id.clone(), activity.clone());
                }
            } // Drop index early

            *activities = new_activities;
        } // RwLock guards dropped here
    }

    /// Get total number of activities
    #[must_use]
    pub fn activity_count(&self) -> usize {
        self.activities
            .read()
            .expect("Synthetic provider activities RwLock poisoned")
            .len()
    }

    /// Calculate aggregate statistics from loaded activities
    fn calculate_stats(&self) -> Stats {
        let total_activities;
        let total_distance;
        let total_duration;
        let total_elevation_gain;

        {
            let activities = self
                .activities
                .read()
                .expect("Synthetic provider activities RwLock poisoned");

            // Activity count is bounded by memory, safe truncation to u64
            #[allow(clippy::cast_possible_truncation)]
            {
                total_activities = activities.len() as u64;
            }
            total_distance = activities.iter().filter_map(|a| a.distance_meters).sum();
            total_duration = activities.iter().map(|a| a.duration_seconds).sum();
            total_elevation_gain = activities.iter().filter_map(|a| a.elevation_gain).sum();
        } // RwLock guard dropped here

        Stats {
            total_activities,
            total_distance,
            total_duration,
            total_elevation_gain,
        }
    }

    /// Extract personal records from activities
    fn extract_personal_records(&self) -> Vec<PersonalRecord> {
        let activities_snapshot = {
            let activities = self
                .activities
                .read()
                .expect("Synthetic provider activities RwLock poisoned");
            activities.clone()
        }; // RwLock guard dropped here

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
                let entry =
                    records
                        .entry(PrMetric::FastestTime)
                        .or_insert_with(|| PersonalRecord {
                            activity_id: activity.id.clone(),
                            metric: PrMetric::FastestTime,
                            value: activity.duration_seconds as f64,
                            date: activity.start_date,
                        });

                #[allow(clippy::cast_precision_loss)]
                let duration = activity.duration_seconds as f64;
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

    async fn set_credentials(&self, _credentials: OAuth2Credentials) -> Result<()> {
        // No-op: synthetic provider doesn't need credentials
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        // Always authenticated for testing
        true
    }

    async fn refresh_token_if_needed(&self) -> Result<()> {
        // No-op: no tokens to refresh
        Ok(())
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        // Return consistent test athlete
        Ok(Athlete {
            id: "synthetic_athlete_001".to_owned(),
            username: "test_athlete".to_owned(),
            firstname: Some("Test".to_owned()),
            lastname: Some("Athlete".to_owned()),
            profile_picture: None,
            provider: "synthetic".to_owned(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>> {
        let sorted = {
            let activities = self
                .activities
                .read()
                .expect("Synthetic provider activities RwLock poisoned");

            let sorted = activities.clone();
            drop(activities); // Drop early to reduce lock contention

            let offset = offset.unwrap_or(0);
            let limit = limit.unwrap_or(30);

            // Sort by start_date descending (most recent first)
            let mut sorted = sorted;
            sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date));

            sorted.into_iter().skip(offset).take(limit).collect()
        };

        Ok(sorted)
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> Result<CursorPage<Activity>> {
        let (sorted, activities_len) = {
            let activities = self
                .activities
                .read()
                .expect("Synthetic provider activities RwLock poisoned");

            // Sort by start_date descending (most recent first)
            let mut sorted = activities.clone();
            let len = activities.len();
            drop(activities); // Drop early to reduce lock contention

            sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date));

            (sorted, len)
        };

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

    async fn get_activity(&self, id: &str) -> Result<Activity> {
        let index = self
            .activity_index
            .read()
            .expect("Synthetic provider index RwLock poisoned");

        index.get(id).cloned().ok_or_else(|| {
            ProviderError::NotFound {
                provider: "synthetic".to_owned(),
                resource_type: "Activity".to_owned(),
                resource_id: id.to_owned(),
            }
            .into()
        })
    }

    async fn get_stats(&self) -> Result<Stats> {
        Ok(self.calculate_stats())
    }

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        Ok(self.extract_personal_records())
    }

    async fn disconnect(&self) -> Result<()> {
        // No-op: nothing to disconnect
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use pierre_mcp_server::models::SportType;

    #[allow(clippy::cast_precision_loss)]
    fn create_test_activity(id: &str, distance_km: f64, duration_min: u64) -> Activity {
        Activity {
            id: id.to_owned(),
            name: format!("Test Activity {id}"),
            sport_type: SportType::Run,
            start_date: Utc::now(),
            duration_seconds: duration_min * 60,
            distance_meters: Some(distance_km * 1000.0),
            elevation_gain: Some(100.0),
            average_heart_rate: Some(150),
            max_heart_rate: Some(175),
            average_speed: Some(distance_km * 1000.0 / (duration_min * 60) as f64),
            max_speed: None,
            calories: Some(300),
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
            temperature: None,
            humidity: None,
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
            start_latitude: Some(45.5017),
            start_longitude: Some(-73.5673),
            city: Some("Montreal".to_owned()),
            region: Some("Quebec".to_owned()),
            country: Some("Canada".to_owned()),
            trail_name: None,
            provider: "synthetic".to_owned(),
        }
    }

    #[tokio::test]
    async fn test_empty_provider() {
        let provider = SyntheticProvider::new();
        assert_eq!(provider.activity_count(), 0);

        let activities = provider.get_activities(None, None).await.unwrap();
        assert_eq!(activities.len(), 0);
    }

    #[tokio::test]
    async fn test_with_activities() {
        let activities = vec![
            create_test_activity("1", 5.0, 30),
            create_test_activity("2", 10.0, 60),
        ];

        let provider = SyntheticProvider::with_activities(activities);
        assert_eq!(provider.activity_count(), 2);
    }

    #[tokio::test]
    async fn test_get_activities_pagination() {
        let mut activities = Vec::new();
        for i in 0..50 {
            activities.push(create_test_activity(&format!("{i}"), 5.0, 30));
        }

        let provider = SyntheticProvider::with_activities(activities);

        // Get first page
        let page1 = provider.get_activities(Some(10), Some(0)).await.unwrap();
        assert_eq!(page1.len(), 10);

        // Get second page
        let page2 = provider.get_activities(Some(10), Some(10)).await.unwrap();
        assert_eq!(page2.len(), 10);

        // Verify pages don't overlap
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[tokio::test]
    async fn test_get_activity_by_id() {
        let activities = vec![
            create_test_activity("activity_1", 5.0, 30),
            create_test_activity("activity_2", 10.0, 60),
        ];

        let provider = SyntheticProvider::with_activities(activities);

        let activity = provider.get_activity("activity_1").await.unwrap();
        assert_eq!(activity.id, "activity_1");
        assert_eq!(activity.distance_meters, Some(5000.0));
    }

    #[tokio::test]
    async fn test_get_activity_not_found() {
        let provider = SyntheticProvider::new();

        let result = provider.get_activity("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_athlete() {
        let provider = SyntheticProvider::new();
        let athlete = provider.get_athlete().await.unwrap();

        assert_eq!(athlete.username, "test_athlete");
        assert_eq!(athlete.provider, "synthetic");
    }

    #[tokio::test]
    async fn test_get_stats() {
        let activities = vec![
            create_test_activity("1", 5.0, 30),  // 5km, 30min
            create_test_activity("2", 10.0, 60), // 10km, 60min
            create_test_activity("3", 8.0, 48),  // 8km, 48min
        ];

        let provider = SyntheticProvider::with_activities(activities);
        let stats = provider.get_stats().await.unwrap();

        assert_eq!(stats.total_activities, 3);
        assert!(
            (stats.total_distance - 23000.0).abs() < 0.01,
            "Total distance should be 23km"
        );
        assert_eq!(stats.total_duration, 8280); // 138 minutes
        assert!(
            (stats.total_elevation_gain - 300.0).abs() < 0.01,
            "Total elevation should be 300m"
        );
    }

    #[tokio::test]
    async fn test_add_activity() {
        let provider = SyntheticProvider::new();
        assert_eq!(provider.activity_count(), 0);

        provider.add_activity(create_test_activity("1", 5.0, 30));
        assert_eq!(provider.activity_count(), 1);

        provider.add_activity(create_test_activity("2", 10.0, 60));
        assert_eq!(provider.activity_count(), 2);
    }

    #[tokio::test]
    async fn test_is_authenticated() {
        let provider = SyntheticProvider::new();
        assert!(provider.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_cursor_pagination() {
        use chrono::{Duration, Utc};
        use pierre_mcp_server::pagination::{PaginationDirection, PaginationParams};

        let mut activities = Vec::new();
        for i in 0..25 {
            let mut activity = create_test_activity(&format!("{i}"), 5.0, 30);
            activity.start_date = Utc::now() - Duration::days(i);
            activities.push(activity);
        }

        let provider = SyntheticProvider::with_activities(activities);

        // First page
        let params1 = PaginationParams {
            cursor: None,
            limit: 10,
            direction: PaginationDirection::Forward,
        };
        let page1 = provider.get_activities_cursor(&params1).await.unwrap();
        assert_eq!(page1.items.len(), 10);
        assert!(page1.next_cursor.is_some());
        assert!(page1.has_more);

        // Second page using cursor
        let params2 = PaginationParams {
            cursor: page1.next_cursor,
            limit: 10,
            direction: PaginationDirection::Forward,
        };
        let page2 = provider.get_activities_cursor(&params2).await.unwrap();
        assert_eq!(page2.items.len(), 10);

        // Verify no overlap
        assert_ne!(page1.items[0].id, page2.items[0].id);
    }

    #[tokio::test]
    async fn test_personal_records() {
        let mut activities = vec![
            create_test_activity("1", 5.0, 25),   // Fast pace
            create_test_activity("2", 5.1, 30),   // Slower pace
            create_test_activity("3", 42.0, 240), // Long distance
        ];

        // Add elevation for PR
        activities[2].elevation_gain = Some(1000.0);

        let provider = SyntheticProvider::with_activities(activities);
        let prs = provider.get_personal_records().await.unwrap();

        assert!(!prs.is_empty());
        assert!(prs.iter().any(|pr| pr.metric == PrMetric::FastestPace));
        assert!(prs.iter().any(|pr| pr.metric == PrMetric::LongestDistance));
        assert!(prs.iter().any(|pr| pr.metric == PrMetric::HighestElevation));
    }
}
