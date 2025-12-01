<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 20: Sports Science Algorithms & Intelligence

This chapter explores how Pierre implements sports science algorithms for training load management, fitness tracking, and performance estimation. You'll learn about TSS calculation, CTL/ATL/TSB (Performance Manager Chart), VO2 max estimation, FTP detection, and algorithm configuration patterns.

## What You'll Learn

- Training Stress Score (TSS) calculation
- CTL/ATL/TSB (Chronic/Acute Training Load, Training Stress Balance)
- VO2 max estimation algorithms
- FTP (Functional Threshold Power) detection
- Algorithm configuration pattern
- Multiple implementation strategies
- Scientific references and validation
- Enum-based algorithm selection

## Algorithm Configuration Pattern

Pierre uses enums to select between multiple algorithm implementations:

```
User selects algorithm → Enum variant → Implementation strategy → Result
```

**Pattern benefits**:
- **Flexibility**: Easy to add new algorithms
- **Testability**: Compare algorithm outputs for validation
- **User choice**: Power users can optimize for their data/use case
- **Backwards compatibility**: Default impl when new algorithms added

## Training Stress Score TSS

TSS quantifies training load from a single workout.

**Source**: src/intelligence/algorithms/tss.rs:10-52
```rust
/// TSS calculation algorithm selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TssAlgorithm {
    /// Average power based TSS (current default)
    ///
    /// Formula: `duration_hours x (avg_power/FTP)² x 100`
    ///
    /// Pros: O(1) computation, works without power stream
    /// Cons: Underestimates variable efforts by 15-30%
    #[default]
    AvgPower,

    /// Normalized Power based TSS (industry standard)
    ///
    /// Formula: `duration_hours x (NP/FTP)² x 100`
    ///
    /// `NP = ⁴√(mean(mean_per_30s_window(power⁴)))`
    ///
    /// Pros: Physiologically accurate (R²=0.92 vs glycogen depletion)
    /// Cons: Requires ≥30s power stream data
    NormalizedPower {
        /// Rolling window size in seconds (standard: 30)
        window_seconds: u32,
    },

    /// Hybrid approach: Try NP, fallback to `avg_power` if stream unavailable
    ///
    /// Best of both worlds for defensive programming
    Hybrid,
}
```

**TSS interpretation**:
- **< 150**: Easy recovery ride/run
- **150-300**: Moderate workout
- **300-450**: Hard training session
- **> 450**: Very hard/race effort

**Average Power TSS** (simple):

**Source**: src/intelligence/algorithms/tss.rs:111-124
```rust
fn calculate_avg_power_tss(
    activity: &Activity,
    ftp: f64,
    duration_hours: f64,
) -> Result<f64, AppError> {
    let avg_power = f64::from(
        activity
            .average_power
            .ok_or_else(|| AppError::not_found("average power data".to_owned()))?,
    );

    let intensity_factor = avg_power / ftp;
    Ok((duration_hours * intensity_factor * intensity_factor * TSS_BASE_MULTIPLIER).round())
}
```

**Normalized Power TSS** (accurate):

**Source**: src/intelligence/algorithms/tss.rs:129-139
```rust
fn calculate_np_tss(
    activity: &Activity,
    ftp: f64,
    duration_hours: f64,
    window_seconds: u32,
) -> Result<f64, AppError> {
    // Calculate TSS using normalized power from activity power stream data
    let np = Self::calculate_normalized_power(activity, window_seconds)?;
    let intensity_factor = np / ftp;
    Ok((duration_hours * intensity_factor * intensity_factor * TSS_BASE_MULTIPLIER).round())
}
```

**Normalized Power formula**:
```
NP = ⁴√(mean(mean_per_30s_window(power⁴)))
```

**Why 4th power**: Matches physiological stress curve (glycogen depletion, lactate accumulation).

## CTL/ATL/TSB (performance Manager Chart)

CTL/ATL/TSB track fitness, fatigue, and form over time.

**Definitions**:
- **CTL (Chronic Training Load)**: 42-day exponential moving average of TSS (fitness)
- **ATL (Acute Training Load)**: 7-day exponential moving average of TSS (fatigue)
- **TSB (Training Stress Balance)**: CTL - ATL (form/freshness)

**Source**: src/intelligence/algorithms/training_load.rs:8-85
```rust
/// Training load calculation algorithm selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingLoadAlgorithm {
    /// Exponential Moving Average (EMA)
    ///
    /// Formula: `α = 2/(N+1)`, `EMA_t = α x TSS_t + (1-α) x EMA_{t-1}`
    ///
    /// Standard method used by TrainingPeaks Performance Manager Chart.
    /// Recent days weighted more heavily with exponential decay.
    Ema {
        /// CTL window in days (default 42 for fitness)
        ctl_days: i64,
        /// ATL window in days (default 7 for fatigue)
        atl_days: i64,
    },

    /// Simple Moving Average (SMA)
    ///
    /// Formula: `SMA = Σ(TSS_i) / N` for i in [t-N+1, t]
    ///
    /// All days in window weighted equally.
    Sma {
        ctl_days: i64,
        atl_days: i64,
    },

    /// Weighted Moving Average (WMA)
    ///
    /// Formula: `WMA = Σ(w_i x TSS_i) / Σ(w_i)` where `w_i = i` (linear weights)
    ///
    /// Recent days weighted linearly more than older days.
    Wma {
        ctl_days: i64,
        atl_days: i64,
    },

    /// Kalman Filter
    ///
    /// State-space model with process and measurement noise.
    /// Optimal estimation when data is noisy or has gaps.
    KalmanFilter {
        /// Process noise (training load variability)
        process_noise: f64,
        /// Measurement noise (TSS measurement error)
        measurement_noise: f64,
    },
}
```

**EMA calculation**:

**Source**: src/intelligence/algorithms/training_load.rs:122-136
```rust
pub fn calculate_ctl(&self, tss_data: &[TssDataPoint]) -> Result<f64, AppError> {
    if tss_data.is_empty() {
        return Ok(0.0);
    }

    match self {
        Self::Ema { ctl_days, .. } => Self::calculate_ema(tss_data, *ctl_days),
        Self::Sma { ctl_days, .. } => Self::calculate_sma(tss_data, *ctl_days),
        Self::Wma { ctl_days, .. } => Self::calculate_wma(tss_data, *ctl_days),
        Self::KalmanFilter {
            process_noise,
            measurement_noise,
        } => Self::calculate_kalman(tss_data, *process_noise, *measurement_noise),
    }
}
```

**TSB interpretation**:
- **TSB > +25**: Well-rested, ready for peak performance
- **TSB +10 to +25**: Fresh, good for races
- **TSB -10 to +10**: Balanced, sustainable training
- **TSB -10 to -30**: Fatigued, productive overload
- **TSB < -30**: High risk of overtraining

**EMA formula**:
```
α = 2 / (N + 1)
CTL_today = α × TSS_today + (1 - α) × CTL_yesterday

For CTL (N=42): α = 2/43 ≈ 0.0465 (slow adaptation)
For ATL (N=7):  α = 2/8  = 0.25   (fast response)
```

## VO2 Max Estimation

VO2 max represents maximal aerobic capacity (ml/kg/min).

**Source**: src/intelligence/algorithms/vo2max.rs:18-100
```rust
/// VO2max estimation algorithm selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Vo2maxAlgorithm {
    /// From Jack Daniels' VDOT
    ///
    /// Formula: `VO2max = VDOT x 3.5`
    ///
    /// VDOT is Jack Daniels' running economy-adjusted VO2max measure.
    FromVdot {
        /// VDOT value (30-85 for recreational to elite)
        vdot: f64,
    },

    /// Cooper 12-Minute Run Test
    ///
    /// Formula: `VO2max = (distance_meters - 504.9) / 44.73`
    ///
    /// Run as far as possible in 12 minutes on a flat track.
    CooperTest {
        /// Distance covered in 12 minutes (meters)
        distance_meters: f64,
    },

    /// Rockport 1-Mile Walk Test
    ///
    /// Formula: `VO2max = 132.853 - 0.0769×weight - 0.3877×age + 6.315×gender - 3.2649×time - 0.1565×HR`
    ///
    /// Walk 1 mile as fast as possible, measure time and heart rate at finish.
    /// Gender: 0 = female, 1 = male
    RockportWalk {
        weight_kg: f64,
        age: u8,
        gender: u8,
        time_seconds: f64,
        heart_rate: f64,
    },

    /// Åstrand-Ryhming Cycle Ergometer Test
    ///
    /// Submaximal cycle test at steady-state heart rate (120-170 bpm).
    AstrandRyhming {
        gender: u8,
        heart_rate: f64,
        power_watts: f64,
    },
}
```

**VO2 max ranges** (ml/kg/min):
- **Untrained**: 30-40 (recreational)
- **Trained**: 40-50 (club runner)
- **Well-trained**: 50-60 (competitive)
- **Elite**: 60-70 (national level)
- **World-class**: 70-85 (Olympic/professional)

**Cooper Test example**:
```
Distance: 3000 meters in 12 minutes
VO2max = (3000 - 504.9) / 44.73 ≈ 55.8 ml/kg/min (well-trained)
```

## Algorithm Selection Pattern

All algorithms follow the same pattern:

```rust
enum Algorithm {
    Method1 { params },
    Method2 { params },
    Method3,
}

impl Algorithm {
    fn calculate(&self, data: &Data) -> Result<f64> {
        match self {
            Self::Method1 { params } => /* implementation */,
            Self::Method2 { params } => /* implementation */,
            Self::Method3 => /* implementation */,
        }
    }
}
```

**Benefits**:
1. **Type safety**: Compiler ensures all enum variants handled
2. **Documentation**: Variant doc comments explain algorithms
3. **Flexibility**: Easy to add new algorithms
4. **Configuration**: Serialize/deserialize for user preferences
5. **Testing**: Compare algorithms for validation

## Scientific Validation

Pierre includes scientific references for algorithms:

**TSS references**:
- Coggan, A. & Allen, H. (2010). "Training and Racing with a Power Meter." VeloPress.
- Sanders, D. & Heijboer, M. (2018). "The anaerobic power reserve." *J Sports Sci*, 36(6), 621-629.

**CTL/ATL/TSB references**:
- Coggan, A. (2003). "Training and Racing Using a Power Meter." Peaksware LLC.
- Banister, E.W. (1991). "Modeling elite athletic performance." *Physiological Testing of Elite Athletes*.

**VO2 max references**:
- Daniels, J. (2013). "Daniels' Running Formula" (3rd ed.). Human Kinetics.
- Cooper, K.H. (1968). "A means of assessing maximal oxygen intake." *JAMA*, 203(3), 201-204.

**Validation approach**:
- **Literature-based**: All formulas from peer-reviewed research
- **Industry standards**: TrainingPeaks, Strava algorithms as benchmarks
- **Correlation studies**: Verify against physiological measurements

## Key Takeaways

1. **Enum-based selection**: All algorithms use enum pattern for flexibility and type safety.

2. **TSS calculation**: Average power (fast, less accurate) vs Normalized Power (slow, more accurate).

3. **CTL/ATL/TSB**: 42-day fitness, 7-day fatigue, balance indicates form/readiness.

4. **EMA standard**: Exponential moving average matches TrainingPeaks industry standard.

5. **VO2 max estimation**: Multiple test protocols (Cooper, Rockport, Åstrand-Ryhming).

6. **Scientific references**: All algorithms cite peer-reviewed research for validation.

7. **Multiple strategies**: Users can choose algorithms based on data availability and preferences.

8. **Hybrid fallback**: Defensive programming with primary + fallback implementations.

9. **Documentation in code**: Variant doc comments explain formulas, pros/cons, scientific basis.

10. **Serde support**: All algorithms serialize/deserialize for configuration persistence.

---

**Next Chapter**: [Chapter 21: Training Load, Recovery & Sleep Analysis](./chapter-21-recovery-sleep.md) - Learn how Pierre analyzes recovery metrics, sleep quality, HRV, and suggests optimal training intensity based on recovery status.
