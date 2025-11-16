# chapter 21: training load, recovery & sleep analysis

This chapter covers how Pierre analyzes recovery metrics, sleep quality, training load management, and provides rest day suggestions. You'll learn about recovery score calculation, sleep stage analysis, HRV interpretation, and overtraining detection.

## what you'll learn

- Recovery score calculation
- Sleep quality analysis
- HRV (Heart Rate Variability) interpretation
- Overtraining detection
- Rest day suggestions
- Sleep stage tracking
- Training load vs recovery balance
- Fatigue indicators

## recovery score calculation

Pierre calculates a composite recovery score from multiple metrics.

**Recovery factors**:
1. **Sleep quality**: Duration, efficiency, deep sleep percentage
2. **Resting heart rate**: Compared to baseline (elevated RHR = fatigue)
3. **HRV**: Heart rate variability (higher = better recovery)
4. **Training load**: Recent TSS vs historical average
5. **Muscle soreness**: Self-reported or inferred from performance
6. **Sleep debt**: Cumulative sleep deficit

**Recovery score formula** (conceptual):
```
Recovery Score = (
  sleep_score × 0.30 +
  hrv_score × 0.25 +
  rhr_score × 0.20 +
  training_load_score × 0.15 +
  sleep_debt_score × 0.10
) × 100
```

**Score interpretation**:
- **90-100**: Fully recovered, ready for hard training
- **70-89**: Good recovery, moderate-hard training OK
- **50-69**: Partial recovery, easy-moderate training
- **< 50**: Poor recovery, rest day recommended

## sleep quality analysis

Pierre analyzes sleep sessions from Fitbit, Garmin, and other providers.

**Sleep metrics**:
- **Total sleep time**: Duration in bed asleep
- **Sleep efficiency**: Time asleep / time in bed × 100%
- **Sleep stages**: Awake, light, deep, REM percentages
- **Sleep onset latency**: Time to fall asleep
- **Wake episodes**: Number of awakenings
- **Sleep debt**: Cumulative shortfall vs target (7-9 hours)

**Sleep stage targets** (% of total sleep):
- **Deep sleep**: 15-25% (restorative, hormone release)
- **REM sleep**: 20-25% (memory consolidation, mental recovery)
- **Light sleep**: 50-60% (transition stages)

**Sleep efficiency benchmarks**:
- **> 90%**: Excellent
- **85-90%**: Good
- **75-85%**: Fair
- **< 75%**: Poor (consider sleep hygiene improvements)

## HRV (heart rate variability)

HRV measures nervous system recovery via beat-to-beat timing variation.

**HRV metrics**:
- **RMSSD**: Root mean square of successive differences (ms)
- **SDNN**: Standard deviation of NN intervals (ms)
- **pNN50**: Percentage of successive intervals > 50ms different

**HRV interpretation** (RMSSD):
- **> 100ms**: Excellent recovery
- **60-100ms**: Good recovery
- **40-60ms**: Moderate recovery
- **20-40ms**: Poor recovery
- **< 20ms**: Very poor recovery, rest day needed

**HRV trends matter more than absolute values**: Compare to personal baseline rather than population norms.

## overtraining detection

Pierre monitors for overtraining syndrome indicators.

**Overtraining warning signs**:
1. **Elevated resting heart rate**: +5-10 BPM above baseline for 3+ days
2. **Decreased HRV**: > 20% below baseline for consecutive days
3. **Excessive TSB**: Training Stress Balance < -30 for extended period
4. **Performance decline**: Slower paces at same effort level
5. **Persistent fatigue**: Low recovery scores despite rest
6. **Sleep disturbances**: Difficulty falling/staying asleep
7. **Mood changes**: Irritability, loss of motivation

**Overtraining prevention**:
```
IF resting_hr > baseline + 8 AND hrv < baseline × 0.8 AND tsb < -30:
    RECOMMEND: 2-3 rest days
    ALERT: Overtraining risk detected
```

## rest day suggestions

Pierre suggests rest days based on accumulated fatigue.

**Rest day algorithm** (conceptual):
```rust
fn suggest_rest_day(
    recovery_score: f64,
    tsb: f64,
    consecutive_hard_days: u32,
    hrv_trend: f64,
) -> RestDaySuggestion {
    // Critical indicators
    if recovery_score < 30.0 || tsb < -40.0 {
        return RestDaySuggestion::Immediate;
    }

    // High fatigue
    if recovery_score < 50.0 && consecutive_hard_days >= 3 {
        return RestDaySuggestion::Soon;
    }

    // Preventive rest
    if consecutive_hard_days >= 6 || tsb < -20.0 {
        return RestDaySuggestion::NextDay;
    }

    RestDaySuggestion::None
}
```

**Rest day types**:
- **Complete rest**: No training, focus on sleep/nutrition
- **Active recovery**: Easy 20-30 min at < 60% max HR
- **Light cross-training**: Different sport, low intensity

## training load vs recovery balance

Pierre tracks the balance between training stress and recovery.

**Optimal balance indicators**:
- **TSB**: -10 to +10 (productive training without excessive fatigue)
- **Weekly TSS**: Consistent with 5-10% week-over-week growth
- **Recovery days**: 1-2 per week for most athletes
- **Hard:Easy ratio**: 1:2 or 1:3 (one hard day per 2-3 easy days)

**Periodization support**:
```
Build Phase:    TSB -10 to -20, weekly TSS +5-10%
Recovery Week:  TSB +10 to +20, weekly TSS -40-50%
Peak Phase:     TSB +15 to +25, weekly TSS -30%
Race Day:       TSB +20 to +30 (fresh and rested)
```

## sleep optimization recommendations

Pierre provides personalized sleep recommendations.

**Sleep hygiene tips**:
1. **Consistent schedule**: Same bedtime/wake time daily (±30 min)
2. **Sleep environment**: Cool (60-67°F), dark, quiet
3. **Pre-bed routine**: Wind down 30-60 min before sleep
4. **Limit caffeine**: No caffeine 6+ hours before bed
5. **Limit screens**: Blue light suppresses melatonin (avoid 1-2hr before bed)

**Sleep timing for athletes**:
- **After hard training**: Need 1-2 hours extra sleep for recovery
- **Before race/key workout**: 8-9 hours recommended
- **Naps**: 20-30 min power naps OK, avoid long naps (>90 min)

## key takeaways

1. **Recovery score**: Composite metric from sleep, HRV, RHR, training load, and sleep debt.

2. **Sleep stages**: Deep sleep (15-25%), REM (20-25%), light (50-60%) for optimal recovery.

3. **HRV**: Beat-to-beat variation indicates nervous system recovery (higher = better).

4. **Overtraining detection**: Elevated RHR + decreased HRV + negative TSB = warning signs.

5. **Rest day algorithm**: Considers recovery score, TSB, consecutive hard days, HRV trends.

6. **TSB sweet spot**: -10 to +10 for sustainable training without overreaching.

7. **Sleep efficiency**: Time asleep / time in bed > 85% indicates good sleep quality.

8. **Personal baselines**: Compare metrics to individual baseline, not population averages.

9. **Periodization**: Planned recovery weeks (TSB +10 to +20) prevent cumulative fatigue.

10. **Holistic approach**: Balance training load, recovery, sleep, nutrition for optimal adaptation.

---

**Next Chapter**: [Chapter 22: Nutrition System & USDA Integration](./chapter-22-nutrition.md) - Learn how Pierre calculates daily nutrition needs, integrates with the USDA food database, analyzes meal nutrition, and provides nutrient timing recommendations.
