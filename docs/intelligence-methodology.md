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
                       FOUNDATION MODULES 
                Shared by all intelligence tools
```

### foundation modules

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
pierre normalizes data from different providers into a unified activity data model containing:
- provider identifier (Strava, Garmin, Fitbit)
- temporal data: start_date (UTC timestamp)
- spatial data: distance (meters), moving_time (seconds)
- sport classification: sport_type (string)
- physiological metrics: heart rate, power, cadence (provider-dependent)

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

**formula**:

max_hr(age) = 208 − (0.7 × age)

**bounds**:

max_hr ∈ [160, 210] bpm to exclude physiologically implausible values.

**reference**: Tanaka, H., Monahan, K.D., & Seals, D.R. (2001). Age-predicted maximal heart rate revisited. *Journal of the American College of Cardiology*, 37(1), 153-156.

**alternative**: fox formula (`220 − age`) available via configuration but tanaka preferred for accuracy.

### heart rate zones

pierre implements **karvonen method** (HR reserve) when `resting_hr` available:

**karvonen formula**:

target_hr(intensity%) = (HR_reserve × intensity%) + HR_rest

where:
- HR_reserve = HR_max − HR_rest
- intensity% ∈ [0, 1]

**five-zone model**:

Zone 1 (Recovery): HR ∈ [HR_rest + 0.50 × HR_reserve, HR_rest + 0.60 × HR_reserve]
Zone 2 (Endurance): HR ∈ [HR_rest + 0.60 × HR_reserve, HR_rest + 0.70 × HR_reserve]
Zone 3 (Tempo): HR ∈ [HR_rest + 0.70 × HR_reserve, HR_rest + 0.80 × HR_reserve]
Zone 4 (Threshold): HR ∈ [HR_rest + 0.80 × HR_reserve, HR_rest + 0.90 × HR_reserve]
Zone 5 (VO2max): HR ∈ [HR_rest + 0.90 × HR_reserve, HR_max]

**fallback**: when `resting_hr` unavailable, uses simple percentage of `max_hr` (50%, 60%, 70%, 80%, 90%).

**reference**: Karvonen, M.J., Kentala, E., & Mustala, O. (1957). The effects of training on heart rate; a longitudinal study. *Annales medicinae experimentalis et biologiae Fenniae*, 35(3), 307-315.

### power zones (cycling)

five-zone model based on functional threshold power (FTP):

**power zones**:

Zone 1 (Active Recovery): P ∈ [0, 0.55 × FTP)
Zone 2 (Endurance): P ∈ [0.55 × FTP, 0.75 × FTP)
Zone 3 (Tempo): P ∈ [0.75 × FTP, 0.90 × FTP)
Zone 4 (Threshold): P ∈ [0.90 × FTP, 1.05 × FTP)
Zone 5 (VO2max+): P ∈ [1.05 × FTP, ∞)

**physiological adaptations**:
- **Z1 (active recovery)**: < 55% FTP - flush metabolites, active rest
- **Z2 (endurance)**: 55-75% FTP - aerobic base building
- **Z3 (tempo)**: 75-90% FTP - muscular endurance
- **Z4 (threshold)**: 90-105% FTP - lactate threshold work
- **Z5 (VO2max+)**: > 105% FTP - maximal aerobic/anaerobic efforts

**reference**: Coggan, A. & Allen, H. (2010). *Training and Racing with a Power Meter* (2nd ed.). VeloPress.

---

## core metrics

### pace vs speed

**pace formula** (time per distance, seconds per kilometer):

         ⎧ 0,                    if d < 1 meter
pace = ⎨
         ⎩ t / (d / 1000),       if d ≥ 1 meter

where:
- t = moving time (seconds)
- d = distance (meters)

**speed formula** (distance per time, meters per second):

          ⎧ 0,        if t = 0
speed = ⎨
          ⎩ d / t,    if t > 0

where:
- d = distance (meters)
- t = moving time (seconds)

---

## training stress score (TSS)

TSS quantifies training load accounting for intensity and duration.

### power-based TSS (preferred)

**formula**:

TSS = duration_hours × IF² × 100

where:
- IF = intensity factor = NP / FTP
- NP = normalized power (watts)
- FTP = functional threshold power (watts)
- duration_hours = activity duration (hours)

**example**: 2-hour ride at 250W NP with FTP=300W
- IF = 250 / 300 = 0.833
- TSS = 2.0 × (0.833)² × 100 = 138.9

### heart rate-based TSS (hrTSS)

**formula**:

hrTSS = duration_hours × (HR_avg / HR_threshold)² × 100

where:
- HR_avg = average heart rate during activity (bpm)
- HR_threshold = lactate threshold heart rate (bpm)
- duration_hours = activity duration (hours)

**interpretation**:
- TSS < 150: low training stress
- 150 ≤ TSS < 300: moderate training stress
- 300 ≤ TSS < 450: high training stress
- TSS ≥ 450: very high training stress

**reference**: Coggan, A. (2003). Training Stress Score. *TrainingPeaks*.

---

## normalized power (NP)

accounts for variability in cycling efforts using a three-step algorithm:

**algorithm**:

1. Calculate 30-second rolling average:
   For each 30-second window i:

   P̄ᵢ = (1/30) × Σⱼ₌₀²⁹ Pᵢ₊ⱼ

2. Raise each rolling average to 4th power:

   Qᵢ = (P̄ᵢ)⁴

3. Calculate 4th root of average:

   NP = ⁴√((1/n) × Σᵢ₌₁ⁿ Qᵢ)

where:
- Pᵢ = instantaneous power at second i (watts)
- n = number of 30-second windows
- P̄ᵢ = 30-second rolling average power (watts)

**fallback** (if data < 30 seconds):

NP = (1/n) × Σᵢ₌₁ⁿ Pᵢ

**physiological basis**: 4th power weighting matches metabolic cost of variable efforts. Alternating 200W/150W has higher physiological cost than steady 175W.

---

## chronic training load (CTL) and acute training load (ATL)

CTL ("fitness") and ATL ("fatigue") track training stress using exponential moving averages.

### mathematical formulation

**exponential moving average (EMA)**:

α = 2 / (N + 1)

EMAₜ = α × TSSₜ + (1 − α) × EMAₜ₋₁

where:
- N = window size (days)
- TSSₜ = training stress score on day t
- EMAₜ = exponential moving average on day t
- α = smoothing factor ∈ (0, 1)

**chronic training load (CTL)**:

CTL = EMA₄₂(TSS_daily)

42-day exponential moving average of daily TSS, representing long-term fitness

**acute training load (ATL)**:

ATL = EMA₇(TSS_daily)

7-day exponential moving average of daily TSS, representing short-term fatigue

**training stress balance (TSB)**:

TSB = CTL − ATL

difference between fitness and fatigue, representing current form

**daily TSS aggregation** (multiple activities per day):

TSS_daily = Σᵢ₌₁ⁿ TSSᵢ

where n = number of activities on a given day

**gap handling** (missing training days):

For days with no activities: TSSₜ = 0

This causes exponential decay: EMAₜ = (1 − α) × EMAₜ₋₁

**edge case handling**:
- **zero activities**: CTL = 0, ATL = 0, TSB = 0
- **training gaps**: TSSₜ = 0 (realistic fitness decay through exponential decline)
- **multiple activities per day**: sum all TSS values for that day
- **failed TSS calculations**: skip invalid activities, continue with valid data

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. Human Kinetics.

---

## training stress balance (TSB)

TSB indicates form/freshness using piecewise classification:

**training status classification**:

                   ⎧ Overreaching,     if TSB < −10
                   ⎪
TrainingStatus = ⎨ Productive,       if −10 ≤ TSB < 0
                   ⎪
                   ⎪ Fresh,            if 0 ≤ TSB ≤ 10
                   ⎩ Detraining,       if TSB > 10

**interpretation**:
- **TSB < −10**: overreaching (high fatigue) - recovery needed
- **−10 ≤ TSB < 0**: productive training - building fitness
- **0 ≤ TSB ≤ 10**: fresh - ready for hard efforts
- **TSB > 10**: risk of detraining

**reference**: Banister, E.W., Calvert, T.W., Savage, M.V., & Bach, T. (1975). A systems model of training. *Australian Journal of Sports Medicine*, 7(3), 57-61.

---

## overtraining risk detection

**three-factor risk assessment**:

Risk Factor 1 (Acute Load Spike):
Triggered when: (CTL > 0) ∧ (ATL > 1.3 × CTL)

Risk Factor 2 (Very High Acute Load):
Triggered when: ATL > 150

Risk Factor 3 (Deep Fatigue):
Triggered when: TSB < −10

**risk level classification**:

                    ⎧ Low,        if |risk_factors| = 0
                    ⎪
RiskLevel = ⎨ Moderate,   if |risk_factors| = 1
                    ⎪
                    ⎩ High,       if |risk_factors| ≥ 2

where |risk_factors| = count of triggered risk factors

**physiological interpretation**:
- **Acute load spike**: fatigue (ATL) exceeds fitness (CTL) by >30%, indicating sudden increase
- **Very high acute load**: average daily TSS >150 in past week, exceeding sustainable threshold
- **Deep fatigue**: negative TSB <−10, indicating accumulated fatigue without recovery

**reference**: Halson, S.L. (2014). Monitoring training load to understand fatigue. *Sports Medicine*, 44(Suppl 2), 139-147.

---

## statistical trend analysis

pierre uses ordinary least squares linear regression for trend detection:

**linear regression formulation**:

Given n data points (xᵢ, yᵢ), fit line: ŷ = β₀ + β₁x

**slope calculation**:

β₁ = (Σᵢ₌₁ⁿ xᵢyᵢ − n × x̄ × ȳ) / (Σᵢ₌₁ⁿ xᵢ² − n × x̄²)

**intercept calculation**:

β₀ = ȳ − β₁ × x̄

where:
- x̄ = (1/n) × Σᵢ₌₁ⁿ xᵢ (mean of x values)
- ȳ = (1/n) × Σᵢ₌₁ⁿ yᵢ (mean of y values)
- n = number of data points

**coefficient of determination (R²)**:

R² = 1 − (SS_res / SS_tot)

where:
- SS_tot = Σᵢ₌₁ⁿ (yᵢ − ȳ)² (total sum of squares)
- SS_res = Σᵢ₌₁ⁿ (yᵢ − ŷᵢ)² (residual sum of squares)
- ŷᵢ = β₀ + β₁xᵢ (predicted value)

**correlation coefficient**:

r = sign(β₁) × √R²

**R² interpretation**:
- 0.0 ≤ R² < 0.3: weak relationship
- 0.3 ≤ R² < 0.5: moderate relationship
- 0.5 ≤ R² < 0.7: strong relationship
- 0.7 ≤ R² ≤ 1.0: very strong relationship

**reference**: Draper, N.R. & Smith, H. (1998). *Applied Regression Analysis* (3rd ed.). Wiley.

---

## performance prediction: VDOT

VDOT is jack daniels' VO2max adjusted for running economy:

### VDOT calculation from race performance

**step 1: convert to velocity** (meters per minute):

v = (d / t) × 60

where:
- d = distance (meters)
- t = time (seconds)
- v ∈ [100, 500] m/min (validated range)

**step 2: calculate VO2 consumption** (Jack Daniels' formula):

VO₂ = −4.60 + 0.182258v + 0.000104v²

**step 3: adjust for race duration**:

                    ⎧ 0.97,   if t_min < 5 (very short, oxygen deficit)
                    ⎪
                    ⎪ 0.99,   if 5 ≤ t_min < 15 (5K range)
                    ⎪
percent_max(t) = ⎨ 1.00,   if 15 ≤ t_min < 30 (10K-15K, optimal)
                    ⎪
                    ⎪ 0.98,   if 30 ≤ t_min < 90 (half marathon)
                    ⎪
                    ⎩ 0.95,   if t_min ≥ 90 (marathon+, fatigue)

where t_min = t / 60 (time in minutes)

**step 4: calculate VDOT**:

VDOT = VO₂ / percent_max(t)

**VDOT ranges**:
- 30-40: beginner
- 40-50: recreational
- 50-60: competitive amateur
- 60-70: sub-elite
- 70-85: elite
- VDOT ∈ [30, 85] (typical range)

### race time prediction from VDOT

**step 1: calculate velocity at VO2max** (inverse of Jack Daniels' formula):

Solve quadratic equation: 0.000104v² + 0.182258v − (VDOT + 4.60) = 0

Using quadratic formula: v = (−b + √(b² − 4ac)) / (2a)

where:
- a = 0.000104
- b = 0.182258
- c = −(VDOT + 4.60)

**step 2: adjust velocity for race distance**:

                      ⎧ 0.98 × v_max,                            if d ≤ 5,000 m
                      ⎪
                      ⎪ 0.94 × v_max,                            if 5,000 < d ≤ 10,000 m
                      ⎪
                      ⎪ 0.91 × v_max,                            if 10,000 < d ≤ 15,000 m
                      ⎪
v_race(d, v_max) = ⎨ 0.88 × v_max,                            if 15,000 < d ≤ 21,097.5 m
                      ⎪
                      ⎪ 0.84 × v_max,                            if 21,097.5 < d ≤ 42,195 m
                      ⎪
                      ⎪ max(0.70, 0.84 − 0.02(r − 1)) × v_max,  if d > 42,195 m
                      ⎩

where r = d / 42,195 (marathon ratio for ultra distances)

**step 3: calculate predicted time**:

t_predicted = (d / v_race) × 60

where:
- d = target distance (meters)
- v_race = race velocity (meters/minute)
- t_predicted = predicted time (seconds)

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

predicts race times across distances using power-law relationship:

**riegel formula**:

T₂ = T₁ × (D₂ / D₁)^1.06

where:
- T₁ = known race time (seconds)
- D₁ = known race distance (meters)
- T₂ = predicted race time (seconds)
- D₂ = target race distance (meters)
- 1.06 = riegel exponent (empirically derived constant)

**domain constraints**:
- D₁ > 0, T₁ > 0, D₂ > 0 (all values must be positive)

**example**: predict marathon from half marathon:
- Given: T₁ = 1:30:00 = 5400s, D₁ = 21,097m
- Target: D₂ = 42,195m
- Calculation: T₂ = 5400 × (42,195 / 21,097)^1.06 ≈ 11,340s ≈ 3:09:00

**reference**: Riegel, P.S. (1981). Athletic records and human endurance. *American Scientist*, 69(3), 285-290.

---

## pattern detection

### weekly schedule

**algorithm**:

1. Count activities by weekday: C(d) = |{activities on weekday d}|
2. Sort weekdays by frequency: rank by descending C(d)
3. Calculate consistency score based on distribution

**output**:
- most_common_days = top 3 weekdays by activity count
- consistency_score ∈ [0, 100]

**consistency interpretation**:
- 0 ≤ score < 30: highly variable
- 30 ≤ score < 60: moderate consistency
- 60 ≤ score < 80: consistent schedule
- 80 ≤ score ≤ 100: very consistent routine

### hard/easy alternation

**algorithm**:

1. Classify each activity intensity: I(a) ∈ {Hard, Easy}
2. Sort activities chronologically by date
3. Count alternations in consecutive activities:

   alternations = |{i : (I(aᵢ) = Hard ∧ I(aᵢ₊₁) = Easy) ∨ (I(aᵢ) = Easy ∧ I(aᵢ₊₁) = Hard)}|

4. Calculate pattern strength:

   pattern_strength = alternations / (n − 1)

   where n = number of activities

**classification**:

follows_pattern = ⎧ true,   if pattern_strength > 0.6
                  ⎩ false,  if pattern_strength ≤ 0.6

### volume progression

**algorithm**:

1. Group activities by week: compute total volume per week
2. Apply linear regression to weekly volumes (see statistical trend analysis section)
3. Classify trend based on slope:

                   ⎧ Increasing,   if slope > 0.05
                   ⎪
VolumeTrend = ⎨ Decreasing,   if slope < −0.05
                   ⎪
                   ⎩ Stable,       if −0.05 ≤ slope ≤ 0.05

**output**:
- trend classification
- slope (rate of change)
- R² (goodness of fit)

**reference**: Esteve-Lanao, J. et al. (2005). How do endurance runners train? *Med Sci Sports Exerc*, 37(3), 496-504.

---

## sleep and recovery analysis

### sleep quality scoring

pierre uses NSF (National Sleep Foundation) and AASM (American Academy of Sleep Medicine) guidelines for sleep quality assessment. the overall sleep quality score (0-100) combines three weighted components:

**sleep quality = (duration_score × 0.35) + (stages_score × 0.40) + (efficiency_score × 0.25)**

#### duration scoring

based on NSF recommendations with athlete-specific adjustments:

**piecewise linear scoring function**:

                         ⎧ 100,                               if d ≥ 8
                         ⎪
                         ⎪ 85 + 15(d − 7),                    if 7 ≤ d < 8
                         ⎪
duration_score(d) = ⎨ 60 + 25(d − 6),                    if 6 ≤ d < 7
                         ⎪
                         ⎪ 30 + 30(d − 5),                    if 5 ≤ d < 6
                         ⎪
                         ⎩ 30(d / 5),                         if d < 5

where:
- d = sleep duration (hours)
- thresholds configurable via environment variables (see configuration section)

**default thresholds**:
- **d ≥ 8 hours**: score = 100 (optimal for athletes)
- **7 ≤ d < 8 hours**: score ∈ [85, 100] (adequate for adults)
- **6 ≤ d < 7 hours**: score ∈ [60, 85] (short sleep)
- **5 ≤ d < 6 hours**: score ∈ [30, 60] (very short)
- **d < 5 hours**: score ∈ [0, 30] (severe deprivation)

**scientific basis**: NSF recommends 7-9h for adults, 8-10h for athletes. <6h linked to increased injury risk and impaired performance.

**reference**: Hirshkowitz, M. et al. (2015). National Sleep Foundation's sleep time duration recommendations. *Sleep Health*, 1(1), 40-43.

#### stages scoring

based on AASM guidelines for healthy sleep stage distribution:

**deep sleep scoring function**:

                      ⎧ 100,                      if p_deep ≥ 20
                      ⎪
deep_score(p_deep) = ⎨ 70 + 30(p_deep − 15)/5,    if 15 ≤ p_deep < 20
                      ⎪
                      ⎩ 70(p_deep / 15),          if p_deep < 15

**REM sleep scoring function**:

                     ⎧ 100,                     if p_rem ≥ 25
                     ⎪
rem_score(p_rem) = ⎨ 70 + 30(p_rem − 20)/5,     if 20 ≤ p_rem < 25
                     ⎪
                     ⎩ 70(p_rem / 20),          if p_rem < 20

**awake time penalty**:

                   ⎧ 0,                     if p_awake ≤ 5
penalty(p_awake) = ⎨
                   ⎩ 2(p_awake − 5),        if p_awake > 5

**combined stages score**:

stages_score = max(0, min(100, 0.4 × deep_score + 0.4 × rem_score + 0.2 × p_light − penalty))

where:
- p_deep = deep sleep percentage (%)
- p_rem = REM sleep percentage (%)
- p_light = light sleep percentage (%)
- p_awake = awake time percentage (%)

**optimal ranges**:
- **deep sleep**: 15-25% (physical recovery, growth hormone release)
- **REM sleep**: 20-25% (memory consolidation, cognitive function)
- **light sleep**: 45-55% (transition stages)
- **awake time**: <5% (sleep fragmentation indicator)

**scientific basis**: AASM sleep stage guidelines. deep sleep critical for physical recovery, REM for cognitive processing.

**reference**: Berry, R.B. et al. (2017). AASM Scoring Manual Version 2.4. *American Academy of Sleep Medicine*.

#### efficiency scoring

based on clinical sleep medicine thresholds:

**sleep efficiency formula**:

efficiency = (t_asleep / t_bed) × 100

where:
- t_asleep = total time asleep (minutes)
- t_bed = total time in bed (minutes)
- efficiency ∈ [0, 100] (percentage)

**piecewise linear scoring function**:

                            ⎧ 100,                        if e ≥ 90
                            ⎪
                            ⎪ 85 + 15(e − 85)/5,          if 85 ≤ e < 90
                            ⎪
efficiency_score(e) = ⎨ 65 + 20(e − 75)/10,               if 75 ≤ e < 85
                            ⎪
                            ⎩ 65(e / 75),                 if e < 75

where e = efficiency percentage

**thresholds**:
- **e ≥ 90%**: score = 100 (excellent, minimal sleep fragmentation)
- **85 ≤ e < 90%**: score ∈ [85, 100] (good, normal range)
- **75 ≤ e < 85%**: score ∈ [65, 85] (fair, moderate fragmentation)
- **e < 75%**: score ∈ [0, 65] (poor, severe fragmentation)

**scientific basis**: sleep efficiency >85% considered normal in clinical sleep medicine.

### recovery score calculation

pierre calculates training readiness by combining TSB, sleep quality, and HRV (when available):

**weighted recovery score formula**:

                     ⎧ 0.4 × TSB_score + 0.4 × sleep_score + 0.2 × HRV_score,   if HRV available
recovery_score = ⎨
                     ⎩ 0.5 × TSB_score + 0.5 × sleep_score,                     if HRV unavailable

where:
- TSB_score = normalized TSB score ∈ [0, 100] (see TSB normalization below)
- sleep_score = overall sleep quality score ∈ [0, 100] (from sleep analysis)
- HRV_score = heart rate variability score ∈ [0, 100] (when available)

**recovery level classification**:

                       ⎧ excellent,    if score ≥ 85
                       ⎪
                       ⎪ good,         if 70 ≤ score < 85
                       ⎪
recovery_level = ⎨ fair,         if 50 ≤ score < 70
                       ⎪
                       ⎩ poor,         if score < 50

#### TSB normalization

training stress balance maps to recovery score using piecewise linear normalization:

**TSB normalization function** (maps TSB ∈ [−30, +30] → score ∈ [0, 100]):

                     ⎧ 90 + 10 × min(10, TSB − 15) / 10,       if TSB ≥ 15
                     ⎪
                     ⎪ 80 + 10(TSB − 5) / 10,                  if 5 ≤ TSB < 15
                     ⎪
                     ⎪ 60 + 20(TSB + 5) / 10,                  if −5 ≤ TSB < 5
TSB_score(TSB) = ⎨
                     ⎪ 40 + 20(TSB + 10) / 5,                  if −10 ≤ TSB < −5
                     ⎪
                     ⎪ 20 + 20(TSB + 15) / 5,                  if −15 ≤ TSB < −10
                     ⎪
                     ⎩ max(0, 20(TSB + 30) / 15),              if TSB < −15

**physiological interpretation**:
- **TSB ≥ +15**: score ∈ [90, 100] - detraining (too much rest)
- **+5 ≤ TSB < +15**: score ∈ [80, 90] - fresh (race ready)
- **−5 ≤ TSB < +5**: score ∈ [60, 80] - optimal (productive training)
- **−10 ≤ TSB < −5**: score ∈ [40, 60] - fatigued (building fitness)
- **−15 ≤ TSB < −10**: score ∈ [20, 40] - very fatigued (overreaching risk)
- **TSB < −15**: score ∈ [0, 20] - overreaching (recovery needed)

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. *Human Kinetics*.

#### HRV scoring

heart rate variability assessment based on RMSSD deviation from baseline:

**RMSSD delta calculation**:

Δ_RMSSD = RMSSD_current − RMSSD_baseline

where:
- RMSSD = root mean square of successive RR interval differences (milliseconds)
- RMSSD_baseline = individual's baseline RMSSD (established over 7-14 days)

**piecewise linear HRV scoring function**:

                    ⎧ 90 + 10 × min(10, Δ − 5) / 10,      if Δ ≥ 5
                    ⎪
                    ⎪ 70 + 20 × Δ / 5,                    if 0 ≤ Δ < 5
                    ⎪
HRV_score(Δ) = ⎨ 50 + 20(Δ + 3) / 3,                 if −3 ≤ Δ < 0
                    ⎪
                    ⎪ 20 + 30(Δ + 10) / 7,                if −10 ≤ Δ < −3
                    ⎪
                    ⎩ max(0, 20(Δ + 20) / 10),            if Δ < −10

where Δ = Δ_RMSSD (milliseconds)

**physiological interpretation**:
- **Δ ≥ +5ms**: score ∈ [90, 100] - excellent recovery, parasympathetic dominance
- **0 ≤ Δ < +5ms**: score ∈ [70, 90] - good recovery, positive adaptation
- **−3 ≤ Δ < 0ms**: score ∈ [50, 70] - adequate recovery, stable state
- **−10 ≤ Δ < −3ms**: score ∈ [20, 50] - poor recovery, accumulated fatigue
- **Δ < −10ms**: score ∈ [0, 20] - very poor recovery, overreaching concern

**scientific basis**: HRV (specifically RMSSD) reflects autonomic nervous system recovery. decreases indicate accumulated fatigue, increases indicate good adaptation.

**reference**: Plews, D.J. et al. (2013). Training adaptation and heart rate variability in elite endurance athletes. *Int J Sports Physiol Perform*, 8(3), 286-293.

### configuration

all sleep/recovery thresholds configurable via environment variables:

**sleep duration thresholds** (hours):
- PIERRE_SLEEP_ADULT_MIN_HOURS = 7.0
- PIERRE_SLEEP_ATHLETE_OPTIMAL_HOURS = 8.0
- PIERRE_SLEEP_SHORT_THRESHOLD = 6.0
- PIERRE_SLEEP_VERY_SHORT_THRESHOLD = 5.0

**sleep stages thresholds** (percentage):
- PIERRE_SLEEP_DEEP_MIN_PERCENT = 15.0
- PIERRE_SLEEP_DEEP_OPTIMAL_PERCENT = 20.0
- PIERRE_SLEEP_REM_MIN_PERCENT = 20.0
- PIERRE_SLEEP_REM_OPTIMAL_PERCENT = 25.0

**sleep efficiency thresholds** (percentage):
- PIERRE_SLEEP_EFFICIENCY_EXCELLENT = 90.0
- PIERRE_SLEEP_EFFICIENCY_GOOD = 85.0
- PIERRE_SLEEP_EFFICIENCY_POOR = 70.0

**HRV thresholds** (milliseconds):
- PIERRE_HRV_RMSSD_DECREASE_CONCERN = −10.0
- PIERRE_HRV_RMSSD_INCREASE_GOOD = 5.0

**TSB thresholds**:
- PIERRE_TSB_HIGHLY_FATIGUED = −15.0
- PIERRE_TSB_FATIGUED = −10.0
- PIERRE_TSB_FRESH_MIN = 5.0
- PIERRE_TSB_FRESH_MAX = 15.0
- PIERRE_TSB_DETRAINING = 25.0

**recovery scoring weights**:
- PIERRE_RECOVERY_TSB_WEIGHT_FULL = 0.4
- PIERRE_RECOVERY_SLEEP_WEIGHT_FULL = 0.4
- PIERRE_RECOVERY_HRV_WEIGHT_FULL = 0.2
- PIERRE_RECOVERY_TSB_WEIGHT_NO_HRV = 0.5
- PIERRE_RECOVERY_SLEEP_WEIGHT_NO_HRV = 0.5

defaults based on peer-reviewed research (NSF, AASM, Shaffer & Ginsberg 2017).

---

## validation and safety

### parameter bounds (physiological ranges)

**physiological parameter ranges**:

max_hr ∈ [100, 220] bpm
resting_hr ∈ [30, 100] bpm
threshold_hr ∈ [100, 200] bpm
VO2max ∈ [20.0, 90.0] ml/kg/min
FTP ∈ [50, 600] watts

**range validation**: each parameter verified against physiologically plausible bounds

**relationship validation**:

resting_hr < threshold_hr < max_hr

validation constraints:
- HR_rest < HR_max (resting heart rate below maximum)
- HR_rest < HR_threshold (resting heart rate below threshold)
- HR_threshold < HR_max (threshold heart rate below maximum)

**references**:
- ACSM Guidelines for Exercise Testing and Prescription, 11th Edition
- European Society of Cardiology guidelines on exercise testing

### confidence levels

**confidence level classification**:

                          ⎧ High,       if (n ≥ 15) ∧ (R² ≥ 0.7)
                          ⎪
                          ⎪ Medium,     if (n ≥ 8) ∧ (R² ≥ 0.5)
                          ⎪
confidence(n, R²) = ⎨ Low,        if (n ≥ 3) ∧ (R² ≥ 0.3)
                          ⎪
                          ⎩ VeryLow,    otherwise

where:
- n = number of data points
- R² = coefficient of determination ∈ [0, 1]

### edge case handling

**1. users with no activities**:

If |activities| = 0, return:
- CTL = 0
- ATL = 0
- TSB = 0
- TSS_history = ∅ (empty set)

**2. training gaps (TSS sequence breaks)**:

For missing days: TSS_daily = 0

Exponential decay: EMAₜ = (1 − α) × EMAₜ₋₁

Result: CTL/ATL naturally decay during breaks (realistic fitness loss)

**3. invalid physiological parameters**:

Range validation checks:
- max_hr = 250 → rejected (exceeds upper bound 220)
- resting_hr = 120 → rejected (exceeds upper bound 100)

Relationship validation checks:
- max_hr = 150, resting_hr = 160 → rejected (violates HR_rest < HR_max)

Returns detailed error messages for each violation

**4. invalid race velocities**:

Velocity constraint: v ∈ [100, 500] m/min

If v ∉ [100, 500], reject with error message

**5. VDOT out of range**:

VDOT constraint: VDOT ∈ [30, 85]

If VDOT ∉ [30, 85], reject with error message

---

## configuration strategies

three strategies adjust training thresholds:

### conservative strategy

**parameters**:
- max_weekly_load_increase = 0.05 (5%)
- recovery_threshold = 1.2

**recommended for**: injury recovery, beginners, older athletes

### default strategy

**parameters**:
- max_weekly_load_increase = 0.10 (10%)
- recovery_threshold = 1.3

**recommended for**: general training, recreational athletes

### aggressive strategy

**parameters**:
- max_weekly_load_increase = 0.15 (15%)
- recovery_threshold = 1.5

**recommended for**: competitive athletes, experienced trainers

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
