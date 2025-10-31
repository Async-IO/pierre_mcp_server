// ABOUTME: Holistic recovery scoring combining TSB, sleep quality, and HRV analysis
// ABOUTME: AI-powered rest day recommendations based on multi-factor recovery assessment
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Recovery Calculator Module
//!
//! This module provides holistic recovery assessment by combining multiple recovery indicators:
//! - Training Stress Balance (TSB) - training load vs. fitness
//! - Sleep Quality - duration, stages, efficiency
//! - Heart Rate Variability (HRV) - autonomic nervous system recovery
//!
//! # Scientific References
//!
//! - Meeusen, R., et al. (2013). Prevention, diagnosis, and treatment of the overtraining syndrome.
//!   *European Journal of Sport Science*, 13(1), 1-24. <https://doi.org/10.1080/17461391.2012.730061>
//!
//! - Halson, S.L. (2014). Monitoring training load to understand fatigue in athletes.
//!   *Sports Medicine*, 44(Suppl 2), S139-147. <https://doi.org/10.1007/s40279-014-0253-z>
//!
//! - Buchheit, M. (2014). Monitoring training status with HR measures: Do all roads lead to Rome?
//!   *Frontiers in Physiology*, 5, 73. <https://doi.org/10.3389/fphys.2014.00073>

use crate::errors::AppError;
use crate::intelligence::sleep_analysis::{
    HrvRecoveryStatus, HrvTrendAnalysis, SleepData, SleepQualityCategory, SleepQualityScore,
};
use crate::intelligence::training_load::TrainingLoad;
use serde::{Deserialize, Serialize};

/// Recovery score thresholds
///
/// Module kept for backward compatibility - use `SleepRecoveryConfig` for actual values
#[deprecated(
    since = "0.3.0",
    note = "Use IntelligenceConfig::global().sleep_recovery instead"
)]
pub mod recovery_thresholds {}

/// TSB (Training Stress Balance) thresholds for recovery interpretation
///
/// Module kept for backward compatibility - use `SleepRecoveryConfig` for actual values
#[deprecated(
    since = "0.3.0",
    note = "Use IntelligenceConfig::global().sleep_recovery instead"
)]
pub mod tsb_thresholds {}

/// Holistic recovery score combining multiple factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryScore {
    /// Overall recovery score (0-100)
    pub overall_score: f64,

    /// Recovery category
    pub recovery_category: RecoveryCategory,

    /// Component scores
    pub components: RecoveryComponents,

    /// Recovery readiness (ready for hard/moderate/easy training, or rest needed)
    pub training_readiness: TrainingReadiness,

    /// Insights
    pub insights: Vec<String>,

    /// Recommendations
    pub recommendations: Vec<String>,

    /// Whether a rest day is recommended
    pub rest_day_recommended: bool,

    /// Reasoning for recommendations
    pub reasoning: Vec<String>,
}

/// Recovery score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryComponents {
    /// TSB-based recovery score (0-100)
    pub tsb_score: f64,

    /// Sleep quality score (0-100)
    pub sleep_score: f64,

    /// HRV-based recovery score (0-100)
    pub hrv_score: Option<f64>,

    /// Number of components used in calculation
    pub components_available: u8,
}

/// Recovery category classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecoveryCategory {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// Training readiness based on recovery
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingReadiness {
    ReadyForHard,
    ReadyForModerate,
    EasyOnly,
    RestNeeded,
}

/// Rest day recommendation with detailed reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestDayRecommendation {
    /// Whether rest is recommended
    pub rest_recommended: bool,

    /// Confidence in recommendation (0-100)
    pub confidence: f64,

    /// Recovery score that led to recommendation
    pub recovery_score: f64,

    /// Primary reasons for recommendation
    pub primary_reasons: Vec<String>,

    /// Supporting factors
    pub supporting_factors: Vec<String>,

    /// Alternative suggestions if not resting
    pub alternatives: Vec<String>,

    /// Estimated recovery time needed (hours)
    pub estimated_recovery_hours: Option<f64>,
}

/// Recovery calculator
pub struct RecoveryCalculator;

impl RecoveryCalculator {
    /// Calculate holistic recovery score
    ///
    /// Combines TSB, sleep quality, and HRV (if available) into a single recovery score.
    ///
    /// Weighting:
    /// - TSB: 40% (training load balance)
    /// - Sleep: 40% (recovery quality)
    /// - HRV: 20% (if available, otherwise redistributed to TSB+Sleep)
    ///
    /// # Errors
    /// Returns error if input data is invalid
    pub fn calculate_recovery_score(
        training_load: &TrainingLoad,
        sleep_quality: &SleepQualityScore,
        hrv_analysis: Option<&HrvTrendAnalysis>,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> Result<RecoveryScore, AppError> {
        // Calculate TSB-based recovery score
        let tsb_score = Self::score_tsb(training_load.tsb, config);

        // Use sleep quality score directly
        let sleep_score = sleep_quality.overall_score;

        // Calculate HRV-based score if available
        let hrv_score = hrv_analysis.map(Self::score_hrv);

        // Calculate weighted overall score using config weights
        let (overall_score, components_available) = hrv_score.map_or(
            // Only TSB and Sleep: use no_hrv weights
            (
                tsb_score.mul_add(
                    config.recovery_scoring.tsb_weight_no_hrv,
                    sleep_score * config.recovery_scoring.sleep_weight_no_hrv,
                ),
                2,
            ),
            |hrv| {
                // All three components available: use full weights
                (
                    hrv.mul_add(
                        config.recovery_scoring.hrv_weight_full,
                        tsb_score.mul_add(
                            config.recovery_scoring.tsb_weight_full,
                            sleep_score * config.recovery_scoring.sleep_weight_full,
                        ),
                    ),
                    3,
                )
            },
        );

        // Determine recovery category
        let recovery_category = Self::categorize_recovery(overall_score, config);

        // Determine training readiness
        let training_readiness = Self::determine_training_readiness(
            overall_score,
            training_load.tsb,
            sleep_quality.quality_category,
            hrv_analysis.map(|h| h.recovery_status),
            config,
        );

        // Check if rest day recommended
        let rest_day_recommended = matches!(training_readiness, TrainingReadiness::RestNeeded);

        // Generate insights
        let insights = Self::generate_recovery_insights(
            overall_score,
            training_load,
            sleep_quality,
            hrv_analysis,
        );

        // Generate recommendations
        let (recommendations, reasoning) = Self::generate_recovery_recommendations(
            training_readiness,
            training_load,
            sleep_quality,
            hrv_analysis,
            config,
        );

        Ok(RecoveryScore {
            overall_score,
            recovery_category,
            components: RecoveryComponents {
                tsb_score,
                sleep_score,
                hrv_score,
                components_available,
            },
            training_readiness,
            insights,
            recommendations,
            rest_day_recommended,
            reasoning,
        })
    }

    /// Score TSB (Training Stress Balance) for recovery
    ///
    /// TSB interpretation based on Banister model:
    /// - Negative TSB = fatigue > fitness (building)
    /// - Positive TSB = fitness > fatigue (fresh)
    #[doc(hidden)]
    #[must_use]
    pub fn score_tsb(
        tsb: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> f64 {
        let detraining_tsb = config.training_stress_balance.detraining_tsb;
        let fresh_tsb_max = config.training_stress_balance.fresh_tsb_max;
        let fresh_tsb_min = config.training_stress_balance.fresh_tsb_min;
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;

        if (fresh_tsb_min..=fresh_tsb_max).contains(&tsb) {
            // Optimal fresh range: 100 points
            100.0
        } else if tsb > detraining_tsb {
            // Too fresh (risk of detraining): penalize
            100.0 - ((tsb - detraining_tsb) * 2.0).min(30.0)
        } else if tsb > fresh_tsb_max {
            // Between optimal and detraining: slight penalty
            ((tsb - fresh_tsb_max) / (detraining_tsb - fresh_tsb_max)).mul_add(-10.0, 100.0)
        } else if tsb >= 0.0 {
            // Slightly fresh (0 to fresh_tsb_min): 85-100 points
            (tsb / fresh_tsb_min).mul_add(15.0, 85.0)
        } else if tsb >= fatigued_tsb {
            // Productive fatigue: 60-85 points
            ((tsb - fatigued_tsb) / fatigued_tsb.abs()).mul_add(25.0, 60.0)
        } else if tsb >= highly_fatigued_tsb {
            // High fatigue: 30-60 points
            ((tsb - highly_fatigued_tsb) / (fatigued_tsb - highly_fatigued_tsb)).mul_add(30.0, 30.0)
        } else {
            // Extreme fatigue: 0-30 points
            30.0 - ((tsb.abs() - highly_fatigued_tsb.abs()) / highly_fatigued_tsb.abs() * 30.0)
                .min(30.0)
        }
    }

    /// Score HRV for recovery
    #[doc(hidden)]
    #[must_use]
    pub const fn score_hrv(hrv: &HrvTrendAnalysis) -> f64 {
        match hrv.recovery_status {
            HrvRecoveryStatus::Recovered => 100.0,
            HrvRecoveryStatus::Normal => 70.0,
            HrvRecoveryStatus::Fatigued => 40.0,
            HrvRecoveryStatus::HighlyFatigued => 20.0,
        }
    }

    /// Categorize overall recovery score
    #[doc(hidden)]
    #[must_use]
    pub fn categorize_recovery(
        score: f64,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> RecoveryCategory {
        let excellent_threshold = config.recovery_scoring.excellent_threshold;
        let good_threshold = config.recovery_scoring.good_threshold;
        let fair_threshold = config.recovery_scoring.fair_threshold;

        if score >= excellent_threshold {
            RecoveryCategory::Excellent
        } else if score >= good_threshold {
            RecoveryCategory::Good
        } else if score >= fair_threshold {
            RecoveryCategory::Fair
        } else {
            RecoveryCategory::Poor
        }
    }

    /// Determine training readiness
    #[doc(hidden)]
    #[must_use]
    pub fn determine_training_readiness(
        overall_score: f64,
        tsb: f64,
        sleep_category: SleepQualityCategory,
        hrv_status: Option<HrvRecoveryStatus>,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> TrainingReadiness {
        let excellent_threshold = config.recovery_scoring.excellent_threshold;
        let good_threshold = config.recovery_scoring.good_threshold;
        let fair_threshold = config.recovery_scoring.fair_threshold;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;

        // Check for critical rest indicators
        let critical_rest_needed = tsb < highly_fatigued_tsb
            || sleep_category == SleepQualityCategory::Poor
            || hrv_status == Some(HrvRecoveryStatus::HighlyFatigued);

        if critical_rest_needed || overall_score < fair_threshold {
            return TrainingReadiness::RestNeeded;
        }

        if overall_score >= excellent_threshold && tsb >= 0.0 {
            TrainingReadiness::ReadyForHard
        } else if overall_score >= good_threshold {
            TrainingReadiness::ReadyForModerate
        } else {
            TrainingReadiness::EasyOnly
        }
    }

    /// Generate recovery insights
    fn generate_recovery_insights(
        overall_score: f64,
        training_load: &TrainingLoad,
        sleep_quality: &SleepQualityScore,
        hrv_analysis: Option<&HrvTrendAnalysis>,
    ) -> Vec<String> {
        let mut insights = Vec::new();

        // Overall recovery insight
        insights.push(format!("Recovery score: {overall_score:.1}/100"));

        // TSB insight
        let tsb_status =
            crate::intelligence::TrainingLoadCalculator::interpret_tsb(training_load.tsb);
        insights.push(format!(
            "Training balance: TSB {:.1} ({:?})",
            training_load.tsb, tsb_status
        ));

        // Sleep insight
        insights.push(format!(
            "Sleep quality: {:.1}/100 ({:?})",
            sleep_quality.overall_score, sleep_quality.quality_category
        ));

        // HRV insight if available
        if let Some(hrv) = hrv_analysis {
            insights.push(format!(
                "HRV status: {:.1}ms ({:?})",
                hrv.current_rmssd, hrv.recovery_status
            ));
        }

        insights
    }

    /// Generate recovery recommendations and reasoning
    #[allow(clippy::type_complexity)]
    fn generate_recovery_recommendations(
        training_readiness: TrainingReadiness,
        training_load: &TrainingLoad,
        sleep_quality: &SleepQualityScore,
        hrv_analysis: Option<&HrvTrendAnalysis>,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> (Vec<String>, Vec<String>) {
        let mut recommendations = Vec::new();
        let mut reasoning = Vec::new();
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;
        let athlete_optimal_hours = config.sleep_duration.athlete_optimal_hours;
        let fair_threshold = config.recovery_scoring.fair_threshold;

        match training_readiness {
            TrainingReadiness::ReadyForHard => {
                recommendations
                    .push("You're well-recovered - ready for high-intensity training".to_string());
                reasoning.push("Excellent recovery score with positive TSB".to_string());
            }
            TrainingReadiness::ReadyForModerate => {
                recommendations.push("Continue with moderate-intensity training".to_string());
                reasoning.push("Good recovery indicators support continued training".to_string());
            }
            TrainingReadiness::EasyOnly => {
                recommendations.push("Limit to easy/recovery workouts today".to_string());
                if sleep_quality.overall_score < 70.0 {
                    reasoning.push(
                        "Suboptimal sleep quality suggests reduced training intensity".to_string(),
                    );
                }
                if training_load.tsb < 0.0 {
                    reasoning.push("Negative TSB indicates accumulated fatigue".to_string());
                }
            }
            TrainingReadiness::RestNeeded => {
                recommendations.push("REST DAY RECOMMENDED - prioritize recovery".to_string());

                if training_load.tsb < highly_fatigued_tsb {
                    reasoning.push(format!(
                        "Extreme training fatigue detected (TSB: {:.1})",
                        training_load.tsb
                    ));
                }
                if sleep_quality.overall_score < fair_threshold {
                    reasoning.push(format!(
                        "Poor sleep quality impacting recovery ({:.1}/100)",
                        sleep_quality.overall_score
                    ));
                }
                if let Some(hrv) = hrv_analysis {
                    if matches!(hrv.recovery_status, HrvRecoveryStatus::HighlyFatigued) {
                        reasoning
                            .push("Significantly suppressed HRV indicates high stress".to_string());
                    }
                }
            }
        }

        // Sleep-specific recommendations
        if sleep_quality.overall_score < 80.0 {
            recommendations.push(format!(
                "Prioritize {athlete_optimal_hours:.1}+ hours of quality sleep tonight"
            ));
        }

        // TSB-specific recommendations
        if training_load.tsb < fatigued_tsb {
            recommendations
                .push("Consider a recovery week to allow fitness gains to consolidate".to_string());
        }

        (recommendations, reasoning)
    }

    /// Generate detailed rest day recommendation
    ///
    /// # Errors
    /// Returns error if input data is invalid
    pub fn recommend_rest_day(
        recovery_score: &RecoveryScore,
        sleep_data: &SleepData,
        training_load: &TrainingLoad,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> Result<RestDayRecommendation, AppError> {
        let rest_recommended = recovery_score.rest_day_recommended;

        // Calculate confidence based on multiple factors alignment
        let confidence = Self::calculate_recommendation_confidence(
            recovery_score,
            sleep_data,
            training_load,
            config,
        );

        // Identify primary reasons
        let mut primary_reasons = Vec::new();
        let mut supporting_factors = Vec::new();
        let fair_threshold = config.recovery_scoring.fair_threshold;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;
        let short_sleep_threshold = config.sleep_duration.short_sleep_threshold;
        let poor_efficiency_threshold = config.sleep_efficiency.poor_threshold + 5.0;

        if recovery_score.overall_score < fair_threshold {
            primary_reasons.push(format!(
                "Low overall recovery score ({:.1}/100)",
                recovery_score.overall_score
            ));
        }

        if training_load.tsb < highly_fatigued_tsb {
            primary_reasons.push(format!(
                "Extreme training fatigue (TSB: {:.1})",
                training_load.tsb
            ));
        }

        if sleep_data.duration_hours < short_sleep_threshold {
            primary_reasons.push(format!(
                "Sleep deprivation detected ({:.1}h < {short_sleep_threshold:.1}h)",
                sleep_data.duration_hours
            ));
        }

        if let Some(efficiency) = sleep_data.efficiency_percent {
            if efficiency < poor_efficiency_threshold {
                supporting_factors.push(format!("Low sleep efficiency ({efficiency:.1}%)"));
            }
        }

        // Generate alternatives if rest not taken
        let alternatives = if rest_recommended {
            vec![
                "If you must train: very easy, short (<30min) active recovery only".to_string(),
                "Focus on mobility, stretching, or yoga instead of traditional training"
                    .to_string(),
                "Monitor HRV tomorrow - if still low, take another rest day".to_string(),
            ]
        } else {
            vec!["Continue with planned training but monitor fatigue levels".to_string()]
        };

        // Estimate recovery time needed
        let estimated_recovery_hours = if rest_recommended {
            Some(Self::estimate_recovery_time(
                recovery_score,
                training_load,
                config,
            ))
        } else {
            None
        };

        Ok(RestDayRecommendation {
            rest_recommended,
            confidence,
            recovery_score: recovery_score.overall_score,
            primary_reasons,
            supporting_factors,
            alternatives,
            estimated_recovery_hours,
        })
    }

    /// Calculate confidence in rest day recommendation
    fn calculate_recommendation_confidence(
        recovery_score: &RecoveryScore,
        sleep_data: &SleepData,
        training_load: &TrainingLoad,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> f64 {
        let mut confidence_factors = 0;
        let mut total_factors = 0;
        let fair_threshold = config.recovery_scoring.fair_threshold;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;
        let short_sleep_threshold = config.sleep_duration.short_sleep_threshold;

        // Factor 1: Recovery score severity
        total_factors += 1;
        if recovery_score.overall_score < fair_threshold {
            confidence_factors += 1;
        }

        // Factor 2: TSB severity
        total_factors += 1;
        if training_load.tsb < highly_fatigued_tsb {
            confidence_factors += 1;
        }

        // Factor 3: Sleep quality
        total_factors += 1;
        if sleep_data.duration_hours < short_sleep_threshold {
            confidence_factors += 1;
        }

        // Factor 4: Multiple components available
        total_factors += 1;
        if recovery_score.components.components_available >= 3 {
            confidence_factors += 1;
        }

        (f64::from(confidence_factors) / f64::from(total_factors)) * 100.0
    }

    /// Estimate recovery time needed (hours)
    fn estimate_recovery_time(
        recovery_score: &RecoveryScore,
        training_load: &TrainingLoad,
        config: &crate::config::intelligence_config::SleepRecoveryConfig,
    ) -> f64 {
        let fair_threshold = config.recovery_scoring.fair_threshold;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;

        // Base recovery time on severity
        let base_hours = if recovery_score.overall_score < 30.0 {
            48.0 // Severe fatigue: 2 days
        } else if recovery_score.overall_score < fair_threshold {
            24.0 // Moderate fatigue: 1 day
        } else {
            12.0 // Mild fatigue: half day
        };

        // Adjust based on TSB
        let tsb_adjustment = if training_load.tsb < highly_fatigued_tsb {
            24.0 // Add extra day for extreme fatigue
        } else if training_load.tsb < fatigued_tsb {
            12.0 // Add half day
        } else {
            0.0
        };

        base_hours + tsb_adjustment
    }
}
