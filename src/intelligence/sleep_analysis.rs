// ABOUTME: Sleep quality analysis using NSF/AASM guidelines and HRV metrics
// ABOUTME: Scientific sleep scoring algorithms for recovery assessment
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Sleep Quality Analysis Module
//!
//! This module implements sleep analysis algorithms based on National Sleep Foundation (NSF)
//! and American Academy of Sleep Medicine (AASM) guidelines. It provides sleep quality scoring,
//! stage analysis, and HRV (Heart Rate Variability) trend detection for recovery assessment.
//!
//! # Scientific References
//!
//! - Watson, N.F., et al. (2015). Recommended Amount of Sleep for a Healthy Adult.
//!   *Sleep*, 38(6), 843-844. <https://doi.org/10.5665/sleep.4716>
//!
//! - Hirshkowitz, M., et al. (2015). National Sleep Foundation's sleep time duration recommendations.
//!   *Sleep Health*, 1(1), 40-43. <https://doi.org/10.1016/j.sleh.2014.12.010>
//!
//! - Shaffer, F., & Ginsberg, J.P. (2017). An Overview of Heart Rate Variability Metrics and Norms.
//!   *Frontiers in Public Health*, 5, 258. <https://doi.org/10.3389/fpubh.2017.00258>
//!
//! - Plews, D.J., et al. (2013). Training adaptation and heart rate variability in elite endurance athletes.
//!   *International Journal of Sports Physiology and Performance*, 8(5), 512-519.

use crate::errors::AppError;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Custom deserializer for flexible date parsing
/// Accepts both full ISO 8601 datetime ("2025-11-26T00:00:00Z") and simple date ("2025-11-26")
fn deserialize_flexible_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Try full ISO 8601 datetime first
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try ISO 8601 without timezone (assume UTC)
    if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&dt));
    }

    // Try simple date format (YYYY-MM-DD), convert to midnight UTC
    if let Ok(date) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        let datetime = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| serde::de::Error::custom("Invalid date"))?;
        return Ok(Utc.from_utc_datetime(&datetime));
    }

    Err(serde::de::Error::custom(format!(
        "Invalid date format: '{s}'. Expected 'YYYY-MM-DD' or 'YYYY-MM-DDTHH:MM:SSZ'"
    )))
}

/// Sleep quality score result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepQualityScore {
    /// Overall sleep quality score (0-100)
    pub overall_score: f64,

    /// Duration score component (0-100)
    pub duration_score: f64,

    /// Sleep stage quality score (0-100)
    pub stage_quality_score: f64,

    /// Sleep efficiency score (0-100)
    pub efficiency_score: f64,

    /// Quality category
    pub quality_category: SleepQualityCategory,

    /// Detailed insights
    pub insights: Vec<String>,

    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Sleep quality category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SleepQualityCategory {
    /// Excellent sleep quality (>8 hours, high efficiency)
    Excellent,
    /// Good sleep quality (7-8 hours, good efficiency)
    Good,
    /// Fair sleep quality (6-7 hours, moderate efficiency)
    Fair,
    /// Poor sleep quality (<6 hours or low efficiency)
    Poor,
}

/// Sleep data from fitness providers (Fitbit, Garmin)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepData {
    /// Date of sleep session
    /// Accepts both "YYYY-MM-DD" and full ISO 8601 "YYYY-MM-DDTHH:MM:SSZ" formats
    #[serde(deserialize_with = "deserialize_flexible_datetime")]
    pub date: DateTime<Utc>,

    /// Total sleep duration (hours)
    pub duration_hours: f64,

    /// Deep sleep duration (hours)
    pub deep_sleep_hours: Option<f64>,

    /// REM sleep duration (hours)
    pub rem_sleep_hours: Option<f64>,

    /// Light sleep duration (hours)
    pub light_sleep_hours: Option<f64>,

    /// Awake time during sleep (hours)
    pub awake_hours: Option<f64>,

    /// Sleep efficiency (time asleep / time in bed)
    pub efficiency_percent: Option<f64>,

    /// HRV RMSSD value (milliseconds)
    pub hrv_rmssd_ms: Option<f64>,

    /// Resting heart rate during sleep (bpm)
    pub resting_hr_bpm: Option<u32>,

    /// Provider-specific sleep score (if available)
    pub provider_score: Option<f64>,
}

/// HRV trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrvTrendAnalysis {
    /// Current HRV RMSSD value (ms)
    pub current_rmssd: f64,

    /// 7-day average HRV RMSSD (ms)
    pub weekly_average_rmssd: f64,

    /// Baseline HRV (30-day average, ms)
    pub baseline_rmssd: Option<f64>,

    /// Change from baseline (percentage)
    pub baseline_deviation_percent: Option<f64>,

    /// Recovery status based on HRV
    pub recovery_status: HrvRecoveryStatus,

    /// Trend direction
    pub trend: HrvTrend,

    /// Insights
    pub insights: Vec<String>,
}

/// HRV-based recovery status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HrvRecoveryStatus {
    /// Fully recovered (HRV above baseline)
    Recovered,
    /// Normal recovery state (HRV at baseline)
    Normal,
    /// Fatigued (HRV below baseline)
    Fatigued,
    /// Highly fatigued (HRV significantly below baseline)
    HighlyFatigued,
}

/// HRV trend direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HrvTrend {
    /// HRV is improving over time
    Improving,
    /// HRV is stable
    Stable,
    /// HRV is declining over time
    Declining,
}

/// Sleep analyzer for calculating sleep quality scores
pub struct SleepAnalyzer;

impl SleepAnalyzer {
    /// Calculate sleep quality score from sleep data
    ///
    /// Uses NSF/AASM guidelines for scoring sleep duration, stages, and efficiency.
    ///
    /// # Errors
    /// Returns `AppError` if sleep data is invalid (negative values, impossible percentages)
    pub fn calculate_sleep_quality(
        sleep: &SleepData,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> Result<SleepQualityScore, AppError> {
        // Validate input
        if sleep.duration_hours < 0.0 {
            return Err(AppError::invalid_input(
                "Sleep duration cannot be negative".to_owned(),
            ));
        }

        // Calculate duration score (0-100)
        let duration_score = Self::score_duration(sleep.duration_hours, config);

        // Calculate stage quality score (0-100) if stage data available
        let stage_quality_score = if let (Some(deep), Some(rem), Some(light)) = (
            sleep.deep_sleep_hours,
            sleep.rem_sleep_hours,
            sleep.light_sleep_hours,
        ) {
            Self::score_sleep_stages(sleep.duration_hours, deep, rem, light, config)?
        } else {
            50.0 // Neutral score if stage data unavailable
        };

        // Calculate efficiency score (0-100)
        let efficiency_score = sleep
            .efficiency_percent
            .map_or(50.0, |eff| Self::score_efficiency(eff, config));

        // Overall score: weighted average
        // Duration: 40%, Stages: 35%, Efficiency: 25%
        let overall_score = efficiency_score.mul_add(
            0.25,
            duration_score.mul_add(0.4, stage_quality_score * 0.35),
        );

        // Determine quality category
        let quality_category = Self::categorize_quality(overall_score);

        // Generate insights
        let mut insights = Vec::new();
        Self::add_duration_insights(&mut insights, sleep.duration_hours, config);
        if let Some(efficiency) = sleep.efficiency_percent {
            Self::add_efficiency_insights(&mut insights, efficiency, config);
        }

        // Generate recommendations
        let recommendations = Self::generate_recommendations(sleep, overall_score, config);

        Ok(SleepQualityScore {
            overall_score,
            duration_score,
            stage_quality_score,
            efficiency_score,
            quality_category,
            insights,
            recommendations,
        })
    }

    /// Score sleep duration based on NSF guidelines
    ///
    /// Reference: Watson et al. (2015)
    #[doc(hidden)]
    #[must_use]
    pub fn score_duration(
        hours: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> f64 {
        let adult_min_hours = config.sleep_duration.adult_min_hours;
        let adult_max_hours = config.sleep_duration.adult_max_hours;
        let athlete_optimal_hours = config.sleep_duration.athlete_optimal_hours;
        let short_sleep_threshold = config.sleep_duration.short_sleep_threshold;
        let very_short_sleep_threshold = config.sleep_duration.very_short_sleep_threshold;

        if (athlete_optimal_hours..=adult_max_hours).contains(&hours) {
            100.0 // Optimal for athletes
        } else if (adult_min_hours..athlete_optimal_hours).contains(&hours) {
            ((hours - adult_min_hours) / (athlete_optimal_hours - adult_min_hours))
                .mul_add(15.0, 85.0)
        } else if hours > adult_max_hours {
            // Excessive sleep (diminishing returns)
            100.0 - ((hours - adult_max_hours) * 10.0).min(30.0)
        } else if hours >= short_sleep_threshold {
            // Between 6-7 hours
            ((hours - short_sleep_threshold) / (adult_min_hours - short_sleep_threshold))
                .mul_add(35.0, 50.0)
        } else if hours >= very_short_sleep_threshold {
            // Between 5-6 hours
            ((hours - very_short_sleep_threshold)
                / (short_sleep_threshold - very_short_sleep_threshold))
                .mul_add(25.0, 25.0)
        } else {
            // Less than 5 hours (severe deprivation)
            (hours / very_short_sleep_threshold * 25.0).max(0.0)
        }
    }

    /// Score sleep stages based on AASM guidelines
    ///
    /// Reference: Hirshkowitz et al. (2015)
    ///
    /// # Errors
    /// Returns error if stage percentages are invalid
    fn score_sleep_stages(
        total_hours: f64,
        deep_hours: f64,
        rem_hours: f64,
        light_hours: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> Result<f64, AppError> {
        let deep_sleep_min_percent = config.sleep_stages.deep_sleep_min_percent;
        let deep_sleep_max_percent = config.sleep_stages.deep_sleep_max_percent;
        let rem_sleep_min_percent = config.sleep_stages.rem_sleep_min_percent;
        let rem_sleep_max_percent = config.sleep_stages.rem_sleep_max_percent;
        let light_sleep_min_percent = config.sleep_stages.light_sleep_min_percent;
        let light_sleep_max_percent = config.sleep_stages.light_sleep_max_percent;

        if total_hours <= 0.0 {
            return Err(AppError::invalid_input(
                "Total sleep hours must be positive".to_owned(),
            ));
        }

        // Calculate percentages
        let deep_percent = (deep_hours / total_hours) * 100.0;
        let rem_percent = (rem_hours / total_hours) * 100.0;
        let light_percent = (light_hours / total_hours) * 100.0;

        // Score deep sleep (0-33.3 points)
        let deep_score =
            if (deep_sleep_min_percent..=deep_sleep_max_percent).contains(&deep_percent) {
                33.3
            } else if deep_percent < deep_sleep_min_percent {
                (deep_percent / deep_sleep_min_percent * 33.3).max(0.0)
            } else {
                33.3 - ((deep_percent - deep_sleep_max_percent) * 0.5).min(20.0)
            };

        // Score REM sleep (0-33.3 points)
        let rem_score = if (rem_sleep_min_percent..=rem_sleep_max_percent).contains(&rem_percent) {
            33.3
        } else if rem_percent < rem_sleep_min_percent {
            (rem_percent / rem_sleep_min_percent * 33.3).max(0.0)
        } else {
            33.3 - ((rem_percent - rem_sleep_max_percent) * 0.5).min(20.0)
        };

        // Score light sleep (0-33.4 points)
        let light_score =
            if (light_sleep_min_percent..=light_sleep_max_percent).contains(&light_percent) {
                33.4
            } else {
                let distance = if light_percent < light_sleep_min_percent {
                    light_sleep_min_percent - light_percent
                } else {
                    light_percent - light_sleep_max_percent
                };
                distance.mul_add(-0.5, 33.4).max(0.0)
            };

        Ok(deep_score + rem_score + light_score)
    }

    /// Score sleep efficiency
    ///
    /// Sleep efficiency = (time asleep / time in bed) x 100
    #[doc(hidden)]
    #[must_use]
    pub fn score_efficiency(
        efficiency_percent: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> f64 {
        let excellent_threshold = config.sleep_efficiency.excellent_threshold;
        let good_threshold = config.sleep_efficiency.good_threshold;
        let poor_threshold = config.sleep_efficiency.poor_threshold;

        if efficiency_percent >= excellent_threshold {
            100.0
        } else if efficiency_percent >= good_threshold {
            ((efficiency_percent - good_threshold) / (excellent_threshold - good_threshold))
                .mul_add(20.0, 80.0)
        } else if efficiency_percent >= poor_threshold + 5.0 {
            ((efficiency_percent - (poor_threshold + 5.0))
                / (good_threshold - poor_threshold - 5.0))
                .mul_add(20.0, 60.0)
        } else if efficiency_percent >= poor_threshold - 5.0 {
            ((efficiency_percent - (poor_threshold - 5.0)) / 10.0).mul_add(20.0, 40.0)
        } else {
            (efficiency_percent / (poor_threshold - 5.0) * 40.0).max(0.0)
        }
    }

    /// Categorize overall sleep quality
    fn categorize_quality(score: f64) -> SleepQualityCategory {
        if score >= 85.0 {
            SleepQualityCategory::Excellent
        } else if score >= 70.0 {
            SleepQualityCategory::Good
        } else if score >= 50.0 {
            SleepQualityCategory::Fair
        } else {
            SleepQualityCategory::Poor
        }
    }

    /// Add duration-specific insights
    fn add_duration_insights(
        insights: &mut Vec<String>,
        hours: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) {
        let adult_max_hours = config.sleep_duration.adult_max_hours;
        let athlete_min_hours = config.sleep_duration.athlete_min_hours;
        let athlete_optimal_hours = config.sleep_duration.athlete_optimal_hours;
        let short_sleep_threshold = config.sleep_duration.short_sleep_threshold;

        if (athlete_optimal_hours..=adult_max_hours).contains(&hours) {
            insights.push(format!(
                "Sleep duration ({hours:.1}h) is optimal for athletic recovery"
            ));
        } else if hours < athlete_min_hours {
            insights.push(format!(
                "Sleep duration ({hours:.1}h) is below recommended for athletes ({athlete_min_hours:.1}-{adult_max_hours:.1}h)"
            ));
        } else if hours > adult_max_hours {
            insights.push(format!(
                "Sleep duration ({hours:.1}h) exceeds typical recommendations ({athlete_optimal_hours:.1}-{adult_max_hours:.1}h)"
            ));
        }

        if hours < short_sleep_threshold {
            insights.push("Sleep deprivation detected - performance may be impaired".to_owned());
        }
    }

    /// Add efficiency-specific insights
    fn add_efficiency_insights(
        insights: &mut Vec<String>,
        efficiency: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) {
        let excellent_threshold = config.sleep_efficiency.excellent_threshold;
        let poor_threshold = config.sleep_efficiency.poor_threshold;

        if efficiency >= excellent_threshold {
            insights.push(format!("Excellent sleep efficiency ({efficiency:.1}%)"));
        } else if efficiency < poor_threshold + 5.0 {
            insights.push(format!(
                "Low sleep efficiency ({efficiency:.1}%) - consider sleep hygiene improvements"
            ));
        }
    }

    /// Generate personalized recommendations
    fn generate_recommendations(
        sleep: &SleepData,
        overall_score: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        let athlete_min_hours = config.sleep_duration.athlete_min_hours;
        let athlete_optimal_hours = config.sleep_duration.athlete_optimal_hours;
        let poor_efficiency_threshold = config.sleep_efficiency.poor_threshold + 5.0;

        // Duration recommendations
        if sleep.duration_hours < athlete_min_hours {
            recommendations.push(format!(
                "Aim for {athlete_optimal_hours:.1} hours of sleep for optimal recovery"
            ));
        }

        // Efficiency recommendations
        if let Some(efficiency) = sleep.efficiency_percent {
            if efficiency < poor_efficiency_threshold {
                recommendations.push(
                    "Improve sleep efficiency with consistent sleep schedule and dark room"
                        .to_owned(),
                );
            }
        }

        // General recommendations based on score
        if overall_score < 70.0 {
            recommendations.push(
                "Consider sleep hygiene: limit screen time 1h before bed, cool room (65-68°F)"
                    .to_owned(),
            );
        }

        if recommendations.is_empty() {
            recommendations
                .push("Maintain current sleep patterns for continued recovery".to_owned());
        }

        recommendations
    }

    /// Analyze HRV trends for recovery assessment
    ///
    /// Reference: Plews et al. (2013), Shaffer & Ginsberg (2017)
    ///
    /// # Errors
    /// Returns error if HRV data is invalid
    pub fn analyze_hrv_trends(
        current_rmssd: f64,
        recent_rmssd_values: &[f64],
        baseline_rmssd: Option<f64>,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> Result<HrvTrendAnalysis, AppError> {
        if current_rmssd <= 0.0 {
            return Err(AppError::invalid_input(
                "HRV RMSSD must be positive".to_owned(),
            ));
        }

        // Calculate weekly average
        #[allow(clippy::cast_precision_loss)]
        // Safe: recent_rmssd_values is user-provided HRV data, typically < 100 samples
        let weekly_average_rmssd = if recent_rmssd_values.is_empty() {
            current_rmssd
        } else {
            recent_rmssd_values.iter().sum::<f64>() / recent_rmssd_values.len() as f64
        };

        // Calculate baseline deviation if baseline available
        let baseline_deviation_percent = baseline_rmssd.map(|baseline| {
            if baseline > 0.0 {
                ((current_rmssd - baseline) / baseline) * 100.0
            } else {
                0.0
            }
        });

        // Determine recovery status
        let recovery_status = Self::determine_hrv_recovery_status(
            current_rmssd,
            weekly_average_rmssd,
            baseline_deviation_percent,
            config,
        );

        // Determine trend
        let trend = Self::determine_hrv_trend(current_rmssd, weekly_average_rmssd);

        // Generate insights
        let insights = Self::generate_hrv_insights(
            current_rmssd,
            weekly_average_rmssd,
            baseline_deviation_percent,
            recovery_status,
        );

        Ok(HrvTrendAnalysis {
            current_rmssd,
            weekly_average_rmssd,
            baseline_rmssd,
            baseline_deviation_percent,
            recovery_status,
            trend,
            insights,
        })
    }

    /// Determine HRV recovery status
    fn determine_hrv_recovery_status(
        current: f64,
        weekly_avg: f64,
        baseline_deviation: Option<f64>,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> HrvRecoveryStatus {
        let baseline_deviation_concern = config.hrv.baseline_deviation_concern_percent;
        let rmssd_decrease_threshold = config.hrv.rmssd_decrease_concern_threshold;
        let rmssd_increase_threshold = config.hrv.rmssd_increase_good_threshold;

        if let Some(deviation) = baseline_deviation {
            if deviation < -baseline_deviation_concern {
                return HrvRecoveryStatus::HighlyFatigued;
            } else if deviation < -5.0 {
                return HrvRecoveryStatus::Fatigued;
            }
        }

        // Compare to weekly average
        let change_from_avg = current - weekly_avg;
        if change_from_avg >= rmssd_increase_threshold {
            HrvRecoveryStatus::Recovered
        } else if change_from_avg <= rmssd_decrease_threshold {
            HrvRecoveryStatus::Fatigued
        } else {
            HrvRecoveryStatus::Normal
        }
    }

    /// Determine HRV trend direction
    fn determine_hrv_trend(current: f64, weekly_avg: f64) -> HrvTrend {
        let change_percent = if weekly_avg > 0.0 {
            ((current - weekly_avg) / weekly_avg) * 100.0
        } else {
            0.0
        };

        if change_percent >= 5.0 {
            HrvTrend::Improving
        } else if change_percent <= -5.0 {
            HrvTrend::Declining
        } else {
            HrvTrend::Stable
        }
    }

    /// Generate HRV-specific insights
    fn generate_hrv_insights(
        current: f64,
        weekly_avg: f64,
        baseline_deviation: Option<f64>,
        status: HrvRecoveryStatus,
    ) -> Vec<String> {
        let mut insights = Vec::new();

        insights.push(format!(
            "Current HRV: {current:.1}ms (7-day avg: {weekly_avg:.1}ms)"
        ));

        if let Some(deviation) = baseline_deviation {
            if deviation.abs() >= 5.0 {
                insights.push(format!(
                    "HRV is {deviation:.1}% {} baseline",
                    if deviation > 0.0 { "above" } else { "below" }
                ));
            }
        }

        match status {
            HrvRecoveryStatus::Recovered => {
                insights.push(
                    "Elevated HRV indicates good recovery - ready for high-intensity training"
                        .to_owned(),
                );
            }
            HrvRecoveryStatus::Normal => {
                insights
                    .push("HRV is within normal range - continue current training load".to_owned());
            }
            HrvRecoveryStatus::Fatigued => {
                insights.push(
                    "Decreased HRV suggests fatigue - consider reducing training intensity"
                        .to_owned(),
                );
            }
            HrvRecoveryStatus::HighlyFatigued => {
                insights.push(
                    "Significantly decreased HRV indicates high fatigue - prioritize recovery"
                        .to_owned(),
                );
            }
        }

        insights
    }
}
