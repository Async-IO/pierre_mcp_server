// ABOUTME: Intelligence module configuration for AI-powered fitness analysis and recommendations
// ABOUTME: Configures analysis algorithms, recommendation engines, and intelligence processing parameters
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Intelligence Configuration Module
//!
//! Provides type-safe, compile-time validated configuration for all intelligence modules
//! including recommendation engine, performance analyzer, goal engine, and weather analysis.

use crate::constants::limits;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::OnceLock;
use thiserror::Error;

/// Configuration error types
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid range: {0}")]
    InvalidRange(&'static str),

    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid weights: {0}")]
    InvalidWeights(&'static str),

    #[error("Value out of range: {0}")]
    ValueOutOfRange(&'static str),
}

/// Algorithm Selection Configuration
///
/// Configures which algorithm implementation to use for various fitness calculations.
/// Each algorithm type uses enum dispatch for type-safe selection with minimal runtime overhead.
///
/// # Algorithm Types
///
/// - **TSS**: Training Stress Score calculation (`avg_power`, `normalized_power`, `hybrid`)
/// - **`MaxHR`**: Maximum heart rate estimation (`fox`, `tanaka`, `nes`, `gulati`)
///
/// # Configuration Methods
///
/// 1. Environment variables (highest priority):
///    ```bash
///    export PIERRE_TSS_ALGORITHM=normalized_power
///    export PIERRE_MAXHR_ALGORITHM=tanaka
///    ```
///
/// 2. Default values (if env vars not set)
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::config::intelligence_config::AlgorithmConfig;
///
/// let config = AlgorithmConfig::default();
/// assert_eq!(config.tss, "avg_power");
/// assert_eq!(config.maxhr, "tanaka");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// TSS calculation algorithm: `avg_power`, `normalized_power`, or `hybrid`
    #[serde(default = "default_tss_algorithm")]
    pub tss: String,

    /// Max HR estimation algorithm: `fox`, `tanaka`, `nes`, or `gulati`
    #[serde(default = "default_maxhr_algorithm")]
    pub maxhr: String,

    /// FTP estimation algorithm: `20min_test`, `from_vo2max`, `ramp_test`, etc.
    #[serde(default = "default_ftp_algorithm")]
    pub ftp: String,

    /// LTHR estimation algorithm: `from_maxhr`, `from_30min`, etc.
    #[serde(default = "default_lthr_algorithm")]
    pub lthr: String,

    /// `VO2max` estimation algorithm: `from_vdot`, `cooper_test`, etc.
    #[serde(default = "default_vo2max_algorithm")]
    pub vo2max: String,
}

/// Default TSS algorithm (`avg_power` for backwards compatibility)
fn default_tss_algorithm() -> String {
    "avg_power".to_string()
}

/// Default Max HR algorithm (tanaka as most accurate)
fn default_maxhr_algorithm() -> String {
    "tanaka".to_string()
}

/// Default FTP algorithm (`from_vo2max` as most accessible)
fn default_ftp_algorithm() -> String {
    "from_vo2max".to_string()
}

/// Default LTHR algorithm (`from_maxhr` as most common)
fn default_lthr_algorithm() -> String {
    "from_maxhr".to_string()
}

/// Default `VO2max` algorithm (`from_vdot` as most validated)
fn default_vo2max_algorithm() -> String {
    "from_vdot".to_string()
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            tss: default_tss_algorithm(),
            maxhr: default_maxhr_algorithm(),
            ftp: default_ftp_algorithm(),
            lthr: default_lthr_algorithm(),
            vo2max: default_vo2max_algorithm(),
        }
    }
}

/// Main intelligence configuration container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig<const VALIDATED: bool = false> {
    pub recommendation_engine: RecommendationEngineConfig,
    pub performance_analyzer: PerformanceAnalyzerConfig,
    pub goal_engine: GoalEngineConfig,
    pub weather_analysis: WeatherAnalysisConfig,
    pub activity_analyzer: ActivityAnalyzerConfig,
    pub metrics: MetricsConfig,
    pub sleep_recovery: SleepRecoveryConfig,
    pub nutrition: NutritionConfig,
    pub algorithms: AlgorithmConfig,
    _phantom: PhantomData<()>,
}

/// Recommendation Engine Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationEngineConfig {
    pub thresholds: RecommendationThresholds,
    pub weights: RecommendationWeights,
    pub limits: RecommendationLimits,
    pub messages: RecommendationMessages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationThresholds {
    pub low_weekly_distance_km: f64,
    pub high_weekly_distance_km: f64,
    pub low_weekly_frequency: i32,
    pub high_weekly_frequency: i32,
    pub pace_improvement_threshold: f64,
    pub consistency_threshold: f64,
    pub rest_day_threshold: i32,
    pub volume_increase_threshold: f64,
    pub intensity_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationWeights {
    pub distance_weight: f64,
    pub frequency_weight: f64,
    pub pace_weight: f64,
    pub consistency_weight: f64,
    pub recovery_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationLimits {
    pub max_recommendations_per_category: usize,
    pub max_total_recommendations: usize,
    pub min_confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationMessages {
    pub low_distance: String,
    pub high_distance: String,
    pub low_frequency: String,
    pub high_frequency: String,
    pub pace_improvement: String,
    pub consistency_improvement: String,
    pub recovery_needed: String,
}

/// Performance Analyzer Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalyzerConfig {
    pub trend_analysis: TrendAnalysisConfig,
    pub statistical: StatisticalConfig,
    pub thresholds: PerformanceThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysisConfig {
    pub min_data_points: usize,
    pub trend_strength_threshold: f64,
    pub significance_threshold: f64,
    pub improvement_threshold: f64,
    pub decline_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalConfig {
    pub confidence_level: f64,
    pub outlier_threshold: f64,
    pub smoothing_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub significant_improvement: f64,
    pub significant_decline: f64,
    pub pace_variance_threshold: f64,
    pub endurance_threshold: f64,
}

/// Goal Engine Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalEngineConfig {
    pub feasibility: FeasibilityConfig,
    pub suggestion: SuggestionConfig,
    pub progression: ProgressionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityConfig {
    pub min_success_probability: f64,
    pub conservative_multiplier: f64,
    pub aggressive_multiplier: f64,
    pub injury_risk_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionConfig {
    pub max_goals_per_type: usize,
    pub difficulty_distribution: DifficultyDistribution,
    pub timeframe_preferences: TimeframePreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyDistribution {
    pub easy_percentage: f64,
    pub moderate_percentage: f64,
    pub hard_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframePreferences {
    pub short_term_weeks: u32,
    pub medium_term_weeks: u32,
    pub long_term_weeks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressionConfig {
    pub weekly_increase_limit: f64,
    pub monthly_increase_limit: f64,
    pub deload_frequency_weeks: u32,
}

/// Weather Analysis Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherAnalysisConfig {
    pub temperature: TemperatureConfig,
    pub conditions: WeatherConditionsConfig,
    pub impact: WeatherImpactConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureConfig {
    pub ideal_min_celsius: f32,
    pub ideal_max_celsius: f32,
    pub cold_threshold_celsius: f32,
    pub hot_threshold_celsius: f32,
    pub extreme_cold_celsius: f32,
    pub extreme_hot_celsius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditionsConfig {
    pub high_humidity_threshold: f64,
    pub strong_wind_threshold: f64,
    pub precipitation_impact_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherImpactConfig {
    pub temperature_impact_weight: f64,
    pub humidity_impact_weight: f64,
    pub wind_impact_weight: f64,
    pub precipitation_impact_weight: f64,
}

/// Activity Analyzer Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityAnalyzerConfig {
    pub analysis: ActivityAnalysisConfig,
    pub scoring: ActivityScoringConfig,
    pub insights: ActivityInsightsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityAnalysisConfig {
    pub min_duration_seconds: u64,
    pub max_reasonable_pace: f64,
    pub heart_rate_zones: HeartRateZonesConfig,
    pub power_zones: PowerZonesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateZonesConfig {
    pub zone1_max_percentage: f32,
    pub zone2_max_percentage: f32,
    pub zone3_max_percentage: f32,
    pub zone4_max_percentage: f32,
    pub zone5_max_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerZonesConfig {
    pub zone1_max_percentage: f32,
    pub zone2_max_percentage: f32,
    pub zone3_max_percentage: f32,
    pub zone4_max_percentage: f32,
    pub zone5_max_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityScoringConfig {
    pub efficiency_weight: f64,
    pub intensity_weight: f64,
    pub duration_weight: f64,
    pub consistency_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInsightsConfig {
    pub min_confidence_threshold: f64,
    pub max_insights_per_activity: usize,
    pub severity_thresholds: SeverityThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityThresholds {
    pub info_threshold: f64,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
}

/// Metrics Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub calculation: MetricsCalculationConfig,
    pub validation: MetricsValidationConfig,
    pub aggregation: MetricsAggregationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCalculationConfig {
    pub smoothing_window_size: usize,
    pub outlier_detection_threshold: f64,
    pub missing_data_interpolation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsValidationConfig {
    pub max_heart_rate: u32,
    pub min_heart_rate: u32,
    pub max_pace_min_per_km: f64,
    pub min_pace_min_per_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAggregationConfig {
    pub weekly_aggregation_method: String,
    pub monthly_aggregation_method: String,
    pub trend_calculation_method: String,
}

/// Sleep and Recovery Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepRecoveryConfig {
    pub sleep_duration: SleepDurationConfig,
    pub sleep_stages: SleepStagesConfig,
    pub sleep_efficiency: SleepEfficiencyConfig,
    pub hrv: HrvConfig,
    pub training_stress_balance: TsbConfig,
    pub recovery_scoring: RecoveryScoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepDurationConfig {
    /// Minimum optimal sleep duration for adults (hours)
    pub adult_min_hours: f64,
    /// Maximum optimal sleep duration for adults (hours)
    pub adult_max_hours: f64,
    /// Optimal sleep duration for athletes (hours)
    pub athlete_optimal_hours: f64,
    /// Minimum optimal sleep for athletes (hours)
    pub athlete_min_hours: f64,
    /// Short sleep threshold (hours)
    pub short_sleep_threshold: f64,
    /// Very short sleep threshold (hours)
    pub very_short_sleep_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepStagesConfig {
    /// Minimum healthy deep sleep percentage
    pub deep_sleep_min_percent: f64,
    /// Optimal deep sleep percentage
    pub deep_sleep_optimal_percent: f64,
    /// Maximum healthy deep sleep percentage
    pub deep_sleep_max_percent: f64,
    /// Minimum healthy REM sleep percentage
    pub rem_sleep_min_percent: f64,
    /// Optimal REM sleep percentage
    pub rem_sleep_optimal_percent: f64,
    /// Maximum healthy REM sleep percentage
    pub rem_sleep_max_percent: f64,
    /// Minimum healthy light sleep percentage
    pub light_sleep_min_percent: f64,
    /// Maximum healthy light sleep percentage
    pub light_sleep_max_percent: f64,
    /// Healthy awake time threshold (percentage)
    pub awake_time_healthy_percent: f64,
    /// Acceptable awake time threshold (percentage)
    pub awake_time_acceptable_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepEfficiencyConfig {
    /// Excellent sleep efficiency threshold (percentage)
    pub excellent_threshold: f64,
    /// Good sleep efficiency threshold (percentage)
    pub good_threshold: f64,
    /// Poor sleep efficiency threshold (percentage)
    pub poor_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrvConfig {
    /// RMSSD decrease threshold indicating concern (ms, negative value)
    pub rmssd_decrease_concern_threshold: f64,
    /// RMSSD increase threshold indicating good recovery (ms)
    pub rmssd_increase_good_threshold: f64,
    /// Baseline deviation percentage indicating concern
    pub baseline_deviation_concern_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsbConfig {
    /// Highly fatigued TSB threshold
    pub highly_fatigued_tsb: f64,
    /// Fatigued TSB threshold
    pub fatigued_tsb: f64,
    /// Fresh TSB minimum (optimal range start)
    pub fresh_tsb_min: f64,
    /// Fresh TSB maximum (optimal range end)
    pub fresh_tsb_max: f64,
    /// Detraining TSB threshold (too much rest)
    pub detraining_tsb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryScoringConfig {
    /// Excellent recovery threshold (score 0-100)
    pub excellent_threshold: f64,
    /// Good recovery threshold (score 0-100)
    pub good_threshold: f64,
    /// Fair recovery threshold (score 0-100)
    pub fair_threshold: f64,
    /// TSB weight when all components available
    pub tsb_weight_full: f64,
    /// Sleep weight when all components available
    pub sleep_weight_full: f64,
    /// HRV weight when all components available
    pub hrv_weight_full: f64,
    /// TSB weight when HRV not available
    pub tsb_weight_no_hrv: f64,
    /// Sleep weight when HRV not available
    pub sleep_weight_no_hrv: f64,
}

/// Nutrition Analysis Configuration
///
/// Scientific references:
/// - BMR: Mifflin et al. (1990) DOI: 10.1093/ajcn/51.2.241
/// - Protein: Phillips & Van Loon (2011) DOI: 10.1080/02640414.2011.619204
/// - Carbs: Burke et al. (2011) DOI: 10.1080/02640414.2011.585473
/// - Timing: Kerksick et al. (2017) DOI: 10.1186/s12970-017-0189-4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutritionConfig {
    pub bmr: BmrConfig,
    pub activity_factors: ActivityFactorsConfig,
    pub macronutrients: MacronutrientConfig,
    pub nutrient_timing: NutrientTimingConfig,
    pub usda_api: UsdaApiConfig,
}

/// BMR (Basal Metabolic Rate) calculation configuration
///
/// Reference: Mifflin, M.D., et al. (1990). A new predictive equation for resting energy expenditure.
/// American Journal of Clinical Nutrition, 51(2), 241-247. DOI: 10.1093/ajcn/51.2.241
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BmrConfig {
    /// Mifflin-St Jeor formula enabled (recommended)
    pub use_mifflin_st_jeor: bool,
    /// Harris-Benedict formula enabled (legacy)
    pub use_harris_benedict: bool,
    /// Mifflin-St Jeor weight coefficient (10.0)
    pub msj_weight_coef: f64,
    /// Mifflin-St Jeor height coefficient (6.25)
    pub msj_height_coef: f64,
    /// Mifflin-St Jeor age coefficient (-5.0)
    pub msj_age_coef: f64,
    /// Mifflin-St Jeor male constant (+5)
    pub msj_male_constant: f64,
    /// Mifflin-St Jeor female constant (-161)
    pub msj_female_constant: f64,
}

/// Activity factor multipliers for TDEE calculation
///
/// Reference: `McArdle`, W.D., Katch, F.I., & Katch, V.L. (2010). Exercise Physiology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityFactorsConfig {
    /// Sedentary (little/no exercise): 1.2
    pub sedentary: f64,
    /// Lightly active (1-3 days/week): 1.375
    pub lightly_active: f64,
    /// Moderately active (3-5 days/week): 1.55
    pub moderately_active: f64,
    /// Very active (6-7 days/week): 1.725
    pub very_active: f64,
    /// Extra active (hard training 2x/day): 1.9
    pub extra_active: f64,
}

/// Macronutrient recommendation configuration
///
/// References:
/// - Protein: Phillips & Van Loon (2011) DOI: 10.1080/02640414.2011.619204
/// - Carbs: Burke et al. (2011) DOI: 10.1080/02640414.2011.585473
/// - Fats: DRI (Dietary Reference Intakes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacronutrientConfig {
    /// Minimum protein (g/kg bodyweight) - sedentary: 0.8
    pub protein_min_g_per_kg: f64,
    /// Moderate activity protein (g/kg): 1.2-1.4
    pub protein_moderate_g_per_kg: f64,
    /// Athlete protein (g/kg): 1.6-2.2
    pub protein_athlete_g_per_kg: f64,
    /// Endurance athlete max protein (g/kg): 2.0
    pub protein_endurance_max_g_per_kg: f64,
    /// Strength athlete max protein (g/kg): 2.2
    pub protein_strength_max_g_per_kg: f64,
    /// Minimum carbs (g/kg) - low activity: 3.0
    pub carbs_low_activity_g_per_kg: f64,
    /// Moderate activity carbs (g/kg): 5-7
    pub carbs_moderate_activity_g_per_kg: f64,
    /// High endurance carbs (g/kg): 8-12
    pub carbs_high_endurance_g_per_kg: f64,
    /// Minimum fat percentage of TDEE: 20%
    pub fat_min_percent_tdee: f64,
    /// Maximum fat percentage of TDEE: 35%
    pub fat_max_percent_tdee: f64,
    /// Optimal fat percentage: 25-30%
    pub fat_optimal_percent_tdee: f64,
}

/// Nutrient timing configuration
///
/// References:
/// - Kerksick et al. (2017) DOI: 10.1186/s12970-017-0189-4
/// - Aragon & Schoenfeld (2013) DOI: 10.1186/1550-2783-10-5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrientTimingConfig {
    /// Pre-workout window (hours before): 1-3 hours
    pub pre_workout_window_hours: f64,
    /// Post-workout anabolic window (hours): 2 hours
    pub post_workout_window_hours: f64,
    /// Pre-workout carbs (g/kg): 0.5-1.0
    pub pre_workout_carbs_g_per_kg: f64,
    /// Post-workout protein minimum (g): 20g
    pub post_workout_protein_g_min: f64,
    /// Post-workout protein maximum (g): 40g
    pub post_workout_protein_g_max: f64,
    /// Post-workout carbs (g/kg): 0.8-1.2
    pub post_workout_carbs_g_per_kg: f64,
    /// Minimum protein meals per day
    pub protein_meals_per_day_min: u8,
    /// Optimal protein meals per day
    pub protein_meals_per_day_optimal: u8,
}

/// USDA `FoodData` Central API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdaApiConfig {
    /// Base URL for USDA `FoodData` Central
    pub base_url: String,
    /// API request timeout (seconds)
    pub timeout_secs: u64,
    /// Cache TTL (hours) - 24 hours recommended
    pub cache_ttl_hours: u64,
    /// Max cached items (LRU eviction)
    pub max_cache_items: usize,
    /// Rate limit: requests per minute
    pub rate_limit_per_minute: u32,
}

/// Global configuration singleton
static INTELLIGENCE_CONFIG: OnceLock<IntelligenceConfig<true>> = OnceLock::new();

impl IntelligenceConfig<true> {
    /// Get the global configuration instance
    pub fn global() -> &'static Self {
        INTELLIGENCE_CONFIG.get_or_init(|| {
            Self::load().unwrap_or_else(|e| {
                tracing::warn!("Failed to load intelligence config: {}, using defaults", e);
                Self::default()
            })
        })
    }

    /// Load configuration from environment and files
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables contain invalid values or validation fails
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Apply environment variable overrides
        config = config.apply_env_overrides()?;

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
    // Long function: Comprehensive validation of all intelligence config subsystems (recommendation, weather, HR zones, sleep/recovery)
    #[allow(clippy::too_many_lines)]
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate recommendation thresholds
        if self.recommendation_engine.thresholds.low_weekly_distance_km
            >= self
                .recommendation_engine
                .thresholds
                .high_weekly_distance_km
        {
            return Err(ConfigError::InvalidRange(
                "low_weekly_distance must be < high_weekly_distance",
            ));
        }

        if self.recommendation_engine.thresholds.low_weekly_frequency
            >= self.recommendation_engine.thresholds.high_weekly_frequency
        {
            return Err(ConfigError::InvalidRange(
                "low_weekly_frequency must be < high_weekly_frequency",
            ));
        }

        // Validate weights sum approximately to 1.0
        let weight_sum = self.recommendation_engine.weights.distance_weight
            + self.recommendation_engine.weights.frequency_weight
            + self.recommendation_engine.weights.pace_weight
            + self.recommendation_engine.weights.consistency_weight
            + self.recommendation_engine.weights.recovery_weight;

        if (weight_sum - 1.0).abs() > 0.1 {
            return Err(ConfigError::InvalidWeights(
                "Recommendation weights should approximately sum to 1.0",
            ));
        }

        // Validate temperature thresholds
        if self.weather_analysis.temperature.ideal_min_celsius
            >= self.weather_analysis.temperature.ideal_max_celsius
        {
            return Err(ConfigError::InvalidRange(
                "ideal_min_temperature must be < ideal_max_temperature",
            ));
        }

        // Validate heart rate zones
        let zones = &self.activity_analyzer.analysis.heart_rate_zones;
        if zones.zone1_max_percentage >= zones.zone2_max_percentage
            || zones.zone2_max_percentage >= zones.zone3_max_percentage
            || zones.zone3_max_percentage >= zones.zone4_max_percentage
            || zones.zone4_max_percentage >= zones.zone5_max_percentage
        {
            return Err(ConfigError::InvalidRange(
                "Heart rate zones must be in ascending order",
            ));
        }

        // Validate sleep duration thresholds
        let sleep_dur = &self.sleep_recovery.sleep_duration;
        if sleep_dur.adult_min_hours >= sleep_dur.adult_max_hours {
            return Err(ConfigError::InvalidRange(
                "adult_min_hours must be < adult_max_hours",
            ));
        }
        if sleep_dur.athlete_min_hours > sleep_dur.athlete_optimal_hours {
            return Err(ConfigError::InvalidRange(
                "athlete_min_hours must be <= athlete_optimal_hours",
            ));
        }
        if sleep_dur.very_short_sleep_threshold >= sleep_dur.short_sleep_threshold {
            return Err(ConfigError::InvalidRange(
                "very_short_sleep_threshold must be < short_sleep_threshold",
            ));
        }

        // Validate sleep stages percentages
        let stages = &self.sleep_recovery.sleep_stages;
        if stages.deep_sleep_min_percent >= stages.deep_sleep_max_percent {
            return Err(ConfigError::InvalidRange(
                "deep_sleep_min_percent must be < deep_sleep_max_percent",
            ));
        }
        if stages.rem_sleep_min_percent >= stages.rem_sleep_max_percent {
            return Err(ConfigError::InvalidRange(
                "rem_sleep_min_percent must be < rem_sleep_max_percent",
            ));
        }
        if stages.light_sleep_min_percent >= stages.light_sleep_max_percent {
            return Err(ConfigError::InvalidRange(
                "light_sleep_min_percent must be < light_sleep_max_percent",
            ));
        }
        if stages.awake_time_healthy_percent >= stages.awake_time_acceptable_percent {
            return Err(ConfigError::InvalidRange(
                "awake_time_healthy_percent must be < awake_time_acceptable_percent",
            ));
        }

        // Validate sleep efficiency thresholds
        let efficiency = &self.sleep_recovery.sleep_efficiency;
        if efficiency.poor_threshold >= efficiency.good_threshold {
            return Err(ConfigError::InvalidRange(
                "sleep efficiency: poor_threshold must be < good_threshold",
            ));
        }
        if efficiency.good_threshold >= efficiency.excellent_threshold {
            return Err(ConfigError::InvalidRange(
                "sleep efficiency: good_threshold must be < excellent_threshold",
            ));
        }

        // Validate TSB thresholds
        let tsb = &self.sleep_recovery.training_stress_balance;
        if tsb.highly_fatigued_tsb >= tsb.fatigued_tsb {
            return Err(ConfigError::InvalidRange(
                "TSB: highly_fatigued must be < fatigued",
            ));
        }
        if tsb.fresh_tsb_min >= tsb.fresh_tsb_max {
            return Err(ConfigError::InvalidRange(
                "TSB: fresh_tsb_min must be < fresh_tsb_max",
            ));
        }
        if tsb.fresh_tsb_max >= tsb.detraining_tsb {
            return Err(ConfigError::InvalidRange(
                "TSB: fresh_tsb_max must be < detraining_tsb",
            ));
        }

        // Validate recovery scoring thresholds
        let recovery = &self.sleep_recovery.recovery_scoring;
        if recovery.fair_threshold >= recovery.good_threshold {
            return Err(ConfigError::InvalidRange(
                "recovery: fair_threshold must be < good_threshold",
            ));
        }
        if recovery.good_threshold >= recovery.excellent_threshold {
            return Err(ConfigError::InvalidRange(
                "recovery: good_threshold must be < excellent_threshold",
            ));
        }

        // Validate recovery weights (full scenario)
        let full_weight_sum =
            recovery.tsb_weight_full + recovery.sleep_weight_full + recovery.hrv_weight_full;
        if (full_weight_sum - 1.0).abs() > 0.01 {
            return Err(ConfigError::InvalidWeights(
                "Recovery weights (full) must sum to 1.0",
            ));
        }

        // Validate recovery weights (no HRV scenario)
        let no_hrv_weight_sum = recovery.tsb_weight_no_hrv + recovery.sleep_weight_no_hrv;
        if (no_hrv_weight_sum - 1.0).abs() > 0.01 {
            return Err(ConfigError::InvalidWeights(
                "Recovery weights (no HRV) must sum to 1.0",
            ));
        }

        // Validate nutrition configuration
        let nutr = &self.nutrition;

        // Validate BMR coefficients are positive
        if nutr.bmr.msj_weight_coef <= 0.0 || nutr.bmr.msj_height_coef <= 0.0 {
            return Err(ConfigError::ValueOutOfRange(
                "BMR weight and height coefficients must be positive",
            ));
        }

        // Validate activity factors are > 1.0 and ascending
        if nutr.activity_factors.sedentary < 1.0 || nutr.activity_factors.extra_active > 2.5 {
            return Err(ConfigError::ValueOutOfRange(
                "Activity factors must be between 1.0 and 2.5",
            ));
        }
        if nutr.activity_factors.sedentary >= nutr.activity_factors.lightly_active
            || nutr.activity_factors.lightly_active >= nutr.activity_factors.moderately_active
            || nutr.activity_factors.moderately_active >= nutr.activity_factors.very_active
            || nutr.activity_factors.very_active >= nutr.activity_factors.extra_active
        {
            return Err(ConfigError::InvalidRange(
                "Activity factors must be in ascending order",
            ));
        }

        // Validate protein recommendations are reasonable (0.5-3.0 g/kg)
        if nutr.macronutrients.protein_min_g_per_kg < 0.5
            || nutr.macronutrients.protein_strength_max_g_per_kg > 3.0
        {
            return Err(ConfigError::ValueOutOfRange(
                "Protein recommendations must be between 0.5 and 3.0 g/kg",
            ));
        }
        if nutr.macronutrients.protein_min_g_per_kg >= nutr.macronutrients.protein_moderate_g_per_kg
        {
            return Err(ConfigError::InvalidRange(
                "protein_min must be < protein_moderate",
            ));
        }

        // Validate carb recommendations are reasonable (1.0-15.0 g/kg)
        if nutr.macronutrients.carbs_low_activity_g_per_kg < 1.0
            || nutr.macronutrients.carbs_high_endurance_g_per_kg > 15.0
        {
            return Err(ConfigError::ValueOutOfRange(
                "Carb recommendations must be between 1.0 and 15.0 g/kg",
            ));
        }

        // Validate fat percentages
        if nutr.macronutrients.fat_min_percent_tdee < 10.0
            || nutr.macronutrients.fat_max_percent_tdee > 50.0
        {
            return Err(ConfigError::ValueOutOfRange(
                "Fat percentage must be between 10% and 50% of TDEE",
            ));
        }
        if nutr.macronutrients.fat_min_percent_tdee >= nutr.macronutrients.fat_max_percent_tdee {
            return Err(ConfigError::InvalidRange(
                "fat_min_percent must be < fat_max_percent",
            ));
        }

        // Validate nutrient timing windows
        if nutr.nutrient_timing.pre_workout_window_hours > 6.0
            || nutr.nutrient_timing.post_workout_window_hours > 6.0
        {
            return Err(ConfigError::ValueOutOfRange(
                "Pre/post workout windows must be <= 6 hours",
            ));
        }
        if nutr.nutrient_timing.post_workout_protein_g_min
            >= nutr.nutrient_timing.post_workout_protein_g_max
        {
            return Err(ConfigError::InvalidRange(
                "post_workout_protein_min must be < post_workout_protein_max",
            ));
        }
        if nutr.nutrient_timing.protein_meals_per_day_min == 0
            || nutr.nutrient_timing.protein_meals_per_day_optimal == 0
        {
            return Err(ConfigError::ValueOutOfRange(
                "Protein meals per day must be at least 1",
            ));
        }

        // Validate USDA API config
        if nutr.usda_api.timeout_secs == 0 || nutr.usda_api.timeout_secs > 60 {
            return Err(ConfigError::ValueOutOfRange(
                "USDA API timeout must be between 1 and 60 seconds",
            ));
        }
        if nutr.usda_api.cache_ttl_hours == 0 || nutr.usda_api.cache_ttl_hours > 168 {
            return Err(ConfigError::ValueOutOfRange(
                "Cache TTL must be between 1 and 168 hours (7 days)",
            ));
        }

        Ok(())
    }

    /// Apply environment variable overrides
    // Long function: Systematic env var parsing for all intelligence subsystems (recommendation, weather, sleep/recovery with 24+ variables)
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::cognitive_complexity)]
    fn apply_env_overrides(mut self) -> Result<Self, ConfigError> {
        // Recommendation engine overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_RECOMMENDATION_LOW_DISTANCE") {
            self.recommendation_engine.thresholds.low_weekly_distance_km =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_RECOMMENDATION_LOW_DISTANCE".into())
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_RECOMMENDATION_HIGH_DISTANCE") {
            self.recommendation_engine
                .thresholds
                .high_weekly_distance_km = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_RECOMMENDATION_HIGH_DISTANCE".into())
            })?;
        }

        // Weather analysis overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_WEATHER_IDEAL_MIN_TEMP") {
            self.weather_analysis.temperature.ideal_min_celsius = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_WEATHER_IDEAL_MIN_TEMP".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_WEATHER_IDEAL_MAX_TEMP") {
            self.weather_analysis.temperature.ideal_max_celsius = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_WEATHER_IDEAL_MAX_TEMP".into())
            })?;
        }

        // Sleep duration overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_ADULT_MIN_HOURS") {
            self.sleep_recovery.sleep_duration.adult_min_hours = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_ADULT_MIN_HOURS".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_ADULT_MAX_HOURS") {
            self.sleep_recovery.sleep_duration.adult_max_hours = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_ADULT_MAX_HOURS".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_ATHLETE_OPTIMAL_HOURS") {
            self.sleep_recovery.sleep_duration.athlete_optimal_hours =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_ATHLETE_OPTIMAL_HOURS".into())
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_ATHLETE_MIN_HOURS") {
            self.sleep_recovery.sleep_duration.athlete_min_hours = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_ATHLETE_MIN_HOURS".into())
            })?;
        }

        // Sleep stages overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_DEEP_MIN_PERCENT") {
            self.sleep_recovery.sleep_stages.deep_sleep_min_percent =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_DEEP_MIN_PERCENT".into())
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_DEEP_MAX_PERCENT") {
            self.sleep_recovery.sleep_stages.deep_sleep_max_percent =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_DEEP_MAX_PERCENT".into())
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_REM_MIN_PERCENT") {
            self.sleep_recovery.sleep_stages.rem_sleep_min_percent = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_REM_MIN_PERCENT".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_REM_MAX_PERCENT") {
            self.sleep_recovery.sleep_stages.rem_sleep_max_percent = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_REM_MAX_PERCENT".into())
            })?;
        }

        // Sleep efficiency overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_EFFICIENCY_EXCELLENT") {
            self.sleep_recovery.sleep_efficiency.excellent_threshold =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_EFFICIENCY_EXCELLENT".into())
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_EFFICIENCY_GOOD") {
            self.sleep_recovery.sleep_efficiency.good_threshold = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_EFFICIENCY_GOOD".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SLEEP_EFFICIENCY_POOR") {
            self.sleep_recovery.sleep_efficiency.poor_threshold = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_SLEEP_EFFICIENCY_POOR".into())
            })?;
        }

        // HRV overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_HRV_RMSSD_DECREASE_CONCERN") {
            self.sleep_recovery.hrv.rmssd_decrease_concern_threshold =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_HRV_RMSSD_DECREASE_CONCERN".into())
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_HRV_RMSSD_INCREASE_GOOD") {
            self.sleep_recovery.hrv.rmssd_increase_good_threshold = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_HRV_RMSSD_INCREASE_GOOD".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_HRV_BASELINE_DEVIATION_CONCERN") {
            self.sleep_recovery.hrv.baseline_deviation_concern_percent =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_HRV_BASELINE_DEVIATION_CONCERN".into())
                })?;
        }

        // TSB (Training Stress Balance) overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_TSB_HIGHLY_FATIGUED") {
            self.sleep_recovery
                .training_stress_balance
                .highly_fatigued_tsb = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_TSB_HIGHLY_FATIGUED".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_TSB_FATIGUED") {
            self.sleep_recovery.training_stress_balance.fatigued_tsb = val
                .parse()
                .map_err(|_| ConfigError::Parse("Invalid INTELLIGENCE_TSB_FATIGUED".into()))?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_TSB_FRESH_MIN") {
            self.sleep_recovery.training_stress_balance.fresh_tsb_min = val
                .parse()
                .map_err(|_| ConfigError::Parse("Invalid INTELLIGENCE_TSB_FRESH_MIN".into()))?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_TSB_FRESH_MAX") {
            self.sleep_recovery.training_stress_balance.fresh_tsb_max = val
                .parse()
                .map_err(|_| ConfigError::Parse("Invalid INTELLIGENCE_TSB_FRESH_MAX".into()))?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_TSB_DETRAINING") {
            self.sleep_recovery.training_stress_balance.detraining_tsb = val
                .parse()
                .map_err(|_| ConfigError::Parse("Invalid INTELLIGENCE_TSB_DETRAINING".into()))?;
        }

        // Recovery scoring overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_RECOVERY_EXCELLENT_THRESHOLD") {
            self.sleep_recovery.recovery_scoring.excellent_threshold =
                val.parse().map_err(|_| {
                    ConfigError::Parse("Invalid INTELLIGENCE_RECOVERY_EXCELLENT_THRESHOLD".into())
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_RECOVERY_GOOD_THRESHOLD") {
            self.sleep_recovery.recovery_scoring.good_threshold = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_RECOVERY_GOOD_THRESHOLD".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_RECOVERY_FAIR_THRESHOLD") {
            self.sleep_recovery.recovery_scoring.fair_threshold = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_RECOVERY_FAIR_THRESHOLD".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_RECOVERY_TSB_WEIGHT_FULL") {
            self.sleep_recovery.recovery_scoring.tsb_weight_full = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_RECOVERY_TSB_WEIGHT_FULL".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_RECOVERY_SLEEP_WEIGHT_FULL") {
            self.sleep_recovery.recovery_scoring.sleep_weight_full = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_RECOVERY_SLEEP_WEIGHT_FULL".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_RECOVERY_HRV_WEIGHT_FULL") {
            self.sleep_recovery.recovery_scoring.hrv_weight_full = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_RECOVERY_HRV_WEIGHT_FULL".into())
            })?;
        }

        // Algorithm selection overrides
        if let Ok(val) = std::env::var("PIERRE_TSS_ALGORITHM") {
            self.algorithms.tss = val;
        }

        if let Ok(val) = std::env::var("PIERRE_MAXHR_ALGORITHM") {
            self.algorithms.maxhr = val;
        }

        Ok(self)
    }
}

impl Default for IntelligenceConfig<true> {
    fn default() -> Self {
        Self {
            recommendation_engine: Self::default_recommendation_engine_config(),
            performance_analyzer: Self::default_performance_analyzer_config(),
            goal_engine: Self::default_goal_engine_config(),
            weather_analysis: Self::default_weather_analysis_config(),
            activity_analyzer: Self::default_activity_analyzer_config(),
            metrics: Self::default_metrics_config(),
            sleep_recovery: Self::default_sleep_recovery_config(),
            nutrition: Self::default_nutrition_config(),
            algorithms: AlgorithmConfig::default(),
            _phantom: PhantomData,
        }
    }
}

impl IntelligenceConfig<true> {
    /// Create default recommendation engine configuration
    fn default_recommendation_engine_config() -> RecommendationEngineConfig {
        RecommendationEngineConfig {
            thresholds: Self::default_recommendation_thresholds(),
            weights: Self::default_recommendation_weights(),
            limits: Self::default_recommendation_limits(),
            messages: Self::default_recommendation_messages(),
        }
    }

    /// Create default recommendation thresholds
    const fn default_recommendation_thresholds() -> RecommendationThresholds {
        RecommendationThresholds {
            low_weekly_distance_km: 20.0,
            high_weekly_distance_km: 80.0,
            low_weekly_frequency: 2,
            high_weekly_frequency: 6,
            pace_improvement_threshold: 0.05,
            consistency_threshold: limits::DEFAULT_CONFIDENCE_THRESHOLD,
            rest_day_threshold: 1,
            volume_increase_threshold: 0.1,
            intensity_threshold: 0.8,
        }
    }

    /// Create default recommendation weights
    const fn default_recommendation_weights() -> RecommendationWeights {
        RecommendationWeights {
            distance_weight: 0.3,
            frequency_weight: 0.25,
            pace_weight: 0.2,
            consistency_weight: 0.15,
            recovery_weight: 0.1,
        }
    }

    /// Create default recommendation limits
    const fn default_recommendation_limits() -> RecommendationLimits {
        RecommendationLimits {
            max_recommendations_per_category: 3,
            max_total_recommendations: 10,
            min_confidence_threshold: 0.6,
        }
    }

    /// Create default recommendation messages
    fn default_recommendation_messages() -> RecommendationMessages {
        RecommendationMessages {
            low_distance: "Consider gradually increasing your weekly distance".into(),
            high_distance: "You're covering good distance - focus on quality".into(),
            low_frequency: "Try to add one more training session per week".into(),
            high_frequency: "You're training frequently - ensure adequate recovery".to_string(),
            pace_improvement: "Focus on tempo runs to improve your pace".into(),
            consistency_improvement: "Try to maintain a more consistent training schedule"
                .to_string(),
            recovery_needed: "Consider adding more recovery time between sessions".to_string(),
        }
    }

    /// Create default performance analyzer configuration
    const fn default_performance_analyzer_config() -> PerformanceAnalyzerConfig {
        PerformanceAnalyzerConfig {
            trend_analysis: Self::default_trend_analysis_config(),
            statistical: Self::default_statistical_config(),
            thresholds: Self::default_performance_thresholds(),
        }
    }

    /// Create default trend analysis configuration
    const fn default_trend_analysis_config() -> TrendAnalysisConfig {
        TrendAnalysisConfig {
            min_data_points: 5,
            trend_strength_threshold: 0.3,
            significance_threshold: 0.05,
            improvement_threshold: 0.02,
            decline_threshold: -0.02,
        }
    }

    /// Create default statistical configuration
    const fn default_statistical_config() -> StatisticalConfig {
        StatisticalConfig {
            confidence_level: 0.95,
            outlier_threshold: 2.0,
            smoothing_factor: 0.3,
        }
    }

    /// Create default performance thresholds
    const fn default_performance_thresholds() -> PerformanceThresholds {
        PerformanceThresholds {
            significant_improvement: 0.05,
            significant_decline: -0.05,
            pace_variance_threshold: 0.2,
            endurance_threshold: 0.8,
        }
    }

    /// Create default goal engine configuration
    const fn default_goal_engine_config() -> GoalEngineConfig {
        GoalEngineConfig {
            feasibility: Self::default_feasibility_config(),
            suggestion: Self::default_suggestion_config(),
            progression: Self::default_progression_config(),
        }
    }

    /// Create default feasibility configuration
    const fn default_feasibility_config() -> FeasibilityConfig {
        FeasibilityConfig {
            min_success_probability: 0.6,
            conservative_multiplier: 0.8,
            aggressive_multiplier: 1.3,
            injury_risk_threshold: 0.3,
        }
    }

    /// Create default suggestion configuration
    const fn default_suggestion_config() -> SuggestionConfig {
        SuggestionConfig {
            max_goals_per_type: 3,
            difficulty_distribution: Self::default_difficulty_distribution(),
            timeframe_preferences: Self::default_timeframe_preferences(),
        }
    }

    /// Create default difficulty distribution
    const fn default_difficulty_distribution() -> DifficultyDistribution {
        DifficultyDistribution {
            easy_percentage: 0.4,
            moderate_percentage: 0.4,
            hard_percentage: 0.2,
        }
    }

    /// Create default timeframe preferences
    const fn default_timeframe_preferences() -> TimeframePreferences {
        TimeframePreferences {
            short_term_weeks: 4,
            medium_term_weeks: 12,
            long_term_weeks: 26,
        }
    }

    /// Create default progression configuration
    const fn default_progression_config() -> ProgressionConfig {
        ProgressionConfig {
            weekly_increase_limit: 0.1,
            monthly_increase_limit: 0.2,
            deload_frequency_weeks: 4,
        }
    }

    /// Create default weather analysis configuration
    const fn default_weather_analysis_config() -> WeatherAnalysisConfig {
        WeatherAnalysisConfig {
            temperature: Self::default_temperature_config(),
            conditions: Self::default_weather_conditions_config(),
            impact: Self::default_weather_impact_config(),
        }
    }

    /// Create default temperature configuration
    const fn default_temperature_config() -> TemperatureConfig {
        TemperatureConfig {
            ideal_min_celsius: 10.0,
            ideal_max_celsius: 20.0,
            cold_threshold_celsius: 5.0,
            hot_threshold_celsius: 25.0,
            extreme_cold_celsius: -5.0,
            extreme_hot_celsius: 35.0,
        }
    }

    /// Create default weather conditions configuration
    const fn default_weather_conditions_config() -> WeatherConditionsConfig {
        WeatherConditionsConfig {
            high_humidity_threshold: 80.0,
            strong_wind_threshold: 20.0,
            precipitation_impact_factor: 0.8,
        }
    }

    /// Create default weather impact configuration
    const fn default_weather_impact_config() -> WeatherImpactConfig {
        WeatherImpactConfig {
            temperature_impact_weight: 0.4,
            humidity_impact_weight: 0.3,
            wind_impact_weight: 0.2,
            precipitation_impact_weight: 0.1,
        }
    }

    /// Create default activity analyzer configuration
    const fn default_activity_analyzer_config() -> ActivityAnalyzerConfig {
        ActivityAnalyzerConfig {
            analysis: Self::default_activity_analysis_config(),
            scoring: Self::default_activity_scoring_config(),
            insights: Self::default_activity_insights_config(),
        }
    }

    /// Create default activity analysis configuration
    const fn default_activity_analysis_config() -> ActivityAnalysisConfig {
        ActivityAnalysisConfig {
            min_duration_seconds: 300, // 5 minutes
            max_reasonable_pace: 15.0, // 15 min/km
            heart_rate_zones: Self::default_heart_rate_zones_config(),
            power_zones: Self::default_power_zones_config(),
        }
    }

    /// Create default heart rate zones configuration
    const fn default_heart_rate_zones_config() -> HeartRateZonesConfig {
        HeartRateZonesConfig {
            zone1_max_percentage: 60.0,
            zone2_max_percentage: 70.0,
            zone3_max_percentage: 80.0,
            zone4_max_percentage: 90.0,
            zone5_max_percentage: 100.0,
        }
    }

    /// Create default power zones configuration
    const fn default_power_zones_config() -> PowerZonesConfig {
        PowerZonesConfig {
            zone1_max_percentage: 55.0,
            zone2_max_percentage: 75.0,
            zone3_max_percentage: 90.0,
            zone4_max_percentage: 105.0,
            zone5_max_percentage: 150.0,
        }
    }

    /// Create default activity scoring configuration
    const fn default_activity_scoring_config() -> ActivityScoringConfig {
        ActivityScoringConfig {
            efficiency_weight: 0.3,
            intensity_weight: 0.3,
            duration_weight: 0.2,
            consistency_weight: 0.2,
        }
    }

    /// Create default activity insights configuration
    const fn default_activity_insights_config() -> ActivityInsightsConfig {
        ActivityInsightsConfig {
            min_confidence_threshold: limits::DEFAULT_CONFIDENCE_THRESHOLD,
            max_insights_per_activity: 5,
            severity_thresholds: Self::default_severity_thresholds(),
        }
    }

    /// Create default severity thresholds
    const fn default_severity_thresholds() -> SeverityThresholds {
        SeverityThresholds {
            info_threshold: 0.3,
            warning_threshold: limits::DEFAULT_CONFIDENCE_THRESHOLD,
            critical_threshold: 0.9,
        }
    }

    /// Create default metrics configuration
    fn default_metrics_config() -> MetricsConfig {
        MetricsConfig {
            calculation: Self::default_metrics_calculation_config(),
            validation: Self::default_metrics_validation_config(),
            aggregation: Self::default_metrics_aggregation_config(),
        }
    }

    /// Create default metrics calculation configuration
    const fn default_metrics_calculation_config() -> MetricsCalculationConfig {
        MetricsCalculationConfig {
            smoothing_window_size: 7,
            outlier_detection_threshold: 2.5,
            missing_data_interpolation: true,
        }
    }

    /// Create default metrics validation configuration
    const fn default_metrics_validation_config() -> MetricsValidationConfig {
        MetricsValidationConfig {
            max_heart_rate: 220,
            min_heart_rate: 40,
            max_pace_min_per_km: 20.0,
            min_pace_min_per_km: 2.0,
        }
    }

    /// Create default metrics aggregation configuration
    fn default_metrics_aggregation_config() -> MetricsAggregationConfig {
        MetricsAggregationConfig {
            weekly_aggregation_method: "average".into(),
            monthly_aggregation_method: "weighted_average".into(),
            trend_calculation_method: "linear_regression".into(),
        }
    }

    /// Create default sleep recovery configuration
    const fn default_sleep_recovery_config() -> SleepRecoveryConfig {
        SleepRecoveryConfig {
            sleep_duration: Self::default_sleep_duration_config(),
            sleep_stages: Self::default_sleep_stages_config(),
            sleep_efficiency: Self::default_sleep_efficiency_config(),
            hrv: Self::default_hrv_config(),
            training_stress_balance: Self::default_tsb_config(),
            recovery_scoring: Self::default_recovery_scoring_config(),
        }
    }

    /// Create default sleep duration configuration
    /// Based on NSF/AASM guidelines (Watson et al. 2015, Hirshkowitz et al. 2015)
    const fn default_sleep_duration_config() -> SleepDurationConfig {
        SleepDurationConfig {
            adult_min_hours: 7.0,
            adult_max_hours: 9.0,
            athlete_optimal_hours: 8.0,
            athlete_min_hours: 7.5,
            short_sleep_threshold: 6.0,
            very_short_sleep_threshold: 5.0,
        }
    }

    /// Create default sleep stages configuration
    /// Based on AASM sleep stage guidelines
    const fn default_sleep_stages_config() -> SleepStagesConfig {
        SleepStagesConfig {
            deep_sleep_min_percent: 15.0,
            deep_sleep_optimal_percent: 20.0,
            deep_sleep_max_percent: 25.0,
            rem_sleep_min_percent: 20.0,
            rem_sleep_optimal_percent: 25.0,
            rem_sleep_max_percent: 30.0,
            light_sleep_min_percent: 45.0,
            light_sleep_max_percent: 55.0,
            awake_time_healthy_percent: 5.0,
            awake_time_acceptable_percent: 10.0,
        }
    }

    /// Create default sleep efficiency configuration
    const fn default_sleep_efficiency_config() -> SleepEfficiencyConfig {
        SleepEfficiencyConfig {
            excellent_threshold: 90.0,
            good_threshold: 85.0,
            poor_threshold: 70.0,
        }
    }

    /// Create default HRV configuration
    /// Based on Shaffer & Ginsberg (2017) and Plews et al. (2013)
    const fn default_hrv_config() -> HrvConfig {
        HrvConfig {
            rmssd_decrease_concern_threshold: -10.0, // -10ms indicates poor recovery
            rmssd_increase_good_threshold: 5.0,      // +5ms indicates good recovery
            baseline_deviation_concern_percent: 15.0, // >15% below baseline = concern
        }
    }

    /// Create default TSB configuration
    /// Based on Banister training load model
    const fn default_tsb_config() -> TsbConfig {
        TsbConfig {
            highly_fatigued_tsb: -15.0,
            fatigued_tsb: -10.0,
            fresh_tsb_min: 5.0,
            fresh_tsb_max: 15.0,
            detraining_tsb: 25.0,
        }
    }

    /// Create default recovery scoring configuration
    const fn default_recovery_scoring_config() -> RecoveryScoringConfig {
        RecoveryScoringConfig {
            excellent_threshold: 85.0,
            good_threshold: 70.0,
            fair_threshold: 50.0,
            // When all components available: TSB 40%, Sleep 40%, HRV 20%
            tsb_weight_full: 0.4,
            sleep_weight_full: 0.4,
            hrv_weight_full: 0.2,
            // When HRV not available: TSB 50%, Sleep 50%
            tsb_weight_no_hrv: 0.5,
            sleep_weight_no_hrv: 0.5,
        }
    }

    /// Create default nutrition configuration
    /// Based on peer-reviewed scientific research (see struct documentation)
    fn default_nutrition_config() -> NutritionConfig {
        NutritionConfig {
            bmr: Self::default_bmr_config(),
            activity_factors: Self::default_activity_factors_config(),
            macronutrients: Self::default_macronutrient_config(),
            nutrient_timing: Self::default_nutrient_timing_config(),
            usda_api: Self::default_usda_api_config(),
        }
    }

    /// Create default BMR configuration
    /// Based on Mifflin-St Jeor equation (Mifflin et al. 1990)
    const fn default_bmr_config() -> BmrConfig {
        BmrConfig {
            use_mifflin_st_jeor: true,
            use_harris_benedict: false,
            msj_weight_coef: 10.0,
            msj_height_coef: 6.25,
            msj_age_coef: -5.0,
            msj_male_constant: 5.0,
            msj_female_constant: -161.0,
        }
    }

    /// Create default activity factors configuration
    /// Based on `McArdle` et al. (2010) Exercise Physiology
    const fn default_activity_factors_config() -> ActivityFactorsConfig {
        ActivityFactorsConfig {
            sedentary: 1.2,
            lightly_active: 1.375,
            moderately_active: 1.55,
            very_active: 1.725,
            extra_active: 1.9,
        }
    }

    /// Create default macronutrient configuration
    /// Based on Phillips & Van Loon (2011), Burke et al. (2011), DRI guidelines
    const fn default_macronutrient_config() -> MacronutrientConfig {
        MacronutrientConfig {
            protein_min_g_per_kg: 0.8,
            protein_moderate_g_per_kg: 1.3,
            protein_athlete_g_per_kg: 1.8,
            protein_endurance_max_g_per_kg: 2.0,
            protein_strength_max_g_per_kg: 2.2,
            carbs_low_activity_g_per_kg: 3.0,
            carbs_moderate_activity_g_per_kg: 6.0,
            carbs_high_endurance_g_per_kg: 10.0,
            fat_min_percent_tdee: 20.0,
            fat_max_percent_tdee: 35.0,
            fat_optimal_percent_tdee: 27.5,
        }
    }

    /// Create default nutrient timing configuration
    /// Based on Kerksick et al. (2017), Aragon & Schoenfeld (2013)
    const fn default_nutrient_timing_config() -> NutrientTimingConfig {
        NutrientTimingConfig {
            pre_workout_window_hours: 2.0,
            post_workout_window_hours: 2.0,
            pre_workout_carbs_g_per_kg: 0.75,
            post_workout_protein_g_min: 20.0,
            post_workout_protein_g_max: 40.0,
            post_workout_carbs_g_per_kg: 1.0,
            protein_meals_per_day_min: 3,
            protein_meals_per_day_optimal: 4,
        }
    }

    /// Create default USDA API configuration
    fn default_usda_api_config() -> UsdaApiConfig {
        UsdaApiConfig {
            base_url: "https://api.nal.usda.gov/fdc/v1".to_string(),
            timeout_secs: 10,
            cache_ttl_hours: 24,
            max_cache_items: 1000,
            rate_limit_per_minute: 30,
        }
    }
}

/// Trait for strategy-based configuration
pub trait IntelligenceStrategy: Send + Sync + 'static {
    fn recommendation_thresholds(&self) -> &RecommendationThresholds;
    fn performance_thresholds(&self) -> &PerformanceThresholds;
    fn weather_config(&self) -> &WeatherAnalysisConfig;

    fn should_recommend_volume_increase(&self, current_km: f64) -> bool {
        current_km < self.recommendation_thresholds().low_weekly_distance_km
    }

    fn should_recommend_recovery(&self, frequency: i32) -> bool {
        frequency > self.recommendation_thresholds().high_weekly_frequency
    }
}

/// Conservative strategy for beginners
#[derive(Debug, Clone)]
pub struct ConservativeStrategy {
    config: IntelligenceConfig<true>,
}

impl Default for ConservativeStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ConservativeStrategy {
    #[must_use]
    pub fn new() -> Self {
        let mut config = IntelligenceConfig::default();

        // Override with conservative values
        config
            .recommendation_engine
            .thresholds
            .low_weekly_distance_km = 15.0;
        config
            .recommendation_engine
            .thresholds
            .high_weekly_distance_km = 50.0;
        config.recommendation_engine.thresholds.low_weekly_frequency = 2;
        config
            .recommendation_engine
            .thresholds
            .high_weekly_frequency = 4;

        Self { config }
    }
}

impl IntelligenceStrategy for ConservativeStrategy {
    fn recommendation_thresholds(&self) -> &RecommendationThresholds {
        &self.config.recommendation_engine.thresholds
    }

    fn performance_thresholds(&self) -> &PerformanceThresholds {
        &self.config.performance_analyzer.thresholds
    }

    fn weather_config(&self) -> &WeatherAnalysisConfig {
        &self.config.weather_analysis
    }
}

/// Aggressive strategy for experienced athletes
#[derive(Debug, Clone)]
pub struct AggressiveStrategy {
    config: IntelligenceConfig<true>,
}

impl Default for AggressiveStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl AggressiveStrategy {
    #[must_use]
    pub fn new() -> Self {
        let mut config = IntelligenceConfig::default();

        // Override with aggressive values
        config
            .recommendation_engine
            .thresholds
            .low_weekly_distance_km = 40.0;
        config
            .recommendation_engine
            .thresholds
            .high_weekly_distance_km = 120.0;
        config.recommendation_engine.thresholds.low_weekly_frequency = 4;
        config
            .recommendation_engine
            .thresholds
            .high_weekly_frequency = 7;

        Self { config }
    }
}

impl IntelligenceStrategy for AggressiveStrategy {
    fn recommendation_thresholds(&self) -> &RecommendationThresholds {
        &self.config.recommendation_engine.thresholds
    }

    fn performance_thresholds(&self) -> &PerformanceThresholds {
        &self.config.performance_analyzer.thresholds
    }

    fn weather_config(&self) -> &WeatherAnalysisConfig {
        &self.config.weather_analysis
    }
}

/// Default strategy using global configuration
#[derive(Debug, Clone)]
pub struct DefaultStrategy;

impl IntelligenceStrategy for DefaultStrategy {
    fn recommendation_thresholds(&self) -> &RecommendationThresholds {
        &IntelligenceConfig::global()
            .recommendation_engine
            .thresholds
    }

    fn performance_thresholds(&self) -> &PerformanceThresholds {
        &IntelligenceConfig::global().performance_analyzer.thresholds
    }

    fn weather_config(&self) -> &WeatherAnalysisConfig {
        &IntelligenceConfig::global().weather_analysis
    }
}
