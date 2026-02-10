// ABOUTME: Fitness insights generation system providing personalized training recommendations
// ABOUTME: Analyzes patterns, trends, and performance data to generate actionable fitness insights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Insight generation and management for athlete intelligence

use crate::models::Activity;
use crate::physiological_constants::{
    activity_scoring::{BASE_ACTIVITY_SCORE, COMPLETION_BONUS, STANDARD_BONUS},
    business_thresholds::ACHIEVEMENT_DISTANCE_THRESHOLD_KM,
    fitness_score_thresholds::GOOD_FITNESS_THRESHOLD,
    performance_calculation::MAX_EFFORT_SCORE,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Safe casting helper functions to avoid clippy warnings
#[inline]
fn safe_f64_to_f32(value: f64) -> f32 {
    // Handle special cases
    if value.is_nan() {
        return 0.0_f32;
    }

    // Use total_cmp for proper comparison without casting warnings
    if value.total_cmp(&f64::from(f32::MAX)) == Ordering::Greater {
        f32::MAX
    } else if value.total_cmp(&f64::from(f32::MIN)) == Ordering::Less {
        f32::MIN
    } else {
        // Value is within f32 range, use rounding conversion
        let rounded = value.round();
        if rounded > f64::from(f32::MAX) {
            f32::MAX
        } else if rounded < f64::from(f32::MIN) {
            f32::MIN
        } else {
            // Safe conversion using IEEE 754 standard rounding
            #[allow(clippy::cast_possible_truncation)] // Safe: bounds checked above
            {
                let bounded = rounded.clamp(f64::from(f32::MIN), f64::from(f32::MAX));
                bounded as f32
            }
        }
    }
}

#[inline]
fn safe_u32_to_u16(value: u32) -> u16 {
    // Use proper conversion approach
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// An insight extracted from activity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// Type of insight
    pub insight_type: InsightType,

    /// Human-readable insight message
    pub message: String,

    /// Confidence level (0-100)
    pub confidence: f32,

    /// Supporting data for the insight
    pub data: Option<serde_json::Value>,
}

/// Categories of insights that can be generated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    /// Performance achievement (PR, improvement)
    Achievement,

    /// Training zone analysis
    ZoneAnalysis,

    /// Effort and recovery insights
    EffortAnalysis,

    /// Recovery and fatigue
    RecoveryInsight,

    /// Goal progression
    GoalProgress,

    /// Location and terrain insights
    LocationInsight,

    /// Anomaly detection
    Anomaly,
}

/// Insight generator for creating intelligent analysis
pub struct InsightGenerator {
    /// Configuration for insight generation
    config: InsightConfig,
}

/// Configuration for insight generation
#[derive(Debug, Clone)]
pub struct InsightConfig {
    /// Minimum confidence score (0-100) to include an insight
    pub min_confidence_threshold: f32,
    /// Maximum number of insights to generate per activity
    pub max_insights_per_activity: usize,
}

impl Default for InsightConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: safe_f64_to_f32(GOOD_FITNESS_THRESHOLD),
            max_insights_per_activity: 5,
        }
    }
}

impl Default for InsightGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl InsightGenerator {
    /// Create a new insight generator with default config
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: InsightConfig::default(),
        }
    }

    /// Create a new insight generator with custom config
    #[must_use]
    pub const fn with_config(config: InsightConfig) -> Self {
        Self { config }
    }

    /// Generate insights for a single activity
    ///
    /// # Panics
    ///
    /// Panics if confidence comparison fails during sorting
    #[must_use]
    pub fn generate_insights(
        &self,
        activity: &Activity,
        context: Option<&ActivityContext>,
    ) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Generate different types of insights
        insights.extend(Self::generate_achievement_insights(activity));
        insights.extend(Self::generate_zone_insights(activity));
        insights.extend(Self::generate_effort_insights(activity));

        if let Some(ctx) = context {
            insights.extend(Self::generate_location_insights(activity, ctx));
        }

        // Filter by confidence and limit count
        insights.retain(|insight| insight.confidence >= self.config.min_confidence_threshold);
        insights.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(Ordering::Equal)
        });
        insights.truncate(self.config.max_insights_per_activity);

        insights
    }

    /// Generate achievement-related insights
    fn generate_achievement_insights(activity: &Activity) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Example: Distance PR detection
        if let Some(distance_m) = activity.distance_meters() {
            let distance_km = distance_m / 1000.0;
            if distance_km > ACHIEVEMENT_DISTANCE_THRESHOLD_KM {
                // Achievement threshold for distance milestones
                insights.push(Insight {
                    insight_type: InsightType::Achievement,
                    message: format!(
                        "Impressive distance! You completed {distance_km:.2} km, showing great endurance.",
                    ),
                    confidence: 85.0,
                    data: Some(serde_json::json!({
                        "distance_km": distance_km,
                        "achievement_type": "distance_milestone"
                    })),
                });
            }
        }

        insights
    }

    /// Generate zone analysis insights
    fn generate_zone_insights(activity: &Activity) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Analyze heart rate zones if available
        if let (Some(avg_hr), Some(max_hr)) =
            (activity.average_heart_rate(), activity.max_heart_rate())
        {
            // Heart rates are small values (30-220), use safe conversion
            let hr_intensity =
                f32::from(safe_u32_to_u16(avg_hr)) / f32::from(safe_u32_to_u16(max_hr));

            let (zone_description, confidence) = match hr_intensity {
                x if x < 0.6 => ("recovery zone", 90.0),
                x if x < 0.7 => ("endurance zone", 95.0),
                x if x < 0.8 => ("tempo zone", 92.0),
                x if x < 0.9 => ("threshold zone", 88.0),
                _ => ("VO2 max zone", 85.0),
            };

            insights.push(Insight {
                insight_type: InsightType::ZoneAnalysis,
                message: format!("Your average heart rate of {avg_hr} bpm indicates most time was spent in the {zone_description}. This is excellent for building aerobic capacity."),
                confidence,
                data: Some(serde_json::json!({
                    "avg_heartrate": avg_hr,
                    "max_heartrate": max_hr,
                    "zone": zone_description,
                    "intensity_ratio": hr_intensity
                })),
            });
        }

        insights
    }

    /// Generate effort analysis insights
    fn generate_effort_insights(activity: &Activity) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Analyze effort based on duration and intensity
        let duration = activity.duration_seconds();
        let effort_score = Self::calculate_relative_effort(activity);

        let effort_description = match effort_score {
            x if x < safe_f64_to_f32(BASE_ACTIVITY_SCORE * 0.6) => {
                ("light", "perfect for recovery")
            }
            x if x < safe_f64_to_f32(BASE_ACTIVITY_SCORE) => ("moderate", "good training stimulus"),
            x if x < safe_f64_to_f32(BASE_ACTIVITY_SCORE * 1.4) => {
                ("hard", "excellent workout intensity")
            }
            x if x < 9.0 => ("very hard", "high training load"),
            _ => ("maximum", "peak effort achieved"),
        };

        insights.push(Insight {
            insight_type: InsightType::EffortAnalysis,
            message: format!(
                "With a {} effort level, this {} session was {} for your training goals.",
                effort_description.0,
                Self::format_duration(duration.try_into().unwrap_or(0)),
                effort_description.1
            ),
            confidence: 80.0,
            data: Some(serde_json::json!({
                "effort_score": effort_score,
                "duration_seconds": duration,
                "effort_category": effort_description.0
            })),
        });

        insights
    }

    /// Calculate relative effort score (1-10 scale)
    fn calculate_relative_effort(activity: &Activity) -> f32 {
        let mut effort_score = safe_f64_to_f32(COMPLETION_BONUS);

        // Factor in duration
        let duration = activity.duration_seconds();
        let duration_f32 = if duration > u64::from(u32::MAX) {
            f32::MAX
        } else {
            let duration_u32 = u32::try_from(duration).unwrap_or(u32::MAX);
            safe_f64_to_f32(f64::from(duration_u32))
        };
        {
            effort_score += (duration_f32 / 3600.0) * (safe_f64_to_f32(COMPLETION_BONUS) * 2.0);
            // +2 per hour
        }

        // Factor in heart rate intensity
        if let (Some(avg_hr), Some(max_hr)) =
            (activity.average_heart_rate(), activity.max_heart_rate())
        {
            // Heart rates are small values (30-220), use safe conversion
            let hr_intensity =
                f32::from(safe_u32_to_u16(avg_hr)) / f32::from(safe_u32_to_u16(max_hr));
            {
                effort_score += hr_intensity * safe_f64_to_f32(BASE_ACTIVITY_SCORE);
            }
        }

        // Factor in elevation gain
        if let Some(elevation) = activity.elevation_gain() {
            {
                effort_score +=
                    safe_f64_to_f32(elevation / 100.0) * safe_f64_to_f32(STANDARD_BONUS);
                // +0.5 per 100m
            }
        }

        effort_score.min(MAX_EFFORT_SCORE)
    }

    /// Generate location and terrain insights
    fn generate_location_insights(activity: &Activity, context: &ActivityContext) -> Vec<Insight> {
        let mut insights = Vec::new();

        if let Some(location) = &context.location {
            // Trail-specific insights
            if let Some(trail_name) = &location.trail_name {
                insights.push(Insight {
                    insight_type: InsightType::LocationInsight,
                    message: format!(
                        "Explored the {trail_name} route, a great choice for your {} training",
                        activity.sport_type().display_name()
                    ),
                    confidence: 80.0,
                    data: Some(serde_json::json!({
                        "trail_name": trail_name,
                        "activity_type": activity.sport_type().display_name()
                    })),
                });
            }

            // Elevation and terrain analysis
            if let Some(elevation_gain) = activity.elevation_gain() {
                if elevation_gain > 500.0 {
                    let location_desc = location
                        .city
                        .as_ref()
                        .map_or_else(String::new, |city| format!(" in {city}"));

                    insights.push(Insight {
                        insight_type: InsightType::LocationInsight,
                        message: format!("Tackled significant elevation gain of {elevation_gain:.0}m{location_desc}, building excellent climbing strength"),
                        confidence: 85.0,
                        data: Some(serde_json::json!({
                            "elevation_gain": elevation_gain,
                            "location": location_desc,
                            "terrain_difficulty": "challenging"
                        })),
                    });
                }
            }

            // Regional insights
            if let (Some(city), Some(region)) = (&location.city, &location.region) {
                insights.push(Insight {
                    insight_type: InsightType::LocationInsight,
                    message: format!("Training in {city}, {region} - taking advantage of the local terrain and environment"),
                    confidence: 75.0,
                    data: Some(serde_json::json!({
                        "city": city,
                        "region": region
                    })),
                });
            }
        }

        insights
    }

    /// Format duration in human-readable form
    #[must_use]
    fn format_duration(seconds: i32) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        if hours > 0 {
            format!(
                "{hours} hour{} {minutes} minute{}",
                if hours == 1 { "" } else { "s" },
                if minutes == 1 { "" } else { "s" }
            )
        } else {
            format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
        }
    }
}

/// Context information for generating insights
#[derive(Debug, Clone)]
pub struct ActivityContext {
    /// Geographic location context for weather and terrain analysis
    pub location: Option<super::LocationContext>,
    /// Recent activity history for trend analysis
    pub recent_activities: Option<Vec<Activity>>,
}
