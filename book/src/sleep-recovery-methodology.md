<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Sleep & Recovery Methodology

This document describes the scientific methodology and implementation of Pierre's sleep analysis and recovery intelligence system.

## Overview

Pierre's sleep and recovery system provides:

- **sleep quality scoring**: Multi-factor analysis based on NSF/AASM guidelines
- **HRV analysis**: Heart rate variability trend detection for recovery assessment
- **holistic recovery scoring**: Combines TSB, sleep quality, and HRV
- **rest day recommendations**: AI-powered training/rest decisions
- **evidence-based**: Algorithms based on peer-reviewed sports science research

**Target audience**: Developers, coaches, athletes, and users seeking deep understanding of Pierre's recovery intelligence.

## Table of Contents

- [Tool-to-Algorithm Mapping](#tool-to-algorithm-mapping)
- [1. Sleep Quality Analysis](#1-sleep-quality-analysis)
- [2. HRV Trend Analysis](#2-hrv-trend-analysis)
- [3. Holistic Recovery Scoring](#3-holistic-recovery-scoring)
- [4. Rest Day Recommendation](#4-rest-day-recommendation)
- [5. Sleep Schedule Optimization](#5-sleep-schedule-optimization)
- [6. Configuration](#6-configuration)
- [7. Scientific References](#7-scientific-references)

---

## Tool-to-Algorithm Mapping

This section provides a comprehensive mapping between MCP tools and their underlying algorithms, implementation files, and test coverage.

### Sleep & Recovery Tools (5 tools)

| Tool Name | Algorithm/Intelligence | Implementation | Test File |
|-----------|----------------------|----------------|-----------|
| `analyze_sleep_quality` | NSF/AASM sleep scoring + HRV trend analysis | `src/tools/implementations/sleep.rs:96-193` | `tests/intelligence_sleep_analysis_test.rs` |
| `calculate_recovery_score` | Weighted multi-factor aggregation (TSB + Sleep + HRV) | `src/tools/implementations/sleep.rs:199-329` | `tests/intelligence_recovery_calculator_test.rs` |
| `suggest_rest_day` | Recovery threshold analysis + confidence scoring | `src/tools/implementations/sleep.rs:335-470` | `tests/sleep_recovery_integration_test.rs` |
| `track_sleep_trends` | Rolling average comparison + trend detection | `src/tools/implementations/sleep.rs:588-686` | `tests/sleep_recovery_integration_test.rs` |
| `optimize_sleep_schedule` | TSB-based sleep duration adjustment | `src/tools/implementations/sleep.rs:696-815` | `tests/sleep_recovery_integration_test.rs` |

### Intelligence Module Dependencies

| Module | Algorithm | Source File |
|--------|-----------|-------------|
| `SleepAnalyzer` | NSF/AASM sleep quality scoring | `src/intelligence/sleep_analysis.rs` |
| `RecoveryCalculator` | Multi-factor recovery aggregation | `src/intelligence/recovery_calculator.rs` |
| `TrainingLoadCalculator` | TSB calculation (CTL/ATL) | `src/intelligence/training_load.rs` |
| `RecoveryAggregationAlgorithm` | Weighted average scoring | `src/intelligence/algorithms.rs` |

### Algorithm Reference Summary

| Algorithm | Scientific Basis | Key Parameters |
|-----------|-----------------|----------------|
| **Sleep Duration Score** | NSF guidelines (Watson et al., 2015) | Optimal: 7-9 hours, athletes: 8-10 hours |
| **Sleep Stage Score** | AASM stage distribution | Deep: 15-25%, REM: 20-25%, Light: 50-60% |
| **Sleep Efficiency Score** | Time asleep / time in bed | Excellent: >90%, Good: 85-90% |
| **HRV Trend Analysis** | Plews et al. (2013) | Baseline deviation, 7-day rolling average |
| **Recovery Aggregation** | Weighted average | TSB: 40%, Sleep: 35%, HRV: 25% (full data) |
| **Rest Day Threshold** | Recovery score thresholds | Rest if score < 40, easy if < 60 |

---

## 1. Sleep Quality Analysis

### Duration Scoring (NSF Guidelines)

| Duration | Score | Category |
|----------|-------|----------|
| ≥ 8.0 hours | 100 | Optimal for athletes |
| 7.0-8.0 hours | 80-100 | Recommended |
| 6.0-7.0 hours | 50-80 | Suboptimal |
| < 6.0 hours | 0-50 | Insufficient |

### Sleep Stage Quality

Pierre analyzes sleep stages based on AASM recommendations:

| Stage | Optimal Range | Purpose |
|-------|--------------|---------|
| **Deep Sleep** | 15-25% | Physical restoration, growth hormone release |
| **REM Sleep** | 20-25% | Cognitive recovery, memory consolidation |
| **Light Sleep** | 50-60% | Transition sleep, body maintenance |

### Efficiency Scoring

| Efficiency | Score | Category |
|------------|-------|----------|
| ≥ 90% | 100 | Excellent |
| 85-90% | 80-100 | Good |
| 75-85% | 50-80 | Fair |
| < 75% | 0-50 | Poor |

### Quality Categories

```rust
pub enum SleepQualityCategory {
    Excellent,  // Score > 85, >8 hours, high efficiency
    Good,       // Score 70-85, 7-8 hours
    Fair,       // Score 50-70, 6-7 hours
    Poor,       // Score < 50, <6 hours or low efficiency
}
```

---

## 2. HRV Trend Analysis

### HRV Metrics

Pierre uses RMSSD (Root Mean Square of Successive Differences) as the primary HRV metric:

| Status | Baseline Deviation | Interpretation |
|--------|-------------------|----------------|
| **Recovered** | > +5% | Parasympathetic dominance, ready for training |
| **Normal** | -5% to +5% | Balanced autonomic state |
| **Fatigued** | < -5% | Sympathetic dominance, recovery needed |
| **Significantly Suppressed** | < -15% | High stress/fatigue, rest recommended |

### Trend Detection

```rust
pub enum HrvTrend {
    Improving,  // 7-day avg > previous 7-day avg
    Stable,     // Within ±5% of previous period
    Declining,  // 7-day avg < previous 7-day avg
}
```

### Recovery Status from HRV

```rust
pub enum HrvRecoveryStatus {
    Recovered,              // HRV elevated, ready for training
    Normal,                 // HRV at baseline
    MildlyFatigued,        // HRV slightly suppressed
    SignificantlySuppressed, // HRV markedly below baseline
}
```

---

## 3. Holistic Recovery Scoring

### Multi-Factor Aggregation

Pierre combines three recovery indicators:

1. **TSB Score** (Training Stress Balance)
   - Derived from CTL (Chronic Training Load) and ATL (Acute Training Load)
   - TSB = CTL - ATL
   - Positive TSB = fresh, negative TSB = fatigued

2. **Sleep Quality Score**
   - Composite of duration, stages, and efficiency
   - Weighted by data completeness

3. **HRV Score**
   - Based on baseline deviation and trend
   - Optional but highly valuable

### Weighting Algorithm

```rust
pub enum RecoveryAggregationAlgorithm {
    WeightedAverage {
        // Full data (TSB + Sleep + HRV)
        tsb_weight_full: f64,    // Default: 0.40
        sleep_weight_full: f64,  // Default: 0.35
        hrv_weight_full: f64,    // Default: 0.25

        // No HRV data (TSB + Sleep only)
        tsb_weight_no_hrv: f64,  // Default: 0.55
        sleep_weight_no_hrv: f64, // Default: 0.45
    },
}
```

### Data Completeness

| Level | Sources | Confidence |
|-------|---------|------------|
| **Full** | TSB + Sleep + HRV | High |
| **Partial** | TSB + Sleep | Medium |
| **TSB Only** | Activity data only | Lower |

### Recovery Categories

| Score | Category | Training Readiness |
|-------|----------|-------------------|
| ≥ 80 | Excellent | Ready for hard training |
| 60-80 | Good | Ready for moderate training |
| 40-60 | Fair | Easy training only |
| < 40 | Poor | Rest needed |

---

## 4. Rest Day Recommendation

### Decision Algorithm

The `suggest_rest_day` tool uses multi-factor analysis:

```
IF recovery_score < 40:
    STRONGLY recommend rest (confidence: high)
ELSE IF recovery_score < 60:
    Suggest rest or easy training (confidence: medium)
ELSE IF TSB < -20 AND sleep_score < 70:
    Suggest rest despite moderate recovery (confidence: medium)
ELSE:
    Training OK (confidence based on data completeness)
```

### Confidence Scoring

| Data Availability | Confidence Boost |
|-------------------|------------------|
| Full (TSB + Sleep + HRV) | +20% |
| Partial (TSB + Sleep) | +10% |
| TSB Only | Base |
| Consistent indicators | +10% |
| Recent trend data | +5% |

### Output Structure

```rust
pub struct RestDayRecommendation {
    pub rest_recommended: bool,
    pub confidence: f64,           // 0-100
    pub recovery_score: f64,       // 0-100
    pub primary_reasons: Vec<String>,
    pub supporting_factors: Vec<String>,
    pub alternatives: Vec<String>, // e.g., "active recovery", "yoga"
}
```

---

## 5. Sleep Schedule Optimization

### Duration Recommendations

Base recommendations adjusted by training load:

| Condition | Adjustment |
|-----------|------------|
| TSB < -10 (fatigued) | +0.5-1.0 hours |
| ATL > 100 (high acute load) | +0.5 hours |
| High-intensity workout planned | Prioritize quality |
| Rest day | Base recommendation |

### Bedtime Calculation

```
bedtime = wake_time - target_hours - wind_down_time
```

Where:
- `wake_time`: User's typical wake time
- `target_hours`: 8-9 hours (adjusted for fatigue)
- `wind_down_time`: 30 minutes default

---

## 6. Configuration

### Sleep Parameters

```toml
[sleep_recovery]
# Duration thresholds
athlete_min_hours = 7.0
athlete_optimal_hours = 8.0
athlete_max_hours = 10.0

# Stage targets (percentage)
deep_sleep_min_percent = 15.0
deep_sleep_optimal_percent = 20.0
rem_sleep_min_percent = 20.0
rem_sleep_optimal_percent = 25.0

# Efficiency thresholds
efficiency_excellent = 90.0
efficiency_good = 85.0
efficiency_fair = 75.0
```

### Recovery Scoring Weights

```toml
[recovery_scoring]
# Full data weights (must sum to 1.0)
tsb_weight_full = 0.40
sleep_weight_full = 0.35
hrv_weight_full = 0.25

# No HRV weights (must sum to 1.0)
tsb_weight_no_hrv = 0.55
sleep_weight_no_hrv = 0.45
```

### TSB Thresholds

```toml
[training_stress_balance]
fresh_tsb = 10.0      # Positive TSB indicates freshness
optimal_tsb = 0.0     # Balance point
fatigued_tsb = -10.0  # Negative indicates fatigue
overtrained_tsb = -30.0  # High risk zone
```

---

## 7. Scientific References

### Sleep Science

1. Watson, N.F., et al. (2015). Recommended Amount of Sleep for a Healthy Adult. *Sleep*, 38(6), 843-844.

2. Hirshkowitz, M., et al. (2015). National Sleep Foundation's sleep time duration recommendations. *Sleep Health*, 1(1), 40-43.

3. Simpson, N.S., et al. (2017). Sleep and recovery in team sport athletes. *British Journal of Sports Medicine*, 51(3), 179-186.

### HRV and Recovery

4. Shaffer, F., & Ginsberg, J.P. (2017). An Overview of Heart Rate Variability Metrics and Norms. *Frontiers in Public Health*, 5, 258.

5. Plews, D.J., et al. (2013). Training adaptation and heart rate variability in elite endurance athletes. *International Journal of Sports Physiology and Performance*, 8(5), 512-519.

6. Buchheit, M. (2014). Monitoring training status with HR measures: Do all roads lead to Rome? *Frontiers in Physiology*, 5, 73.

### Overtraining and Recovery

7. Meeusen, R., et al. (2013). Prevention, diagnosis, and treatment of the overtraining syndrome. *European Journal of Sport Science*, 13(1), 1-24.

8. Halson, S.L. (2014). Monitoring training load to understand fatigue in athletes. *Sports Medicine*, 44(Suppl 2), S139-147.

### Athlete Sleep

9. Leeder, J., et al. (2012). Sleep duration and quality in elite athletes. *International Journal of Sports Physiology and Performance*, 7(4), 340-345.

10. Fullagar, H.H., et al. (2015). Sleep and athletic performance: The effects of sleep loss on exercise performance. *Sports Medicine*, 45(2), 161-186.
