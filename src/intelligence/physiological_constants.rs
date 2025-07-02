//! Physiological constants based on sports science research
//!
//! This module contains scientifically-established constants used throughout
//! the intelligence analysis system. These values are based on peer-reviewed
//! research and guidelines from sports science organizations.

/// Heart rate zone thresholds based on exercise physiology
///
/// References:
/// - American College of Sports Medicine (ACSM) Guidelines for Exercise Testing and Prescription, 11th Edition
/// - <https://www.acsm.org/education-resources/books/guidelines-exercise-testing-prescription>
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
    /// <https://journals.lww.com/acsm-msse/Fulltext/2011/07000/Quantity_and_Quality_of_Exercise_for_Developing.26.aspx>
    pub const MODERATE_HR_THRESHOLD: u32 = 140;

    /// High intensity heart rate threshold
    /// Reference: Seiler, S. (2010). What is best practice for training intensity distribution?
    /// <https://www.ncbi.nlm.nih.gov/pmc/articles/PMC2914523/>
    pub const HIGH_INTENSITY_HR_THRESHOLD: u32 = 160;

    /// Very high intensity heart rate threshold
    /// Reference: Billat, L.V. (2001). Interval training for performance
    pub const VERY_HIGH_INTENSITY_HR_THRESHOLD: u32 = 180;

    /// Maximum realistic heart rate (safety limit)
    /// Based on Fox formula upper bound with safety margin
    /// Reference: Tanaka, H., Monahan, K.D., & Seals, D.R. (2001). Age-predicted maximal heart rate revisited
    /// <https://pubmed.ncbi.nlm.nih.gov/11153730/>
    pub const MAX_REALISTIC_HEART_RATE: u32 = 220;
}

/// Power-to-weight ratio thresholds for cycling performance
///
/// References:
/// - Coggan, A. & Allen, H. (2010). Training and Racing with a Power Meter
/// - <https://www.trainingpeaks.com/learn/articles/power-profiling/>
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
/// - <https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4213373/>
pub mod training_load {
    /// Weekly training load increase that triggers recovery need
    /// Based on acute:chronic workload ratio research
    /// Reference: Gabbett, T.J. (2016). The training-injury prevention paradox
    /// <https://bjsm.bmj.com/content/50/5/273>
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
/// - <https://www.acsm.org/education-resources/trending-topics-resources/physical-activity-guidelines>
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
/// - <https://www.sportsci.org/jour/04/wghtests.htm>
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
/// <https://pubmed.ncbi.nlm.nih.gov/12627304/>
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
/// <https://pubmed.ncbi.nlm.nih.gov/11474337/>
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

    /// Fitness score classification thresholds
    /// Reference: General fitness assessment standards
    ///
    /// Excellent fitness level threshold (80%+)
    pub const EXCELLENT_FITNESS_THRESHOLD: f64 = 80.0;

    /// Good fitness level threshold (60%+)
    pub const GOOD_FITNESS_THRESHOLD: f64 = 60.0;

    /// Moderate fitness level threshold (40%+)
    pub const MODERATE_FITNESS_THRESHOLD: f64 = 40.0;

    /// Beginner fitness level threshold (20%+)
    pub const BEGINNER_FITNESS_THRESHOLD: f64 = 20.0;

    /// Performance classification thresholds
    ///
    /// Excellent performance threshold (90%+)
    pub const EXCELLENT_PERFORMANCE_THRESHOLD: f64 = 0.90;

    /// Good performance threshold (70%+)
    pub const GOOD_PERFORMANCE_THRESHOLD: f64 = 0.70;

    /// Moderate performance threshold (40%+)
    pub const MODERATE_PERFORMANCE_THRESHOLD: f64 = 0.40;
}

/// Efficiency calculation defaults and factors
///
/// Reference: Exercise physiology efficiency metrics
pub mod efficiency_defaults {
    /// Default efficiency score when no data available
    pub const DEFAULT_EFFICIENCY_SCORE: f64 = 50.0;

    /// Default efficiency when distance available but no elevation
    pub const DEFAULT_EFFICIENCY_WITH_DISTANCE: f64 = 75.0;

    /// Base efficiency score for activity scoring
    pub const BASE_EFFICIENCY_SCORE: f32 = 50.0;

    /// Pace per kilometer conversion factor
    pub const PACE_PER_KM_FACTOR: f32 = 1000.0;

    /// Heart rate efficiency calculation factor
    pub const HR_EFFICIENCY_FACTOR: f32 = 1000.0;
}

/// Activity scoring system constants
///
/// Reference: Activity assessment and scoring methodology
pub mod activity_scoring {
    /// Base score for completing an activity
    pub const BASE_ACTIVITY_SCORE: f64 = 5.0;

    /// Completion bonus points
    pub const COMPLETION_BONUS: f64 = 1.0;

    /// Standard bonus increment
    pub const STANDARD_BONUS: f64 = 0.5;

    /// Heart rate zone bonus
    pub const HR_ZONE_BONUS: f64 = 1.0;

    /// Duration bonus increment
    pub const DURATION_BONUS: f64 = 0.5;

    /// Distance achievement bonus
    pub const DISTANCE_BONUS: f64 = 1.0;

    /// Intensity bonus increment
    pub const INTENSITY_BONUS: f64 = 0.5;
}

/// Weather impact factors for training difficulty
///
/// Reference: Environmental physiology and exercise performance
pub mod weather_impact_factors {
    /// Extreme cold difficulty modifier
    pub const EXTREME_COLD_DIFFICULTY: f64 = 3.0;

    /// Cold temperature difficulty modifier
    pub const COLD_DIFFICULTY: f64 = 2.0;

    /// Strong wind difficulty modifier
    pub const STRONG_WIND_DIFFICULTY: f64 = 2.0;

    /// Extreme heat difficulty modifier
    pub const EXTREME_HOT_DIFFICULTY: f64 = 2.5;

    /// Warm temperature difficulty modifier
    pub const WARM_DIFFICULTY: f64 = 1.0;

    /// Moderate wind difficulty modifier
    pub const MODERATE_WIND_DIFFICULTY: f64 = 1.0;

    /// Rain difficulty modifier
    pub const RAIN_DIFFICULTY: f64 = 1.5;

    /// Snow difficulty modifier
    pub const SNOW_DIFFICULTY: f64 = 2.5;

    /// High humidity difficulty modifier
    pub const HIGH_HUMIDITY_DIFFICULTY: f64 = 1.5;
}

/// API limits and fetch constraints
///
/// Reference: System performance and resource management
pub mod api_limits {
    /// Default activity fetch limit for analysis
    pub const DEFAULT_ACTIVITY_LIMIT: usize = 100;

    /// Small activity fetch limit for quick analysis
    pub const SMALL_ACTIVITY_LIMIT: usize = 50;

    /// Large activity fetch limit for comprehensive analysis
    pub const LARGE_ACTIVITY_LIMIT: usize = 200;

    /// Maximum activity fetch limit
    pub const MAX_ACTIVITY_LIMIT: usize = 300;

    /// Activities for goal analysis
    pub const GOAL_ANALYSIS_ACTIVITY_LIMIT: usize = 100;
}

/// Demo and mock data constants for development
pub mod demo_data {
    /// Previous best time for demo purposes (minutes)
    pub const DEMO_PREVIOUS_BEST_TIME: f64 = 18.5;

    /// Previous best pace for demo purposes (seconds per km)
    pub const DEMO_PREVIOUS_BEST_PACE: f64 = 320.0;

    /// Demo goal distance for examples (km)
    pub const DEMO_GOAL_DISTANCE: f64 = 1000.0;
}

/// Goal progress and feasibility thresholds
pub mod goal_feasibility {
    /// Simple heuristic threshold for goal progress (50%)
    pub const SIMPLE_PROGRESS_THRESHOLD: f64 = 50.0;

    /// High feasibility threshold (75%)
    pub const HIGH_FEASIBILITY_THRESHOLD: f64 = 75.0;

    /// Moderate feasibility threshold (50%)
    pub const MODERATE_FEASIBILITY_THRESHOLD: f64 = 50.0;
}

/// Performance calculation factors for effort and intensity analysis
pub mod performance_calculation {
    /// Hour conversion factor for duration-based effort calculation
    /// Used to normalize activity duration to hourly basis for scoring
    pub const EFFORT_HOUR_FACTOR: f32 = 1.5;

    /// Heart rate intensity multiplier for effort calculation
    /// Higher values increase the impact of HR intensity on effort scores
    pub const HR_INTENSITY_EFFORT_FACTOR: f32 = 4.0;

    /// Distance divisor for running activities in effort calculation
    /// Lower values make distance contribute more to effort score
    pub const RUN_DISTANCE_DIVISOR: f32 = 10.0;

    /// Effort multiplier for running activities
    /// Adjusts the relative effort contribution for running vs other sports
    pub const RUN_EFFORT_MULTIPLIER: f32 = 0.8;

    /// Distance divisor for cycling activities in effort calculation
    /// Accounts for cycling being generally less effort per km than running
    pub const BIKE_DISTANCE_DIVISOR: f32 = 50.0;

    /// Effort multiplier for cycling activities
    /// Adjusts the relative effort contribution for cycling
    pub const BIKE_EFFORT_MULTIPLIER: f32 = 0.6;

    /// Distance divisor for swimming activities in effort calculation
    /// Swimming typically covers less distance but requires high effort
    pub const SWIM_DISTANCE_DIVISOR: f32 = 20.0;

    /// Effort multiplier for swimming activities
    /// Adjusts the relative effort contribution for swimming
    pub const SWIM_EFFORT_MULTIPLIER: f32 = 0.5;

    /// Elevation gain divisor for effort calculation (meters)
    /// Used to convert elevation gain to effort contribution
    pub const ELEVATION_EFFORT_DIVISOR: f32 = 100.0;

    /// Elevation gain multiplier for effort calculation
    /// Adjusts how much elevation gain contributes to overall effort
    pub const ELEVATION_EFFORT_FACTOR: f32 = 0.3;

    /// Minimum effort score boundary
    /// Prevents effort scores from going below this value
    pub const MIN_EFFORT_SCORE: f32 = 1.0;

    /// Maximum effort score boundary
    /// Caps effort scores at this maximum value
    pub const MAX_EFFORT_SCORE: f32 = 10.0;

    /// Assumed resting heart rate for calculations when not available
    /// Used as fallback when user's resting HR is unknown
    pub const ASSUMED_RESTING_HR: u32 = 60;
}

/// Personal record detection thresholds
pub mod personal_records {
    /// Distance threshold for considering a distance PR (kilometers)
    /// Activities must be at least this distance to qualify for distance PRs
    pub const DISTANCE_PR_THRESHOLD_KM: f64 = 20.0;

    /// Pace improvement threshold for considering a pace PR (seconds)
    /// Pace must improve by at least this amount to qualify as a PR
    pub const PACE_PR_THRESHOLD_SECONDS: f64 = 300.0;
}

/// Business logic thresholds for fitness analysis
pub mod business_thresholds {
    /// Official marathon distance in kilometers
    /// Used for marathon-specific analysis and predictions
    pub const MARATHON_DISTANCE_KM: f64 = 42.195;

    /// Pace threshold for categorizing as "slow" (minutes per kilometer)
    /// Paces slower than this are considered recreational/recovery
    pub const SLOW_PACE_THRESHOLD_MIN_PER_KM: f64 = 7.0;

    /// Fatigue exponent for endurance calculations
    /// Used in exponential fatigue models for long-distance events
    pub const FATIGUE_EXPONENT: f64 = 0.06;

    /// Default heart rate effort score when HR data is unavailable
    /// Fallback scoring value for activities without heart rate data
    pub const DEFAULT_HR_EFFORT_SCORE: f32 = 5.0;

    /// Distance score normalization divisor
    /// Used to normalize distance into scoring range
    pub const DISTANCE_SCORE_DIVISOR: f32 = 100.0;

    /// Maximum distance contribution to fitness score
    /// Caps the distance component of fitness scoring
    pub const MAX_DISTANCE_SCORE: f32 = 30.0;

    /// Duration score multiplication factor
    /// Adjusts how much activity duration contributes to scores
    pub const DURATION_SCORE_FACTOR: f32 = 4.0;

    /// Minimum valid distance for analysis (kilometers)
    /// Activities below this distance may be excluded from analysis
    pub const MIN_VALID_DISTANCE: f32 = 0.0;

    /// Maximum score value (percentage)
    /// Upper bound for all percentage-based scores
    pub const MAX_SCORE: f32 = 100.0;

    /// Minimum score value (percentage)
    /// Lower bound for all percentage-based scores
    pub const MIN_SCORE: f32 = 0.0;

    /// Effort score multiplier for heart rate calculations
    /// Used to scale heart rate intensity into effort scoring range
    pub const EFFORT_SCORE_MULTIPLIER: f32 = 10.0;

    /// Pace scoring base divisor for fitness calculations
    /// Used in pace-based fitness score calculations
    pub const PACE_SCORING_BASE: f64 = 10.0;

    /// Pace scoring multiplier for fitness calculations
    /// Applied after pace division for final scoring
    pub const PACE_SCORING_MULTIPLIER: f64 = 10.0;

    /// Maximum pace score contribution to fitness calculations
    /// Caps the pace component of fitness scoring
    pub const MAX_PACE_SCORE: f64 = 40.0;

    /// Confidence calculation base divisor
    /// Used to calculate confidence from data availability
    pub const CONFIDENCE_BASE_DIVISOR: f64 = 20.0;

    /// Maximum confidence percentage before percentage conversion
    /// Applied before converting to 0-100 percentage
    pub const MAX_CONFIDENCE_RATIO: f64 = 0.95;

    /// Distance threshold for achievement insights (kilometers)
    /// Activities above this distance generate achievement insights
    pub const ACHIEVEMENT_DISTANCE_THRESHOLD_KM: f64 = 10.0;
}

/// Unit conversion constants for various measurements
pub mod unit_conversions {
    /// Conversion factor from meters per second to kilometers per hour
    /// Standard physics conversion: m/s * 3.6 = km/h
    pub const MS_TO_KMH_FACTOR: f64 = 3.6;

    /// Conversion factor from meters per second to kilometers per hour (f32)
    /// For calculations requiring f32 precision
    pub const MS_TO_KMH_FACTOR_F32: f32 = 3.6;
}

/// Heart rate zone distribution defaults for different intensity levels
/// These percentages represent typical time spent in each zone based on average HR intensity
pub mod zone_distributions {
    /// Low intensity training (< 50% HR reserve) - primarily recovery
    pub mod low_intensity {
        /// Percentage time in Zone 1 (Recovery) for low intensity activities
        pub const ZONE1_RECOVERY: f32 = 80.0;
        /// Percentage time in Zone 2 (Endurance) for low intensity activities
        pub const ZONE2_ENDURANCE: f32 = 20.0;
        /// Percentage time in Zone 3 (Tempo) for low intensity activities
        pub const ZONE3_TEMPO: f32 = 0.0;
        /// Percentage time in Zone 4 (Threshold) for low intensity activities
        pub const ZONE4_THRESHOLD: f32 = 0.0;
        /// Percentage time in Zone 5 (VO2 Max) for low intensity activities
        pub const ZONE5_VO2MAX: f32 = 0.0;
    }

    /// Moderate low intensity training (50-60% HR reserve) - aerobic base building
    pub mod moderate_low_intensity {
        /// Percentage time in Zone 1 (Recovery) for moderate low intensity
        pub const ZONE1_RECOVERY: f32 = 20.0;
        /// Percentage time in Zone 2 (Endurance) for moderate low intensity
        pub const ZONE2_ENDURANCE: f32 = 70.0;
        /// Percentage time in Zone 3 (Tempo) for moderate low intensity
        pub const ZONE3_TEMPO: f32 = 10.0;
        /// Percentage time in Zone 4 (Threshold) for moderate low intensity
        pub const ZONE4_THRESHOLD: f32 = 0.0;
        /// Percentage time in Zone 5 (VO2 Max) for moderate low intensity
        pub const ZONE5_VO2MAX: f32 = 0.0;
    }

    /// Moderate intensity training (60-70% HR reserve) - tempo work
    pub mod moderate_intensity {
        /// Percentage time in Zone 1 (Recovery) for moderate intensity
        pub const ZONE1_RECOVERY: f32 = 10.0;
        /// Percentage time in Zone 2 (Endurance) for moderate intensity
        pub const ZONE2_ENDURANCE: f32 = 40.0;
        /// Percentage time in Zone 3 (Tempo) for moderate intensity
        pub const ZONE3_TEMPO: f32 = 45.0;
        /// Percentage time in Zone 4 (Threshold) for moderate intensity
        pub const ZONE4_THRESHOLD: f32 = 5.0;
        /// Percentage time in Zone 5 (VO2 Max) for moderate intensity
        pub const ZONE5_VO2MAX: f32 = 0.0;
    }

    /// High intensity training (70-85% HR reserve) - threshold work
    pub mod high_intensity {
        /// Percentage time in Zone 1 (Recovery) for high intensity
        pub const ZONE1_RECOVERY: f32 = 5.0;
        /// Percentage time in Zone 2 (Endurance) for high intensity
        pub const ZONE2_ENDURANCE: f32 = 20.0;
        /// Percentage time in Zone 3 (Tempo) for high intensity
        pub const ZONE3_TEMPO: f32 = 30.0;
        /// Percentage time in Zone 4 (Threshold) for high intensity
        pub const ZONE4_THRESHOLD: f32 = 40.0;
        /// Percentage time in Zone 5 (VO2 Max) for high intensity
        pub const ZONE5_VO2MAX: f32 = 5.0;
    }

    /// Very high intensity training (>85% HR reserve) - VO2 max work
    pub mod very_high_intensity {
        /// Percentage time in Zone 1 (Recovery) for very high intensity
        pub const ZONE1_RECOVERY: f32 = 0.0;
        /// Percentage time in Zone 2 (Endurance) for very high intensity
        pub const ZONE2_ENDURANCE: f32 = 10.0;
        /// Percentage time in Zone 3 (Tempo) for very high intensity
        pub const ZONE3_TEMPO: f32 = 20.0;
        /// Percentage time in Zone 4 (Threshold) for very high intensity
        pub const ZONE4_THRESHOLD: f32 = 40.0;
        /// Percentage time in Zone 5 (VO2 Max) for very high intensity
        pub const ZONE5_VO2MAX: f32 = 30.0;
    }

    /// HR intensity thresholds for zone distribution selection
    pub mod intensity_thresholds {
        /// Threshold between low and moderate-low intensity (HR reserve percentage)
        pub const LOW_TO_MODERATE_LOW: f32 = 0.5;
        /// Threshold between moderate-low and moderate intensity (HR reserve percentage)  
        pub const MODERATE_LOW_TO_MODERATE: f32 = 0.6;
        /// Threshold between moderate and high intensity (HR reserve percentage)
        pub const MODERATE_TO_HIGH: f32 = 0.7;
        /// Threshold between high and very high intensity (HR reserve percentage)
        pub const HIGH_TO_VERY_HIGH: f32 = 0.85;
    }

    /// Zone analysis thresholds for determining training effectiveness
    pub mod zone_analysis_thresholds {
        /// Threshold for significant endurance zone time (percentage)
        /// Activities with more than this percentage in Zone 2 are considered endurance-focused
        pub const SIGNIFICANT_ENDURANCE_ZONE_THRESHOLD: f32 = 50.0;

        /// Effort rating threshold for "hard intensity" classification (1-10 scale)
        /// Effort ratings below this are considered moderate to hard intensity
        pub const HARD_INTENSITY_EFFORT_THRESHOLD: f32 = 7.0;

        /// Demo consistency score for testing and examples
        pub const DEMO_CONSISTENCY_SCORE: f32 = 85.0;
    }
}
