# pierre intelligence and analytics methodology

## what this document covers

this comprehensive guide explains the scientific methods, algorithms, and decision rules behind pierre's analytics engine. it provides transparency into:

- **mathematical foundations**: formulas, statistical methods, and physiological models
- **data sources and processing**: inputs, validation, and transformation pipelines
- **calculation methodologies**: step-by-step algorithms with code examples
- **scientific references**: peer-reviewed research backing each metric
- **implementation details**: rust code architecture and design patterns
- **limitations and guardrails**: edge cases, confidence levels, and safety mechanisms
- **verification**: validation against published sports science data

**target audience**: developers, data scientists, coaches, and advanced users seeking deep understanding of pierre's intelligence system.

---

## architecture overview

pierre's intelligence system uses a **foundation modules** approach for code reuse and consistency:

```
┌─────────────────────────────────────────────┐
│   mcp/a2a protocol layer                    │
│   (src/protocols/universal/)                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│   intelligence tools (30 tools)             │
│   (src/protocols/universal/handlers/)       │
└──────────────────┬──────────────────────────┘
                   │
    ┌──────────────┼──────────────────┬───────────┬────────────┐
    ▼              ▼                  ▼           ▼            ▼
┌─────────────┐ ┌──────────────┐ ┌──────────┐ ┌───────────┐ ┌──────────────┐
│ Training    │ │ Performance  │ │ Pattern  │ │Statistical│ │ Sleep &      │
│ Load Calc   │ │ Predictor    │ │ Detector │ │ Analyzer  │ │ Recovery     │
│             │ │              │ │          │ │           │ │              │
│ TSS/CTL/ATL │ │ VDOT/Riegel  │ │ Weekly   │ │Regression │ │ Sleep Score  │
│ TSB/Risk    │ │ Race Times   │ │ Patterns │ │ Trends    │ │ Recovery Calc│
└─────────────┘ └──────────────┘ └──────────┘ └───────────┘ └──────────────┘
         FOUNDATION MODULES (Phase 1 + Phase 2)
         Shared by all intelligence tools
```

### foundation modules (phase 1)

**`src/intelligence/training_load.rs`** - training stress calculations
- TSS (Training Stress Score) from power or heart rate
- CTL (Chronic Training Load) - 42-day EMA for fitness
- ATL (Acute Training Load) - 7-day EMA for fatigue
- TSB (Training Stress Balance) - form indicator
- Overtraining risk assessment with 3 risk factors
- Gap handling: zero-fills missing days in EMA calculation

**`src/intelligence/performance_prediction.rs`** - race predictions
- VDOT calculation from race performance (Jack Daniels formula)
- Race time prediction for 5K, 10K, 15K, Half Marathon, Marathon
- Riegel formula for distance-based predictions
- Accuracy: 0.2-5.5% vs. published VDOT tables
- Verified against VDOT 40, 50, 60 reference values

**`src/intelligence/pattern_detection.rs`** - pattern recognition
- Weekly schedule detection with consistency scoring
- Hard/easy alternation pattern analysis
- Volume progression trend detection (increasing/stable/decreasing)
- Overtraining signals detection (3 risk factors)

**`src/intelligence/statistical_analysis.rs`** - statistical methods
- Linear regression with R² calculation
- Trend detection (improving/stable/declining)
- Correlation analysis
- Moving averages and smoothing
- Significance level assessment

### foundation modules (phase 2)

**`src/intelligence/sleep_analysis.rs`** - sleep quality scoring
- Duration scoring with NSF guidelines (7-9 hours optimal for adults, 8-10 for athletes)
- Stages scoring with AASM recommendations (deep 15-25%, REM 20-25%)
- Efficiency scoring with clinical thresholds (excellent >90%, good >85%, poor <70%)
- Overall quality calculation (weighted average of components)
- Dependency injection with `SleepRecoveryConfig` for all thresholds

**`src/intelligence/recovery_calculator.rs`** - recovery assessment
- TSB normalization (-30 to +30 → 0-100 recovery score)
- HRV scoring based on RMSSD baseline comparison (±3ms stable, >5ms good recovery)
- Weighted recovery calculation (40% TSB, 40% sleep, 20% HRV when available)
- Fallback scoring when HRV unavailable (50% TSB, 50% sleep)
- Recovery classification (excellent/good/fair/poor) with actionable thresholds
- Dependency injection with `SleepRecoveryConfig` for configurability

### core modules

**`src/intelligence/metrics.rs`** - advanced metrics calculation
**`src/intelligence/performance_analyzer_v2.rs`** - performance analysis framework
**`src/intelligence/physiological_constants.rs`** - sport science constants
**`src/intelligence/recommendation_engine.rs`** - training recommendations
**`src/intelligence/goal_engine.rs`** - goal tracking and progress

### intelligence tools (30 tools)

all 30 MCP tools now use real calculations from foundation modules:

**group 1: analysis** (use StatisticalAnalyzer + PatternDetector)
- analyze_performance_trends
- detect_patterns
- compare_activities

**group 2: recommendations** (use TrainingLoadCalculator + PatternDetector)
- generate_recommendations
- calculate_fitness_score
- analyze_training_load

**group 3: predictions** (use PerformancePredictor)
- predict_performance

**group 4: configuration** (use physiological_constants validation)
- validate_configuration (ranges + relationships)
- suggest_goals (real profile from activities)

**group 5: goals** (use 10% improvement rule)
- analyze_goal_feasibility

**group 6: sleep and recovery** (use SleepAnalyzer + RecoveryCalculator)
- analyze_sleep_quality (NSF/AASM-based scoring)
- calculate_recovery_score (TSB + sleep + HRV)
- track_sleep_trends (longitudinal analysis)
- optimize_sleep_schedule (personalized timing)
- get_rest_day_recommendations (training load-based)

---

## data sources and permissions

### primary data
fitness activities via oauth2 authorization from multiple providers:

**supported providers**: strava, garmin, fitbit

**activity data**:
- **temporal**: `start_date`, `elapsed_time`, `moving_time`
- **spatial**: `distance`, `total_elevation_gain`, GPS polyline (optional)
- **physiological**: `average_heartrate`, `max_heartrate`, heart rate stream
- **power**: `average_watts`, `weighted_average_watts`, `kilojoules`, power stream (strava, garmin)
- **sport metadata**: `type`, `sport_type`, `workout_type`

### user profile (optional)
- **demographics**: `age`, `gender`, `weight_kg`, `height_cm`
- **thresholds**: `max_hr`, `resting_hr`, `lthr`, `ftp`, `cp`, `vo2max`
- **preferences**: `units`, `training_focus`, `injury_history`
- **fitness level**: `beginner`, `intermediate`, `advanced`, `elite`

### configuration
- **strategy**: `conservative`, `default`, `aggressive` (affects thresholds)
- **units**: metric (km, m, kg) or imperial (mi, ft, lb)
- **zone model**: karvonen (HR reserve) or percentage max HR

### provider normalization
pierre normalizes data from different providers into a unified format:

```rust
// src/providers/ - unified activity model
pub struct Activity {
    pub provider: Provider, // Strava, Garmin, Fitbit
    pub start_date: DateTime<Utc>,
    pub distance: Option<f64>,
    pub moving_time: u64,
    pub sport_type: String,
    // ... normalized fields
}
```

**provider-specific features**:
- **strava**: full power metrics, segments, kudos
- **garmin**: advanced running dynamics, training effect, recovery time
- **fitbit**: all-day heart rate, sleep tracking, steps

### data retention and privacy
- activities cached for 7 days (configurable)
- analysis results cached for 24 hours
- token revocation purges all cached data within 1 hour
- no third-party data sharing
- encryption: AES-256-GCM for tokens, tenant-specific keys
- provider tokens stored separately, isolated per tenant

---

## personalization engine

### age-based max heart rate estimation

when `max_hr` not provided, pierre uses tanaka formula (more accurate than fox):

```rust
// src/intelligence/physiological_constants.rs
pub const TANAKA_CONSTANT: f64 = 208.0;
pub const TANAKA_AGE_COEFFICIENT: f64 = 0.7;

fn estimate_max_hr(age: i32) -> u32 {
    let estimated = TANAKA_CONSTANT - (TANAKA_AGE_COEFFICIENT * age as f64);
    estimated.clamp(160.0, 210.0) as u32
}
```

**formula**: `max_hr = 208 − (0.7 × age)`

**bounds**: [160, 210] bpm to exclude physiologically implausible values.

**reference**: Tanaka, H., Monahan, K.D., & Seals, D.R. (2001). Age-predicted maximal heart rate revisited. *Journal of the American College of Cardiology*, 37(1), 153-156.

**alternative**: fox formula (`220 − age`) available via configuration but tanaka preferred for accuracy.

### heart rate zones

pierre implements **karvonen method** (HR reserve) when `resting_hr` available:

```rust
// src/intelligence/metrics.rs
pub fn calculate_hr_zones(max_hr: u32, resting_hr: u32) -> HRZones {
    let reserve = (max_hr - resting_hr) as f64;

    HRZones {
        zone1: Zone { // Recovery (50-60% reserve)
            lower: (reserve * 0.50 + resting_hr as f64) as u32,
            upper: (reserve * 0.60 + resting_hr as f64) as u32,
        },
        zone2: Zone { // Endurance (60-70%)
            lower: (reserve * 0.60 + resting_hr as f64) as u32,
            upper: (reserve * 0.70 + resting_hr as f64) as u32,
        },
        zone3: Zone { // Tempo (70-80%)
            lower: (reserve * 0.70 + resting_hr as f64) as u32,
            upper: (reserve * 0.80 + resting_hr as f64) as u32,
        },
        zone4: Zone { // Threshold (80-90%)
            lower: (reserve * 0.80 + resting_hr as f64) as u32,
            upper: (reserve * 0.90 + resting_hr as f64) as u32,
        },
        zone5: Zone { // VO2max (90-100%)
            lower: (reserve * 0.90 + resting_hr as f64) as u32,
            upper: max_hr,
        },
    }
}
```

**formula**: `target_hr = (hr_reserve × intensity%) + resting_hr`

**fallback**: when `resting_hr` unavailable, uses simple percentage of `max_hr` (50%, 60%, 70%, 80%, 90%).

**reference**: Karvonen, M.J., Kentala, E., & Mustala, O. (1957). The effects of training on heart rate; a longitudinal study. *Annales medicinae experimentalis et biologiae Fenniae*, 35(3), 307-315.

### power zones (cycling)

five-zone model based on functional threshold power (FTP):

```rust
// src/intelligence/physiological_constants.rs
pub fn calculate_power_zones(ftp: f64) -> PowerZones {
    PowerZones {
        zone1: (0.0,         ftp * 0.55), // Active recovery
        zone2: (ftp * 0.55,  ftp * 0.75), // Endurance
        zone3: (ftp * 0.75,  ftp * 0.90), // Tempo
        zone4: (ftp * 0.90,  ftp * 1.05), // Threshold
        zone5: (ftp * 1.05,  f64::MAX),   // VO2max+
    }
}
```

**zones**:
- **Z1 (active recovery)**: < 55% FTP - flush metabolites, active rest
- **Z2 (endurance)**: 55-75% FTP - aerobic base building
- **Z3 (tempo)**: 75-90% FTP - muscular endurance
- **Z4 (threshold)**: 90-105% FTP - lactate threshold work
- **Z5 (VO2max+)**: > 105% FTP - maximal aerobic/anaerobic efforts

**reference**: Coggan, A. & Allen, H. (2010). *Training and Racing with a Power Meter* (2nd ed.). VeloPress.

---

## core metrics

### pace vs speed

```rust
// src/intelligence/metrics.rs

// pace: time per distance (seconds per km)
pub fn calculate_pace(moving_time_s: u64, distance_m: f64) -> f64 {
    if distance_m < 1.0 { return 0.0; }
    (moving_time_s as f64) / (distance_m / 1000.0)
}

// speed: distance per time (m/s)
pub fn calculate_speed(distance_m: f64, moving_time_s: u64) -> f64 {
    if moving_time_s == 0 { return 0.0; }
    distance_m / (moving_time_s as f64)
}
```

---

## training stress score (TSS)

TSS quantifies training load accounting for intensity and duration.

### power-based TSS (preferred)

```rust
// src/intelligence/training_load.rs
pub fn calculate_tss_power(
    normalized_power: f64,
    duration_hours: f64,
    ftp: f64,
) -> f64 {
    let intensity_factor = normalized_power / ftp;
    duration_hours * intensity_factor.powi(2) * 100.0
}
```

**formula**: `TSS = (duration_hours × IF² × 100)` where `IF = normalized_power / FTP`

**example**: 2-hour ride at 250W NP with FTP=300W
- IF = 250/300 = 0.833
- TSS = 2.0 × (0.833)² × 100 = 138.9

### heart rate-based TSS (hrTSS)

```rust
pub fn calculate_tss_hr(
    avg_hr: u32,
    duration_hours: f64,
    lthr: u32,
) -> f64 {
    let hr_ratio = (avg_hr as f64) / (lthr as f64);
    duration_hours * hr_ratio.powi(2) * 100.0
}
```

**formula**: `hrTSS = (duration_hours × (avg_hr / lthr)² × 100)`

**interpretation**:
- < 150: low training stress
- 150-300: moderate training stress
- 300-450: high training stress
- > 450: very high training stress

**reference**: Coggan, A. (2003). Training Stress Score. *TrainingPeaks*.

---

## normalized power (NP)

accounts for variability in cycling efforts:

```rust
// src/intelligence/metrics.rs
pub fn calculate_normalized_power(power_stream: &[f64]) -> f64 {
    if power_stream.len() < 30 {
        return power_stream.iter().sum::<f64>() / power_stream.len() as f64;
    }

    // Step 1: 30-second rolling average
    let mut rolling_avg = Vec::new();
    for window in power_stream.windows(30) {
        rolling_avg.push(window.iter().sum::<f64>() / 30.0);
    }

    // Step 2: Raise to 4th power
    let sum_fourth_power: f64 = rolling_avg
        .iter()
        .map(|&p| p.powi(4))
        .sum();

    // Step 3: Take 4th root
    (sum_fourth_power / rolling_avg.len() as f64).powf(0.25)
}
```

**formula**: `NP = ⁴√(average(30s_rolling_avg⁴))`

**why 4th power?** matches physiological cost of variable efforts. 200W/150W alternating costs more than steady 175W.

---

## chronic training load (CTL) and acute training load (ATL)

CTL ("fitness") and ATL ("fatigue") track training stress using exponential moving averages.

### implementation

```rust
// src/intelligence/training_load.rs
const CTL_WINDOW_DAYS: i64 = 42; // 6 weeks
const ATL_WINDOW_DAYS: i64 = 7;  // 1 week

pub fn calculate_training_load(
    activities: &[Activity],
    ftp: Option<f64>,
    lthr: Option<f64>,
    max_hr: Option<f64>,
    resting_hr: Option<f64>,
    weight_kg: Option<f64>,
) -> Result<TrainingLoad> {
    // Handle empty activities
    if activities.is_empty() {
        return Ok(TrainingLoad {
            ctl: 0.0,
            atl: 0.0,
            tsb: 0.0,
            tss_history: Vec::new(),
        });
    }

    // Calculate TSS for each activity
    let mut tss_data: Vec<TssDataPoint> = Vec::new();
    for activity in activities {
        if let Ok(tss) = calculate_tss(activity, ftp, lthr, max_hr, resting_hr, weight_kg) {
            tss_data.push(TssDataPoint {
                date: activity.start_date,
                tss,
            });
        }
    }

    // Handle no valid TSS calculations
    if tss_data.is_empty() {
        return Ok(TrainingLoad {
            ctl: 0.0,
            atl: 0.0,
            tsb: 0.0,
            tss_history: Vec::new(),
        });
    }

    let ctl = calculate_ema(&tss_data, CTL_WINDOW_DAYS);
    let atl = calculate_ema(&tss_data, ATL_WINDOW_DAYS);
    let tsb = ctl - atl;

    Ok(TrainingLoad { ctl, atl, tsb, tss_history: tss_data })
}

fn calculate_ema(tss_data: &[TssDataPoint], window_days: i64) -> f64 {
    if tss_data.is_empty() {
        return 0.0;
    }

    let alpha = 2.0 / (window_days as f64 + 1.0);

    // Create daily TSS map (handles multiple activities per day)
    let mut tss_map = std::collections::HashMap::new();
    for point in tss_data {
        let date_key = point.date.date_naive();
        *tss_map.entry(date_key).or_insert(0.0) += point.tss;
    }

    // Calculate EMA day by day, filling gaps with 0.0
    let first_date = tss_data[0].date;
    let last_date = tss_data[tss_data.len() - 1].date;
    let days_span = (last_date - first_date).num_days();

    let mut ema = 0.0;
    for day_offset in 0..=days_span {
        let current_date = first_date + Duration::days(day_offset);
        let date_key = current_date.date_naive();
        let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0); // Gap = 0

        ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
    }

    ema
}
```

**formulas**:
```
α = 2 / (N + 1)
EMA_today = (TSS_today × α) + (EMA_yesterday × (1 - α))

CTL = 42-day EMA of daily TSS
ATL = 7-day EMA of daily TSS
TSB = CTL - ATL
```

**edge case handling**:
- **zero activities**: returns CTL=0, ATL=0, TSB=0
- **training gaps**: zero-fills missing days (realistic fitness decay)
- **multiple activities per day**: sums TSS values
- **failed TSS calculations**: skips activities, continues with valid data

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. Human Kinetics.

---

## training stress balance (TSB)

TSB indicates form/freshness:

```rust
pub fn interpret_tsb(tsb: f64) -> TrainingStatus {
    match tsb {
        t if t < -10.0 => TrainingStatus::Overreaching,
        t if t < 0.0   => TrainingStatus::Productive,
        t if t <= 10.0 => TrainingStatus::Fresh,
        _              => TrainingStatus::Detraining,
    }
}
```

**interpretation**:
- **< -10**: overreaching (high fatigue) - recovery needed
- **-10 to 0**: productive training - building fitness
- **0 to +10**: fresh - ready for hard efforts
- **> +10**: risk of detraining

**reference**: Banister, E.W., Calvert, T.W., Savage, M.V., & Bach, T. (1975). A systems model of training. *Australian Journal of Sports Medicine*, 7(3), 57-61.

---

## overtraining risk detection

```rust
// src/intelligence/training_load.rs
pub fn check_overtraining_risk(training_load: &TrainingLoad) -> OvertrainingRisk {
    let mut risk_factors = Vec::new();

    // 1. Acute load spike
    if training_load.ctl > 0.0 && training_load.atl > training_load.ctl * 1.3 {
        risk_factors.push(
            "Acute load spike >30% above chronic load".to_string()
        );
    }

    // 2. Very high acute load
    if training_load.atl > 150.0 {
        risk_factors.push(
            "Very high acute load (>150 TSS/day)".to_string()
        );
    }

    // 3. Deep fatigue
    if training_load.tsb < -10.0 {
        risk_factors.push(
            "Deep fatigue (TSB < -10)".to_string()
        );
    }

    let risk_level = match risk_factors.len() {
        0 => RiskLevel::Low,
        1 => RiskLevel::Moderate,
        _ => RiskLevel::High,
    };

    OvertrainingRisk { risk_level, risk_factors }
}
```

**reference**: Halson, S.L. (2014). Monitoring training load to understand fatigue. *Sports Medicine*, 44(Suppl 2), 139-147.

---

## statistical trend analysis

pierre uses proper linear regression for trend detection:

```rust
// src/intelligence/statistical_analysis.rs
pub fn linear_regression(data_points: &[TrendDataPoint]) -> Result<RegressionResult> {
    let n = data_points.len() as f64;
    let x_values: Vec<f64> = (0..data_points.len()).map(|i| i as f64).collect();
    let y_values: Vec<f64> = data_points.iter().map(|p| p.value).collect();

    let sum_x = x_values.iter().sum::<f64>();
    let sum_y = y_values.iter().sum::<f64>();
    let sum_xx = x_values.iter().map(|x| x * x).sum::<f64>();
    let sum_xy = x_values.iter().zip(&y_values).map(|(x, y)| x * y).sum::<f64>();
    let sum_yy = y_values.iter().map(|y| y * y).sum::<f64>();

    let mean_x = sum_x / n;
    let mean_y = sum_y / n;

    // Calculate slope and intercept
    let numerator = sum_xy - n * mean_x * mean_y;
    let denominator = sum_xx - n * mean_x * mean_x;

    let slope = numerator / denominator;
    let intercept = mean_y - slope * mean_x;

    // Calculate R² (coefficient of determination)
    let ss_tot = sum_yy - n * mean_y * mean_y;
    let ss_res: f64 = y_values
        .iter()
        .zip(&x_values)
        .map(|(y, x)| {
            let predicted = slope * x + intercept;
            (y - predicted).powi(2)
        })
        .sum();

    let r_squared = 1.0 - (ss_res / ss_tot);
    let correlation = r_squared.sqrt() * slope.signum();

    Ok(RegressionResult {
        slope,
        intercept,
        r_squared,
        correlation,
    })
}
```

**R² interpretation**:
- 0.0-0.3: weak relationship
- 0.3-0.5: moderate relationship
- 0.5-0.7: strong relationship
- 0.7-1.0: very strong relationship

**reference**: Draper, N.R. & Smith, H. (1998). *Applied Regression Analysis* (3rd ed.). Wiley.

---

## performance prediction: VDOT

VDOT is jack daniels' VO2max adjusted for running economy:

```rust
// src/intelligence/performance_prediction.rs
pub fn calculate_vdot(distance_m: f64, time_s: f64) -> Result<f64> {
    // Convert to velocity (m/min)
    let velocity = (distance_m / time_s) * 60.0;

    // Validate velocity range
    if !(100.0..=500.0).contains(&velocity) {
        return Err(AppError::invalid_input(
            format!("Velocity {velocity:.1} m/min outside valid range (100-500)")
        ));
    }

    // Jack Daniels' VO2 formula
    // VO2 = -4.60 + 0.182258×v + 0.000104×v²
    let vo2 = (0.000104 * velocity).mul_add(
        velocity,
        0.182258f64.mul_add(velocity, -4.60)
    );

    // Adjust for race duration
    let percent_max = calculate_percent_max_adjustment(time_s);

    // VDOT = VO2 / percent_used
    Ok(vo2 / percent_max)
}

fn calculate_percent_max_adjustment(time_s: f64) -> f64 {
    let time_minutes = time_s / 60.0;

    match time_minutes {
        t if t < 5.0  => 0.97, // Very short - oxygen deficit
        t if t < 15.0 => 0.99, // 5K range
        t if t < 30.0 => 1.00, // 10K-15K range - optimal
        t if t < 90.0 => 0.98, // Half marathon range
        _             => 0.95, // Marathon+ - fatigue accumulation
    }
}
```

**VDOT ranges**:
- 30-40: beginner
- 40-50: recreational
- 50-60: competitive amateur
- 60-70: sub-elite
- 70-85: elite

**race time prediction**:

```rust
pub fn predict_time_vdot(vdot: f64, target_distance_m: f64) -> Result<f64> {
    // Validate VDOT range
    if !(30.0..=85.0).contains(&vdot) {
        return Err(AppError::invalid_input(
            format!("VDOT {vdot:.1} outside typical range (30-85)")
        ));
    }

    // Calculate velocity at VO2max (reverse of VDOT formula)
    // vo2 = -4.60 + 0.182258 × v + 0.000104 × v²
    // Solve quadratic: 0.000104v² + 0.182258v - (vo2 + 4.60) = 0

    let a = 0.000104;
    let b = 0.182258;
    let c = -(vdot + 4.60);

    let discriminant = b.mul_add(b, -(4.0 * a * c));
    let velocity_max = (-b + discriminant.sqrt()) / (2.0 * a);

    // Adjust for race distance
    let race_velocity = calculate_race_velocity(velocity_max, target_distance_m);

    // Calculate time
    Ok((target_distance_m / race_velocity) * 60.0)
}

fn calculate_race_velocity(velocity_max: f64, distance_m: f64) -> f64 {
    let percent_max = if distance_m <= 5_000.0 {
        0.98 // 5K: 98% of VO2max velocity
    } else if distance_m <= 10_000.0 {
        0.94 // 10K: 94%
    } else if distance_m <= 15_000.0 {
        0.91 // 15K: 91%
    } else if distance_m <= 21_097.5 {
        0.88 // Half: 88%
    } else if distance_m <= 42_195.0 {
        0.84 // Marathon: 84%
    } else {
        // Ultra: progressively lower
        let marathon_ratio = distance_m / 42_195.0;
        (marathon_ratio - 1.0).mul_add(-0.02, 0.84).max(0.70)
    };

    velocity_max * percent_max
}
```

### VDOT accuracy verification ✅

pierre's VDOT predictions have been verified against jack daniels' published tables:

```
VDOT 50 (recreational competitive):
  5K:        19:34 vs 19:31 reference → 0.2% difference ✅
  10K:       40:48 vs 40:31 reference → 0.7% difference ✅
  Half:    1:31:56 vs 1:30:00 reference → 2.2% difference ✅
  Marathon: 3:12:38 vs 3:08:00 reference → 2.5% difference ✅

VDOT 60 (sub-elite):
  5K:        16:53 vs 16:39 reference → 1.4% difference ✅
  10K:       35:11 vs 34:40 reference → 1.5% difference ✅
  Marathon: 2:46:10 vs 2:40:00 reference → 3.9% difference ✅

VDOT 40 (recreational):
  5K:        23:26 vs 24:44 reference → 5.2% difference ✅
  10K:       48:52 vs 51:42 reference → 5.5% difference ✅
  Marathon: 3:50:46 vs 3:57:00 reference → 2.6% difference ✅

Overall accuracy: 0.2-5.5% difference across all distances
```

**why differences exist**:
- jack daniels' tables use empirical adjustments from real runner data
- pierre uses pure mathematical VDOT formula
- 6% tolerance is excellent for race predictions (weather, course, pacing all affect actual performance)

**test verification**: `tests/vdot_table_verification_test.rs`

**reference**: Daniels, J. (2013). *Daniels' Running Formula* (3rd ed.). Human Kinetics.

---

## performance prediction: riegel formula

predicts race times across distances:

```rust
// src/intelligence/performance_prediction.rs
const RIEGEL_EXPONENT: f64 = 1.06;

pub fn predict_time_riegel(
    known_distance_m: f64,
    known_time_s: f64,
    target_distance_m: f64,
) -> Result<f64> {
    if known_distance_m <= 0.0 || known_time_s <= 0.0 || target_distance_m <= 0.0 {
        return Err(AppError::invalid_input(
            "All distances and times must be positive"
        ));
    }

    let distance_ratio = target_distance_m / known_distance_m;
    Ok(known_time_s * distance_ratio.powf(RIEGEL_EXPONENT))
}
```

**formula**: `T₂ = T₁ × (D₂ / D₁)^1.06`

**example**: predict marathon from half:
- half: 1:30:00 (5400s), 21097m
- marathon: 42195m
- predicted: 5400 × (42195/21097)^1.06 ≈ 11340s ≈ 3:09:00

**reference**: Riegel, P.S. (1981). Athletic records and human endurance. *American Scientist*, 69(3), 285-290.

---

## pattern detection

### weekly schedule

```rust
// src/intelligence/pattern_detection.rs
pub fn detect_weekly_schedule(activities: &[Activity]) -> WeeklySchedulePattern {
    let mut day_counts: HashMap<Weekday, u32> = HashMap::new();

    for activity in activities {
        *day_counts.entry(activity.start_date.weekday()).or_insert(0) += 1;
    }

    let mut day_freq: Vec<(Weekday, u32)> = day_counts.into_iter().collect();
    day_freq.sort_by(|a, b| b.1.cmp(&a.1));

    let consistency_score = calculate_consistency(&day_freq, activities.len());

    WeeklySchedulePattern {
        most_common_days: day_freq.iter().take(3).map(|(d, _)| *d).collect(),
        consistency_score,
    }
}
```

**consistency interpretation**:
- 0-30%: highly variable
- 30-60%: moderate consistency
- 60-80%: consistent schedule
- 80-100%: very consistent routine

### hard/easy alternation

```rust
pub fn detect_hard_easy_pattern(activities: &[Activity]) -> HardEasyPattern {
    let mut intensities = Vec::new();

    for activity in activities {
        let intensity = calculate_relative_intensity(activity);
        intensities.push((activity.start_date, intensity));
    }

    intensities.sort_by_key(|(date, _)| *date);

    // Detect alternation
    let mut alternations = 0;
    for window in intensities.windows(2) {
        if (window[0].1 == Intensity::Hard && window[1].1 == Intensity::Easy)
            || (window[0].1 == Intensity::Easy && window[1].1 == Intensity::Hard)
        {
            alternations += 1;
        }
    }

    let pattern_strength = (alternations as f64) / (intensities.len() as f64 - 1.0);

    HardEasyPattern {
        follows_pattern: pattern_strength > 0.6,
        pattern_strength,
    }
}
```

### volume progression

```rust
pub fn detect_volume_progression(activities: &[Activity]) -> VolumeProgressionPattern {
    // Group by weeks
    let weekly_volumes = group_by_weeks(activities);

    // Calculate trend
    let trend_result = StatisticalAnalyzer::linear_regression(&weekly_volumes)?;

    let trend = if trend_result.slope > 0.05 {
        VolumeTrend::Increasing
    } else if trend_result.slope < -0.05 {
        VolumeTrend::Decreasing
    } else {
        VolumeTrend::Stable
    };

    VolumeProgressionPattern {
        trend,
        slope: trend_result.slope,
        r_squared: trend_result.r_squared,
    }
}
```

**reference**: Esteve-Lanao, J. et al. (2005). How do endurance runners train? *Med Sci Sports Exerc*, 37(3), 496-504.

---

## sleep and recovery analysis

### sleep quality scoring

pierre uses NSF (National Sleep Foundation) and AASM (American Academy of Sleep Medicine) guidelines for sleep quality assessment. the overall sleep quality score (0-100) combines three weighted components:

**sleep quality = (duration_score × 0.35) + (stages_score × 0.40) + (efficiency_score × 0.25)**

#### duration scoring

based on NSF recommendations with athlete-specific adjustments:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_duration_score(duration_hours: f64, config: &SleepRecoveryConfig) -> f64 {
    if duration_hours >= config.athlete_optimal_hours {        // >=8h → 100
        100.0
    } else if duration_hours >= config.adult_min_hours {       // 7-8h → 85-100
        85.0 + ((duration_hours - 7.0) / 1.0) * 15.0
    } else if duration_hours >= config.short_sleep_threshold { // 6-7h → 60-85
        60.0 + ((duration_hours - 6.0) / 1.0) * 25.0
    } else if duration_hours >= config.very_short_sleep_threshold { // 5-6h → 30-60
        30.0 + ((duration_hours - 5.0) / 1.0) * 30.0
    } else {                                                   // <5h → 0-30
        (duration_hours / 5.0) * 30.0
    }
}
```

**thresholds** (configurable via `PIERRE_SLEEP_ADULT_MIN_HOURS` etc):
- **8+ hours**: 100 (optimal for athletes)
- **7-8 hours**: 85-100 (adequate for adults)
- **6-7 hours**: 60-85 (short sleep)
- **5-6 hours**: 30-60 (very short)
- **<5 hours**: 0-30 (severe deprivation)

**scientific basis**: NSF recommends 7-9h for adults, 8-10h for athletes. <6h linked to increased injury risk and impaired performance.

**reference**: Hirshkowitz, M. et al. (2015). National Sleep Foundation's sleep time duration recommendations. *Sleep Health*, 1(1), 40-43.

#### stages scoring

based on AASM guidelines for healthy sleep stage distribution:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_stages_score(
    deep_percent: f64,
    rem_percent: f64,
    light_percent: f64,
    awake_percent: f64,
    config: &SleepRecoveryConfig
) -> f64 {
    // Deep sleep: 40% weight (physical recovery)
    let deep_score = if deep_percent >= 20.0 { 100.0 }
                     else if deep_percent >= 15.0 { 70.0 + ((deep_percent - 15.0) / 5.0) * 30.0 }
                     else { (deep_percent / 15.0) * 70.0 };

    // REM sleep: 40% weight (cognitive recovery)
    let rem_score = if rem_percent >= 25.0 { 100.0 }
                    else if rem_percent >= 20.0 { 70.0 + ((rem_percent - 20.0) / 5.0) * 30.0 }
                    else { (rem_percent / 20.0) * 70.0 };

    // Awake time penalty: >5% awake reduces score
    let awake_penalty = if awake_percent > 5.0 { (awake_percent - 5.0) * 2.0 } else { 0.0 };

    // Combined: 40% deep, 40% REM, 20% light, minus awake penalty
    ((deep_score * 0.4) + (rem_score * 0.4) + (light_percent * 0.2) - awake_penalty).clamp(0.0, 100.0)
}
```

**optimal ranges**:
- **deep sleep**: 15-25% (physical recovery, growth hormone release)
- **REM sleep**: 20-25% (memory consolidation, cognitive function)
- **light sleep**: 45-55% (transition stages)
- **awake time**: <5% (sleep fragmentation indicator)

**scientific basis**: AASM sleep stage guidelines. deep sleep critical for physical recovery, REM for cognitive processing.

**reference**: Berry, R.B. et al. (2017). AASM Scoring Manual Version 2.4. *American Academy of Sleep Medicine*.

#### efficiency scoring

based on clinical sleep medicine thresholds:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_efficiency_score(efficiency_percent: f64, config: &SleepRecoveryConfig) -> f64 {
    if efficiency_percent >= 90.0 {       // >=90% → 100 (excellent)
        100.0
    } else if efficiency_percent >= 85.0 { // 85-90% → 85-100 (good)
        85.0 + ((efficiency_percent - 85.0) / 5.0) * 15.0
    } else if efficiency_percent >= 75.0 { // 75-85% → 65-85 (fair)
        65.0 + ((efficiency_percent - 75.0) / 10.0) * 20.0
    } else {                              // <75% → 0-65 (poor)
        (efficiency_percent / 75.0) * 65.0
    }
}
```

**formula**: `efficiency = (time_asleep / time_in_bed) × 100`

**thresholds**:
- **>90%**: excellent (minimal sleep fragmentation)
- **85-90%**: good (normal range)
- **75-85%**: fair (moderate fragmentation)
- **<75%**: poor (severe fragmentation)

**scientific basis**: sleep efficiency >85% considered normal in clinical sleep medicine.

### recovery score calculation

pierre calculates training readiness by combining TSB, sleep quality, and HRV (when available):

```rust
// src/intelligence/recovery_calculator.rs
pub fn calculate_recovery_score(
    tsb: f64,
    sleep_quality: f64,
    hrv_data: Option<HrvData>,
    config: &SleepRecoveryConfig
) -> RecoveryScore {
    // 1. Normalize TSB from [-30, +30] to [0, 100]
    let tsb_score = normalize_tsb(tsb);

    // 2. Sleep already scored [0, 100]

    // 3. Score HRV if available
    let (recovery_score, components) = match hrv_data {
        Some(hrv) => {
            let hrv_score = score_hrv(hrv, config);
            // Weights: 40% TSB, 40% sleep, 20% HRV
            let score = (tsb_score * 0.4) + (sleep_quality * 0.4) + (hrv_score * 0.2);
            (score, (tsb_score, sleep_quality, Some(hrv_score)))
        },
        None => {
            // Weights: 50% TSB, 50% sleep (no HRV)
            let score = (tsb_score * 0.5) + (sleep_quality * 0.5);
            (score, (tsb_score, sleep_quality, None))
        }
    };

    // 4. Classify recovery level
    let level = if recovery_score >= 85.0 { "excellent" }
                else if recovery_score >= 70.0 { "good" }
                else if recovery_score >= 50.0 { "fair" }
                else { "poor" };

    RecoveryScore { score: recovery_score, level, components }
}
```

#### TSB normalization

training stress balance maps to recovery score:

```rust
fn normalize_tsb(tsb: f64) -> f64 {
    // TSB ranges and recovery interpretation:
    // +25 to +15: overtrained/detraining (score 90-100)
    // +15 to +5:  fresh/race ready (score 80-90)
    // +5 to -5:   optimal training (score 60-80)
    // -5 to -10:  fatigued (score 40-60)
    // -10 to -15: very fatigued (score 20-40)
    // -15 to -30: overreaching (score 0-20)

    if tsb >= 15.0 {
        90.0 + ((tsb - 15.0).min(10.0) / 10.0) * 10.0
    } else if tsb >= 5.0 {
        80.0 + ((tsb - 5.0) / 10.0) * 10.0
    } else if tsb >= -5.0 {
        60.0 + ((tsb + 5.0) / 10.0) * 20.0
    } else if tsb >= -10.0 {
        40.0 + ((tsb + 10.0) / 5.0) * 20.0
    } else if tsb >= -15.0 {
        20.0 + ((tsb + 15.0) / 5.0) * 20.0
    } else {
        (0.0_f64).max((tsb + 30.0) / 15.0 * 20.0)
    }
}
```

**interpretation**:
- **TSB > +15**: detraining (too much rest)
- **TSB +5 to +15**: fresh (race ready)
- **TSB -5 to +5**: optimal (productive training)
- **TSB -10 to -5**: fatigued (building fitness)
- **TSB < -10**: overreaching (recovery needed)

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. *Human Kinetics*.

#### HRV scoring

heart rate variability assessment based on RMSSD deviation from baseline:

```rust
fn score_hrv(hrv: HrvData, config: &SleepRecoveryConfig) -> f64 {
    let rmssd_delta = hrv.rmssd - hrv.baseline_rmssd;

    if rmssd_delta >= 5.0 {
        // +5ms or more: excellent recovery (score 90-100)
        90.0 + (rmssd_delta.min(10.0) / 10.0) * 10.0
    } else if rmssd_delta >= 0.0 {
        // 0 to +5ms: good recovery (score 70-90)
        70.0 + (rmssd_delta / 5.0) * 20.0
    } else if rmssd_delta >= -3.0 {
        // -3 to 0ms: adequate (score 50-70)
        50.0 + ((rmssd_delta + 3.0) / 3.0) * 20.0
    } else if rmssd_delta >= -10.0 {
        // -3 to -10ms: poor recovery (score 20-50)
        20.0 + ((rmssd_delta + 10.0) / 7.0) * 30.0
    } else {
        // < -10ms: very poor (score 0-20)
        (0.0_f64).max((rmssd_delta + 20.0) / 10.0 * 20.0)
    }
}
```

**RMSSD thresholds** (root mean square of successive differences):
- **+5ms or higher**: excellent recovery, parasympathetic dominance
- **±3ms**: stable, normal recovery
- **-10ms or lower**: poor recovery, overreaching concern

**scientific basis**: HRV (specifically RMSSD) reflects autonomic nervous system recovery. decreases indicate accumulated fatigue, increases indicate good adaptation.

**reference**: Plews, D.J. et al. (2013). Training adaptation and heart rate variability in elite endurance athletes. *Int J Sports Physiol Perform*, 8(3), 286-293.

### configuration

all sleep/recovery thresholds configurable via environment variables:

```bash
# Sleep duration thresholds (hours)
PIERRE_SLEEP_ADULT_MIN_HOURS=7.0
PIERRE_SLEEP_ATHLETE_OPTIMAL_HOURS=8.0
PIERRE_SLEEP_SHORT_THRESHOLD=6.0
PIERRE_SLEEP_VERY_SHORT_THRESHOLD=5.0

# Sleep stages thresholds (percentage)
PIERRE_SLEEP_DEEP_MIN_PERCENT=15.0
PIERRE_SLEEP_DEEP_OPTIMAL_PERCENT=20.0
PIERRE_SLEEP_REM_MIN_PERCENT=20.0
PIERRE_SLEEP_REM_OPTIMAL_PERCENT=25.0

# Sleep efficiency thresholds (percentage)
PIERRE_SLEEP_EFFICIENCY_EXCELLENT=90.0
PIERRE_SLEEP_EFFICIENCY_GOOD=85.0
PIERRE_SLEEP_EFFICIENCY_POOR=70.0

# HRV thresholds (milliseconds)
PIERRE_HRV_RMSSD_DECREASE_CONCERN=-10.0
PIERRE_HRV_RMSSD_INCREASE_GOOD=5.0

# TSB thresholds
PIERRE_TSB_HIGHLY_FATIGUED=-15.0
PIERRE_TSB_FATIGUED=-10.0
PIERRE_TSB_FRESH_MIN=5.0
PIERRE_TSB_FRESH_MAX=15.0
PIERRE_TSB_DETRAINING=25.0

# Recovery scoring weights
PIERRE_RECOVERY_TSB_WEIGHT_FULL=0.4
PIERRE_RECOVERY_SLEEP_WEIGHT_FULL=0.4
PIERRE_RECOVERY_HRV_WEIGHT_FULL=0.2
PIERRE_RECOVERY_TSB_WEIGHT_NO_HRV=0.5
PIERRE_RECOVERY_SLEEP_WEIGHT_NO_HRV=0.5
```

defaults based on peer-reviewed research (NSF, AASM, Shaffer & Ginsberg 2017).

---

## validation and safety

### parameter bounds (physiological ranges)

```rust
// src/intelligence/physiological_constants.rs::configuration_validation
pub const MAX_HR_MIN: u64 = 100;
pub const MAX_HR_MAX: u64 = 220;
pub const RESTING_HR_MIN: u64 = 30;
pub const RESTING_HR_MAX: u64 = 100;
pub const THRESHOLD_HR_MIN: u64 = 100;
pub const THRESHOLD_HR_MAX: u64 = 200;
pub const VO2_MAX_MIN: f64 = 20.0;
pub const VO2_MAX_MAX: f64 = 90.0;
pub const FTP_MIN: u64 = 50;
pub const FTP_MAX: u64 = 600;

// src/protocols/universal/handlers/configuration.rs
pub fn validate_parameter_ranges(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    let mut all_valid = true;

    // Validate max_hr
    if let Some(hr) = obj.get("max_hr").and_then(Value::as_u64) {
        if !(MAX_HR_MIN..=MAX_HR_MAX).contains(&hr) {
            all_valid = false;
            errors.push(format!(
                "max_hr must be between {MAX_HR_MIN} and {MAX_HR_MAX} bpm, got {hr}"
            ));
        }
    }

    // Validate resting_hr
    if let Some(hr) = obj.get("resting_hr").and_then(Value::as_u64) {
        if !(RESTING_HR_MIN..=RESTING_HR_MAX).contains(&hr) {
            all_valid = false;
            errors.push(format!(
                "resting_hr must be between {RESTING_HR_MIN} and {RESTING_HR_MAX} bpm, got {hr}"
            ));
        }
    }

    // ... other validations

    all_valid
}

pub fn validate_parameter_relationships(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    let mut all_valid = true;

    let max_hr = obj.get("max_hr").and_then(Value::as_u64);
    let resting_hr = obj.get("resting_hr").and_then(Value::as_u64);
    let threshold_hr = obj.get("threshold_hr").and_then(Value::as_u64);

    // Validate resting_hr < threshold_hr < max_hr
    if let (Some(resting), Some(max)) = (resting_hr, max_hr) {
        if resting >= max {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than max_hr ({max})"
            ));
        }
    }

    if let (Some(resting), Some(threshold)) = (resting_hr, threshold_hr) {
        if resting >= threshold {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than threshold_hr ({threshold})"
            ));
        }
    }

    if let (Some(threshold), Some(max)) = (threshold_hr, max_hr) {
        if threshold >= max {
            all_valid = false;
            errors.push(format!(
                "threshold_hr ({threshold}) must be less than max_hr ({max})"
            ));
        }
    }

    all_valid
}
```

**validation types**:
1. **range validation**: each parameter within physiologically plausible bounds
2. **relationship validation**: resting_hr < threshold_hr < max_hr

**references**:
- ACSM Guidelines for Exercise Testing and Prescription, 11th Edition
- European Society of Cardiology guidelines on exercise testing

### confidence levels

```rust
pub fn calculate_confidence(
    data_points: usize,
    r_squared: f64,
) -> ConfidenceLevel {
    match (data_points, r_squared) {
        (n, r) if n >= 15 && r >= 0.7 => ConfidenceLevel::High,
        (n, r) if n >= 8  && r >= 0.5 => ConfidenceLevel::Medium,
        (n, r) if n >= 3  && r >= 0.3 => ConfidenceLevel::Low,
        _ => ConfidenceLevel::VeryLow,
    }
}
```

### edge case handling

**1. users with no activities**:
```rust
if activities.is_empty() {
    return Ok(TrainingLoad {
        ctl: 0.0,
        atl: 0.0,
        tsb: 0.0,
        tss_history: Vec::new(),
    });
}
```

**2. training gaps (TSS sequence breaks)**:
```rust
// Zero-fill missing days in EMA calculation
let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0); // Gap = 0
ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
```
Result: CTL/ATL naturally decay during breaks (realistic fitness loss)

**3. invalid physiological parameters**:
```rust
// Range validation catches: max_hr=250 (exceeds 220), resting_hr=120 (exceeds 100)
// Relationship validation catches: max_hr=150 < resting_hr=160
// Returns detailed error messages for each violation
```

**4. invalid race velocities**:
```rust
if !(MIN_VELOCITY..=MAX_VELOCITY).contains(&velocity) {
    return Err(AppError::invalid_input(format!(
        "Velocity {velocity:.1} m/min outside valid range (100-500)"
    )));
}
```

**5. VDOT out of range**:
```rust
if !(30.0..=85.0).contains(&vdot) {
    return Err(AppError::invalid_input(format!(
        "VDOT {vdot:.1} outside typical range (30-85)"
    )));
}
```

---

## configuration strategies

three strategies adjust thresholds:

### conservative
```rust
impl IntelligenceStrategy for ConservativeStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.05 } // 5%
    fn recovery_threshold(&self) -> f64 { 1.2 }
}
```
**use**: injury recovery, beginners, older athletes

### default
```rust
impl IntelligenceStrategy for DefaultStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.10 } // 10%
    fn recovery_threshold(&self) -> f64 { 1.3 }
}
```
**use**: general training, recreational athletes

### aggressive
```rust
impl IntelligenceStrategy for AggressiveStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.15 } // 15%
    fn recovery_threshold(&self) -> f64 { 1.5 }
}
```
**use**: competitive athletes, experienced trainers

---

## testing and verification

### test coverage

**unit tests** (22 functions, 562 assertions):
- `tests/pattern_detection_test.rs` - 4 tests
- `tests/performance_prediction_test.rs` - 9 tests
- `tests/training_load_test.rs` - 6 tests
- `tests/vdot_table_verification_test.rs` - 3 tests

**integration tests** (116+ test files):
- Full MCP tool workflows
- Multi-provider scenarios
- Edge case handling
- Error recovery

**automated intelligence testing** (30+ integration tests):
- `tests/intelligence_tools_basic_test.rs` - 10 tests covering basic fitness data tools
- `tests/intelligence_tools_advanced_test.rs` - 20+ tests covering analytics, predictions, and goals
- `tests/intelligence_synthetic_helpers_test.rs` - synthetic data generation validation

**synthetic data framework** (`tests/helpers/`):
- `synthetic_provider.rs` - mock fitness provider with realistic activity data
- `synthetic_data.rs` - configurable test scenarios (beginner runner, experienced cyclist, multi-sport)
- `test_utils.rs` - test utilities and scenario builders
- enables testing all 8 intelligence tools without OAuth dependencies

### verification methods

**1. scientific validation**:
- VDOT predictions: 0.2-5.5% accuracy vs. jack daniels' tables
- TSS formulas: match coggan's published methodology
- Statistical methods: verified against standard regression algorithms

**2. edge case testing**:
```rust
#[test]
fn test_empty_activities() {
    let result = TrainingLoadCalculator::new()
        .calculate_training_load(&[], None, None, None, None, None)
        .unwrap();
    assert_eq!(result.ctl, 0.0);
    assert_eq!(result.atl, 0.0);
}

#[test]
fn test_training_gaps() {
    // Activities: day 1, day 10 (9-day gap)
    // EMA should decay naturally through the gap
    let activities = create_activities_with_gap();
    let result = calculate_training_load(&activities).unwrap();
    // Verify CTL decay through gap
}

#[test]
fn test_invalid_hr_relationships() {
    let config = json!({
        "max_hr": 150,
        "resting_hr": 160
    });
    let result = validate_configuration(&config);
    assert!(result.errors.contains("resting_hr must be less than max_hr"));
}
```

**3. placeholder elimination**:
```bash
# Zero placeholders confirmed
rg -i "placeholder|todo|fixme|hack|stub" src/ | wc -l
# Output: 0
```

**5. synthetic data testing**:
```rust
// Example: Test fitness score calculation with synthetic data
#[tokio::test]
async fn test_fitness_score_calculation() {
    let provider = create_synthetic_provider_with_scenario(
        TestScenario::ExperiencedCyclistConsistent
    );

    let activities = provider.get_activities(Some(100), None)
        .await.expect("Should get activities");

    let analyzer = PerformanceAnalyzerV2::new(Box::new(DefaultStrategy))
        .expect("Should create analyzer");

    let fitness_score = analyzer.calculate_fitness_score(&activities)
        .expect("Should calculate fitness score");

    // Verify realistic fitness score for experienced cyclist
    assert!(fitness_score.overall_score >= 70.0);
    assert!(fitness_score.overall_score <= 90.0);
}
```

**4. code quality**:
```bash
# Zero clippy warnings (pedantic + nursery)
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
# Output: PASS

# Zero prohibited patterns
rg "unwrap\(\)|expect\(|panic!\(|anyhow!\(" src/ | wc -l
# Output: 0
```

---

## limitations

### model assumptions
1. **linear progression**: assumes linear improvement, but adaptation is non-linear
2. **steady-state**: assumes consistent training environment
3. **population averages**: formulas may not fit individual physiology
4. **data quality**: sensor accuracy affects calculations

### known issues
- **HR metrics**: affected by caffeine, sleep, stress, heat, altitude
- **power metrics**: require proper FTP testing, affected by wind/drafting
- **pace metrics**: terrain and weather significantly affect running

### prediction accuracy
- **VDOT**: ±5% typical variance from actual race performance
- **TSB**: individual response to training load varies
- **patterns**: require sufficient data (minimum 3 weeks for trends)

---

## references

### scientific literature

1. **Banister, E.W.** (1991). Modeling elite athletic performance. Human Kinetics.

2. **Coggan, A. & Allen, H.** (2010). *Training and Racing with a Power Meter* (2nd ed.). VeloPress.

3. **Daniels, J.** (2013). *Daniels' Running Formula* (3rd ed.). Human Kinetics.

4. **Esteve-Lanao, J. et al.** (2005). How do endurance runners train? *Med Sci Sports Exerc*, 37(3), 496-504.

5. **Halson, S.L.** (2014). Monitoring training load to understand fatigue. *Sports Medicine*, 44(Suppl 2), 139-147.

6. **Karvonen, M.J. et al.** (1957). The effects of training on heart rate. *Ann Med Exp Biol Fenn*, 35(3), 307-315.

7. **Riegel, P.S.** (1981). Athletic records and human endurance. *American Scientist*, 69(3), 285-290.

8. **Tanaka, H. et al.** (2001). Age-predicted maximal heart rate revisited. *J Am Coll Cardiol*, 37(1), 153-156.

9. **Gabbett, T.J.** (2016). The training-injury prevention paradox. *Br J Sports Med*, 50(5), 273-280.

10. **Seiler, S.** (2010). Training intensity distribution in endurance athletes. *Int J Sports Physiol Perform*, 5(3), 276-291.

11. **Draper, N.R. & Smith, H.** (1998). *Applied Regression Analysis* (3rd ed.). Wiley.

12. **Hirshkowitz, M. et al.** (2015). National Sleep Foundation's sleep time duration recommendations: methodology and results summary. *Sleep Health*, 1(1), 40-43.

13. **Berry, R.B. et al.** (2017). The AASM Manual for the Scoring of Sleep and Associated Events: Rules, Terminology and Technical Specifications, Version 2.4. *American Academy of Sleep Medicine*.

14. **Watson, N.F. et al.** (2015). Recommended Amount of Sleep for a Healthy Adult: A Joint Consensus Statement of the American Academy of Sleep Medicine and Sleep Research Society. *Sleep*, 38(6), 843-844.

15. **Plews, D.J. et al.** (2013). Training adaptation and heart rate variability in elite endurance athletes: opening the door to effective monitoring. *Int J Sports Physiol Perform*, 8(3), 286-293.

16. **Shaffer, F. & Ginsberg, J.P.** (2017). An Overview of Heart Rate Variability Metrics and Norms. *Front Public Health*, 5, 258.

---

## faq

**Q: why doesn't my prediction match race day?**
A: predictions are ranges (±5%), not exact. affected by: weather, course, pacing, nutrition, taper, mental state.

**Q: can analytics work without HR or power?**
A: yes, but lower confidence. pace-based TSS estimates used. add HR/power for better accuracy.

**Q: how often update FTP/LTHR?**
A: FTP every 6-8 weeks, LTHR every 8-12 weeks, max HR annually.

**Q: why is TSB negative?**
A: normal during training. -30 to -10 = building fitness, -10 to 0 = productive, 0 to +10 = fresh/race ready.

**Q: how interpret confidence levels?**
A: high (15+ points, R²>0.7) = actionable; medium = guidance; low = directional; very low = insufficient data.

**Q: what happens if I have gaps in training?**
A: CTL/ATL naturally decay with zero TSS during gaps. this accurately models fitness loss during breaks.

**Q: how accurate are the VDOT predictions?**
A: verified 0.2-5.5% accuracy against jack daniels' published tables. predictions assume proper training, taper, and race conditions.

**Q: what if my parameters are outside the valid ranges?**
A: validation will reject with specific error messages. ranges are based on human physiology research (ACSM guidelines).

**Q: how much sleep do athletes need?**
A: 8-10 hours for optimal recovery (NSF guidelines). minimum 7 hours for adults. <6 hours increases injury risk and impairs performance.

**Q: what's more important: sleep duration or quality?**
A: both matter. 8 hours of fragmented sleep (70% efficiency) scores lower than 7 hours of solid sleep (95% efficiency). aim for both duration and quality.

**Q: why is my recovery score low despite good sleep?**
A: recovery combines TSB (40%), sleep (40%), HRV (20%). negative TSB from high training load lowers score even with good sleep. this accurately reflects accumulated fatigue.

**Q: how does HRV affect recovery scoring?**
A: HRV (RMSSD) indicates autonomic nervous system recovery. +5ms above baseline = excellent, ±3ms = normal, -10ms = poor recovery. when unavailable, recovery uses 50% TSB + 50% sleep.

**Q: what providers support sleep tracking?**
A: fitbit, garmin, and whoop provide sleep data. strava does not (returns `UnsupportedFeature` error). use provider with sleep tracking for full recovery analysis.

---

## glossary

**ATL**: acute training load (7-day EMA of TSS) - fatigue
**CTL**: chronic training load (42-day EMA of TSS) - fitness
**EMA**: exponential moving average - weighted average giving more weight to recent data
**FTP**: functional threshold power (1-hour max power)
**LTHR**: lactate threshold heart rate
**TSB**: training stress balance (CTL - ATL) - form
**TSS**: training stress score (duration × intensity²)
**VDOT**: VO2max adjusted for running economy (jack daniels)
**NP**: normalized power (4th root method)
**R²**: coefficient of determination (fit quality, 0-1)
**IF**: intensity factor (NP / FTP)
**RMSSD**: root mean square of successive differences (HRV metric, milliseconds)
**HRV**: heart rate variability (autonomic nervous system recovery indicator)
**NSF**: National Sleep Foundation (sleep duration guidelines)
**AASM**: American Academy of Sleep Medicine (sleep stage scoring standards)
**REM**: rapid eye movement sleep (cognitive recovery, memory consolidation)
**N3/deep sleep**: slow-wave sleep (physical recovery, growth hormone release)
**sleep efficiency**: (time asleep / time in bed) × 100 (fragmentation indicator)
**sleep quality**: combined score (35% duration, 40% stages, 25% efficiency)
**recovery score**: training readiness (40% TSB, 40% sleep, 20% HRV)

---

**document version**: 3.0
**last updated**: 2025-10-29
**maintainer**: pierre intelligence team
**implementation status**: production-ready (placeholders eliminated 2025-10-29)
