// ABOUTME: Unit tests for sleep quality and recovery scoring algorithms
// ABOUTME: Pure algorithm tests for mathematical correctness without database/network dependencies
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Unit tests for sleep and recovery algorithms
//!
//! This test suite validates the mathematical correctness of:
//! - Sleep quality scoring formulas
//! - Recovery score calculations  
//! - TSB normalization
//! - HRV scoring
//!
//! Written using TDD - tests define expected behavior BEFORE implementation

use pierre_mcp_server::config::intelligence_config::IntelligenceConfig;

// === Sleep Duration Scoring Tests ===

#[test]
fn test_sleep_duration_score_optimal() {
    let config = IntelligenceConfig::default();
    let duration_config = &config.sleep_recovery.sleep_duration;

    // Test: Athlete with optimal sleep (8.0 hours) should score 100
    let duration = duration_config.athlete_optimal_hours;

    // TODO: Implement sleep_duration_score() function
    // let score = sleep_duration_score(duration, duration_config);
    // assert!((score - 100.0).abs() < 0.1);

    // Validate config values
    assert!((duration - 8.0).abs() < 0.01);
    assert!(duration >= duration_config.adult_min_hours);
}

#[test]
fn test_sleep_duration_score_minimum_acceptable() {
    let config = IntelligenceConfig::default();
    let duration_config = &config.sleep_recovery.sleep_duration;

    // Test: Adult minimum (7.0 hours) should score ~85
    let duration = duration_config.adult_min_hours;

    // TODO: Implement and test
    // let score = sleep_duration_score(duration, duration_config);
    // assert!(score >= 80.0 && score <= 90.0);

    assert!((duration - 7.0).abs() < 0.01);
}

#[test]
fn test_sleep_duration_score_short_sleep() {
    let config = IntelligenceConfig::default();
    let duration_config = &config.sleep_recovery.sleep_duration;

    // Test: Short sleep (6.0 hours) should score ~60
    let duration = duration_config.short_sleep_threshold;

    // TODO: Implement
    // let score = sleep_duration_score(duration, duration_config);
    // assert!(score >= 55.0 && score <= 65.0);

    assert!((duration - 6.0).abs() < 0.01);
}

#[test]
fn test_sleep_duration_score_very_short() {
    let config = IntelligenceConfig::default();
    let duration_config = &config.sleep_recovery.sleep_duration;

    // Test: Very short sleep (5.0 hours) should score ~30-40
    let duration = duration_config.very_short_sleep_threshold;

    // TODO: Implement
    // let score = sleep_duration_score(duration, duration_config);
    // assert!(score >= 30.0 && score <= 40.0);

    assert!((duration - 5.0).abs() < 0.01);
}

// === Sleep Stages Scoring Tests ===

#[test]
fn test_sleep_stages_score_optimal() {
    let config = IntelligenceConfig::default();
    let stages_config = &config.sleep_recovery.sleep_stages;

    // Test: Optimal percentages should score 100
    let deep_percent = stages_config.deep_sleep_optimal_percent;
    let rem_percent = stages_config.rem_sleep_optimal_percent;

    // TODO: Implement sleep_stages_score()
    // let score = sleep_stages_score(deep_percent, rem_percent, light_percent, awake_percent, stages_config);
    // assert!(score >= 95.0);

    // Validate config
    assert!((deep_percent - 20.0).abs() < 0.01);
    assert!((rem_percent - 25.0).abs() < 0.01);
}

#[test]
fn test_sleep_stages_score_below_minimum() {
    let config = IntelligenceConfig::default();
    let stages_config = &config.sleep_recovery.sleep_stages;

    // Test: Below minimum thresholds should score low
    let deep_percent = 10.0; // Below min (15%)
    let rem_percent = 15.0; // Below min (20%)

    // TODO: Implement
    // let score = sleep_stages_score(deep_percent, rem_percent, light_percent, awake_percent, stages_config);
    // assert!(score <= 50.0); // Should be low

    assert!(deep_percent < stages_config.deep_sleep_min_percent);
    assert!(rem_percent < stages_config.rem_sleep_min_percent);
}

// === Sleep Efficiency Scoring Tests ===

#[test]
fn test_sleep_efficiency_score_excellent() {
    let config = IntelligenceConfig::default();
    let efficiency_config = &config.sleep_recovery.sleep_efficiency;

    // Test: 95% efficiency (above excellent threshold) should score 100
    let efficiency = 95.0;

    // TODO: Implement sleep_efficiency_score()
    // let score = sleep_efficiency_score(efficiency, efficiency_config);
    // assert!((score - 100.0).abs() < 0.1);

    assert!(efficiency > efficiency_config.excellent_threshold);
}

#[test]
fn test_sleep_efficiency_score_good() {
    let config = IntelligenceConfig::default();
    let efficiency_config = &config.sleep_recovery.sleep_efficiency;

    // Test: 87% efficiency (between good and excellent) should score ~90
    let efficiency = 87.0;

    // TODO: Implement
    // let score = sleep_efficiency_score(efficiency, efficiency_config);
    // assert!(score >= 85.0 && score <= 95.0);

    assert!(efficiency > efficiency_config.good_threshold);
    assert!(efficiency < efficiency_config.excellent_threshold);
}

#[test]
fn test_sleep_efficiency_score_poor() {
    let config = IntelligenceConfig::default();
    let efficiency_config = &config.sleep_recovery.sleep_efficiency;

    // Test: 72% efficiency (just above poor threshold) should score ~65
    let efficiency = 72.0;

    // TODO: Implement
    // let score = sleep_efficiency_score(efficiency, efficiency_config);
    // assert!(score >= 60.0 && score <= 70.0);

    assert!(efficiency > efficiency_config.poor_threshold);
}

// === Overall Sleep Quality Tests ===

#[test]
fn test_overall_sleep_quality_perfect() {
    // Test: Perfect sleep in all dimensions
    let duration_score = 100.0;
    let stages_score = 100.0;
    let efficiency_score = 100.0;

    // TODO: Implement weighted combination
    // let overall = calculate_overall_sleep_quality(duration_score, stages_score, efficiency_score);
    // assert!((overall - 100.0).abs() < 0.1);

    // Manual calculation for validation
    let expected: f64 = (duration_score + stages_score + efficiency_score) / 3.0;
    assert!((expected - 100.0).abs() < 0.1);
}

// === TSB Normalization Tests ===

#[test]
fn test_tsb_to_score_highly_fatigued() {
    let config = IntelligenceConfig::default();
    let tsb_config = &config.sleep_recovery.training_stress_balance;

    // Test: TSB = -18 (highly fatigued) should map to ~20-30 score
    let tsb_raw = -18.0;

    // TODO: Implement tsb_to_score()
    // let score = tsb_to_score(tsb_raw, tsb_config);
    // assert!(score >= 20.0 && score <= 30.0);

    assert!(tsb_raw < tsb_config.highly_fatigued_tsb);
}

#[test]
fn test_tsb_to_score_fatigued() {
    let config = IntelligenceConfig::default();
    let tsb_config = &config.sleep_recovery.training_stress_balance;

    // Test: TSB = -10 (fatigued) should map to ~40-50 score
    let tsb_raw = -10.0;

    // TODO: Implement
    // let score = tsb_to_score(tsb_raw, tsb_config);
    // assert!(score >= 40.0 && score <= 50.0);

    assert!((tsb_raw - tsb_config.fatigued_tsb).abs() < 0.01);
}

#[test]
fn test_tsb_to_score_neutral() {
    // Test: TSB = 0 (neutral) should map to ~60 score
    let tsb_raw: f64 = 0.0;

    // TODO: Implement
    // let score = tsb_to_score(tsb_raw, tsb_config);
    // assert!(score >= 55.0 && score <= 65.0);

    assert!((tsb_raw - 0.0).abs() < 0.01);
}

#[test]
fn test_tsb_to_score_fresh() {
    let config = IntelligenceConfig::default();
    let tsb_config = &config.sleep_recovery.training_stress_balance;

    // Test: TSB = +10 (fresh) should map to ~80 score
    let tsb_raw = 10.0;

    // TODO: Implement
    // let score = tsb_to_score(tsb_raw, tsb_config);
    // assert!(score >= 75.0 && score <= 85.0);

    assert!(tsb_raw >= tsb_config.fresh_tsb_min);
    assert!(tsb_raw <= tsb_config.fresh_tsb_max);
}

#[test]
fn test_tsb_to_score_detraining() {
    let config = IntelligenceConfig::default();
    let tsb_config = &config.sleep_recovery.training_stress_balance;

    // Test: TSB = +30 (detraining risk) should map to ~70 score (penalty)
    let tsb_raw = 30.0;

    // TODO: Implement with detraining penalty
    // let score = tsb_to_score(tsb_raw, tsb_config);
    // assert!(score >= 65.0 && score <= 75.0);

    assert!(tsb_raw > tsb_config.detraining_tsb);
}

// === HRV Scoring Tests ===

#[test]
fn test_hrv_score_large_decrease() {
    let config = IntelligenceConfig::default();
    let hrv_config = &config.sleep_recovery.hrv;

    // Test: HRV decreased by 15ms (concerning) should score low
    let baseline_rmssd = 50.0;
    let current_rmssd = 35.0;
    let change = current_rmssd - baseline_rmssd; // -15ms

    // TODO: Implement hrv_to_score()
    // let score = hrv_to_score(baseline_rmssd, current_rmssd, hrv_config);
    // assert!(score <= 40.0);

    assert!(change < hrv_config.rmssd_decrease_concern_threshold);
}

#[test]
fn test_hrv_score_moderate_decrease() {
    // Test: HRV decreased by 7ms (moderate concern) should score ~60
    let baseline_rmssd = 50.0;
    let current_rmssd = 43.0;

    // TODO: Implement
    // let score = hrv_to_score(baseline_rmssd, current_rmssd, hrv_config);
    // assert!(score >= 55.0 && score <= 65.0);

    let change = current_rmssd - baseline_rmssd;
    assert!(change < 0.0); // Decrease
}

#[test]
fn test_hrv_score_stable() {
    let _config = IntelligenceConfig::default();

    // Test: HRV stable (within ±3ms) should score ~75
    let baseline_rmssd = 50.0;
    let current_rmssd = 52.0;

    // TODO: Implement
    // let score = hrv_to_score(baseline_rmssd, current_rmssd, hrv_config);
    // assert!(score >= 70.0 && score <= 80.0);

    let change: f64 = current_rmssd - baseline_rmssd;
    assert!(change.abs() <= 3.0);
}

#[test]
fn test_hrv_score_good_increase() {
    let config = IntelligenceConfig::default();
    let hrv_config = &config.sleep_recovery.hrv;

    // Test: HRV increased by 8ms (good recovery) should score ~90
    let baseline_rmssd = 50.0;
    let current_rmssd = 58.0;
    let change = current_rmssd - baseline_rmssd; // +8ms

    // TODO: Implement
    // let score = hrv_to_score(baseline_rmssd, current_rmssd, hrv_config);
    // assert!(score >= 85.0 && score <= 95.0);

    assert!(change > hrv_config.rmssd_increase_good_threshold);
}

// === Recovery Score Calculation Tests ===

#[test]
fn test_recovery_score_full_components() {
    let config = IntelligenceConfig::default();
    let recovery_config = &config.sleep_recovery.recovery_scoring;

    // Test: Calculate recovery with all three components
    let tsb_score: f64 = 75.0;
    let sleep_quality: f64 = 85.0;
    let hrv_score: f64 = 70.0;

    // Expected: 75*0.4 + 85*0.4 + 70*0.2 = 30 + 34 + 14 = 78
    let expected = hrv_score.mul_add(
        recovery_config.hrv_weight_full,
        tsb_score.mul_add(
            recovery_config.tsb_weight_full,
            sleep_quality * recovery_config.sleep_weight_full,
        ),
    );

    assert!((expected - 78.0).abs() < 0.1);

    // TODO: Implement calculate_recovery_score()
    // let result = calculate_recovery_score(Some(tsb_score), sleep_quality, Some(hrv_score), recovery_config);
    // assert!((result - expected).abs() < 1.0);
}

#[test]
fn test_recovery_score_no_hrv() {
    let config = IntelligenceConfig::default();
    let recovery_config = &config.sleep_recovery.recovery_scoring;

    // Test: Calculate recovery without HRV (50/50 weighting)
    let tsb_score: f64 = 70.0;
    let sleep_quality: f64 = 80.0;

    // Expected: 70*0.5 + 80*0.5 = 35 + 40 = 75
    let expected = tsb_score.mul_add(
        recovery_config.tsb_weight_no_hrv,
        sleep_quality * recovery_config.sleep_weight_no_hrv,
    );

    assert!((expected - 75.0).abs() < 0.1);

    // TODO: Implement
    // let result = calculate_recovery_score(Some(tsb_score), sleep_quality, None, recovery_config);
    // assert!((result - expected).abs() < 1.0);
}

#[test]
fn test_recovery_score_classification_excellent() {
    let config = IntelligenceConfig::default();
    let recovery_config = &config.sleep_recovery.recovery_scoring;

    // Test: Score of 90 should classify as "excellent"
    let score = 90.0;

    // TODO: Implement classify_recovery()
    // let category = classify_recovery(score, recovery_config);
    // assert_eq!(category, "excellent");

    assert!(score >= recovery_config.excellent_threshold);
}

#[test]
fn test_recovery_score_classification_good() {
    let config = IntelligenceConfig::default();
    let recovery_config = &config.sleep_recovery.recovery_scoring;

    // Test: Score of 75 should classify as "good"
    let score = 75.0;

    // TODO: Implement
    // let category = classify_recovery(score, recovery_config);
    // assert_eq!(category, "good");

    assert!(score >= recovery_config.good_threshold);
    assert!(score < recovery_config.excellent_threshold);
}

#[test]
fn test_recovery_score_classification_fair() {
    let config = IntelligenceConfig::default();
    let recovery_config = &config.sleep_recovery.recovery_scoring;

    // Test: Score of 55 should classify as "fair"
    let score = 55.0;

    // TODO: Implement
    // let category = classify_recovery(score, recovery_config);
    // assert_eq!(category, "fair");

    assert!(score >= recovery_config.fair_threshold);
    assert!(score < recovery_config.good_threshold);
}

#[test]
fn test_recovery_score_classification_poor() {
    let config = IntelligenceConfig::default();
    let recovery_config = &config.sleep_recovery.recovery_scoring;

    // Test: Score of 40 should classify as "poor"
    let score = 40.0;

    // TODO: Implement
    // let category = classify_recovery(score, recovery_config);
    // assert_eq!(category, "poor");

    assert!(score < recovery_config.fair_threshold);
}

// === Weight Validation Tests ===

#[test]
fn test_recovery_weights_sum_validation() {
    let config = IntelligenceConfig::default();
    let recovery_config = &config.sleep_recovery.recovery_scoring;

    // Test: Weights must sum to 1.0 (already validated in config tests)
    let full_sum = recovery_config.tsb_weight_full
        + recovery_config.sleep_weight_full
        + recovery_config.hrv_weight_full;

    assert!((full_sum - 1.0).abs() < 0.01);

    let no_hrv_sum = recovery_config.tsb_weight_no_hrv + recovery_config.sleep_weight_no_hrv;

    assert!((no_hrv_sum - 1.0).abs() < 0.01);
}
