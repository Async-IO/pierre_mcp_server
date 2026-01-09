// ABOUTME: Holistic recovery scoring combining TSB, sleep quality, and HRV analysis
// ABOUTME: AI-powered rest day recommendations based on multi-factor recovery assessment
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

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

use crate::config::intelligence::SleepRecoveryConfig;
use crate::errors::AppError;
use crate::intelligence::algorithms::RecoveryAggregationAlgorithm;
use crate::intelligence::sleep_analysis::{
    HrvRecoveryStatus, HrvTrendAnalysis, SleepData, SleepQualityCategory, SleepQualityScore,
};
use crate::intelligence::training_load::TrainingLoad;
use crate::intelligence::TrainingLoadCalculator;
use serde::{Deserialize, Serialize};

/// Recovery recommendations and reasoning
#[derive(Debug, Clone)]
struct RecoveryRecommendations {
    recommendations: Vec<String>,
    reasoning: Vec<String>,
}

/// Holistic recovery score combining multiple factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryScore {
    /// Overall recovery score (0-100)
    pub overall_score: f64,

    /// Recovery category
    pub recovery_category: RecoveryCategory,

    /// Data completeness indicator
    pub data_completeness: DataCompleteness,

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

    /// Limitations due to missing data sources
    pub limitations: Vec<String>,
}

/// Data completeness indicator for recovery scoring
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataCompleteness {
    /// All data sources available (TSB + Sleep + HRV)
    Full,
    /// Some data sources available (TSB + Sleep, or TSB + HRV)
    Partial,
    /// Only TSB available (activity data only)
    TsbOnly,
}

/// Recovery score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryComponents {
    /// TSB-based recovery score (0-100)
    pub tsb_score: f64,

    /// Sleep quality score (0-100), None when in TSB-only mode
    pub sleep_score: Option<f64>,

    /// HRV-based recovery score (0-100)
    pub hrv_score: Option<f64>,

    /// Number of components used in calculation
    pub components_available: u8,
}

/// Recovery category classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecoveryCategory {
    /// Excellent recovery (low fatigue, ready for high intensity)
    Excellent,
    /// Good recovery (moderate fatigue, ready for moderate intensity)
    Good,
    /// Fair recovery (elevated fatigue, easy training only)
    Fair,
    /// Poor recovery (high fatigue, rest recommended)
    Poor,
}

/// Training readiness based on recovery
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingReadiness {
    /// Ready for hard/intense training
    ReadyForHard,
    /// Ready for moderate intensity training
    ReadyForModerate,
    /// Only easy/recovery training recommended
    EasyOnly,
    /// Rest day needed
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
    /// Combines TSB, sleep quality, and HRV (if available) into a single recovery score
    /// using the specified aggregation algorithm.
    ///
    /// # Errors
    /// Returns error if input data is invalid or algorithm fails
    pub fn calculate_recovery_score(
        training_load: &TrainingLoad,
        sleep_quality: &SleepQualityScore,
        hrv_analysis: Option<&HrvTrendAnalysis>,
        config: &SleepRecoveryConfig,
        algorithm: &RecoveryAggregationAlgorithm,
    ) -> Result<RecoveryScore, AppError> {
        // Calculate TSB-based recovery score
        let tsb_score = Self::score_tsb(training_load.tsb, config);

        // Use sleep quality score directly
        let sleep_score = sleep_quality.overall_score;

        // Calculate HRV-based score if available
        let hrv_score = hrv_analysis.map(Self::score_hrv);

        // Calculate overall score using the specified algorithm
        let overall_score = algorithm.aggregate(tsb_score, sleep_score, hrv_score)?;

        // Determine number of components available and data completeness
        let (components_available, data_completeness) = if hrv_score.is_some() {
            (3, DataCompleteness::Full)
        } else {
            (2, DataCompleteness::Partial)
        };

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
        let recovery_recommendations = Self::generate_recovery_recommendations(
            training_readiness,
            training_load,
            sleep_quality,
            hrv_analysis,
            config,
        );
        let recommendations = recovery_recommendations.recommendations;
        let reasoning = recovery_recommendations.reasoning;

        // Generate limitations based on missing data
        let limitations = if hrv_score.is_none() {
            vec!["HRV data unavailable - consider using a device that tracks HRV for more accurate recovery assessment".to_owned()]
        } else {
            vec![]
        };

        Ok(RecoveryScore {
            overall_score,
            recovery_category,
            data_completeness,
            components: RecoveryComponents {
                tsb_score,
                sleep_score: Some(sleep_score),
                hrv_score,
                components_available,
            },
            training_readiness,
            insights,
            recommendations,
            rest_day_recommended,
            reasoning,
            limitations,
        })
    }

    /// Calculate TSB-only recovery score when sleep data is unavailable
    ///
    /// Uses Training Stress Balance from activity data alone to provide a partial
    /// recovery assessment. This is a fallback mode when no sleep provider is connected
    /// and no manual sleep data is provided.
    ///
    /// # TSB-Only Scoring
    /// - TSB > 20: Fresh (score 90-100)
    /// - TSB 10-20: Recovered (score 75-89)
    /// - TSB 0-10: Neutral (score 60-74)
    /// - TSB -10-0: Tired (score 45-59)
    /// - TSB < -10: Fatigued (score 30-44)
    ///
    /// # Errors
    /// Returns error if training load data is invalid
    pub fn calculate_recovery_score_tsb_only(
        training_load: &TrainingLoad,
        config: &SleepRecoveryConfig,
    ) -> Result<RecoveryScore, AppError> {
        // Calculate TSB-based recovery score (100% weight in TSB-only mode)
        let tsb_score = Self::score_tsb(training_load.tsb, config);

        // In TSB-only mode, the overall score is entirely based on TSB
        let overall_score = tsb_score;

        // Determine recovery category
        let recovery_category = Self::categorize_recovery(overall_score, config);

        // Determine training readiness (conservative without sleep/HRV data)
        let training_readiness =
            Self::determine_training_readiness_tsb_only(overall_score, training_load.tsb, config);

        // Check if rest day recommended
        let rest_day_recommended = matches!(training_readiness, TrainingReadiness::RestNeeded);

        // Generate TSB-only insights
        let insights = Self::generate_tsb_only_insights(overall_score, training_load);

        // Generate TSB-only recommendations
        let (recommendations, reasoning) =
            Self::generate_tsb_only_recommendations(training_readiness, training_load, config);

        // Limitations for TSB-only mode
        let limitations = vec![
            "Sleep data unavailable - score based on training load only".to_owned(),
            "Connect a sleep provider (WHOOP, Garmin, Fitbit) for more accurate recovery assessment".to_owned(),
        ];

        Ok(RecoveryScore {
            overall_score,
            recovery_category,
            data_completeness: DataCompleteness::TsbOnly,
            components: RecoveryComponents {
                tsb_score,
                sleep_score: None,
                hrv_score: None,
                components_available: 1,
            },
            training_readiness,
            insights,
            recommendations,
            rest_day_recommended,
            reasoning,
            limitations,
        })
    }

    /// Determine training readiness for TSB-only mode (more conservative)
    fn determine_training_readiness_tsb_only(
        overall_score: f64,
        tsb: f64,
        config: &SleepRecoveryConfig,
    ) -> TrainingReadiness {
        let excellent_threshold = config.recovery_scoring.excellent_threshold;
        let good_threshold = config.recovery_scoring.good_threshold;
        let fair_threshold = config.recovery_scoring.fair_threshold;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;

        // Critical rest indicator based on TSB alone
        if tsb < highly_fatigued_tsb || overall_score < fair_threshold {
            return TrainingReadiness::RestNeeded;
        }

        // Be more conservative in TSB-only mode (require higher scores)
        if overall_score >= excellent_threshold && tsb >= 10.0 {
            TrainingReadiness::ReadyForHard
        } else if overall_score >= good_threshold && tsb >= 0.0 {
            TrainingReadiness::ReadyForModerate
        } else {
            TrainingReadiness::EasyOnly
        }
    }

    /// Generate insights for TSB-only mode
    fn generate_tsb_only_insights(overall_score: f64, training_load: &TrainingLoad) -> Vec<String> {
        let mut insights = Vec::new();

        // Overall recovery insight
        insights.push(format!(
            "Recovery score: {overall_score:.1}/100 (TSB-only, partial assessment)"
        ));

        // TSB interpretation
        let tsb_status = TrainingLoadCalculator::interpret_tsb(training_load.tsb);
        insights.push(format!(
            "Training balance: TSB {:.1} ({:?})",
            training_load.tsb, tsb_status
        ));

        // Fitness/fatigue context
        insights.push(format!(
            "Fitness (CTL): {:.1}, Fatigue (ATL): {:.1}",
            training_load.ctl, training_load.atl
        ));

        // Note about limited data
        insights.push(
            "Note: Assessment based on training load only; sleep quality not factored".to_owned(),
        );

        insights
    }

    /// Generate recommendations for TSB-only mode
    fn generate_tsb_only_recommendations(
        training_readiness: TrainingReadiness,
        training_load: &TrainingLoad,
        config: &SleepRecoveryConfig,
    ) -> (Vec<String>, Vec<String>) {
        let mut recommendations = Vec::new();
        let mut reasoning = Vec::new();
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;

        match training_readiness {
            TrainingReadiness::ReadyForHard => {
                recommendations.push(
                    "TSB indicates good recovery - likely ready for high-intensity training"
                        .to_owned(),
                );
                reasoning.push("Positive TSB suggests fitness exceeds fatigue".to_owned());
            }
            TrainingReadiness::ReadyForModerate => {
                recommendations.push("TSB suggests moderate training is appropriate".to_owned());
                reasoning.push("Neutral to slightly positive TSB".to_owned());
            }
            TrainingReadiness::EasyOnly => {
                recommendations.push("Consider limiting to easy/recovery workouts".to_owned());
                if training_load.tsb < 0.0 {
                    reasoning.push("Negative TSB indicates accumulated fatigue".to_owned());
                }
            }
            TrainingReadiness::RestNeeded => {
                recommendations
                    .push("REST DAY RECOMMENDED based on training load analysis".to_owned());

                if training_load.tsb < highly_fatigued_tsb {
                    reasoning.push(format!(
                        "Extreme training fatigue detected (TSB: {:.1})",
                        training_load.tsb
                    ));
                } else if training_load.tsb < fatigued_tsb {
                    reasoning.push(format!(
                        "High training fatigue (TSB: {:.1})",
                        training_load.tsb
                    ));
                }
            }
        }

        // Always recommend adding sleep tracking for better accuracy
        recommendations.push(
            "For more accurate recovery assessment, connect a sleep tracking provider".to_owned(),
        );

        // TSB-specific recommendations
        if training_load.tsb < fatigued_tsb {
            recommendations
                .push("Consider a recovery week to allow fitness gains to consolidate".to_owned());
        }

        (recommendations, reasoning)
    }

    /// Score TSB (Training Stress Balance) for recovery
    ///
    /// TSB interpretation based on Banister model:
    /// - Negative TSB = fatigue > fitness (building)
    /// - Positive TSB = fitness > fatigue (fresh)
    #[doc(hidden)]
    #[must_use]
    pub fn score_tsb(tsb: f64, config: &SleepRecoveryConfig) -> f64 {
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
    pub fn categorize_recovery(score: f64, config: &SleepRecoveryConfig) -> RecoveryCategory {
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
        config: &SleepRecoveryConfig,
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
        let tsb_status = TrainingLoadCalculator::interpret_tsb(training_load.tsb);
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
    fn generate_recovery_recommendations(
        training_readiness: TrainingReadiness,
        training_load: &TrainingLoad,
        sleep_quality: &SleepQualityScore,
        hrv_analysis: Option<&HrvTrendAnalysis>,
        config: &SleepRecoveryConfig,
    ) -> RecoveryRecommendations {
        let mut recommendations = Vec::new();
        let mut reasoning = Vec::new();
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;
        let athlete_optimal_hours = config.sleep_duration.athlete_optimal_hours;
        let fair_threshold = config.recovery_scoring.fair_threshold;

        match training_readiness {
            TrainingReadiness::ReadyForHard => {
                recommendations
                    .push("You're well-recovered - ready for high-intensity training".to_owned());
                reasoning.push("Excellent recovery score with positive TSB".to_owned());
            }
            TrainingReadiness::ReadyForModerate => {
                recommendations.push("Continue with moderate-intensity training".to_owned());
                reasoning.push("Good recovery indicators support continued training".to_owned());
            }
            TrainingReadiness::EasyOnly => {
                recommendations.push("Limit to easy/recovery workouts today".to_owned());
                if sleep_quality.overall_score < 70.0 {
                    reasoning.push(
                        "Suboptimal sleep quality suggests reduced training intensity".to_owned(),
                    );
                }
                if training_load.tsb < 0.0 {
                    reasoning.push("Negative TSB indicates accumulated fatigue".to_owned());
                }
            }
            TrainingReadiness::RestNeeded => {
                recommendations.push("REST DAY RECOMMENDED - prioritize recovery".to_owned());

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
                            .push("Significantly suppressed HRV indicates high stress".to_owned());
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
                .push("Consider a recovery week to allow fitness gains to consolidate".to_owned());
        }

        RecoveryRecommendations {
            recommendations,
            reasoning,
        }
    }

    /// Generate detailed rest day recommendation
    ///
    /// # Errors
    /// Returns error if input data is invalid
    pub fn recommend_rest_day(
        recovery_score: &RecoveryScore,
        sleep_data: &SleepData,
        training_load: &TrainingLoad,
        config: &SleepRecoveryConfig,
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
                "If you must train: very easy, short (<30min) active recovery only".to_owned(),
                "Focus on mobility, stretching, or yoga instead of traditional training".to_owned(),
                "Monitor HRV tomorrow - if still low, take another rest day".to_owned(),
            ]
        } else {
            vec!["Continue with planned training but monitor fatigue levels".to_owned()]
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
        config: &SleepRecoveryConfig,
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
        config: &SleepRecoveryConfig,
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

    /// Generate rest day recommendation for TSB-only mode
    ///
    /// Provides a recommendation based solely on training load analysis when
    /// sleep data is unavailable.
    ///
    /// # Errors
    /// Returns error if recovery score data is invalid
    pub fn recommend_rest_day_tsb_only(
        recovery_score: &RecoveryScore,
        training_load: &TrainingLoad,
        config: &SleepRecoveryConfig,
    ) -> Result<RestDayRecommendation, AppError> {
        let rest_recommended = recovery_score.rest_day_recommended;

        // Calculate confidence (lower for TSB-only mode due to missing sleep data)
        let confidence = Self::calculate_recommendation_confidence_tsb_only(
            recovery_score,
            training_load,
            config,
        );

        // Identify primary reasons based on TSB only
        let mut primary_reasons = Vec::new();
        let mut supporting_factors = Vec::new();
        let fair_threshold = config.recovery_scoring.fair_threshold;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;

        if recovery_score.overall_score < fair_threshold {
            primary_reasons.push(format!(
                "Low recovery score ({:.1}/100, TSB-only assessment)",
                recovery_score.overall_score
            ));
        }

        if training_load.tsb < highly_fatigued_tsb {
            primary_reasons.push(format!(
                "Extreme training fatigue (TSB: {:.1})",
                training_load.tsb
            ));
        } else if training_load.tsb < fatigued_tsb {
            primary_reasons.push(format!(
                "High training fatigue (TSB: {:.1})",
                training_load.tsb
            ));
        }

        // Add supporting factors
        if training_load.atl > training_load.ctl * 1.2 {
            supporting_factors
                .push("Recent training load significantly exceeds fitness baseline".to_owned());
        }

        // Note the limitation of TSB-only assessment
        supporting_factors.push(
            "Note: Assessment based on training load only; sleep data not available".to_owned(),
        );

        // Generate alternatives if rest not taken
        let alternatives = if rest_recommended {
            vec![
                "If you must train: very easy, short (<30min) active recovery only".to_owned(),
                "Focus on mobility, stretching, or yoga instead of traditional training".to_owned(),
                "Connect a sleep tracker for more comprehensive recovery monitoring".to_owned(),
            ]
        } else {
            vec![
                "Continue with planned training but monitor fatigue levels".to_owned(),
                "Consider adding sleep tracking for better recovery insights".to_owned(),
            ]
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

    /// Calculate confidence for TSB-only rest day recommendation
    ///
    /// Lower confidence than full assessment due to missing sleep data
    fn calculate_recommendation_confidence_tsb_only(
        recovery_score: &RecoveryScore,
        training_load: &TrainingLoad,
        config: &SleepRecoveryConfig,
    ) -> f64 {
        let mut confidence_factors = 0;
        // Only 2 total factors in TSB-only mode (vs 4 in full mode)
        let total_factors = 2;
        let fair_threshold = config.recovery_scoring.fair_threshold;
        let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;

        // Factor 1: Recovery score severity
        if recovery_score.overall_score < fair_threshold {
            confidence_factors += 1;
        }

        // Factor 2: TSB severity
        if training_load.tsb < highly_fatigued_tsb {
            confidence_factors += 1;
        }

        // Base confidence is lower in TSB-only mode (max 75% vs 100%)
        let base_confidence = (f64::from(confidence_factors) / f64::from(total_factors)) * 75.0;

        // Apply a 25% reduction for missing sleep data
        base_confidence.max(25.0) // Minimum 25% confidence
    }
}
