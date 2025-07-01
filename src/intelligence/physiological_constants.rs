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