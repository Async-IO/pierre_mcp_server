// ABOUTME: Intelligence module configuration for AI-powered fitness analysis and recommendations
// ABOUTME: Orchestrates domain-specific configs and provides unified validation and loading
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Intelligence Configuration Module
//!
//! Provides type-safe, compile-time validated configuration for all intelligence modules
//! including recommendation engine, performance analyzer, goal engine, and weather analysis.
//!
//! # Module Structure
//!
//! Configuration is organized into domain-specific modules:
//! - `algorithms` - Algorithm selection (TSS, MaxHR, FTP, etc.)
//! - `recommendation` - Recommendation engine thresholds and weights
//! - `performance` - Performance analyzer trend and statistical analysis
//! - `goals` - Goal engine feasibility and progression
//! - `weather` - Weather impact analysis
//! - `activity` - Activity analyzer zones and scoring
//! - `metrics` - Metrics calculation and validation
//! - `sleep_recovery` - Sleep and recovery analysis
//! - `nutrition` - Nutrition recommendations and meal timing

// Domain configuration modules
pub mod activity;
pub mod algorithms;
pub mod error;
pub mod goals;
pub mod metrics;
pub mod nutrition;
pub mod performance;
pub mod recommendation;
pub mod sleep_recovery;
pub mod weather;

// Re-export all types for backward compatibility
pub use activity::{
    ActivityAnalysisConfig, ActivityAnalyzerConfig, ActivityInsightsConfig, ActivityScoringConfig,
    HeartRateZonesConfig, PowerZonesConfig, SeverityThresholds,
};
pub use algorithms::AlgorithmConfig;
pub use error::ConfigError;
pub use goals::{
    DifficultyDistribution, FeasibilityConfig, GoalEngineConfig, ProgressionConfig,
    SuggestionConfig, TimeframePreferences,
};
pub use metrics::{
    MetricsAggregationConfig, MetricsCalculationConfig, MetricsConfig, MetricsValidationConfig,
};
pub use nutrition::{
    ActivityFactorsConfig, BmrConfig, MacroDistribution, MacronutrientConfig,
    MealFallbackCaloriesConfig, MealTdeeProportionsConfig, MealTimingMacrosConfig,
    NutrientTimingConfig, NutritionConfig, UsdaApiConfig,
};
pub use performance::{
    PerformanceAnalyzerConfig, PerformanceThresholds, StatisticalConfig, TrendAnalysisConfig,
};
pub use recommendation::{
    RecommendationEngineConfig, RecommendationLimits, RecommendationMessages,
    RecommendationThresholds, RecommendationWeights,
};
pub use sleep_recovery::{
    HrvConfig, RecoveryScoringConfig, SleepDurationConfig, SleepEfficiencyConfig,
    SleepRecoveryConfig, SleepStagesConfig, TsbConfig,
};
pub use weather::{
    TemperatureConfig, WeatherAnalysisConfig, WeatherConditionsConfig, WeatherImpactConfig,
};

use serde::{Deserialize, Serialize};
use std::env;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::OnceLock;
use tracing::warn;

/// Global configuration singleton
static INTELLIGENCE_CONFIG: OnceLock<IntelligenceConfig<true>> = OnceLock::new();

/// Main intelligence configuration container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig<const VALIDATED: bool = false> {
    /// Configuration for workout recommendation engine
    pub recommendation_engine: RecommendationEngineConfig,
    /// Configuration for performance analysis algorithms
    pub performance_analyzer: PerformanceAnalyzerConfig,
    /// Configuration for goal tracking and achievement engine
    pub goal_engine: GoalEngineConfig,
    /// Configuration for weather impact analysis
    pub weather_analysis: WeatherAnalysisConfig,
    /// Configuration for activity classification and analysis
    pub activity_analyzer: ActivityAnalyzerConfig,
    /// Configuration for metrics calculation and thresholds
    pub metrics: MetricsConfig,
    /// Configuration for sleep tracking and recovery calculation
    pub sleep_recovery: SleepRecoveryConfig,
    /// Configuration for nutrition analysis and recommendations
    pub nutrition: NutritionConfig,
    /// Configuration for algorithm selection (TSS, FTP, `VO2max`, etc.)
    pub algorithms: AlgorithmConfig,
    #[serde(skip)]
    _phantom: PhantomData<()>,
}

impl IntelligenceConfig<true> {
    /// Get the global configuration instance
    pub fn global() -> &'static Self {
        INTELLIGENCE_CONFIG.get_or_init(|| {
            Self::load().unwrap_or_else(|e| {
                warn!("Failed to load intelligence config: {}, using defaults", e);
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
    // Long function: Comprehensive validation of all intelligence config subsystems
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
        self.validate_nutrition()?;

        Ok(())
    }

    /// Validate nutrition configuration
    fn validate_nutrition(&self) -> Result<(), ConfigError> {
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

        // Validate meal timing macro distributions sum to 100%
        nutr.meal_timing_macros.validate()?;

        Ok(())
    }

    /// Helper function to parse and apply an environment variable override
    fn apply_env_var<T: FromStr>(env_var_name: &str, target: &mut T) -> Result<(), ConfigError> {
        if let Ok(val) = env::var(env_var_name) {
            *target = val
                .parse()
                .map_err(|_| ConfigError::Parse(format!("Invalid {env_var_name}")))?;
        }
        Ok(())
    }

    /// Apply environment variable overrides
    // Long function: Systematic env var parsing for all intelligence subsystems
    #[allow(clippy::too_many_lines)]
    fn apply_env_overrides(mut self) -> Result<Self, ConfigError> {
        // Recommendation engine overrides
        Self::apply_env_var(
            "INTELLIGENCE_RECOMMENDATION_LOW_DISTANCE",
            &mut self.recommendation_engine.thresholds.low_weekly_distance_km,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_RECOMMENDATION_HIGH_DISTANCE",
            &mut self
                .recommendation_engine
                .thresholds
                .high_weekly_distance_km,
        )?;

        // Weather analysis overrides
        Self::apply_env_var(
            "INTELLIGENCE_WEATHER_IDEAL_MIN_TEMP",
            &mut self.weather_analysis.temperature.ideal_min_celsius,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_WEATHER_IDEAL_MAX_TEMP",
            &mut self.weather_analysis.temperature.ideal_max_celsius,
        )?;

        // Sleep duration overrides
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_ADULT_MIN_HOURS",
            &mut self.sleep_recovery.sleep_duration.adult_min_hours,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_ADULT_MAX_HOURS",
            &mut self.sleep_recovery.sleep_duration.adult_max_hours,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_ATHLETE_OPTIMAL_HOURS",
            &mut self.sleep_recovery.sleep_duration.athlete_optimal_hours,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_ATHLETE_MIN_HOURS",
            &mut self.sleep_recovery.sleep_duration.athlete_min_hours,
        )?;

        // Sleep stages overrides
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_DEEP_MIN_PERCENT",
            &mut self.sleep_recovery.sleep_stages.deep_sleep_min_percent,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_DEEP_MAX_PERCENT",
            &mut self.sleep_recovery.sleep_stages.deep_sleep_max_percent,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_REM_MIN_PERCENT",
            &mut self.sleep_recovery.sleep_stages.rem_sleep_min_percent,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_REM_MAX_PERCENT",
            &mut self.sleep_recovery.sleep_stages.rem_sleep_max_percent,
        )?;

        // Sleep efficiency overrides
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_EFFICIENCY_EXCELLENT",
            &mut self.sleep_recovery.sleep_efficiency.excellent_threshold,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_EFFICIENCY_GOOD",
            &mut self.sleep_recovery.sleep_efficiency.good_threshold,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_SLEEP_EFFICIENCY_POOR",
            &mut self.sleep_recovery.sleep_efficiency.poor_threshold,
        )?;

        // HRV overrides
        Self::apply_env_var(
            "INTELLIGENCE_HRV_RMSSD_DECREASE_CONCERN",
            &mut self.sleep_recovery.hrv.rmssd_decrease_concern_threshold,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_HRV_RMSSD_INCREASE_GOOD",
            &mut self.sleep_recovery.hrv.rmssd_increase_good_threshold,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_HRV_BASELINE_DEVIATION_CONCERN",
            &mut self.sleep_recovery.hrv.baseline_deviation_concern_percent,
        )?;

        // TSB (Training Stress Balance) overrides
        Self::apply_env_var(
            "INTELLIGENCE_TSB_HIGHLY_FATIGUED",
            &mut self
                .sleep_recovery
                .training_stress_balance
                .highly_fatigued_tsb,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_TSB_FATIGUED",
            &mut self.sleep_recovery.training_stress_balance.fatigued_tsb,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_TSB_FRESH_MIN",
            &mut self.sleep_recovery.training_stress_balance.fresh_tsb_min,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_TSB_FRESH_MAX",
            &mut self.sleep_recovery.training_stress_balance.fresh_tsb_max,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_TSB_DETRAINING",
            &mut self.sleep_recovery.training_stress_balance.detraining_tsb,
        )?;

        // Recovery scoring overrides
        Self::apply_env_var(
            "INTELLIGENCE_RECOVERY_EXCELLENT_THRESHOLD",
            &mut self.sleep_recovery.recovery_scoring.excellent_threshold,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_RECOVERY_GOOD_THRESHOLD",
            &mut self.sleep_recovery.recovery_scoring.good_threshold,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_RECOVERY_FAIR_THRESHOLD",
            &mut self.sleep_recovery.recovery_scoring.fair_threshold,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_RECOVERY_TSB_WEIGHT_FULL",
            &mut self.sleep_recovery.recovery_scoring.tsb_weight_full,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_RECOVERY_SLEEP_WEIGHT_FULL",
            &mut self.sleep_recovery.recovery_scoring.sleep_weight_full,
        )?;
        Self::apply_env_var(
            "INTELLIGENCE_RECOVERY_HRV_WEIGHT_FULL",
            &mut self.sleep_recovery.recovery_scoring.hrv_weight_full,
        )?;

        // Algorithm selection overrides
        Self::apply_env_var("PIERRE_TSS_ALGORITHM", &mut self.algorithms.tss)?;
        Self::apply_env_var("PIERRE_MAXHR_ALGORITHM", &mut self.algorithms.maxhr)?;

        Ok(self)
    }
}

impl Default for IntelligenceConfig<true> {
    fn default() -> Self {
        Self {
            recommendation_engine: RecommendationEngineConfig::default(),
            performance_analyzer: PerformanceAnalyzerConfig::default(),
            goal_engine: GoalEngineConfig::default(),
            weather_analysis: WeatherAnalysisConfig::default(),
            activity_analyzer: ActivityAnalyzerConfig::default(),
            metrics: MetricsConfig::default(),
            sleep_recovery: SleepRecoveryConfig::default(),
            nutrition: NutritionConfig::default(),
            algorithms: AlgorithmConfig::default(),
            _phantom: PhantomData,
        }
    }
}

/// Trait for strategy-based configuration
pub trait IntelligenceStrategy: Send + Sync + 'static {
    /// Get recommendation thresholds for this strategy
    fn recommendation_thresholds(&self) -> &RecommendationThresholds;
    /// Get performance analysis thresholds for this strategy
    fn performance_thresholds(&self) -> &PerformanceThresholds;
    /// Get weather analysis configuration for this strategy
    fn weather_config(&self) -> &WeatherAnalysisConfig;

    /// Check if volume increase recommendation should be triggered based on current weekly distance
    fn should_recommend_volume_increase(&self, current_km: f64) -> bool {
        current_km < self.recommendation_thresholds().low_weekly_distance_km
    }

    /// Check if recovery recommendation should be triggered based on weekly frequency
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
    /// Creates a new conservative training strategy configuration
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
    /// Creates a new aggressive training strategy configuration
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
