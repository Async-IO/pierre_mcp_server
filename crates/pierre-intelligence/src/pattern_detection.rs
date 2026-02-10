// ABOUTME: Pattern detection for training analysis including weekly schedules and overtraining signals
// ABOUTME: Detects training patterns, hard/easy day alternation, and early warning signs of overtraining
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::models::Activity;
use crate::training_load::RiskLevel;
use chrono::{Datelike, Timelike, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum activities needed for pattern detection
const MIN_ACTIVITIES_FOR_PATTERN: usize = 6;

/// HR drift threshold for overtraining detection (percent)
const HR_DRIFT_THRESHOLD_PERCENT: f64 = 5.0;

/// Volume spike threshold for injury risk (percent)
const VOLUME_SPIKE_THRESHOLD_PERCENT: f64 = 10.0;

/// Weekly schedule pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklySchedulePattern {
    /// Most common training days (sorted by frequency)
    pub most_common_days: Vec<Weekday>,
    /// Day frequency (how many times each day was used)
    pub day_frequencies: HashMap<String, u32>,
    /// Most common training times (hour of day)
    pub most_common_times: Vec<u32>,
    /// Consistency score (0-100) based on regularity
    pub consistency_score: f64,
    /// Average activities per week
    pub avg_activities_per_week: f64,
}

/// Hard/easy day alternation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardEasyPattern {
    /// Whether a hard/easy pattern is detected
    pub pattern_detected: bool,
    /// Description of the pattern
    pub pattern_description: String,
    /// Percentage of activities that are "hard" (high intensity)
    pub hard_percentage: f64,
    /// Percentage of activities that are "easy" (low intensity)
    pub easy_percentage: f64,
    /// Whether proper recovery is being taken
    pub adequate_recovery: bool,
}

/// Overtraining warning signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvertrainingSignals {
    /// HR drift detected (elevated HR for same pace)
    pub hr_drift_detected: bool,
    /// Average HR drift percentage
    pub hr_drift_percent: Option<f64>,
    /// Performance decline detected
    pub performance_decline: bool,
    /// Insufficient recovery detected (high frequency without rest)
    pub insufficient_recovery: bool,
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Detailed warning messages
    pub warnings: Vec<String>,
}

/// Volume progression pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeProgressionPattern {
    /// Weekly volumes in chronological order (total distance in km)
    pub weekly_volumes: Vec<f64>,
    /// Week numbers (relative, starting from 0)
    pub week_numbers: Vec<u32>,
    /// Volume trend (increasing/stable/decreasing)
    pub trend: VolumeTrend,
    /// Whether dangerous volume spikes detected (>10% increase)
    pub volume_spikes_detected: bool,
    /// Weeks with volume spikes
    pub spike_weeks: Vec<u32>,
    /// Recommended action
    pub recommendation: String,
}

/// Volume trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeTrend {
    /// Training volume is increasing over time
    Increasing,
    /// Training volume is remaining steady
    Stable,
    /// Training volume is decreasing over time
    Decreasing,
}

/// Pattern detection engine
pub struct PatternDetector;

impl PatternDetector {
    /// Detect weekly training schedule patterns
    ///
    /// Analyzes which days of the week are most commonly used for training
    #[must_use]
    pub fn detect_weekly_schedule(activities: &[Activity]) -> WeeklySchedulePattern {
        if activities.len() < MIN_ACTIVITIES_FOR_PATTERN {
            return Self::empty_schedule_pattern();
        }

        // Count activities by day of week
        let mut day_counts: HashMap<Weekday, u32> = HashMap::new();
        let mut hour_counts: HashMap<u32, u32> = HashMap::new();

        for activity in activities {
            let weekday = activity.start_date().weekday();
            *day_counts.entry(weekday).or_insert(0) += 1;

            let hour = activity.start_date().hour();
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        // Sort days by frequency
        let mut day_freq_vec: Vec<(Weekday, u32)> = day_counts.into_iter().collect();
        day_freq_vec.sort_by(|a, b| b.1.cmp(&a.1));

        let most_common_days: Vec<Weekday> =
            day_freq_vec.iter().take(3).map(|(day, _)| *day).collect();

        // Convert to string map for serialization
        let day_frequencies: HashMap<String, u32> = day_freq_vec
            .iter()
            .map(|(day, count)| (format!("{day:?}"), *count))
            .collect();

        // Sort hours by frequency
        let mut hour_freq_vec: Vec<(u32, u32)> = hour_counts.into_iter().collect();
        hour_freq_vec.sort_by(|a, b| b.1.cmp(&a.1));

        let most_common_times: Vec<u32> = hour_freq_vec
            .iter()
            .take(2)
            .map(|(hour, _)| *hour)
            .collect();

        // Calculate consistency score
        let consistency_score =
            Self::calculate_schedule_consistency(&day_freq_vec, activities.len());

        // Calculate activities per week
        let weeks_span = Self::calculate_weeks_span(activities);
        #[allow(clippy::cast_precision_loss)]
        let avg_activities_per_week = if weeks_span > 0.0 {
            activities.len() as f64 / weeks_span
        } else {
            0.0
        };

        WeeklySchedulePattern {
            most_common_days,
            day_frequencies,
            most_common_times,
            consistency_score,
            avg_activities_per_week,
        }
    }

    /// Detect hard/easy day alternation patterns
    #[must_use]
    pub fn detect_hard_easy_pattern(activities: &[Activity]) -> HardEasyPattern {
        if activities.len() < MIN_ACTIVITIES_FOR_PATTERN {
            return Self::empty_hard_easy_pattern();
        }

        // Classify activities as hard or easy based on intensity
        let mut hard_count = 0;
        let mut easy_count = 0;

        for activity in activities {
            if Self::is_hard_activity(activity) {
                hard_count += 1;
            } else {
                easy_count += 1;
            }
        }

        #[allow(clippy::cast_precision_loss)]
        let total = activities.len() as f64;
        let hard_percentage = (f64::from(hard_count) / total) * 100.0;
        let easy_percentage = (f64::from(easy_count) / total) * 100.0;

        // Check for alternation pattern
        let pattern_detected = Self::detect_alternation(activities);

        // Check for adequate recovery (not too many consecutive hard days)
        let adequate_recovery = Self::check_adequate_recovery(activities);

        let pattern_description = if pattern_detected {
            "Regular hard/easy alternation detected".to_owned()
        } else if hard_percentage > 70.0 {
            "Too many high-intensity sessions - add recovery days".to_owned()
        } else if easy_percentage > 90.0 {
            "Mostly easy sessions - consider adding intensity".to_owned()
        } else {
            "No clear pattern - mixed intensity distribution".to_owned()
        };

        HardEasyPattern {
            pattern_detected,
            pattern_description,
            hard_percentage,
            easy_percentage,
            adequate_recovery,
        }
    }

    /// Detect overtraining warning signals
    #[must_use]
    pub fn detect_overtraining_signals(activities: &[Activity]) -> OvertrainingSignals {
        if activities.len() < MIN_ACTIVITIES_FOR_PATTERN {
            return Self::empty_overtraining_signals();
        }

        let mut warnings = Vec::new();

        // Check for HR drift
        let (hr_drift_detected, hr_drift_percent) = Self::detect_hr_drift(activities);
        if hr_drift_detected {
            if let Some(drift) = hr_drift_percent {
                warnings.push(format!(
                    "HR drift detected: {drift:.1}% increase (possible fatigue)"
                ));
            }
        }

        // Check for performance decline
        let performance_decline = Self::detect_performance_decline(activities);
        if performance_decline {
            warnings
                .push("Performance decline detected: pace slowing for similar efforts".to_owned());
        }

        // Check for insufficient recovery
        let insufficient_recovery = Self::detect_insufficient_recovery(activities);
        if insufficient_recovery {
            warnings.push(
                "Insufficient recovery: multiple high-intensity days without rest".to_owned(),
            );
        }

        // Determine overall risk level
        let risk_level = if warnings.len() >= 2 {
            RiskLevel::High
        } else if warnings.len() == 1 {
            RiskLevel::Moderate
        } else {
            RiskLevel::Low
        };

        OvertrainingSignals {
            hr_drift_detected,
            hr_drift_percent,
            performance_decline,
            insufficient_recovery,
            risk_level,
            warnings,
        }
    }

    /// Detect volume progression patterns and dangerous spikes
    #[must_use]
    pub fn detect_volume_progression(activities: &[Activity]) -> VolumeProgressionPattern {
        if activities.len() < MIN_ACTIVITIES_FOR_PATTERN {
            return Self::empty_volume_progression();
        }

        // Group activities by week and calculate total distance per week
        let weekly_data = Self::calculate_weekly_volumes(activities);

        let mut weekly_volumes = Vec::new();
        let mut week_numbers = Vec::new();
        let mut volume_spikes_detected = false;
        let mut spike_weeks = Vec::new();

        for (week_num, volume) in &weekly_data {
            week_numbers.push(*week_num);
            weekly_volumes.push(*volume);
        }

        // Check for volume spikes (>10% increase week-over-week)
        for i in 1..weekly_volumes.len() {
            let prev_volume = weekly_volumes[i - 1];
            let curr_volume = weekly_volumes[i];

            if prev_volume > 0.0 {
                let increase_percent = ((curr_volume - prev_volume) / prev_volume) * 100.0;
                if increase_percent > VOLUME_SPIKE_THRESHOLD_PERCENT {
                    volume_spikes_detected = true;
                    spike_weeks.push(week_numbers[i]);
                }
            }
        }

        // Determine volume trend
        let trend = Self::determine_volume_trend(&weekly_volumes);

        // Generate recommendation
        let recommendation = if volume_spikes_detected {
            format!("Reduce volume spikes - detected {VOLUME_SPIKE_THRESHOLD_PERCENT:.0}%+ increases in weeks: {spike_weeks:?}")
        } else if matches!(trend, VolumeTrend::Increasing) {
            "Volume increasing steadily - maintain 10% rule to prevent injury".to_owned()
        } else {
            "Volume progression looks safe - no dangerous spikes detected".to_owned()
        };

        VolumeProgressionPattern {
            weekly_volumes,
            week_numbers,
            trend,
            volume_spikes_detected,
            spike_weeks,
            recommendation,
        }
    }

    // === Helper Functions ===

    fn calculate_schedule_consistency(day_freq: &[(Weekday, u32)], total_activities: usize) -> f64 {
        if day_freq.is_empty() || total_activities == 0 {
            return 0.0;
        }

        // Consistency is high if activities are concentrated on specific days
        // Calculate entropy-like measure
        let mut consistency = 0.0;
        #[allow(clippy::cast_precision_loss)]
        let total_activities_f64 = total_activities as f64;
        for (_, count) in day_freq {
            let prob = f64::from(*count) / total_activities_f64;
            consistency += prob * prob; // Concentration measure
        }

        // Scale to 0-100 (higher = more consistent)
        (consistency * 100.0).min(100.0)
    }

    fn calculate_weeks_span(activities: &[Activity]) -> f64 {
        if activities.len() < 2 {
            return 1.0;
        }

        let mut dates: Vec<_> = activities.iter().map(Activity::start_date).collect();
        dates.sort();

        #[allow(clippy::cast_precision_loss)]
        if let (Some(first), Some(last)) = (dates.first(), dates.last()) {
            (*last - *first).num_days() as f64 / 7.0
        } else {
            1.0
        }
    }

    fn is_hard_activity(activity: &Activity) -> bool {
        // Classify as hard if: high HR, long duration, or high speed
        let high_hr = activity.average_heart_rate().is_some_and(|hr| hr > 150);

        let long_duration = activity.duration_seconds() > 3600; // >1 hour

        #[allow(clippy::cast_precision_loss)]
        let high_speed = activity
            .distance_meters()
            .is_some_and(|d| (d / activity.duration_seconds() as f64) > 3.5); // >3.5 m/s (~4:45 min/km)

        high_hr || (long_duration && high_speed)
    }

    fn detect_alternation(activities: &[Activity]) -> bool {
        if activities.len() < 4 {
            return false;
        }

        // Check if hard and easy days alternate
        let mut alternations = 0;
        let mut expected_alternations = 0;

        for i in 1..activities.len() {
            let prev_hard = Self::is_hard_activity(&activities[i - 1]);
            let curr_hard = Self::is_hard_activity(&activities[i]);

            if prev_hard != curr_hard {
                alternations += 1;
            }
            expected_alternations += 1;
        }

        // Pattern detected if >60% alternation
        #[allow(clippy::cast_precision_loss)]
        let alternation_rate = f64::from(alternations) / f64::from(expected_alternations);
        alternation_rate > 0.6
    }

    fn check_adequate_recovery(activities: &[Activity]) -> bool {
        if activities.len() < 3 {
            return true;
        }

        // Check for no more than 2 consecutive hard days
        let mut consecutive_hard = 0;
        let mut max_consecutive_hard = 0;

        for activity in activities {
            if Self::is_hard_activity(activity) {
                consecutive_hard += 1;
                max_consecutive_hard = max_consecutive_hard.max(consecutive_hard);
            } else {
                consecutive_hard = 0;
            }
        }

        max_consecutive_hard <= 2
    }

    fn detect_hr_drift(activities: &[Activity]) -> (bool, Option<f64>) {
        // Compare HR in first third vs last third of recent activities
        if activities.len() < 9 {
            return (false, None);
        }

        let third = activities.len() / 3;
        let first_third = &activities[0..third];
        let last_third = &activities[activities.len() - third..];

        let avg_hr_first: Vec<u32> = first_third
            .iter()
            .filter_map(Activity::average_heart_rate)
            .collect();

        let avg_hr_last: Vec<u32> = last_third
            .iter()
            .filter_map(Activity::average_heart_rate)
            .collect();

        if avg_hr_first.is_empty() || avg_hr_last.is_empty() {
            return (false, None);
        }

        #[allow(clippy::cast_precision_loss)]
        let mean_first = f64::from(avg_hr_first.iter().sum::<u32>()) / avg_hr_first.len() as f64;
        #[allow(clippy::cast_precision_loss)]
        let mean_last = f64::from(avg_hr_last.iter().sum::<u32>()) / avg_hr_last.len() as f64;

        let drift_percent = ((mean_last - mean_first) / mean_first) * 100.0;

        let drift_detected = drift_percent > HR_DRIFT_THRESHOLD_PERCENT;

        (drift_detected, Some(drift_percent))
    }

    fn detect_performance_decline(activities: &[Activity]) -> bool {
        // Compare pace in first half vs second half
        if activities.len() < 8 {
            return false;
        }

        let half = activities.len() / 2;
        let first_half = &activities[0..half];
        let second_half = &activities[half..];

        let avg_pace_first = Self::calculate_average_pace(first_half);
        let avg_pace_second = Self::calculate_average_pace(second_half);

        if let (Some(pace1), Some(pace2)) = (avg_pace_first, avg_pace_second) {
            // Decline if pace got >5% slower
            ((pace2 - pace1) / pace1) > 0.05
        } else {
            false
        }
    }

    fn calculate_average_pace(activities: &[Activity]) -> Option<f64> {
        let paces: Vec<f64> = activities
            .iter()
            .filter_map(|a| {
                let distance = a.distance_meters()?;
                let duration = a.duration_seconds();
                #[allow(clippy::cast_precision_loss)]
                if distance > 0.0 && duration > 0 {
                    Some(duration as f64 / distance) // seconds per meter
                } else {
                    None
                }
            })
            .collect();

        if paces.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            let avg_pace = paces.iter().sum::<f64>() / paces.len() as f64;
            Some(avg_pace)
        }
    }

    fn detect_insufficient_recovery(activities: &[Activity]) -> bool {
        // Check if there are 4+ hard days in a row
        let mut consecutive_hard = 0;

        for activity in activities {
            if Self::is_hard_activity(activity) {
                consecutive_hard += 1;
                if consecutive_hard >= 4 {
                    return true;
                }
            } else {
                consecutive_hard = 0;
            }
        }

        false
    }

    fn calculate_weekly_volumes(activities: &[Activity]) -> Vec<(u32, f64)> {
        let mut weekly_volumes: HashMap<u32, f64> = HashMap::new();

        if activities.is_empty() {
            return Vec::new();
        }

        // Safe: we just checked activities is not empty
        let Some(first_date) = activities.iter().map(Activity::start_date).min() else {
            return Vec::new();
        };

        for activity in activities {
            let days_since_start = (activity.start_date() - first_date).num_days();
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_sign_loss)]
            let week_num = (days_since_start / 7) as u32;

            let distance_km = activity.distance_meters().unwrap_or(0.0) / 1000.0;
            *weekly_volumes.entry(week_num).or_insert(0.0) += distance_km;
        }

        let mut result: Vec<(u32, f64)> = weekly_volumes.into_iter().collect();
        result.sort_by_key(|(week, _)| *week);
        result
    }

    fn determine_volume_trend(volumes: &[f64]) -> VolumeTrend {
        if volumes.len() < 3 {
            return VolumeTrend::Stable;
        }

        // Simple trend: compare first third to last third
        let third = volumes.len() / 3;
        #[allow(clippy::cast_precision_loss)]
        let first_third_avg = volumes[0..third].iter().sum::<f64>() / third as f64;
        #[allow(clippy::cast_precision_loss)]
        let last_third_avg = volumes[volumes.len() - third..].iter().sum::<f64>() / third as f64;

        let change_percent = ((last_third_avg - first_third_avg) / first_third_avg) * 100.0;

        if change_percent > 10.0 {
            VolumeTrend::Increasing
        } else if change_percent < -10.0 {
            VolumeTrend::Decreasing
        } else {
            VolumeTrend::Stable
        }
    }

    // === Empty Pattern Functions ===

    fn empty_schedule_pattern() -> WeeklySchedulePattern {
        WeeklySchedulePattern {
            most_common_days: Vec::new(),
            day_frequencies: HashMap::new(),
            most_common_times: Vec::new(),
            consistency_score: 0.0,
            avg_activities_per_week: 0.0,
        }
    }

    fn empty_hard_easy_pattern() -> HardEasyPattern {
        HardEasyPattern {
            pattern_detected: false,
            pattern_description: "Insufficient data for pattern detection".to_owned(),
            hard_percentage: 0.0,
            easy_percentage: 0.0,
            adequate_recovery: true,
        }
    }

    fn empty_overtraining_signals() -> OvertrainingSignals {
        OvertrainingSignals {
            hr_drift_detected: false,
            hr_drift_percent: None,
            performance_decline: false,
            insufficient_recovery: false,
            risk_level: RiskLevel::Low,
            warnings: vec!["Insufficient data for overtraining detection".to_owned()],
        }
    }

    fn empty_volume_progression() -> VolumeProgressionPattern {
        VolumeProgressionPattern {
            weekly_volumes: Vec::new(),
            week_numbers: Vec::new(),
            trend: VolumeTrend::Stable,
            volume_spikes_detected: false,
            spike_weeks: Vec::new(),
            recommendation: "Insufficient data for volume analysis".to_owned(),
        }
    }
}
