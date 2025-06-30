//! Intelligence Configuration Module
//!
//! Provides type-safe, compile-time validated configuration for all intelligence modules
//! including recommendation engine, performance analyzer, goal engine, and weather analysis.

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

/// Main intelligence configuration container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig<const VALIDATED: bool = false> {
    pub recommendation_engine: RecommendationEngineConfig,
    pub performance_analyzer: PerformanceAnalyzerConfig,
    pub goal_engine: GoalEngineConfig,
    pub weather_analysis: WeatherAnalysisConfig,
    pub activity_analyzer: ActivityAnalyzerConfig,
    pub metrics: MetricsConfig,
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
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Apply environment variable overrides
        config = config.apply_env_overrides()?;

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
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

        Ok(())
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(mut self) -> Result<Self, ConfigError> {
        // Recommendation engine overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_RECOMMENDATION_LOW_DISTANCE") {
            self.recommendation_engine.thresholds.low_weekly_distance_km =
                val.parse().map_err(|_| {
                    ConfigError::Parse(
                        "Invalid INTELLIGENCE_RECOMMENDATION_LOW_DISTANCE".to_string(),
                    )
                })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_RECOMMENDATION_HIGH_DISTANCE") {
            self.recommendation_engine
                .thresholds
                .high_weekly_distance_km = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_RECOMMENDATION_HIGH_DISTANCE".to_string())
            })?;
        }

        // Weather analysis overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_WEATHER_IDEAL_MIN_TEMP") {
            self.weather_analysis.temperature.ideal_min_celsius = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_WEATHER_IDEAL_MIN_TEMP".to_string())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_WEATHER_IDEAL_MAX_TEMP") {
            self.weather_analysis.temperature.ideal_max_celsius = val.parse().map_err(|_| {
                ConfigError::Parse("Invalid INTELLIGENCE_WEATHER_IDEAL_MAX_TEMP".to_string())
            })?;
        }

        Ok(self)
    }
}

impl Default for IntelligenceConfig<true> {
    fn default() -> Self {
        Self {
            recommendation_engine: RecommendationEngineConfig {
                thresholds: RecommendationThresholds {
                    low_weekly_distance_km: 20.0,
                    high_weekly_distance_km: 80.0,
                    low_weekly_frequency: 2,
                    high_weekly_frequency: 6,
                    pace_improvement_threshold: 0.05,
                    consistency_threshold: 0.7,
                    rest_day_threshold: 1,
                    volume_increase_threshold: 0.1,
                    intensity_threshold: 0.8,
                },
                weights: RecommendationWeights {
                    distance_weight: 0.3,
                    frequency_weight: 0.25,
                    pace_weight: 0.2,
                    consistency_weight: 0.15,
                    recovery_weight: 0.1,
                },
                limits: RecommendationLimits {
                    max_recommendations_per_category: 3,
                    max_total_recommendations: 10,
                    min_confidence_threshold: 0.6,
                },
                messages: RecommendationMessages {
                    low_distance: "Consider gradually increasing your weekly distance".to_string(),
                    high_distance: "You're covering good distance - focus on quality".to_string(),
                    low_frequency: "Try to add one more training session per week".to_string(),
                    high_frequency: "You're training frequently - ensure adequate recovery"
                        .to_string(),
                    pace_improvement: "Focus on tempo runs to improve your pace".to_string(),
                    consistency_improvement: "Try to maintain a more consistent training schedule"
                        .to_string(),
                    recovery_needed: "Consider adding more recovery time between sessions"
                        .to_string(),
                },
            },
            performance_analyzer: PerformanceAnalyzerConfig {
                trend_analysis: TrendAnalysisConfig {
                    min_data_points: 5,
                    trend_strength_threshold: 0.3,
                    significance_threshold: 0.05,
                    improvement_threshold: 0.02,
                    decline_threshold: -0.02,
                },
                statistical: StatisticalConfig {
                    confidence_level: 0.95,
                    outlier_threshold: 2.0,
                    smoothing_factor: 0.3,
                },
                thresholds: PerformanceThresholds {
                    significant_improvement: 0.05,
                    significant_decline: -0.05,
                    pace_variance_threshold: 0.2,
                    endurance_threshold: 0.8,
                },
            },
            goal_engine: GoalEngineConfig {
                feasibility: FeasibilityConfig {
                    min_success_probability: 0.6,
                    conservative_multiplier: 0.8,
                    aggressive_multiplier: 1.3,
                    injury_risk_threshold: 0.3,
                },
                suggestion: SuggestionConfig {
                    max_goals_per_type: 3,
                    difficulty_distribution: DifficultyDistribution {
                        easy_percentage: 0.4,
                        moderate_percentage: 0.4,
                        hard_percentage: 0.2,
                    },
                    timeframe_preferences: TimeframePreferences {
                        short_term_weeks: 4,
                        medium_term_weeks: 12,
                        long_term_weeks: 26,
                    },
                },
                progression: ProgressionConfig {
                    weekly_increase_limit: 0.1,
                    monthly_increase_limit: 0.2,
                    deload_frequency_weeks: 4,
                },
            },
            weather_analysis: WeatherAnalysisConfig {
                temperature: TemperatureConfig {
                    ideal_min_celsius: 10.0,
                    ideal_max_celsius: 20.0,
                    cold_threshold_celsius: 5.0,
                    hot_threshold_celsius: 25.0,
                    extreme_cold_celsius: -5.0,
                    extreme_hot_celsius: 35.0,
                },
                conditions: WeatherConditionsConfig {
                    high_humidity_threshold: 80.0,
                    strong_wind_threshold: 20.0,
                    precipitation_impact_factor: 0.8,
                },
                impact: WeatherImpactConfig {
                    temperature_impact_weight: 0.4,
                    humidity_impact_weight: 0.3,
                    wind_impact_weight: 0.2,
                    precipitation_impact_weight: 0.1,
                },
            },
            activity_analyzer: ActivityAnalyzerConfig {
                analysis: ActivityAnalysisConfig {
                    min_duration_seconds: 300, // 5 minutes
                    max_reasonable_pace: 15.0, // 15 min/km
                    heart_rate_zones: HeartRateZonesConfig {
                        zone1_max_percentage: 60.0,
                        zone2_max_percentage: 70.0,
                        zone3_max_percentage: 80.0,
                        zone4_max_percentage: 90.0,
                        zone5_max_percentage: 100.0,
                    },
                    power_zones: PowerZonesConfig {
                        zone1_max_percentage: 55.0,
                        zone2_max_percentage: 75.0,
                        zone3_max_percentage: 90.0,
                        zone4_max_percentage: 105.0,
                        zone5_max_percentage: 150.0,
                    },
                },
                scoring: ActivityScoringConfig {
                    efficiency_weight: 0.3,
                    intensity_weight: 0.3,
                    duration_weight: 0.2,
                    consistency_weight: 0.2,
                },
                insights: ActivityInsightsConfig {
                    min_confidence_threshold: 0.7,
                    max_insights_per_activity: 5,
                    severity_thresholds: SeverityThresholds {
                        info_threshold: 0.3,
                        warning_threshold: 0.7,
                        critical_threshold: 0.9,
                    },
                },
            },
            metrics: MetricsConfig {
                calculation: MetricsCalculationConfig {
                    smoothing_window_size: 7,
                    outlier_detection_threshold: 2.5,
                    missing_data_interpolation: true,
                },
                validation: MetricsValidationConfig {
                    max_heart_rate: 220,
                    min_heart_rate: 40,
                    max_pace_min_per_km: 20.0,
                    min_pace_min_per_km: 2.0,
                },
                aggregation: MetricsAggregationConfig {
                    weekly_aggregation_method: "average".to_string(),
                    monthly_aggregation_method: "weighted_average".to_string(),
                    trend_calculation_method: "linear_regression".to_string(),
                },
            },
            _phantom: PhantomData,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = IntelligenceConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_distance_range() {
        let mut config = IntelligenceConfig::default();
        config
            .recommendation_engine
            .thresholds
            .low_weekly_distance_km = 100.0;
        config
            .recommendation_engine
            .thresholds
            .high_weekly_distance_km = 50.0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_weights() {
        let mut config = IntelligenceConfig::default();
        config.recommendation_engine.weights.distance_weight = 0.8;
        config.recommendation_engine.weights.frequency_weight = 0.8;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_conservative_strategy() {
        let strategy = ConservativeStrategy::new();
        assert_eq!(
            strategy.recommendation_thresholds().low_weekly_distance_km,
            15.0
        );
        assert!(strategy.should_recommend_volume_increase(10.0));
        assert!(!strategy.should_recommend_volume_increase(20.0));
    }

    #[test]
    fn test_aggressive_strategy() {
        let strategy = AggressiveStrategy::new();
        assert_eq!(
            strategy.recommendation_thresholds().low_weekly_distance_km,
            40.0
        );
        assert!(strategy.should_recommend_volume_increase(30.0));
        assert!(!strategy.should_recommend_volume_increase(50.0));
    }
}
