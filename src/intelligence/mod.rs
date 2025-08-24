// ABOUTME: Fitness intelligence module containing analysis algorithms and data processing
// ABOUTME: Core engine for fitness metrics calculation, training zone analysis, and performance insights
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

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

pub mod analyzer;
pub mod insights;
pub mod location;
pub mod weather;
// Re-enabling advanced intelligence modules
pub mod activity_analyzer;
pub mod goal_engine;
pub mod metrics;
pub mod performance_analyzer;
pub mod physiological_constants;
pub mod recommendation_engine;

pub use activity_analyzer::*;
pub use analyzer::ActivityAnalyzer;
pub use goal_engine::*;
pub use insights::Insight;
pub use metrics::*;
pub use performance_analyzer::*;
pub use recommendation_engine::*;

// Re-export configuration types for external use
pub use crate::config::intelligence_config::{
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
    pub zone1_recovery: f32,
    pub zone2_endurance: f32,
    pub zone3_tempo: f32,
    pub zone4_threshold: f32,
    pub zone5_vo2max: f32,
}

/// Personal record information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecord {
    pub record_type: String,
    pub value: f64,
    pub unit: String,
    pub previous_best: Option<f64>,
    pub improvement_percentage: Option<f32>,
}

/// Trend indicators comparing to recent activities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrendIndicators {
    pub pace_trend: TrendDirection,
    pub effort_trend: TrendDirection,
    pub distance_trend: TrendDirection,
    pub consistency_score: f32, // 0-100
}

/// Direction of a trend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Improving,
    #[default]
    Stable,
    Declining,
}

/// Contextual factors that might affect performance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextualFactors {
    pub weather: Option<WeatherConditions>,
    pub location: Option<LocationContext>,
    pub time_of_day: TimeOfDay,
    pub days_since_last_activity: Option<i32>,
    pub weekly_load: Option<WeeklyLoad>,
}

/// Weather conditions during activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    pub temperature_celsius: f32,
    pub humidity_percentage: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub conditions: String, // "sunny", "rainy", "cloudy", etc.
}

/// Location context for the activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationContext {
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub trail_name: Option<String>,
    pub terrain_type: Option<String>,
    pub display_name: String,
}

/// Time of day categorization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    EarlyMorning, // 5-7 AM
    #[default]
    Morning, // 7-11 AM
    Midday,       // 11 AM - 2 PM
    Afternoon,    // 2-6 PM
    Evening,      // 6-9 PM
    Night,        // 9 PM - 5 AM
}

/// Weekly training load summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyLoad {
    pub total_distance_km: f64,
    pub total_duration_hours: f64,
    pub activity_count: i32,
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
            summary: "No analysis available".to_string(),
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
    Week,
    Month,
    Quarter,
    SixMonths,
    Year,
    Custom {
        start: DateTime<Utc>,
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
    Low = 1,
    Medium = 2,
    High = 3,
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
    pub activity_id: String,
    pub overall_score: f64,
    pub insights: Vec<AdvancedInsight>,
    pub metrics: AdvancedMetrics,
    pub recommendations: Vec<String>,
    pub anomalies: Vec<Anomaly>,
}

/// Advanced insight with confidence and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedInsight {
    pub insight_type: String,
    pub message: String,
    pub confidence: Confidence,
    pub severity: InsightSeverity,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Severity level for insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightSeverity {
    Info,
    Warning,
    Critical,
}

/// Detected anomaly in activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub anomaly_type: String,
    pub description: String,
    pub severity: InsightSeverity,
    pub confidence: Confidence,
    pub affected_metric: String,
    pub expected_value: Option<f64>,
    pub actual_value: Option<f64>,
}

/// Performance trend analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub timeframe: TimeFrame,
    pub metric: String,
    pub trend_direction: TrendDirection,
    pub trend_strength: f64,
    pub statistical_significance: f64,
    pub data_points: Vec<TrendDataPoint>,
    pub insights: Vec<AdvancedInsight>,
}

/// Data point in a trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub date: DateTime<Utc>,
    pub value: f64,
    pub smoothed_value: Option<f64>,
}

/// Fitness goal definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: String,
    pub goal_type: GoalType,
    pub target_value: f64,
    pub target_date: DateTime<Utc>,
    pub current_value: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: GoalStatus,
}

/// Type of fitness goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    Distance {
        sport: String,
        timeframe: TimeFrame,
    },
    Time {
        sport: String,
        distance: f64,
    },
    Frequency {
        sport: String,
        sessions_per_week: i32,
    },
    Performance {
        metric: String,
        improvement_percent: f64,
    },
    Custom {
        metric: String,
        unit: String,
    },
}

/// Status of a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Completed,
    Paused,
    Cancelled,
}

/// Progress report for a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReport {
    pub goal_id: String,
    pub progress_percentage: f64,
    pub completion_date_estimate: Option<DateTime<Utc>>,
    pub milestones_achieved: Vec<Milestone>,
    pub insights: Vec<AdvancedInsight>,
    pub recommendations: Vec<String>,
    pub on_track: bool,
}

/// Milestone in goal progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub target_value: f64,
    pub achieved_date: Option<DateTime<Utc>>,
    pub achieved: bool,
}

/// Training recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecommendation {
    pub recommendation_type: RecommendationType,
    pub title: String,
    pub description: String,
    pub priority: RecommendationPriority,
    pub confidence: Confidence,
    pub rationale: String,
    pub actionable_steps: Vec<String>,
}

/// Type of training recommendation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecommendationType {
    Intensity,
    Volume,
    Recovery,
    Technique,
    Nutrition,
    Equipment,
    Strategy,
}

/// Priority level for recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// User fitness profile for personalized analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFitnessProfile {
    pub user_id: String,
    pub age: Option<i32>,
    pub gender: Option<String>,
    pub weight: Option<f64>,
    pub height: Option<f64>,
    pub fitness_level: FitnessLevel,
    pub primary_sports: Vec<String>,
    pub training_history_months: i32,
    pub preferences: UserPreferences,
}

/// Fitness level classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FitnessLevel {
    Beginner,
    Intermediate,
    Advanced,
    Elite,
}

/// User preferences for training and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub preferred_units: String,
    pub training_focus: Vec<String>,
    pub injury_history: Vec<String>,
    pub time_availability: TimeAvailability,
}

/// Available time for training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeAvailability {
    pub hours_per_week: f64,
    pub preferred_days: Vec<String>,
    pub preferred_duration_minutes: Option<i32>,
}
