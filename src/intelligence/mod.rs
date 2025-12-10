// ABOUTME: Fitness intelligence module containing analysis algorithms and data processing
// ABOUTME: Core engine for fitness metrics calculation, training zone analysis, and performance insights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Intelligence Module
//!
//! Advanced analytics and intelligence for fitness data analysis.
//! Provides sophisticated analysis tools for Claude/LLM integration via MCP.
//!
//! This module includes:
//! - Activity analysis and insights
//! - Performance trend analysis
//! - Goal tracking and progress monitoring
//! - Training recommendations
//! - Advanced metrics calculation

use chrono::{DateTime, Utc};
use physiological_constants::fitness_score_thresholds::{
    EXCELLENT_PERFORMANCE_THRESHOLD, FITNESS_IMPROVING_THRESHOLD, FITNESS_STABLE_THRESHOLD,
    GOOD_PERFORMANCE_THRESHOLD, MIN_STATISTICAL_SIGNIFICANCE_POINTS,
    MODERATE_PERFORMANCE_THRESHOLD, SMALL_DATASET_REDUCTION_FACTOR,
    STATISTICAL_SIGNIFICANCE_THRESHOLD, STRENGTH_ENDURANCE_DIVISOR,
};
use serde::{Deserialize, Serialize};

/// Core activity analyzer for single activity analysis
pub mod analyzer;
/// Insight generation and categorization
pub mod insights;
/// Location and geographic context
pub mod location;
/// Weather data integration and analysis
pub mod weather;

// Re-enabling advanced intelligence modules

/// Advanced activity analysis with contextual insights
pub mod activity_analyzer;
/// Goal tracking and progress monitoring engine
pub mod goal_engine;
/// Performance metrics calculation
pub mod metrics;
/// Performance analysis engine (v1)
pub mod performance_analyzer;
/// Physiological constants and thresholds
pub mod physiological_constants;
/// Training recommendation generation
pub mod recommendation_engine;

// New improved modules addressing critical issues

/// Configuration for analysis parameters and thresholds
pub mod analysis_config;
/// Safe metrics extraction with validation
pub mod metrics_extractor;
/// Training pattern detection and analysis
pub mod pattern_detection;
/// Performance analyzer v2 with improved algorithms
pub mod performance_analyzer_v2;
/// Performance prediction and race time estimation
pub mod performance_prediction;
/// Statistical analysis and regression
pub mod statistical_analysis;
/// Training load calculation and monitoring
pub mod training_load;

// Sleep and recovery analysis modules (Phase 1)

/// Recovery score calculation and recommendations
pub mod recovery_calculator;
/// Sleep quality analysis and HRV tracking
pub mod sleep_analysis;

// Nutrition analysis module (Phase 2)

/// Nutrition needs calculation and meal planning
pub mod nutrition_calculator;
/// Recipe management with training-aware suggestions
pub mod recipes;

// Algorithm selection and pluggable implementations

/// Pluggable algorithms for FTP, LTHR, VO2max, etc.
pub mod algorithms;

// Activity analysis capabilities

/// Trait for activity analysis implementations
pub use activity_analyzer::ActivityAnalyzerTrait;
/// Advanced activity analyzer with contextual insights
pub use activity_analyzer::AdvancedActivityAnalyzer;
/// Core single-activity analyzer
pub use analyzer::ActivityAnalyzer;

// Goal engine for training targets and progress tracking

/// Type of goal adjustment (increase/decrease/maintain)
pub use goal_engine::AdjustmentType;
/// Advanced goal tracking engine
pub use goal_engine::AdvancedGoalEngine;
/// Goal adjustment recommendation
pub use goal_engine::GoalAdjustment;
/// Goal difficulty classification
pub use goal_engine::GoalDifficulty;
/// Trait for goal engine implementations
pub use goal_engine::GoalEngineTrait;
/// Suggested goal for user
pub use goal_engine::GoalSuggestion;

// Insights generation and analysis

/// Fitness insight with type and confidence
pub use insights::Insight;

// Metrics calculation and zone analysis

/// Advanced metrics beyond basic stats
pub use metrics::AdvancedMetrics;
/// Calculator for fitness metrics
pub use metrics::MetricsCalculator;
/// Training zone analysis results
pub use metrics::ZoneAnalysis;

// Performance analysis (v1) - avoiding conflicting types with v2

/// Advanced performance analyzer with trends
pub use performance_analyzer::AdvancedPerformanceAnalyzer;
/// Trait for performance analyzer implementations
pub use performance_analyzer::PerformanceAnalyzerTrait;

// Recommendation engine for training suggestions

/// Advanced training recommendation engine
pub use recommendation_engine::AdvancedRecommendationEngine;
/// Trait for recommendation engine implementations
pub use recommendation_engine::RecommendationEngineTrait;

// Re-export improved modules with v2 types (preferred versions)

/// Configuration for analysis parameters
pub use analysis_config::AnalysisConfig;
/// Analysis configuration errors
pub use analysis_config::AnalysisConfigError;
/// Confidence level for insights
pub use analysis_config::ConfidenceLevel;
/// Summary of extracted metric
pub use metrics_extractor::MetricSummary;
/// Type of fitness metric
pub use metrics_extractor::MetricType;
/// Safe metric extraction with validation
pub use metrics_extractor::SafeMetricExtractor;
/// Hard-easy training pattern detection
pub use pattern_detection::HardEasyPattern;
/// Signs of overtraining
pub use pattern_detection::OvertrainingSignals;
/// Pattern detection engine
pub use pattern_detection::PatternDetector;
/// Volume progression pattern analysis
pub use pattern_detection::VolumeProgressionPattern;
/// Volume trend direction
pub use pattern_detection::VolumeTrend;
/// Weekly schedule pattern
pub use pattern_detection::WeeklySchedulePattern;
/// Fitness goal with target and deadline
pub use performance_analyzer_v2::ActivityGoal;
/// Overall fitness score
pub use performance_analyzer_v2::FitnessScore;
/// Performance analyzer v2 (preferred)
pub use performance_analyzer_v2::PerformanceAnalyzerV2;
/// Performance prediction with confidence
pub use performance_analyzer_v2::PerformancePrediction;
/// Training load analysis results
pub use performance_analyzer_v2::TrainingLoadAnalysis;
/// Weekly training load metrics
pub use performance_analyzer_v2::WeeklyLoad;
/// Race time predictor
pub use performance_prediction::PerformancePredictor;
/// Race time predictions for distances
pub use performance_prediction::RacePredictions;
/// Regression analysis result
pub use statistical_analysis::RegressionResult;
/// Statistical significance level
pub use statistical_analysis::SignificanceLevel;
/// Statistical analysis engine
pub use statistical_analysis::StatisticalAnalyzer;
/// Overtraining risk assessment
pub use training_load::OvertrainingRisk;
/// Risk level classification
pub use training_load::RiskLevel;
/// Training load data structure
pub use training_load::TrainingLoad;
/// Training load calculator
pub use training_load::TrainingLoadCalculator;
/// Current training status
pub use training_load::TrainingStatus;
/// TSS data point for training stress
pub use training_load::TssDataPoint;

// Re-export sleep and recovery types

/// Recovery score calculator
pub use recovery_calculator::RecoveryCalculator;
/// Recovery category classification
pub use recovery_calculator::RecoveryCategory;
/// Recovery component scores
pub use recovery_calculator::RecoveryComponents;
/// Overall recovery score with recommendations
pub use recovery_calculator::RecoveryScore;
/// Rest day recommendation
pub use recovery_calculator::RestDayRecommendation;
/// Training readiness level
pub use recovery_calculator::TrainingReadiness;
/// HRV-based recovery status
pub use sleep_analysis::HrvRecoveryStatus;
/// HRV trend direction
pub use sleep_analysis::HrvTrend;
/// HRV trend analysis results
pub use sleep_analysis::HrvTrendAnalysis;
/// Sleep quality analyzer
pub use sleep_analysis::SleepAnalyzer;
/// Sleep session data
pub use sleep_analysis::SleepData;
/// Sleep quality category
pub use sleep_analysis::SleepQualityCategory;
/// Sleep quality score with insights
pub use sleep_analysis::SleepQualityScore;

// Re-export nutrition types

/// Calculate carbohydrate needs
pub use nutrition_calculator::calculate_carb_needs;
/// Calculate complete daily nutrition needs
pub use nutrition_calculator::calculate_daily_nutrition_needs;
/// Calculate fat needs
pub use nutrition_calculator::calculate_fat_needs;
/// Calculate BMR using Mifflin-St Jeor equation
pub use nutrition_calculator::calculate_mifflin_st_jeor;
/// Calculate nutrient timing recommendations
pub use nutrition_calculator::calculate_nutrient_timing;
/// Calculate protein needs
pub use nutrition_calculator::calculate_protein_needs;
/// Calculate TDEE (Total Daily Energy Expenditure)
pub use nutrition_calculator::calculate_tdee;
/// Activity level for TDEE calculation
pub use nutrition_calculator::ActivityLevel;
/// Complete daily nutrition needs
pub use nutrition_calculator::DailyNutritionNeeds;
/// Parameters for nutrition calculation
pub use nutrition_calculator::DailyNutritionParams;
/// Gender for BMR calculation
pub use nutrition_calculator::Gender;
/// Macronutrient percentages
pub use nutrition_calculator::MacroPercentages;
/// Nutrient timing plan for workouts
pub use nutrition_calculator::NutrientTimingPlan;
/// Post-workout nutrition recommendations
pub use nutrition_calculator::PostWorkoutNutrition;
/// Pre-workout nutrition recommendations
pub use nutrition_calculator::PreWorkoutNutrition;
/// Protein distribution strategy
pub use nutrition_calculator::ProteinDistribution;
/// Training goal for nutrition planning
pub use nutrition_calculator::TrainingGoal;
/// Workout intensity level
pub use nutrition_calculator::WorkoutIntensity;

// Re-export recipe types

/// Unit conversion for recipe ingredients
pub use recipes::convert_to_grams;
/// Conversion error types
pub use recipes::ConversionError;
/// Dietary restriction for recipe filtering
pub use recipes::DietaryRestriction;
/// Ingredient density lookup
pub use recipes::IngredientDensity;
/// Ingredient measurement unit
pub use recipes::IngredientUnit;
/// Macro nutrient targets
pub use recipes::MacroTargets;
/// Meal timing context for training-aware recipes
pub use recipes::MealTiming;
/// A complete recipe with ingredients and instructions
pub use recipes::Recipe;
/// Constraints for recipe suggestions
pub use recipes::RecipeConstraints;
/// Single ingredient in a recipe
pub use recipes::RecipeIngredient;
/// Cooking skill level
pub use recipes::SkillLevel;
/// USDA-validated nutrition data
pub use recipes::ValidatedNutrition;

// Re-export configuration types for external use
pub use crate::config::intelligence::{
    AggressiveStrategy, ConfigError, ConservativeStrategy, DefaultStrategy, IntelligenceConfig,
    IntelligenceStrategy,
};

/// Activity intelligence summary with insights and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityIntelligence {
    /// Natural language summary of the activity
    pub summary: String,

    /// Key insights extracted from the activity
    pub key_insights: Vec<Insight>,

    /// Performance metrics and indicators
    pub performance_indicators: PerformanceMetrics,

    /// Contextual factors affecting the activity
    pub contextual_factors: ContextualFactors,

    /// Timestamp when the analysis was generated
    pub generated_at: DateTime<Utc>,
}

/// Performance metrics derived from activity analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    /// Relative effort (1-10 scale)
    pub relative_effort: Option<f32>,

    /// Zone distribution (percentage in each zone)
    pub zone_distribution: Option<ZoneDistribution>,

    /// Personal records achieved
    pub personal_records: Vec<PersonalRecord>,

    /// Efficiency score (0-100)
    pub efficiency_score: Option<f32>,

    /// Comparison with recent activities
    pub trend_indicators: TrendIndicators,
}

/// Heart rate or power zone distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDistribution {
    /// Percentage of time in Zone 1 (Recovery, <60% max HR)
    pub zone1_recovery: f32,
    /// Percentage of time in Zone 2 (Endurance, 60-70% max HR)
    pub zone2_endurance: f32,
    /// Percentage of time in Zone 3 (Tempo, 70-80% max HR)
    pub zone3_tempo: f32,
    /// Percentage of time in Zone 4 (Threshold, 80-90% max HR)
    pub zone4_threshold: f32,
    /// Percentage of time in Zone 5 (VO2 Max, >90% max HR)
    pub zone5_vo2max: f32,
}

/// Personal record information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecord {
    /// Type of record (e.g., `fastest_5k`, `longest_run`)
    pub record_type: String,
    /// Record value
    pub value: f64,
    /// Unit of measurement (e.g., "seconds", "meters")
    pub unit: String,
    /// Previous best value before this record
    pub previous_best: Option<f64>,
    /// Improvement over previous best as percentage
    pub improvement_percentage: Option<f32>,
}

/// Trend indicators comparing to recent activities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrendIndicators {
    /// Trend in pace performance
    pub pace_trend: TrendDirection,
    /// Trend in effort levels
    pub effort_trend: TrendDirection,
    /// Trend in distance covered
    pub distance_trend: TrendDirection,
    /// Consistency score (0-100, higher is more consistent)
    pub consistency_score: f32,
}

/// Direction of a trend
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    /// Performance is improving
    Improving,
    /// Performance is stable
    #[default]
    Stable,
    /// Performance is declining
    Declining,
}

/// Contextual factors that might affect performance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextualFactors {
    /// Weather conditions during the activity
    pub weather: Option<WeatherConditions>,
    /// Location where the activity took place
    pub location: Option<LocationContext>,
    /// Time of day when activity occurred
    pub time_of_day: TimeOfDay,
    /// Number of days since last activity
    pub days_since_last_activity: Option<i32>,
    /// Weekly training load context
    pub weekly_load: Option<ContextualWeeklyLoad>,
}

/// Weather conditions during activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    /// Temperature in degrees Celsius
    pub temperature_celsius: f32,
    /// Relative humidity as percentage
    pub humidity_percentage: Option<f32>,
    /// Wind speed in kilometers per hour
    pub wind_speed_kmh: Option<f32>,
    /// Weather conditions description (e.g., "sunny", "rainy", "cloudy")
    pub conditions: String,
}

/// Location context for the activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationContext {
    /// City name
    pub city: Option<String>,
    /// State or region name
    pub region: Option<String>,
    /// Country name
    pub country: Option<String>,
    /// Trail or route name if applicable
    pub trail_name: Option<String>,
    /// Terrain type (e.g., "road", "trail", "track")
    pub terrain_type: Option<String>,
    /// Human-readable display name for the location
    pub display_name: String,
}

/// Time of day categorization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    /// Early morning (5-7 AM)
    EarlyMorning,
    /// Morning (7-11 AM)
    #[default]
    Morning,
    /// Midday (11 AM - 2 PM)
    Midday,
    /// Afternoon (2-6 PM)
    Afternoon,
    /// Evening (6-9 PM)
    Evening,
    /// Night (9 PM - 5 AM)
    Night,
}

/// Weekly training load summary for contextual factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualWeeklyLoad {
    /// Total distance covered this week in kilometers
    pub total_distance_km: f64,
    /// Total training duration this week in hours
    pub total_duration_hours: f64,
    /// Number of activities completed this week
    pub activity_count: i32,
    /// Trend in training load compared to previous weeks
    pub load_trend: TrendDirection,
}

impl ActivityIntelligence {
    /// Create a new activity intelligence analysis
    #[must_use]
    pub fn new(
        summary: String,
        insights: Vec<Insight>,
        performance: PerformanceMetrics,
        context: ContextualFactors,
    ) -> Self {
        Self {
            summary,
            key_insights: insights,
            performance_indicators: performance,
            contextual_factors: context,
            generated_at: Utc::now(),
        }
    }

    /// Create an empty `ActivityIntelligence` instance for default initialization
    #[must_use]
    pub fn create_empty() -> Self {
        Self {
            summary: "No analysis available".to_owned(),
            key_insights: vec![],
            performance_indicators: PerformanceMetrics {
                relative_effort: None,
                zone_distribution: None,
                personal_records: vec![],
                efficiency_score: None,
                trend_indicators: TrendIndicators {
                    pace_trend: TrendDirection::Stable,
                    effort_trend: TrendDirection::Stable,
                    distance_trend: TrendDirection::Stable,
                    consistency_score: 0.0,
                },
            },
            contextual_factors: ContextualFactors {
                weather: None,
                location: None,
                time_of_day: TimeOfDay::Morning,
                days_since_last_activity: None,
                weekly_load: None,
            },
            generated_at: Utc::now(),
        }
    }
}

// === ADVANCED ANALYTICS TYPES ===
// Re-enabled for full AI functionality

/// Time frame for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeFrame {
    /// Last 7 days
    Week,
    /// Last 30 days
    Month,
    /// Last 90 days
    Quarter,
    /// Last 180 days
    SixMonths,
    /// Last 365 days
    Year,
    /// Custom date range
    Custom {
        /// Start of the time range
        start: DateTime<Utc>,
        /// End of the time range
        end: DateTime<Utc>,
    },
}

impl TimeFrame {
    /// Get the duration in days
    #[must_use]
    pub fn to_days(&self) -> i64 {
        match self {
            Self::Week => 7,
            Self::Month => 30,
            Self::Quarter => 90,
            Self::SixMonths => 180,
            Self::Year => 365,
            Self::Custom { start, end } => (*end - *start).num_days(),
        }
    }

    /// Get start date relative to now
    #[must_use]
    pub fn start_date(&self) -> DateTime<Utc> {
        match self {
            Self::Week => Utc::now() - chrono::Duration::days(7),
            Self::Month => Utc::now() - chrono::Duration::days(30),
            Self::Quarter => Utc::now() - chrono::Duration::days(90),
            Self::SixMonths => Utc::now() - chrono::Duration::days(180),
            Self::Year => Utc::now() - chrono::Duration::days(365),
            Self::Custom { start, .. } => *start,
        }
    }

    /// Get end date
    #[must_use]
    pub fn end_date(&self) -> DateTime<Utc> {
        match self {
            Self::Custom { end, .. } => *end,
            _ => Utc::now(),
        }
    }
}

/// Confidence level for insights and recommendations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Confidence {
    /// Low confidence (25%)
    Low = 1,
    /// Medium confidence (50%)
    Medium = 2,
    /// High confidence (75%)
    High = 3,
    /// Very high confidence (95%)
    VeryHigh = 4,
}

impl Confidence {
    /// Convert confidence to a 0-1 score
    #[must_use]
    pub const fn as_score(&self) -> f64 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.50,
            Self::High => 0.75,
            Self::VeryHigh => 0.95,
        }
    }

    /// Create confidence from a 0-1 score
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if score >= EXCELLENT_PERFORMANCE_THRESHOLD {
            Self::VeryHigh
        } else if score >= GOOD_PERFORMANCE_THRESHOLD {
            Self::High
        } else if score >= MODERATE_PERFORMANCE_THRESHOLD {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

/// Enhanced activity insights with advanced analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInsights {
    /// Unique identifier for the activity
    pub activity_id: String,
    /// Overall performance score (0-100)
    pub overall_score: f64,
    /// List of advanced insights discovered
    pub insights: Vec<AdvancedInsight>,
    /// Advanced performance metrics
    pub metrics: AdvancedMetrics,
    /// Actionable recommendations for improvement
    pub recommendations: Vec<String>,
    /// Detected anomalies in the activity data
    pub anomalies: Vec<Anomaly>,
}

/// Advanced insight with confidence and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedInsight {
    /// Type of insight (e.g., `pace_improvement`, `fatigue_warning`)
    pub insight_type: String,
    /// Human-readable insight message
    pub message: String,
    /// Confidence level in this insight
    pub confidence: Confidence,
    /// Severity/importance of the insight
    pub severity: InsightSeverity,
    /// Additional metadata for the insight
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Severity level for insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightSeverity {
    /// Informational insight
    Info,
    /// Warning that needs attention
    Warning,
    /// Critical issue requiring immediate action
    Critical,
}

/// Detected anomaly in activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Type of anomaly detected
    pub anomaly_type: String,
    /// Description of the anomaly
    pub description: String,
    /// Severity of the anomaly
    pub severity: InsightSeverity,
    /// Confidence in the anomaly detection
    pub confidence: Confidence,
    /// Metric that shows the anomaly
    pub affected_metric: String,
    /// Expected value for the metric
    pub expected_value: Option<f64>,
    /// Actual observed value
    pub actual_value: Option<f64>,
}

/// Performance trend analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Time period analyzed
    pub timeframe: TimeFrame,
    /// Metric being analyzed
    pub metric: String,
    /// Direction of the trend
    pub trend_direction: TrendDirection,
    /// Strength of the trend (0-1, higher is stronger)
    pub trend_strength: f64,
    /// Statistical significance (p-value)
    pub statistical_significance: f64,
    /// Individual data points in the trend
    pub data_points: Vec<TrendDataPoint>,
    /// Insights derived from the trend
    pub insights: Vec<AdvancedInsight>,
}

/// Data point in a trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    /// Date of this data point
    pub date: DateTime<Utc>,
    /// Raw value at this point
    pub value: f64,
    /// Smoothed value (moving average) if available
    pub smoothed_value: Option<f64>,
}

/// Fitness goal definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    /// Unique identifier for the goal
    pub id: String,
    /// User who owns this goal
    pub user_id: String,
    /// Goal title
    pub title: String,
    /// Detailed description of the goal
    pub description: String,
    /// Type and specifics of the goal
    pub goal_type: GoalType,
    /// Target value to achieve
    pub target_value: f64,
    /// Target completion date
    pub target_date: DateTime<Utc>,
    /// Current progress value
    pub current_value: f64,
    /// When the goal was created
    pub created_at: DateTime<Utc>,
    /// When the goal was last updated
    pub updated_at: DateTime<Utc>,
    /// Current status of the goal
    pub status: GoalStatus,
}

/// Type of fitness goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    /// Distance goal (e.g., run 100km this month)
    Distance {
        /// Sport type
        sport: String,
        /// Time period for the goal
        timeframe: TimeFrame,
    },
    /// Time goal (e.g., run 5km in under 20 minutes)
    Time {
        /// Sport type
        sport: String,
        /// Target distance
        distance: f64,
    },
    /// Frequency goal (e.g., run 3 times per week)
    Frequency {
        /// Sport type
        sport: String,
        /// Target sessions per week
        sessions_per_week: i32,
    },
    /// Performance improvement goal
    Performance {
        /// Performance metric to improve
        metric: String,
        /// Target improvement percentage
        improvement_percent: f64,
    },
    /// Custom user-defined goal
    Custom {
        /// Custom metric name
        metric: String,
        /// Unit of measurement
        unit: String,
    },
}

/// Status of a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    /// Goal is active and in progress
    Active,
    /// Goal has been completed
    Completed,
    /// Goal is temporarily paused
    Paused,
    /// Goal was cancelled
    Cancelled,
}

/// Progress report for a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReport {
    /// ID of the goal being reported on
    pub goal_id: String,
    /// Progress as a percentage (0-100)
    pub progress_percentage: f64,
    /// Estimated completion date based on current progress
    pub completion_date_estimate: Option<DateTime<Utc>>,
    /// Milestones that have been achieved
    pub milestones_achieved: Vec<Milestone>,
    /// Insights about goal progress
    pub insights: Vec<AdvancedInsight>,
    /// Recommendations for achieving the goal
    pub recommendations: Vec<String>,
    /// Whether the goal is on track for completion
    pub on_track: bool,
}

/// Milestone in goal progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Name of the milestone
    pub name: String,
    /// Target value for this milestone
    pub target_value: f64,
    /// When the milestone was achieved (if achieved)
    pub achieved_date: Option<DateTime<Utc>>,
    /// Whether this milestone has been achieved
    pub achieved: bool,
}

/// Training recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecommendation {
    /// Type of recommendation
    pub recommendation_type: RecommendationType,
    /// Recommendation title
    pub title: String,
    /// Detailed description of the recommendation
    pub description: String,
    /// Priority level for acting on this recommendation
    pub priority: RecommendationPriority,
    /// Confidence in this recommendation
    pub confidence: Confidence,
    /// Explanation of why this recommendation is made
    pub rationale: String,
    /// Specific actionable steps to implement the recommendation
    pub actionable_steps: Vec<String>,
}

/// Type of training recommendation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecommendationType {
    /// Recommendation about training intensity
    Intensity,
    /// Recommendation about training volume
    Volume,
    /// Recommendation about recovery and rest
    Recovery,
    /// Recommendation about technique and form
    Technique,
    /// Recommendation about nutrition and fueling
    Nutrition,
    /// Recommendation about equipment
    Equipment,
    /// Recommendation about training strategy
    Strategy,
}

/// Priority level for recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    /// Low priority, nice to have
    Low,
    /// Medium priority, should consider
    Medium,
    /// High priority, important to address
    High,
    /// Critical priority, urgent action needed
    Critical,
}

/// User fitness profile for personalized analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFitnessProfile {
    /// Unique user identifier
    pub user_id: String,
    /// User's age in years
    pub age: Option<i32>,
    /// User's gender
    pub gender: Option<String>,
    /// User's weight in kilograms
    pub weight: Option<f64>,
    /// User's height in centimeters
    pub height: Option<f64>,
    /// Current fitness level
    pub fitness_level: FitnessLevel,
    /// List of sports the user primarily participates in
    pub primary_sports: Vec<String>,
    /// Months of training history
    pub training_history_months: i32,
    /// User's training preferences and constraints
    pub preferences: UserPreferences,
}

/// Fitness level classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FitnessLevel {
    /// New to training, building base fitness
    Beginner,
    /// Some training experience, consistent activity
    Intermediate,
    /// Experienced athlete with solid training background
    Advanced,
    /// Elite/professional level athlete
    Elite,
}

/// User preferences for training and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Preferred units (metric/imperial)
    pub preferred_units: String,
    /// Areas the user wants to focus training on
    pub training_focus: Vec<String>,
    /// History of injuries to consider
    pub injury_history: Vec<String>,
    /// Available time for training
    pub time_availability: TimeAvailability,
}

/// Available time for training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeAvailability {
    /// Total hours available per week
    pub hours_per_week: f64,
    /// Preferred days for training
    pub preferred_days: Vec<String>,
    /// Preferred session duration in minutes
    pub preferred_duration_minutes: Option<i32>,
}
