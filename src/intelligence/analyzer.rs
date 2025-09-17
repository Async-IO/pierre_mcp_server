// ABOUTME: Fitness data analysis engine providing comprehensive workout and performance analytics
// ABOUTME: Calculates training zones, efficiency metrics, power analysis, and personalized insights
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Activity analyzer for generating intelligent insights

use super::{
    insights::{ActivityContext, InsightGenerator},
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, PersonalRecord, TimeOfDay,
    TrendDirection, TrendIndicators, ZoneDistribution,
};
use crate::intelligence::physiological_constants::{
    efficiency_defaults::{BASE_EFFICIENCY_SCORE, HR_EFFICIENCY_FACTOR, PACE_PER_KM_FACTOR},
    performance_calculation::{
        ASSUMED_RESTING_HR, BIKE_DISTANCE_DIVISOR, BIKE_EFFORT_MULTIPLIER, EFFORT_HOUR_FACTOR,
        ELEVATION_EFFORT_DIVISOR, ELEVATION_EFFORT_FACTOR, HR_INTENSITY_EFFORT_FACTOR,
        MAX_EFFORT_SCORE, MIN_EFFORT_SCORE, RUN_DISTANCE_DIVISOR, RUN_EFFORT_MULTIPLIER,
        SWIM_DISTANCE_DIVISOR, SWIM_EFFORT_MULTIPLIER,
    },
    performance_defaults::{DEFAULT_PREVIOUS_BEST_PACE, DEFAULT_PREVIOUS_BEST_TIME},
    personal_records::{DISTANCE_PR_THRESHOLD_KM, PACE_PR_THRESHOLD_SECONDS},
    zone_distributions::{
        efficiency_calculation::{CONSISTENCY_MULTIPLIER, HR_EFFICIENCY_MULTIPLIER},
        high_intensity, intensity_thresholds, low_intensity, moderate_intensity,
        moderate_low_intensity, very_high_intensity,
        zone_analysis_thresholds::{
            DEMO_CONSISTENCY_SCORE, HARD_INTENSITY_EFFORT_THRESHOLD,
            SIGNIFICANT_ENDURANCE_ZONE_THRESHOLD, TEMPO_ZONE_THRESHOLD, THRESHOLD_ZONE_THRESHOLD,
        },
    },
};
use crate::models::{Activity, SportType};
use chrono::{DateTime, Local, Timelike, Utc};
use std::fmt::Write;

/// Safe cast from f64 to f32 with bounds checking
/// Note: Direct casting is required here for numeric conversion - this is a fundamental
/// limitation when converting between floating point types of different precision
#[inline]
fn safe_f64_to_f32(value: f64) -> f32 {
    match value {
        v if v.is_nan() => 0.0_f32,
        v if v.is_infinite() => {
            if v.is_sign_positive() {
                f32::MAX
            } else {
                f32::MIN
            }
        }
        v if v <= f64::from(f32::MIN) => f32::MIN,
        v if v >= f64::from(f32::MAX) => f32::MAX,
        v => {
            // Safe: bounds checked above to ensure value is within f32 range
            #[allow(clippy::cast_possible_truncation)]
            {
                v as f32
            }
        }
    }
}

/// Safe cast from u32 to f32 with precision awareness
#[inline]
fn safe_u32_to_f32(value: u32) -> f32 {
    // Use f64 intermediate for all conversions
    let as_f64 = f64::from(value);
    safe_f64_to_f32(as_f64)
}

/// Main analyzer for generating activity intelligence
pub struct ActivityAnalyzer {
    insight_generator: InsightGenerator,
}

impl ActivityAnalyzer {
    /// Create a new activity analyzer
    #[must_use]
    pub fn new() -> Self {
        Self {
            insight_generator: InsightGenerator::new(),
        }
    }

    /// Analyze a single activity and generate intelligence
    ///
    /// # Errors
    ///
    /// Returns an error if analysis fails due to invalid data or computation errors
    pub fn analyze_activity(
        &self,
        activity: &Activity,
        context: Option<&ActivityContext>,
    ) -> Result<ActivityIntelligence, AnalysisError> {
        // Generate insights
        let insights = self.insight_generator.generate_insights(activity, context);

        // Calculate performance metrics
        let performance = Self::calculate_performance_metrics(activity);

        // Determine contextual factors
        let contextual_factors = Self::analyze_contextual_factors(activity, context);

        // Generate natural language summary
        let summary =
            Self::generate_summary(activity, &insights, &performance, &contextual_factors);

        Ok(ActivityIntelligence::new(
            summary,
            insights,
            performance,
            contextual_factors,
        ))
    }

    /// Calculate performance metrics for an activity
    fn calculate_performance_metrics(activity: &Activity) -> PerformanceMetrics {
        let relative_effort = Self::calculate_relative_effort(activity);
        let zone_distribution = Self::calculate_zone_distribution(activity);
        let personal_records = Self::detect_personal_records(activity);
        let efficiency_score = Self::calculate_efficiency_score(activity);
        let trend_indicators = Self::calculate_trend_indicators(activity);

        PerformanceMetrics {
            relative_effort: Some(relative_effort),
            zone_distribution,
            personal_records,
            efficiency_score: Some(efficiency_score),
            trend_indicators,
        }
    }

    /// Calculate relative effort score (1-10 scale)
    fn calculate_relative_effort(activity: &Activity) -> f32 {
        let mut effort = 1.0;

        // Base effort from duration
        let duration = activity.duration_seconds;
        // Safe conversion: clamp duration to avoid precision loss
        let duration_f32 = if duration > u64::from(u32::MAX) {
            f32::MAX
        } else {
            // Safe conversion within bounds check - use from() for safe cast to f32
            let duration_u32 = u32::try_from(duration).unwrap_or(u32::MAX);
            // Convert to f32, safely handling potential precision loss
            safe_u32_to_f32(duration_u32)
        };
        effort += (duration_f32 / 3600.0) * EFFORT_HOUR_FACTOR; // Duration-based effort

        // Heart rate intensity
        if let (Some(avg_hr), Some(max_hr)) = (activity.average_heart_rate, activity.max_heart_rate)
        {
            // Heart rates are typically in range 30-220, safe conversion with bounds check
            let hr_intensity =
                f32::from(u16::try_from(avg_hr.min(u32::from(u16::MAX))).unwrap_or(u16::MAX))
                    / f32::from(u16::try_from(max_hr.min(u32::from(u16::MAX))).unwrap_or(u16::MAX));
            effort += hr_intensity * HR_INTENSITY_EFFORT_FACTOR;
        }

        // Distance factor
        if let Some(distance_m) = activity.distance_meters {
            let distance_km = distance_m / 1000.0;
            match activity.sport_type {
                SportType::Run => {
                    let distance_factor =
                        safe_f64_to_f32(distance_km / f64::from(RUN_DISTANCE_DIVISOR));
                    effort += distance_factor * RUN_EFFORT_MULTIPLIER;
                }
                SportType::Ride => {
                    let distance_factor =
                        safe_f64_to_f32(distance_km / f64::from(BIKE_DISTANCE_DIVISOR));
                    effort += distance_factor * BIKE_EFFORT_MULTIPLIER;
                }
                _ => {
                    let distance_factor =
                        safe_f64_to_f32(distance_km / f64::from(SWIM_DISTANCE_DIVISOR));
                    effort += distance_factor * SWIM_EFFORT_MULTIPLIER;
                }
            }
        }

        // Elevation factor
        if let Some(elevation) = activity.elevation_gain {
            let elevation_factor = safe_f64_to_f32(elevation / f64::from(ELEVATION_EFFORT_DIVISOR));
            effort += elevation_factor * ELEVATION_EFFORT_FACTOR;
        }

        effort.clamp(MIN_EFFORT_SCORE, MAX_EFFORT_SCORE)
    }

    /// Calculate heart rate zone distribution
    fn calculate_zone_distribution(activity: &Activity) -> Option<ZoneDistribution> {
        // This is a simplified version - real implementation would need detailed HR data
        if let (Some(avg_hr), Some(max_hr)) = (activity.average_heart_rate, activity.max_heart_rate)
        {
            let hr_reserve = max_hr - ASSUMED_RESTING_HR; // Using configured resting HR
            let hr_diff = avg_hr.saturating_sub(ASSUMED_RESTING_HR);
            // Heart rate differences are small, safe conversion with bounds check
            let intensity =
                f32::from(u16::try_from(hr_diff.min(u32::from(u16::MAX))).unwrap_or(u16::MAX))
                    / f32::from(
                        u16::try_from(hr_reserve.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
                    );

            // Estimated distribution based on average intensity using defined thresholds
            let zones = match intensity {
                x if x < intensity_thresholds::LOW_TO_MODERATE_LOW => ZoneDistribution {
                    zone1_recovery: low_intensity::ZONE1_RECOVERY,
                    zone2_endurance: low_intensity::ZONE2_ENDURANCE,
                    zone3_tempo: low_intensity::ZONE3_TEMPO,
                    zone4_threshold: low_intensity::ZONE4_THRESHOLD,
                    zone5_vo2max: low_intensity::ZONE5_VO2MAX,
                },
                x if x < intensity_thresholds::MODERATE_LOW_TO_MODERATE => ZoneDistribution {
                    zone1_recovery: moderate_low_intensity::ZONE1_RECOVERY,
                    zone2_endurance: moderate_low_intensity::ZONE2_ENDURANCE,
                    zone3_tempo: moderate_low_intensity::ZONE3_TEMPO,
                    zone4_threshold: moderate_low_intensity::ZONE4_THRESHOLD,
                    zone5_vo2max: moderate_low_intensity::ZONE5_VO2MAX,
                },
                x if x < intensity_thresholds::MODERATE_TO_HIGH => ZoneDistribution {
                    zone1_recovery: moderate_intensity::ZONE1_RECOVERY,
                    zone2_endurance: moderate_intensity::ZONE2_ENDURANCE,
                    zone3_tempo: moderate_intensity::ZONE3_TEMPO,
                    zone4_threshold: moderate_intensity::ZONE4_THRESHOLD,
                    zone5_vo2max: moderate_intensity::ZONE5_VO2MAX,
                },
                x if x < intensity_thresholds::HIGH_TO_VERY_HIGH => ZoneDistribution {
                    zone1_recovery: high_intensity::ZONE1_RECOVERY,
                    zone2_endurance: high_intensity::ZONE2_ENDURANCE,
                    zone3_tempo: high_intensity::ZONE3_TEMPO,
                    zone4_threshold: high_intensity::ZONE4_THRESHOLD,
                    zone5_vo2max: high_intensity::ZONE5_VO2MAX,
                },
                _ => ZoneDistribution {
                    zone1_recovery: very_high_intensity::ZONE1_RECOVERY,
                    zone2_endurance: very_high_intensity::ZONE2_ENDURANCE,
                    zone3_tempo: very_high_intensity::ZONE3_TEMPO,
                    zone4_threshold: very_high_intensity::ZONE4_THRESHOLD,
                    zone5_vo2max: very_high_intensity::ZONE5_VO2MAX,
                },
            };

            Some(zones)
        } else {
            None
        }
    }

    /// Detect personal records (simplified version)
    fn detect_personal_records(activity: &Activity) -> Vec<PersonalRecord> {
        let mut records = Vec::new();

        // Example: Distance PR detection (would normally compare with historical data)
        if let Some(distance_m) = activity.distance_meters {
            let distance_km = distance_m / 1000.0;
            if distance_km > DISTANCE_PR_THRESHOLD_KM {
                // Arbitrary threshold for demo
                const PREVIOUS_BEST: f64 = DEFAULT_PREVIOUS_BEST_TIME;
                records.push(PersonalRecord {
                    record_type: "Longest Distance".into(),
                    value: distance_km,
                    unit: "km".into(),
                    previous_best: Some(PREVIOUS_BEST),
                    improvement_percentage: Some(safe_f64_to_f32(
                        (distance_km - PREVIOUS_BEST) / PREVIOUS_BEST * 100.0,
                    )),
                });
            }
        }

        // Example: Speed PR detection
        if let Some(avg_speed) = activity.average_speed {
            let pace_per_km = f64::from(PACE_PER_KM_FACTOR) / avg_speed;
            if pace_per_km < PACE_PR_THRESHOLD_SECONDS {
                const PREVIOUS_BEST_PACE: f64 = DEFAULT_PREVIOUS_BEST_PACE;
                records.push(PersonalRecord {
                    record_type: "Fastest Average Pace".into(),
                    value: pace_per_km,
                    unit: "seconds/km".into(),
                    previous_best: Some(PREVIOUS_BEST_PACE),
                    improvement_percentage: Some(safe_f64_to_f32(
                        (PREVIOUS_BEST_PACE - pace_per_km) / PREVIOUS_BEST_PACE * 100.0,
                    )),
                });
            }
        }

        records
    }

    /// Calculate efficiency score
    fn calculate_efficiency_score(activity: &Activity) -> f32 {
        let mut efficiency: f32 = BASE_EFFICIENCY_SCORE; // Base score

        // Heart rate efficiency
        if let (Some(avg_hr), Some(avg_speed)) =
            (activity.average_heart_rate, activity.average_speed)
        {
            let pace_per_km = PACE_PER_KM_FACTOR / safe_f64_to_f32(avg_speed);
            // Heart rates are typically small values (30-220), safe to convert to f32
            let hr_efficiency = HR_EFFICIENCY_FACTOR
                / (f32::from(u16::try_from(avg_hr.min(u32::from(u16::MAX))).unwrap_or(u16::MAX))
                    * pace_per_km);
            efficiency += hr_efficiency * HR_EFFICIENCY_MULTIPLIER;
        }

        // Consistency factor calculation
        if let (Some(avg_speed), Some(max_speed)) = (activity.average_speed, activity.max_speed) {
            let speed_variance = max_speed - avg_speed;
            let consistency = 1.0 - safe_f64_to_f32((speed_variance / max_speed).min(1.0));
            efficiency += consistency * CONSISTENCY_MULTIPLIER;
        }

        efficiency.clamp(0.0, 100.0)
    }

    /// Calculate trend indicators (simplified - would need historical data)
    const fn calculate_trend_indicators(_activity: &Activity) -> TrendIndicators {
        // Basic implementation using configured defaults - historical comparison would require database access
        TrendIndicators {
            pace_trend: TrendDirection::Improving,
            effort_trend: TrendDirection::Stable,
            distance_trend: TrendDirection::Stable,
            consistency_score: DEMO_CONSISTENCY_SCORE,
        }
    }

    /// Analyze contextual factors
    fn analyze_contextual_factors(
        activity: &Activity,
        context: Option<&ActivityContext>,
    ) -> ContextualFactors {
        let time_of_day = Self::determine_time_of_day(&activity.start_date);

        ContextualFactors {
            weather: None, // Weather analysis was removed
            location: context.and_then(|c| c.location.clone()),
            time_of_day,
            days_since_last_activity: None, // Would calculate from historical data
            weekly_load: None,              // Would calculate from recent activities
        }
    }

    /// Determine time of day category based on local time
    fn determine_time_of_day(start_date: &DateTime<Utc>) -> TimeOfDay {
        // Convert UTC to local time for proper categorization
        let local_time = start_date.with_timezone(&Local);
        match local_time.hour() {
            5..=6 => TimeOfDay::EarlyMorning, // 5-7 AM
            7..=10 => TimeOfDay::Morning,     // 7-11 AM
            11..=13 => TimeOfDay::Midday,     // 11 AM - 2 PM
            14..=17 => TimeOfDay::Afternoon,  // 2-6 PM
            18..=20 => TimeOfDay::Evening,    // 6-9 PM
            _ => TimeOfDay::Night,            // 9 PM - 5 AM
        }
    }

    /// Generate natural language summary
    fn generate_summary(
        activity: &Activity,
        insights: &[super::insights::Insight],
        performance: &PerformanceMetrics,
        context: &ContextualFactors,
    ) -> String {
        let mut summary_parts = Vec::new();

        // Activity type with weather context - use the display_name method
        let activity_type = activity.sport_type.display_name();

        // Add weather context if available
        let weather_context = context.weather.as_ref().map_or("", |weather| {
            match weather.conditions.to_lowercase().as_str() {
                c if c.contains("rain")
                    || c.contains("shower")
                    || c.contains("storm")
                    || c.contains("thunderstorm") =>
                {
                    " in the rain"
                }
                c if c.contains("snow") => " in the snow",
                c if c.contains("wind") && weather.wind_speed_kmh.unwrap_or(0.0) > 15.0 => {
                    " in windy conditions"
                }
                c if c.contains("hot") || weather.temperature_celsius > 28.0 => " in hot weather",
                c if c.contains("cold") || weather.temperature_celsius < 5.0 => " in cold weather",
                _ => "",
            }
        });

        // Add location context
        let location_context = context.location.as_ref().map_or(String::new(), |location| {
            location.trail_name.as_ref().map_or_else(
                || match (&location.city, &location.region) {
                    (Some(city), Some(region)) => format!(" in {city}, {region}"),
                    (Some(city), None) => format!(" in {city}"),
                    _ => String::new(),
                },
                |trail_name| format!(" on {trail_name}"),
            )
        });

        // Effort categorization
        let effort_desc =
            performance
                .relative_effort
                .map_or("moderate effort", |relative_effort| match relative_effort {
                    r if r < 3.0 => "light intensity",
                    r if r < 5.0 => "moderate intensity",
                    r if r < HARD_INTENSITY_EFFORT_THRESHOLD => "hard intensity",
                    _ => "very high intensity",
                });

        // Zone analysis
        let zone_desc = performance
            .zone_distribution
            .as_ref()
            .map_or("training zones", |zones| {
                if zones.zone2_endurance > SIGNIFICANT_ENDURANCE_ZONE_THRESHOLD {
                    "endurance zones"
                } else if zones.zone4_threshold > THRESHOLD_ZONE_THRESHOLD {
                    "threshold zones"
                } else if zones.zone3_tempo > TEMPO_ZONE_THRESHOLD {
                    "tempo zones"
                } else {
                    "mixed training zones"
                }
            });

        // Personal records context
        let pr_context = match performance.personal_records.len() {
            0 => String::new(),
            1 => " with 1 new personal record".into(),
            n => format!(" with {n} new personal records"),
        };

        // Build the summary
        summary_parts.push(format!(
            "{}{}{}",
            Self::to_title_case(activity_type),
            weather_context,
            location_context
        ));

        summary_parts.push(format!("{pr_context} and {effort_desc} in {zone_desc}"));

        let mut summary = summary_parts.join("");

        // Add detailed insights
        if let Some(distance) = activity.distance_meters {
            let distance_km = distance / 1000.0;
            summary.push_str(". During this ");
            let _ = write!(summary, "{distance_km:.1}");
            summary.push_str(" km session");
        }

        // Add primary insight from analysis
        if let Some(main_insight) = insights.first() {
            let message = main_insight.message.to_lowercase();
            summary.push_str(", ");
            summary.push_str(&message);
        }

        summary
    }

    /// Helper to capitalize first letter of a string
    fn to_title_case(s: &str) -> String {
        let mut chars = s.chars();
        chars.next().map_or(String::new(), |first| {
            first.to_uppercase().chain(chars).collect()
        })
    }
}

impl Default for ActivityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during analysis
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Insufficient activity data for analysis")]
    InsufficientData,

    #[error("Invalid activity data: {0}")]
    InvalidData(String),

    #[error("Analysis computation failed: {0}")]
    ComputationError(String),
}
