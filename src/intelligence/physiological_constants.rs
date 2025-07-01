//! Physiological constants based on sports science research
//!
//! This module contains scientifically-established constants used throughout
//! the intelligence analysis system. These values are based on peer-reviewed
//! research and guidelines from sports science organizations.

/// Heart rate zone thresholds based on exercise physiology
///
/// References:
/// - American College of Sports Medicine (ACSM) Guidelines for Exercise Testing and Prescription, 11th Edition
/// - https://www.acsm.org/education-resources/books/guidelines-exercise-testing-prescription
pub mod heart_rate {
    /// Anaerobic threshold as percentage of heart rate reserve
    /// Above this threshold, the body relies primarily on anaerobic metabolism
    /// Reference: ACSM Guidelines, Chapter 6: General Principles of Exercise Prescription
    pub const ANAEROBIC_THRESHOLD_PERCENTAGE: f32 = 85.0;

    /// Aerobic threshold as percentage of heart rate reserve  
    /// Above this threshold marks the transition from moderate to vigorous intensity
    /// Reference: ACSM Guidelines, Table 6.3: Classification of Exercise Intensity
    pub const AEROBIC_THRESHOLD_PERCENTAGE: f32 = 70.0;

    /// Heart rate zones for training (beats per minute)
    /// Based on Karvonen method and ACSM intensity classifications
    ///
    /// Recovery/Light intensity heart rate threshold
    /// Reference: Laursen, P.B. & Buchheit, M. (2019). Science and Application of High-Intensity Interval Training
    pub const RECOVERY_HR_THRESHOLD: u32 = 120;

    /// Moderate intensity heart rate threshold
    /// Reference: ACSM position stand on exercise intensity (2011)
    /// https://journals.lww.com/acsm-msse/Fulltext/2011/07000/Quantity_and_Quality_of_Exercise_for_Developing.26.aspx
    pub const MODERATE_HR_THRESHOLD: u32 = 140;

    /// High intensity heart rate threshold
    /// Reference: Seiler, S. (2010). What is best practice for training intensity distribution?
    /// https://www.ncbi.nlm.nih.gov/pmc/articles/PMC2914523/
    pub const HIGH_INTENSITY_HR_THRESHOLD: u32 = 160;

    /// Very high intensity heart rate threshold
    /// Reference: Billat, L.V. (2001). Interval training for performance
    pub const VERY_HIGH_INTENSITY_HR_THRESHOLD: u32 = 180;

    /// Maximum realistic heart rate (safety limit)
    /// Based on Fox formula upper bound with safety margin
    /// Reference: Tanaka, H., Monahan, K.D., & Seals, D.R. (2001). Age-predicted maximal heart rate revisited
    /// https://pubmed.ncbi.nlm.nih.gov/11153730/
    pub const MAX_REALISTIC_HEART_RATE: u32 = 220;
}

/// Power-to-weight ratio thresholds for cycling performance
///
/// References:
/// - Coggan, A. & Allen, H. (2010). Training and Racing with a Power Meter
/// - https://www.trainingpeaks.com/learn/articles/power-profiling/
pub mod power {
    /// Elite level power-to-weight ratio (W/kg)
    /// Professional/elite cyclists typically exceed this threshold
    pub const ELITE_POWER_TO_WEIGHT: f64 = 4.0;

    /// Competitive level power-to-weight ratio (W/kg)
    /// Cat 1-3 racers typically achieve this level
    pub const COMPETITIVE_POWER_TO_WEIGHT: f64 = 3.0;

    /// Recreational level power-to-weight ratio (W/kg)
    /// Trained recreational cyclists typically achieve this level
    pub const RECREATIONAL_POWER_TO_WEIGHT: f64 = 2.0;
}

/// Training load and recovery thresholds
///
/// References:
/// - Halson, S.L. (2014). Monitoring training load to understand fatigue in athletes
/// - https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4213373/
pub mod training_load {
    /// Weekly training load increase that triggers recovery need
    /// Based on acute:chronic workload ratio research
    /// Reference: Gabbett, T.J. (2016). The training-injury prevention paradox
    /// https://bjsm.bmj.com/content/50/5/273
    pub const RECOVERY_LOAD_MULTIPLIER: f64 = 1.3;

    /// Two-week combined load threshold for recovery
    /// Reference: Mujika, I. & Padilla, S. (2003). Scientific bases for precompetition tapering
    pub const TWO_WEEK_RECOVERY_THRESHOLD: f64 = 2.2;

    /// Training Stress Score thresholds
    /// Reference: Coggan, A. (2003). Training Stress Score (TSS) explained
    pub const HIGH_TSS_THRESHOLD: f64 = 150.0;
    pub const LOW_TSS_THRESHOLD: f64 = 50.0;
}

/// Duration thresholds for workout classification
///
/// References:
/// - ACSM Position Stand on Exercise Duration (2018)
/// - https://www.acsm.org/education-resources/trending-topics-resources/physical-activity-guidelines
pub mod duration {
    /// Minimum duration for aerobic benefits (seconds)
    /// Reference: ACSM Guidelines recommend minimum 30 minutes
    pub const MIN_AEROBIC_DURATION: u64 = 1800; // 30 minutes

    /// Duration threshold for endurance workouts (seconds)
    /// Reference: McArdle, W.D., Katch, F.I., & Katch, V.L. (2015). Exercise Physiology
    pub const ENDURANCE_DURATION_THRESHOLD: u64 = 3600; // 60 minutes

    /// Long workout duration threshold (seconds)
    /// Reference: Laursen, P.B. (2010). Training for intense exercise performance
    pub const LONG_WORKOUT_DURATION: u64 = 7200; // 2 hours
}

/// Performance improvement and adaptation thresholds
///
/// References:
/// - Hopkins, W.G. (2004). How to interpret changes in an athletic performance test
/// - https://www.sportsci.org/jour/04/wghtests.htm
pub mod performance {
    /// Meaningful pace improvement threshold (percentage)
    /// Based on smallest worthwhile change in endurance performance
    /// Reference: Hopkins, W.G. (2004). How to interpret changes
    pub const PACE_IMPROVEMENT_THRESHOLD: f64 = 5.0;

    /// Heart rate efficiency improvement threshold (percentage)
    /// Indicates cardiovascular adaptation
    /// Reference: Buchheit, M. & Laursen, P.B. (2013). High-intensity interval training
    pub const HR_EFFICIENCY_IMPROVEMENT_THRESHOLD: f64 = 3.0;

    /// Training consistency target (activities per week)
    /// Reference: ACSM Guidelines recommend 3-5 days/week for fitness
    pub const TARGET_WEEKLY_ACTIVITIES: f64 = 5.0;
}

/// Sport-specific maximum speed thresholds (m/s)
///
/// Used for anomaly detection and data validation
/// References:
/// - International Association of Athletics Federations (IAAF) world records
/// - UCI Hour Record data
pub mod max_speeds {
    /// Maximum realistic running speed (m/s)
    /// Based on world record 100m with buffer (~43 km/h)
    /// Reference: Usain Bolt 9.58s = 10.44 m/s average
    pub const MAX_RUNNING_SPEED: f64 = 12.0;

    /// Maximum realistic cycling speed (m/s)  
    /// Based on professional sprint speeds (~90 km/h)
    /// Reference: UCI track cycling sprint records
    pub const MAX_CYCLING_SPEED: f64 = 25.0;

    /// Maximum realistic swimming speed (m/s)
    /// Based on world record 50m freestyle (~11 km/h)
    /// Reference: FINA world records
    pub const MAX_SWIMMING_SPEED: f64 = 3.0;

    /// Default maximum speed for unknown sports (m/s)
    pub const DEFAULT_MAX_SPEED: f64 = 30.0;
}

/// Fitness score component weights
///
/// Based on multi-component fitness model
/// Reference: Bouchard, C. & Shephard, R.J. (1994). Physical activity, fitness, and health
pub mod fitness_weights {
    /// Aerobic component weight in overall fitness score
    pub const AEROBIC_WEIGHT: f64 = 0.4;

    /// Strength/power component weight in overall fitness score  
    pub const STRENGTH_WEIGHT: f64 = 0.3;

    /// Consistency component weight in overall fitness score
    pub const CONSISTENCY_WEIGHT: f64 = 0.3;
}

/// Training adaptation factors
///
/// Reference: Busso, T. (2003). Variable dose-response relationship
/// https://pubmed.ncbi.nlm.nih.gov/12627304/
pub mod adaptations {
    /// Performance improvement factor for high training volume (>20 sessions)
    pub const HIGH_VOLUME_IMPROVEMENT_FACTOR: f64 = 1.1; // 10% improvement

    /// Performance improvement factor for moderate training volume (>10 sessions)
    pub const MODERATE_VOLUME_IMPROVEMENT_FACTOR: f64 = 1.05; // 5% improvement

    /// No improvement factor for low training volume
    pub const LOW_VOLUME_IMPROVEMENT_FACTOR: f64 = 1.0;
}

/// Statistical analysis thresholds
pub mod statistics {
    /// Trend strength threshold for significance
    /// Based on correlation coefficient interpretation
    /// Reference: Cohen, J. (1988). Statistical Power Analysis
    pub const STRONG_TREND_THRESHOLD: f64 = 0.7;

    /// Change threshold for stable performance (percentage)
    /// Within this range, performance is considered stable
    pub const STABILITY_THRESHOLD: f64 = 0.05; // 5%
}

/// Aerobic efficiency thresholds
///
/// Reference: Lucia, A., Hoyos, J., & Chicharro, J.L. (2001). Physiological response to professional road cycling
/// https://pubmed.ncbi.nlm.nih.gov/11474337/
pub mod efficiency {
    /// Excellent aerobic efficiency threshold (speed/HR ratio)
    pub const EXCELLENT_AEROBIC_EFFICIENCY: f64 = 0.1;

    /// Good aerobic efficiency threshold (speed/HR ratio)
    pub const GOOD_AEROBIC_EFFICIENCY: f64 = 0.08;
}

/// Running pace thresholds
pub mod running {
    /// Fast running pace threshold (m/s)
    /// Equivalent to ~6:40 min/mile pace
    pub const FAST_PACE_THRESHOLD: f64 = 4.0;
}

/// Time periods for training analysis
///
/// Reference: Coggan, A. & Allen, H. (2010). Training and Racing with a Power Meter
pub mod time_periods {
    /// Standard training cycle analysis period (weeks)
    /// Reference: Periodization training theory
    pub const TRAINING_PATTERN_ANALYSIS_WEEKS: i64 = 4;

    /// Extended analysis period for goal suggestions (weeks)
    pub const GOAL_ANALYSIS_WEEKS: i64 = 8;

    /// Recovery analysis lookback period (days)
    pub const RECOVERY_ANALYSIS_DAYS: i64 = 7;

    /// Training gap warning threshold (days)
    /// Reference: Training adaptation research
    pub const SHORT_TRAINING_GAP_DAYS: i64 = 7;

    /// Long training gap warning threshold (days)
    pub const LONG_TRAINING_GAP_DAYS: i64 = 14;

    /// Maximum consecutive training days before rest recommended
    /// Reference: Overtraining syndrome prevention
    pub const MAX_CONSECUTIVE_TRAINING_DAYS: usize = 5;

    /// Goal adjustment evaluation threshold (percentage of timeline)
    pub const GOAL_ADJUSTMENT_THRESHOLD: f64 = 0.25; // 25%

    /// Days remaining threshold for goal adjustment strategy
    pub const GOAL_DAYS_REMAINING_THRESHOLD: f64 = 30.0;
}

/// Training intensity balance thresholds
///
/// Reference: Seiler, S. (2010). What is best practice for training intensity distribution?
pub mod intensity_balance {
    /// High intensity training upper limit (percentage of total)
    /// Reference: 80/20 training principle
    pub const HIGH_INTENSITY_UPPER_LIMIT: f64 = 0.6; // 60%

    /// Low intensity training lower limit (percentage of total)
    pub const LOW_INTENSITY_LOWER_LIMIT: f64 = 0.2; // 20%

    /// Moderate intensity nutrition threshold (HR)
    pub const MODERATE_NUTRITION_HR_THRESHOLD: u32 = 150;
}

/// Training volume thresholds
///
/// Reference: ACSM Guidelines for Exercise Testing and Prescription
pub mod volume_thresholds {
    /// Minimum weekly training volume (hours)
    pub const MIN_WEEKLY_VOLUME_HOURS: f64 = 3.0;

    /// High weekly training volume threshold (hours)
    pub const HIGH_WEEKLY_VOLUME_HOURS: f64 = 15.0;

    /// High weekly training load threshold (seconds)
    /// 5 hours = 18000 seconds
    pub const HIGH_WEEKLY_LOAD_SECONDS: u64 = 18000;

    /// Maximum high intensity sessions per week
    /// Reference: Polarized training model
    pub const MAX_HIGH_INTENSITY_SESSIONS_PER_WEEK: usize = 3;
}

/// Consistency and progress thresholds
pub mod consistency {
    /// Consistency score threshold for recommendations
    pub const CONSISTENCY_SCORE_THRESHOLD: f64 = 0.5;

    /// Progress tolerance for on-track assessment (percentage)
    pub const PROGRESS_TOLERANCE_PERCENTAGE: f64 = 10.0;

    /// Milestone achievement threshold (percentage)
    pub const MILESTONE_ACHIEVEMENT_THRESHOLD: f64 = 0.5; // 50%

    /// Minimum activity count for meaningful analysis
    pub const MIN_ACTIVITY_COUNT_FOR_ANALYSIS: usize = 3;
}

/// Goal difficulty assessment ratios
///
/// Reference: Goal-setting theory (Locke & Latham, 2002)
pub mod goal_difficulty {
    /// Easy goal improvement ratio (10% improvement)
    pub const EASY_GOAL_RATIO: f64 = 1.1;

    /// Moderate goal improvement ratio (30% improvement)
    pub const MODERATE_GOAL_RATIO: f64 = 1.3;

    /// Challenging goal improvement ratio (50% improvement)
    pub const CHALLENGING_GOAL_RATIO: f64 = 1.5;

    /// Goal distance tolerance for time goals (percentage)
    pub const GOAL_DISTANCE_TOLERANCE: f64 = 0.2; // 20%

    /// Goal distance precision tolerance (percentage)
    pub const GOAL_DISTANCE_PRECISION: f64 = 0.05; // 5%
}

/// Goal progress assessment thresholds
pub mod goal_progress {
    /// Significantly ahead of schedule threshold
    pub const AHEAD_OF_SCHEDULE_THRESHOLD: f64 = 1.3; // 30% ahead

    /// Behind schedule threshold
    pub const BEHIND_SCHEDULE_THRESHOLD: f64 = 0.7; // 30% behind

    /// Goal target increase multiplier for ambitious adjustment
    pub const TARGET_INCREASE_MULTIPLIER: f64 = 1.2; // 20% increase

    /// Goal target decrease multiplier for realistic adjustment
    pub const TARGET_DECREASE_MULTIPLIER: f64 = 0.8; // 20% decrease
}

/// Nutrition timing thresholds
///
/// Reference: Burke, L.M. & Hawley, J.A. (2018). Swifter, higher, stronger: What's on the menu?
pub mod nutrition {
    /// Pre-exercise nutrition duration threshold (hours)
    pub const PRE_EXERCISE_DURATION_THRESHOLD: f64 = 1.5;

    /// During-exercise nutrition duration threshold (hours)
    pub const DURING_EXERCISE_DURATION_THRESHOLD: f64 = 2.0;

    /// Post-exercise nutrition duration threshold (hours)
    pub const POST_EXERCISE_DURATION_THRESHOLD: f64 = 1.0;
}

/// Milestone structure constants
pub mod milestones {
    /// Standard milestone percentages for goal tracking
    pub const MILESTONE_PERCENTAGES: [f64; 4] = [25.0, 50.0, 75.0, 100.0];

    /// Milestone names corresponding to percentages
    pub const MILESTONE_NAMES: [&str; 4] = [
        "First Quarter",
        "Halfway Point",
        "Three Quarters",
        "Goal Complete",
    ];
}

/// Maximum heart rate estimation constants
///
/// Reference: Tanaka, H., Monahan, K.D., & Seals, D.R. (2001)
pub mod hr_estimation {
    /// Assumed maximum heart rate for calculations (age-independent baseline)
    /// Used when individual max HR is unknown
    pub const ASSUMED_MAX_HR: f64 = 180.0;

    /// Recovery heart rate percentage (70% of max HR)
    /// Used for intensity calculations
    pub const RECOVERY_HR_PERCENTAGE: f64 = 0.7;
}

/// Training frequency targets
///
/// Reference: Physical Activity Guidelines for Americans (2018)
pub mod frequency_targets {
    /// Maximum recommended weekly training frequency
    pub const MAX_WEEKLY_FREQUENCY: f64 = 5.0;

    /// Target performance improvement percentage for goals
    pub const TARGET_PERFORMANCE_IMPROVEMENT: f64 = 5.0;
}

/// Weather analysis thresholds
///
/// Reference: Environmental physiology and human performance research
pub mod weather_thresholds {
    /// Extreme cold temperature threshold (°C)
    pub const EXTREME_COLD_CELSIUS: f32 = -5.0;

    /// Cold temperature threshold (°C)
    pub const COLD_THRESHOLD_CELSIUS: f32 = 0.0;

    /// Hot temperature threshold (°C)
    pub const HOT_THRESHOLD_CELSIUS: f32 = 25.0;

    /// Extreme hot temperature threshold (°C)
    pub const EXTREME_HOT_THRESHOLD_CELSIUS: f32 = 30.0;

    /// Strong wind speed threshold (km/h)
    pub const STRONG_WIND_THRESHOLD: f32 = 30.0;

    /// Moderate wind speed threshold (km/h)
    pub const MODERATE_WIND_THRESHOLD: f32 = 15.0;

    /// High humidity threshold (percentage)
    pub const HIGH_HUMIDITY_THRESHOLD: f32 = 80.0;

    /// Temperature threshold for humidity impact (°C)
    pub const HUMIDITY_IMPACT_TEMP_THRESHOLD: f32 = 20.0;
}

/// Training zone percentages for heart rate and power
///
/// Reference: Exercise physiology training zone models
pub mod zone_percentages {
    /// Heart rate training zones (percentage of LTHR)
    ///
    /// Zone 1 upper limit (80% of LTHR)
    pub const HR_ZONE1_UPPER_LIMIT: f64 = 0.80;

    /// Zone 2 upper limit (90% of LTHR)
    pub const HR_ZONE2_UPPER_LIMIT: f64 = 0.90;

    /// Zone 3 upper limit (100% of LTHR)
    pub const HR_ZONE3_UPPER_LIMIT: f64 = 1.00;

    /// Zone 4 upper limit (110% of LTHR)
    pub const HR_ZONE4_UPPER_LIMIT: f64 = 1.10;

    /// Power training zones (percentage of FTP)
    ///
    /// Power Zone 1 upper limit (55% of FTP)
    pub const POWER_ZONE1_UPPER_LIMIT: f64 = 0.55;

    /// Power Zone 2 upper limit (75% of FTP)
    pub const POWER_ZONE2_UPPER_LIMIT: f64 = 0.75;

    /// Power Zone 3 upper limit (90% of FTP)
    pub const POWER_ZONE3_UPPER_LIMIT: f64 = 0.90;

    /// Power Zone 4 upper limit (105% of FTP)
    pub const POWER_ZONE4_UPPER_LIMIT: f64 = 1.05;
}

/// Metrics calculation constants
pub mod metrics_constants {
    /// TRIMP calculation exponential factor
    /// Reference: Banister, E.W. (1991). Modeling elite athletic performance
    pub const TRIMP_EXPONENTIAL_FACTOR: f64 = 1.92;

    /// TRIMP calculation base multiplier
    pub const TRIMP_BASE_MULTIPLIER: f64 = 0.64;

    /// TSS calculation base multiplier
    /// Reference: Coggan, A. (2003). Training Stress Score
    pub const TSS_BASE_MULTIPLIER: f64 = 100.0;

    /// Minimum data points for decoupling analysis
    pub const MIN_DECOUPLING_DATA_POINTS: usize = 20;

    /// Efficiency factor time multiplier (minutes per hour)
    pub const EFFICIENCY_TIME_MULTIPLIER: f64 = 60.0;
}

/// Fitness score thresholds
///
/// Reference: General fitness assessment standards
pub mod fitness_score_thresholds {
    /// Fitness score threshold for "improving" trend
    pub const FITNESS_IMPROVING_THRESHOLD: f64 = 70.0;

    /// Fitness score threshold for "stable" trend
    pub const FITNESS_STABLE_THRESHOLD: f64 = 40.0;

    /// Statistical significance minimum data points
    pub const MIN_STATISTICAL_SIGNIFICANCE_POINTS: usize = 10;

    /// Statistical significance trend strength threshold
    pub const STATISTICAL_SIGNIFICANCE_THRESHOLD: f64 = 0.5;

    /// Statistical significance reduction factor for small datasets
    pub const SMALL_DATASET_REDUCTION_FACTOR: f64 = 0.7;

    /// Fitness calculation divisor for strength endurance
    pub const STRENGTH_ENDURANCE_DIVISOR: f64 = 5.0;
}
