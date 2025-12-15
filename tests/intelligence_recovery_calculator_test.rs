// ABOUTME: Unit tests for recovery calculator module, moved from src/intelligence/recovery_calculator.rs
// ABOUTME: Tests holistic recovery scoring combining TSB, sleep quality, and HRV analysis
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::{
    config::intelligence::{IntelligenceConfig, SleepRecoveryConfig},
    intelligence::{
        algorithms::RecoveryAggregationAlgorithm,
        recovery_calculator::{
            RecoveryCalculator, RecoveryCategory, RecoveryComponents, RecoveryScore,
            TrainingReadiness,
        },
        sleep_analysis::{
            HrvRecoveryStatus, HrvTrend, HrvTrendAnalysis, SleepData, SleepQualityCategory,
            SleepQualityScore,
        },
        training_load::TrainingLoad,
    },
};

/// Helper to get default test config
fn test_config() -> SleepRecoveryConfig {
    IntelligenceConfig::default().sleep_recovery
}

/// Helper to get default test algorithm (matches config weights)
fn test_algorithm() -> RecoveryAggregationAlgorithm {
    let config = test_config();
    RecoveryAggregationAlgorithm::WeightedAverage {
        tsb_weight_full: config.recovery_scoring.tsb_weight_full,
        sleep_weight_full: config.recovery_scoring.sleep_weight_full,
        hrv_weight_full: config.recovery_scoring.hrv_weight_full,
        tsb_weight_no_hrv: config.recovery_scoring.tsb_weight_no_hrv,
        sleep_weight_no_hrv: config.recovery_scoring.sleep_weight_no_hrv,
    }
}

#[test]
fn test_tsb_scoring_optimal_range() {
    let config = test_config();
    let score = RecoveryCalculator::score_tsb(10.0, &config);
    assert!((99.0..=100.0).contains(&score));
}

#[test]
fn test_tsb_scoring_highly_fatigued() {
    let config = test_config();
    let score = RecoveryCalculator::score_tsb(-20.0, &config);
    assert!(score < 30.0);
}

#[test]
fn test_recovery_category_excellent() {
    let config = test_config();
    let category = RecoveryCalculator::categorize_recovery(90.0, &config);
    assert_eq!(category, RecoveryCategory::Excellent);
}

#[test]
fn test_recovery_category_poor() {
    let config = test_config();
    let category = RecoveryCalculator::categorize_recovery(40.0, &config);
    assert_eq!(category, RecoveryCategory::Poor);
}

// ============================================================================
// COMPREHENSIVE TESTS FOR TSB SCORING ACROSS ALL RANGES
// ============================================================================

#[test]
fn test_tsb_scoring_extreme_fatigue() {
    let config = test_config();
    // TSB = -25 (well below -15 threshold)
    let score = RecoveryCalculator::score_tsb(-25.0, &config);
    assert!(
        score < 20.0,
        "Extreme fatigue (TSB=-25) should score very low (<20)"
    );
}

#[test]
fn test_tsb_scoring_highly_fatigued_boundary() {
    let config = test_config();
    // TSB = -15 (exactly at highly fatigued threshold)
    let score =
        RecoveryCalculator::score_tsb(config.training_stress_balance.highly_fatigued_tsb, &config);
    assert!((20.0..=35.0).contains(&score));
}

#[test]
fn test_tsb_scoring_fatigued_range() {
    let config = test_config();
    // TSB = -12 (between -15 and -10)
    let score = RecoveryCalculator::score_tsb(-12.0, &config);
    assert!(
        (30.0..50.0).contains(&score),
        "Moderate fatigue should score 30-50"
    );
}

#[test]
fn test_tsb_scoring_fatigued_boundary() {
    let config = test_config();
    // TSB = -10 (exactly at fatigued threshold)
    let score = RecoveryCalculator::score_tsb(config.training_stress_balance.fatigued_tsb, &config);
    assert!((40.0..=60.0).contains(&score));
}

#[test]
fn test_tsb_scoring_slightly_fatigued() {
    let config = test_config();
    // TSB = -5 (between -10 and 0)
    let score = RecoveryCalculator::score_tsb(-5.0, &config);
    assert!(
        (50.0..75.0).contains(&score),
        "Slight fatigue should score 50-75"
    );
}

#[test]
fn test_tsb_scoring_neutral() {
    let config = test_config();
    // TSB = 0 (neutral point)
    let score = RecoveryCalculator::score_tsb(0.0, &config);
    assert!(
        (70.0..=85.0).contains(&score),
        "Neutral TSB should score 70-85"
    );
}

#[test]
fn test_tsb_scoring_fresh_lower_boundary() {
    let config = test_config();
    // TSB = +5 (entering optimal range)
    let score =
        RecoveryCalculator::score_tsb(config.training_stress_balance.fresh_tsb_min, &config);
    assert!(score >= 90.0, "Fresh lower boundary should score >=90");
}

#[test]
fn test_tsb_scoring_fresh_upper_boundary() {
    let config = test_config();
    // TSB = +15 (upper optimal range)
    let score =
        RecoveryCalculator::score_tsb(config.training_stress_balance.fresh_tsb_max, &config);
    assert!(score >= 95.0, "Fresh upper boundary should score >=95");
}

#[test]
fn test_tsb_scoring_overtrained() {
    let config = test_config();
    // TSB = +25 (too much rest, detraining risk)
    let score = RecoveryCalculator::score_tsb(25.0, &config);
    assert!(
        score <= 90.0,
        "Excessive rest (TSB=+25) should score lower due to detraining"
    );
}

#[test]
fn test_tsb_scoring_extreme_overtrained() {
    let config = test_config();
    // TSB = +35 (severe detraining)
    let score = RecoveryCalculator::score_tsb(35.0, &config);
    assert!(score <= 80.0, "Severe detraining should score <=80");
}

// ============================================================================
// COMPREHENSIVE TESTS FOR HRV SCORING
// ============================================================================

#[test]
fn test_hrv_scoring_highly_fatigued() {
    let hrv = HrvTrendAnalysis {
        current_rmssd: 30.0,
        weekly_average_rmssd: 50.0,
        baseline_rmssd: Some(45.0),
        baseline_deviation_percent: Some(-40.0),
        trend: HrvTrend::Declining,
        recovery_status: HrvRecoveryStatus::HighlyFatigued,
        insights: vec![],
    };
    let score = RecoveryCalculator::score_hrv(&hrv);
    assert!(score < 30.0, "Highly fatigued HRV should score <30");
}

#[test]
fn test_hrv_scoring_fatigued() {
    let hrv = HrvTrendAnalysis {
        current_rmssd: 42.0,
        weekly_average_rmssd: 50.0,
        baseline_rmssd: Some(45.0),
        baseline_deviation_percent: Some(-16.0),
        trend: HrvTrend::Declining,
        recovery_status: HrvRecoveryStatus::Fatigued,
        insights: vec![],
    };
    let score = RecoveryCalculator::score_hrv(&hrv);
    assert!(
        (30.0..60.0).contains(&score),
        "Fatigued HRV should score 30-60"
    );
}

#[test]
fn test_hrv_scoring_normal() {
    let hrv = HrvTrendAnalysis {
        current_rmssd: 50.0,
        weekly_average_rmssd: 49.0,
        baseline_rmssd: Some(45.0),
        baseline_deviation_percent: Some(2.0),
        trend: HrvTrend::Stable,
        recovery_status: HrvRecoveryStatus::Normal,
        insights: vec![],
    };
    let score = RecoveryCalculator::score_hrv(&hrv);
    assert!(
        (60.0..85.0).contains(&score),
        "Normal HRV should score 60-85"
    );
}

#[test]
fn test_hrv_scoring_recovered() {
    let hrv = HrvTrendAnalysis {
        current_rmssd: 58.0,
        weekly_average_rmssd: 50.0,
        baseline_rmssd: Some(45.0),
        baseline_deviation_percent: Some(16.0),
        trend: HrvTrend::Improving,
        recovery_status: HrvRecoveryStatus::Recovered,
        insights: vec![],
    };
    let score = RecoveryCalculator::score_hrv(&hrv);
    assert!(score >= 85.0, "Recovered HRV should score >=85");
}

// ============================================================================
// COMPREHENSIVE TESTS FOR RECOVERY SCORING WITH DIFFERENT COMPONENT COMBINATIONS
// ============================================================================

#[test]
fn test_recovery_score_tsb_only() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 50.0,
        atl: 45.0,
        tsb: 5.0,
        tss_history: vec![],
    };
    let sleep_quality = SleepQualityScore {
        overall_score: 75.0,
        duration_score: 80.0,
        stage_quality_score: 70.0,
        efficiency_score: 75.0,
        quality_category: SleepQualityCategory::Good,
        insights: vec![],
        recommendations: vec![],
    };

    let result = RecoveryCalculator::calculate_recovery_score(
        &training_load,
        &sleep_quality,
        None, // No HRV
        &config,
        &test_algorithm(),
    );
    assert!(result.is_ok());
    let recovery = result.unwrap();
    assert_eq!(recovery.components.components_available, 2);
    // Should weight TSB 50%, Sleep 50%
    assert!(recovery.overall_score > 0.0);
}

#[test]
fn test_recovery_score_all_components() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 50.0,
        atl: 40.0,
        tsb: 10.0,
        tss_history: vec![],
    };
    let sleep_quality = SleepQualityScore {
        overall_score: 85.0,
        duration_score: 90.0,
        stage_quality_score: 85.0,
        efficiency_score: 90.0,
        quality_category: SleepQualityCategory::Excellent,
        insights: vec![],
        recommendations: vec![],
    };
    let hrv = HrvTrendAnalysis {
        current_rmssd: 55.0,
        weekly_average_rmssd: 50.0,
        baseline_rmssd: Some(45.0),
        baseline_deviation_percent: Some(10.0),
        trend: HrvTrend::Improving,
        recovery_status: HrvRecoveryStatus::Recovered,
        insights: vec![],
    };

    let result = RecoveryCalculator::calculate_recovery_score(
        &training_load,
        &sleep_quality,
        Some(&hrv),
        &config,
        &test_algorithm(),
    );
    assert!(result.is_ok());
    let recovery = result.unwrap();
    assert_eq!(recovery.components.components_available, 3);
    // Should weight TSB 40%, Sleep 40%, HRV 20%
    assert!(
        recovery.overall_score >= 85.0,
        "All excellent components should score high"
    );
}

#[test]
fn test_recovery_score_conflicting_signals() {
    let config = test_config();
    // Good TSB but poor sleep
    let training_load = TrainingLoad {
        ctl: 50.0,
        atl: 40.0,
        tsb: 10.0, // Fresh
        tss_history: vec![],
    };
    let sleep_quality = SleepQualityScore {
        overall_score: 35.0, // Poor
        duration_score: 30.0,
        stage_quality_score: 40.0,
        efficiency_score: 35.0,
        quality_category: SleepQualityCategory::Poor,
        insights: vec![],
        recommendations: vec![],
    };

    let result = RecoveryCalculator::calculate_recovery_score(
        &training_load,
        &sleep_quality,
        None,
        &config,
        &test_algorithm(),
    );
    assert!(result.is_ok());
    let recovery = result.unwrap();
    // Should average out to moderate range
    assert!(
        recovery.overall_score >= 50.0 && recovery.overall_score < 75.0,
        "Conflicting signals should result in moderate score"
    );
}

#[test]
fn test_recovery_score_all_poor() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 60.0,
        atl: 80.0,
        tsb: -20.0, // Highly fatigued
        tss_history: vec![],
    };
    let sleep_quality = SleepQualityScore {
        overall_score: 30.0,
        duration_score: 25.0,
        stage_quality_score: 35.0,
        efficiency_score: 30.0,
        quality_category: SleepQualityCategory::Poor,
        insights: vec![],
        recommendations: vec![],
    };
    let hrv = HrvTrendAnalysis {
        current_rmssd: 25.0,
        weekly_average_rmssd: 50.0,
        baseline_rmssd: Some(50.0),
        baseline_deviation_percent: Some(-50.0),
        trend: HrvTrend::Declining,
        recovery_status: HrvRecoveryStatus::HighlyFatigued,
        insights: vec![],
    };

    let result = RecoveryCalculator::calculate_recovery_score(
        &training_load,
        &sleep_quality,
        Some(&hrv),
        &config,
        &test_algorithm(),
    );
    assert!(result.is_ok());
    let recovery = result.unwrap();
    assert!(
        recovery.overall_score < 35.0,
        "All poor components should score very low"
    );
    assert_eq!(recovery.recovery_category, RecoveryCategory::Poor);
}

// ============================================================================
// COMPREHENSIVE TESTS FOR RECOVERY CATEGORY BOUNDARIES
// ============================================================================

#[test]
fn test_recovery_category_all_thresholds() {
    let config = test_config();
    // Test Excellent boundary (≥85)
    assert_eq!(
        RecoveryCalculator::categorize_recovery(85.0, &config),
        RecoveryCategory::Excellent
    );
    assert_eq!(
        RecoveryCalculator::categorize_recovery(95.0, &config),
        RecoveryCategory::Excellent
    );

    // Test Good boundary (70-84)
    assert_eq!(
        RecoveryCalculator::categorize_recovery(70.0, &config),
        RecoveryCategory::Good
    );
    assert_eq!(
        RecoveryCalculator::categorize_recovery(84.9, &config),
        RecoveryCategory::Good
    );

    // Test Fair boundary (50-69)
    assert_eq!(
        RecoveryCalculator::categorize_recovery(50.0, &config),
        RecoveryCategory::Fair
    );
    assert_eq!(
        RecoveryCalculator::categorize_recovery(69.9, &config),
        RecoveryCategory::Fair
    );

    // Test Poor boundary (<50)
    assert_eq!(
        RecoveryCalculator::categorize_recovery(49.9, &config),
        RecoveryCategory::Poor
    );
    assert_eq!(
        RecoveryCalculator::categorize_recovery(20.0, &config),
        RecoveryCategory::Poor
    );
}

#[test]
fn test_recovery_category_edge_cases() {
    let config = test_config();
    assert_eq!(
        RecoveryCalculator::categorize_recovery(0.0, &config),
        RecoveryCategory::Poor
    );
    assert_eq!(
        RecoveryCalculator::categorize_recovery(100.0, &config),
        RecoveryCategory::Excellent
    );
}

// ============================================================================
// COMPREHENSIVE TESTS FOR TRAINING READINESS DETERMINATION
// ============================================================================

#[test]
fn test_training_readiness_excellent_recovery() {
    let config = test_config();
    let readiness = RecoveryCalculator::determine_training_readiness(
        90.0, // overall_score
        10.0, // tsb
        SleepQualityCategory::Excellent,
        Some(HrvRecoveryStatus::Recovered),
        &config,
    );
    assert_eq!(
        readiness,
        TrainingReadiness::ReadyForHard,
        "Excellent recovery should indicate ready for hard training"
    );
}

#[test]
fn test_training_readiness_good_recovery() {
    let config = test_config();
    let readiness = RecoveryCalculator::determine_training_readiness(
        75.0, // overall_score
        5.0,  // tsb
        SleepQualityCategory::Good,
        Some(HrvRecoveryStatus::Normal),
        &config,
    );
    assert_eq!(
        readiness,
        TrainingReadiness::ReadyForModerate,
        "Good recovery should indicate ready for moderate training"
    );
}

#[test]
fn test_training_readiness_fair_recovery() {
    let config = test_config();
    let readiness = RecoveryCalculator::determine_training_readiness(
        60.0, // overall_score
        -2.0, // tsb
        SleepQualityCategory::Fair,
        Some(HrvRecoveryStatus::Normal),
        &config,
    );
    assert_eq!(
        readiness,
        TrainingReadiness::EasyOnly,
        "Fair recovery should indicate easy training only"
    );
}

#[test]
fn test_training_readiness_poor_recovery() {
    let config = test_config();
    let readiness = RecoveryCalculator::determine_training_readiness(
        35.0,  // overall_score
        -15.0, // tsb
        SleepQualityCategory::Poor,
        Some(HrvRecoveryStatus::Fatigued),
        &config,
    );
    assert_eq!(
        readiness,
        TrainingReadiness::RestNeeded,
        "Poor recovery should indicate rest needed"
    );
}

#[test]
fn test_training_readiness_high_tsb_but_poor_sleep() {
    let config = test_config();
    // Edge case: Good TSB but poor sleep should limit readiness
    let readiness = RecoveryCalculator::determine_training_readiness(
        55.0, // overall_score - fair due to poor sleep
        12.0, // tsb - fresh
        SleepQualityCategory::Poor,
        None,
        &config,
    );
    assert_eq!(
        readiness,
        TrainingReadiness::RestNeeded,
        "Poor sleep should limit training readiness despite good TSB"
    );
}

// ============================================================================
// COMPREHENSIVE TESTS FOR REST DAY RECOMMENDATION
// ============================================================================

#[test]
fn test_rest_day_not_needed() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 50.0,
        atl: 40.0,
        tsb: 10.0,
        tss_history: vec![],
    };
    let recovery_score = RecoveryScore {
        overall_score: 85.0,
        recovery_category: RecoveryCategory::Excellent,
        components: RecoveryComponents {
            tsb_score: 90.0,
            sleep_score: 85.0,
            hrv_score: Some(80.0),
            components_available: 3,
        },
        training_readiness: TrainingReadiness::ReadyForHard,
        insights: vec![],
        recommendations: vec![],
        rest_day_recommended: false,
        reasoning: vec![],
    };
    let sleep_data = SleepData {
        date: Utc::now(),
        duration_hours: 8.0,
        deep_sleep_hours: Some(1.6),
        rem_sleep_hours: Some(2.0),
        light_sleep_hours: Some(4.0),
        awake_hours: Some(0.4),
        efficiency_percent: Some(90.0),
        hrv_rmssd_ms: None,
        resting_hr_bpm: None,
        provider_score: None,
    };

    let recommendation = RecoveryCalculator::recommend_rest_day(
        &recovery_score,
        &sleep_data,
        &training_load,
        &config,
    )
    .unwrap();
    assert!(
        !recommendation.rest_recommended,
        "Excellent recovery should not need rest"
    );
    assert!(
        recommendation.confidence < 50.0,
        "Should have low confidence when rest not needed"
    );
}

#[test]
fn test_rest_day_strongly_recommended() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 60.0,
        atl: 80.0,
        tsb: -20.0,
        tss_history: vec![],
    };
    let recovery_score = RecoveryScore {
        overall_score: 25.0,
        recovery_category: RecoveryCategory::Poor,
        components: RecoveryComponents {
            tsb_score: 20.0,
            sleep_score: 30.0,
            hrv_score: Some(25.0),
            components_available: 3,
        },
        training_readiness: TrainingReadiness::RestNeeded,
        insights: vec![],
        recommendations: vec![],
        rest_day_recommended: true,
        reasoning: vec![],
    };
    let sleep_data = SleepData {
        date: Utc::now(),
        duration_hours: 5.5, // Short sleep
        deep_sleep_hours: Some(0.5),
        rem_sleep_hours: Some(1.0),
        light_sleep_hours: Some(3.5),
        awake_hours: Some(0.5),
        efficiency_percent: Some(70.0),
        hrv_rmssd_ms: None,
        resting_hr_bpm: None,
        provider_score: None,
    };

    let recommendation = RecoveryCalculator::recommend_rest_day(
        &recovery_score,
        &sleep_data,
        &training_load,
        &config,
    )
    .unwrap();
    assert!(
        recommendation.rest_recommended,
        "Severe fatigue should strongly recommend rest"
    );
    assert!(
        recommendation.confidence >= 75.0,
        "Should have high confidence with all factors poor"
    );
    assert!(
        recommendation.estimated_recovery_hours.unwrap() >= 36.0,
        "Should estimate significant recovery time"
    );
}

#[test]
fn test_rest_day_moderate_confidence() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 55.0,
        atl: 60.0,
        tsb: -5.0,
        tss_history: vec![],
    };
    let recovery_score = RecoveryScore {
        overall_score: 55.0,
        recovery_category: RecoveryCategory::Fair,
        components: RecoveryComponents {
            tsb_score: 60.0,
            sleep_score: 55.0,
            hrv_score: Some(50.0),
            components_available: 3,
        },
        training_readiness: TrainingReadiness::EasyOnly,
        insights: vec![],
        recommendations: vec![],
        rest_day_recommended: false,
        reasoning: vec![],
    };
    let sleep_data = SleepData {
        date: Utc::now(),
        duration_hours: 7.0,
        deep_sleep_hours: Some(1.2),
        rem_sleep_hours: Some(1.6),
        light_sleep_hours: Some(4.0),
        awake_hours: Some(0.2),
        efficiency_percent: Some(85.0),
        hrv_rmssd_ms: None,
        resting_hr_bpm: None,
        provider_score: None,
    };

    let recommendation = RecoveryCalculator::recommend_rest_day(
        &recovery_score,
        &sleep_data,
        &training_load,
        &config,
    )
    .unwrap();
    // Borderline case - might or might not recommend rest
    if recommendation.rest_recommended {
        assert!(
            recommendation.confidence < 75.0,
            "Borderline case should have moderate confidence"
        );
    }
}

#[test]
fn test_rest_day_reasoning_generated() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 60.0,
        atl: 75.0,
        tsb: -15.0,
        tss_history: vec![],
    };
    let recovery_score = RecoveryScore {
        overall_score: 40.0,
        recovery_category: RecoveryCategory::Poor,
        components: RecoveryComponents {
            tsb_score: 35.0,
            sleep_score: 45.0,
            hrv_score: None,
            components_available: 2,
        },
        training_readiness: TrainingReadiness::RestNeeded,
        insights: vec![],
        recommendations: vec![],
        rest_day_recommended: true,
        reasoning: vec![],
    };
    let sleep_data = SleepData {
        date: Utc::now(),
        duration_hours: 6.5,
        deep_sleep_hours: Some(1.0),
        rem_sleep_hours: Some(1.5),
        light_sleep_hours: Some(3.5),
        awake_hours: Some(0.5),
        efficiency_percent: Some(80.0),
        hrv_rmssd_ms: None,
        resting_hr_bpm: None,
        provider_score: None,
    };

    let recommendation = RecoveryCalculator::recommend_rest_day(
        &recovery_score,
        &sleep_data,
        &training_load,
        &config,
    )
    .unwrap();
    assert!(
        !recommendation.primary_reasons.is_empty(),
        "Should provide reasoning for recommendation"
    );
}

// ============================================================================
// COMPREHENSIVE TESTS FOR EDGE CASES AND BOUNDARY CONDITIONS
// ============================================================================

#[test]
fn test_recovery_score_zero_sleep() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 50.0,
        atl: 45.0,
        tsb: 5.0,
        tss_history: vec![],
    };
    let sleep_quality = SleepQualityScore {
        overall_score: 0.0,
        duration_score: 0.0,
        stage_quality_score: 0.0,
        efficiency_score: 0.0,
        quality_category: SleepQualityCategory::Poor,
        insights: vec![],
        recommendations: vec![],
    };

    let result = RecoveryCalculator::calculate_recovery_score(
        &training_load,
        &sleep_quality,
        None,
        &config,
        &test_algorithm(),
    );
    assert!(result.is_ok());
    let recovery = result.unwrap();
    assert!(
        recovery.overall_score <= 50.0,
        "Zero sleep should result in poor overall score"
    );
}

#[test]
fn test_recovery_components_display() {
    let components = RecoveryComponents {
        tsb_score: 85.0,
        sleep_score: 75.0,
        hrv_score: Some(90.0),
        components_available: 3,
    };
    // Test that components struct is properly constructed
    assert_eq!(components.components_available, 3);
    assert!(components.hrv_score.is_some());
}

#[test]
fn test_recovery_insights_generation() {
    let config = test_config();
    let training_load = TrainingLoad {
        ctl: 50.0,
        atl: 40.0,
        tsb: 10.0,
        tss_history: vec![],
    };
    let sleep_quality = SleepQualityScore {
        overall_score: 80.0,
        duration_score: 85.0,
        stage_quality_score: 80.0,
        efficiency_score: 75.0,
        quality_category: SleepQualityCategory::Good,
        insights: vec![],
        recommendations: vec![],
    };

    let result = RecoveryCalculator::calculate_recovery_score(
        &training_load,
        &sleep_quality,
        None,
        &config,
        &test_algorithm(),
    );
    assert!(result.is_ok());
    let recovery = result.unwrap();
    assert!(
        !recovery.insights.is_empty(),
        "Should generate recovery insights"
    );
}
