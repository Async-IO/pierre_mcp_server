// ABOUTME: Comprehensive tests for intelligence modules to improve coverage
// ABOUTME: Tests activity analyzer, performance analyzer, and intelligence engines
//! Comprehensive tests for intelligence modules to improve coverage
//!
//! This test suite focuses on intelligence modules (activity analyzer, performance analyzer)
//! which have 44-57% coverage

use chrono::Utc;
use pierre_mcp_server::intelligence::*;
use std::collections::HashMap;

mod common;

// === ActivityIntelligence Tests ===

#[test]
fn test_activity_intelligence_creation() {
    let performance = PerformanceMetrics {
        relative_effort: Some(7.5),
        zone_distribution: Some(ZoneDistribution {
            zone1_recovery: 10.0,
            zone2_endurance: 60.0,
            zone3_tempo: 20.0,
            zone4_threshold: 8.0,
            zone5_vo2max: 2.0,
        }),
        personal_records: vec![PersonalRecord {
            record_type: "fastest_5k".to_string(),
            value: 20.5,
            unit: "minutes".to_string(),
            previous_best: Some(21.2),
            improvement_percentage: Some(3.3),
        }],
        efficiency_score: Some(88.5),
        trend_indicators: TrendIndicators {
            pace_trend: TrendDirection::Improving,
            effort_trend: TrendDirection::Stable,
            distance_trend: TrendDirection::Improving,
            consistency_score: 85.0,
        },
    };

    let context = ContextualFactors {
        weather: Some(WeatherConditions {
            temperature_celsius: 15.0,
            humidity_percentage: Some(65.0),
            wind_speed_kmh: Some(8.0),
            conditions: "partly cloudy".to_string(),
        }),
        location: Some(LocationContext {
            city: Some("Boston".to_string()),
            region: Some("MA".to_string()),
            country: Some("USA".to_string()),
            trail_name: Some("Charles River Trail".to_string()),
            terrain_type: Some("paved".to_string()),
            display_name: "Charles River Trail, Boston, MA".to_string(),
        }),
        time_of_day: TimeOfDay::Morning,
        days_since_last_activity: Some(2),
        weekly_load: Some(WeeklyLoad {
            total_distance_km: 45.0,
            total_duration_hours: 4.5,
            activity_count: 6,
            load_trend: TrendDirection::Stable,
        }),
    };

    let intelligence = ActivityIntelligence::new(
        "Excellent morning run with strong pace and good recovery.".to_string(),
        vec![],
        performance,
        context,
    );

    assert_eq!(
        intelligence.summary,
        "Excellent morning run with strong pace and good recovery."
    );
    assert_eq!(
        intelligence.performance_indicators.relative_effort,
        Some(7.5)
    );
    assert_eq!(
        intelligence.performance_indicators.efficiency_score,
        Some(88.5)
    );
    // Test time of day matches (using pattern matching since TimeOfDay doesn't implement PartialEq)
    match intelligence.contextual_factors.time_of_day {
        TimeOfDay::Morning => (),
        _ => panic!("Expected Morning time of day"),
    }
    assert!(intelligence.generated_at <= Utc::now());
}

#[test]
fn test_zone_distribution_calculations() {
    let zones = ZoneDistribution {
        zone1_recovery: 15.0,
        zone2_endurance: 55.0,
        zone3_tempo: 20.0,
        zone4_threshold: 8.0,
        zone5_vo2max: 2.0,
    };

    // Test individual zones
    assert!((zones.zone1_recovery - 15.0).abs() < f32::EPSILON);
    assert!((zones.zone2_endurance - 55.0).abs() < f32::EPSILON);
    assert!((zones.zone3_tempo - 20.0).abs() < f32::EPSILON);
    assert!((zones.zone4_threshold - 8.0).abs() < f32::EPSILON);
    assert!((zones.zone5_vo2max - 2.0).abs() < f32::EPSILON);

    // Test total adds up to 100%
    let total = zones.zone1_recovery
        + zones.zone2_endurance
        + zones.zone3_tempo
        + zones.zone4_threshold
        + zones.zone5_vo2max;
    assert!((total - 100.0).abs() < f32::EPSILON);
}

#[test]
fn test_trend_indicators() {
    let trends = TrendIndicators {
        pace_trend: TrendDirection::Improving,
        effort_trend: TrendDirection::Declining,
        distance_trend: TrendDirection::Stable,
        consistency_score: 75.5,
    };

    assert_eq!(trends.pace_trend, TrendDirection::Improving);
    assert_eq!(trends.effort_trend, TrendDirection::Declining);
    assert_eq!(trends.distance_trend, TrendDirection::Stable);
    assert!((trends.consistency_score - 75.5).abs() < f32::EPSILON);
}

#[test]
fn test_personal_record() {
    let pr = PersonalRecord {
        record_type: "longest_run".to_string(),
        value: 25.0,
        unit: "km".to_string(),
        previous_best: Some(22.0),
        improvement_percentage: Some(13.6),
    };

    assert_eq!(pr.record_type, "longest_run");
    assert!((pr.value - 25.0).abs() < f64::EPSILON);
    assert_eq!(pr.unit, "km");
    assert!((pr.previous_best.unwrap() - 22.0).abs() < f64::EPSILON);
    assert!((pr.improvement_percentage.unwrap() - 13.6).abs() < f32::EPSILON);
}

// === TimeFrame Tests ===

#[test]
fn test_timeframe_durations() {
    assert_eq!(TimeFrame::Week.to_days(), 7);
    assert_eq!(TimeFrame::Month.to_days(), 30);
    assert_eq!(TimeFrame::Quarter.to_days(), 90);
    assert_eq!(TimeFrame::SixMonths.to_days(), 180);
    assert_eq!(TimeFrame::Year.to_days(), 365);

    let custom = TimeFrame::Custom {
        start: Utc::now() - chrono::Duration::days(14),
        end: Utc::now(),
    };
    assert_eq!(custom.to_days(), 14);
}

#[test]
fn test_timeframe_dates() {
    let now = Utc::now();

    // Test start dates are in the past
    assert!(TimeFrame::Week.start_date() < now);
    assert!(TimeFrame::Month.start_date() < now);
    assert!(TimeFrame::Quarter.start_date() < now);

    // Test end dates (allow small timing differences)
    let end_week = TimeFrame::Week.end_date();
    let end_month = TimeFrame::Month.end_date();
    assert!((end_week - now).num_seconds().abs() < 2);
    assert!((end_month - now).num_seconds().abs() < 2);

    let custom_start = now - chrono::Duration::days(7);
    let custom_end = now - chrono::Duration::days(1);
    let custom = TimeFrame::Custom {
        start: custom_start,
        end: custom_end,
    };
    assert_eq!(custom.start_date(), custom_start);
    assert_eq!(custom.end_date(), custom_end);
}

// === Confidence Tests ===

#[test]
fn test_confidence_scores() {
    assert!((Confidence::Low.as_score() - 0.25).abs() < f64::EPSILON);
    assert!((Confidence::Medium.as_score() - 0.50).abs() < f64::EPSILON);
    assert!((Confidence::High.as_score() - 0.75).abs() < f64::EPSILON);
    assert!((Confidence::VeryHigh.as_score() - 0.95).abs() < f64::EPSILON);

    // Test conversion back from scores (using match since Confidence doesn't implement PartialEq)
    match Confidence::from_score(0.95) {
        Confidence::VeryHigh => (),
        _ => panic!("Expected VeryHigh confidence"),
    }
    match Confidence::from_score(0.75) {
        Confidence::High => (),
        _ => panic!("Expected High confidence"),
    }
    match Confidence::from_score(0.50) {
        Confidence::Medium => (),
        _ => panic!("Expected Medium confidence"),
    }
    match Confidence::from_score(0.25) {
        Confidence::Low => (),
        _ => panic!("Expected Low confidence"),
    }
    match Confidence::from_score(0.10) {
        Confidence::Low => (),
        _ => panic!("Expected Low confidence"),
    }
}

// === Goal Tests ===

#[test]
fn test_goal_creation() {
    let goal = Goal {
        id: "goal_123".to_string(),
        user_id: "user_456".to_string(),
        title: "Run 5K in under 25 minutes".to_string(),
        description: "Improve 5K time for upcoming race".to_string(),
        goal_type: GoalType::Time {
            sport: "running".to_string(),
            distance: 5000.0,
        },
        target_value: 25.0,
        target_date: Utc::now() + chrono::Duration::days(60),
        current_value: 27.5,
        created_at: Utc::now() - chrono::Duration::days(7),
        updated_at: Utc::now(),
        status: GoalStatus::Active,
    };

    assert_eq!(goal.id, "goal_123");
    assert_eq!(goal.title, "Run 5K in under 25 minutes");
    assert!((goal.target_value - 25.0).abs() < f64::EPSILON);
    assert!((goal.current_value - 27.5).abs() < f64::EPSILON);

    match goal.goal_type {
        GoalType::Time { sport, distance } => {
            assert_eq!(sport, "running");
            assert!((distance - 5000.0).abs() < f64::EPSILON);
        }
        _ => panic!("Expected Time goal type"),
    }
}

#[test]
fn test_goal_types() {
    let distance_goal = GoalType::Distance {
        sport: "cycling".to_string(),
        timeframe: TimeFrame::Month,
    };

    let frequency_goal = GoalType::Frequency {
        sport: "swimming".to_string(),
        sessions_per_week: 3,
    };

    let performance_goal = GoalType::Performance {
        metric: "VO2_max".to_string(),
        improvement_percent: 10.0,
    };

    let custom_goal = GoalType::Custom {
        metric: "weekly_distance".to_string(),
        unit: "km".to_string(),
    };

    // Test that all variants are created successfully
    match distance_goal {
        GoalType::Distance { sport, .. } => assert_eq!(sport, "cycling"),
        _ => panic!("Expected Distance goal"),
    }

    match frequency_goal {
        GoalType::Frequency {
            sessions_per_week, ..
        } => assert_eq!(sessions_per_week, 3),
        _ => panic!("Expected Frequency goal"),
    }

    match performance_goal {
        GoalType::Performance {
            improvement_percent,
            ..
        } => assert!((improvement_percent - 10.0).abs() < f64::EPSILON),
        _ => panic!("Expected Performance goal"),
    }

    match custom_goal {
        GoalType::Custom { unit, .. } => assert_eq!(unit, "km"),
        _ => panic!("Expected Custom goal"),
    }
}

#[test]
fn test_progress_report() {
    let milestone1 = Milestone {
        name: "25% Complete".to_string(),
        target_value: 25.0,
        achieved_date: Some(Utc::now() - chrono::Duration::days(14)),
        achieved: true,
    };

    let milestone2 = Milestone {
        name: "50% Complete".to_string(),
        target_value: 50.0,
        achieved_date: None,
        achieved: false,
    };

    let progress = ProgressReport {
        goal_id: "goal_123".to_string(),
        progress_percentage: 35.0,
        completion_date_estimate: Some(Utc::now() + chrono::Duration::days(45)),
        milestones_achieved: vec![milestone1, milestone2],
        insights: vec![],
        recommendations: vec!["Increase training frequency".to_string()],
        on_track: true,
    };

    assert_eq!(progress.goal_id, "goal_123");
    assert!((progress.progress_percentage - 35.0).abs() < f64::EPSILON);
    assert!(progress.on_track);
    assert_eq!(progress.milestones_achieved.len(), 2);
    assert!(progress.milestones_achieved[0].achieved);
    assert!(!progress.milestones_achieved[1].achieved);
}

// === Training Recommendations Tests ===

#[test]
fn test_training_recommendation() {
    let recommendation = TrainingRecommendation {
        recommendation_type: RecommendationType::Intensity,
        title: "Increase Threshold Training".to_string(),
        description: "Add more tempo runs to improve lactate threshold".to_string(),
        priority: RecommendationPriority::High,
        confidence: Confidence::High,
        rationale: "Recent data shows room for improvement in sustained pace".to_string(),
        actionable_steps: vec![
            "Add 2x20min tempo intervals weekly".to_string(),
            "Monitor heart rate zones 3-4".to_string(),
        ],
    };

    assert_eq!(
        recommendation.recommendation_type,
        RecommendationType::Intensity
    );
    assert_eq!(recommendation.title, "Increase Threshold Training");
    assert_eq!(recommendation.actionable_steps.len(), 2);

    match recommendation.priority {
        RecommendationPriority::High => (),
        _ => panic!("Expected High priority"),
    }
}

#[test]
fn test_recommendation_types() {
    let types = [
        RecommendationType::Intensity,
        RecommendationType::Volume,
        RecommendationType::Recovery,
        RecommendationType::Technique,
        RecommendationType::Nutrition,
        RecommendationType::Equipment,
        RecommendationType::Strategy,
    ];

    assert_eq!(types.len(), 7);
    assert!(types.contains(&RecommendationType::Intensity));
    assert!(types.contains(&RecommendationType::Recovery));
    assert!(types.contains(&RecommendationType::Nutrition));
}

// === User Fitness Profile Tests ===

#[test]
fn test_user_fitness_profile() {
    let preferences = UserPreferences {
        preferred_units: "metric".to_string(),
        training_focus: vec!["endurance".to_string(), "speed".to_string()],
        injury_history: vec!["knee".to_string()],
        time_availability: TimeAvailability {
            hours_per_week: 8.0,
            preferred_days: vec![
                "Tuesday".to_string(),
                "Thursday".to_string(),
                "Sunday".to_string(),
            ],
            preferred_duration_minutes: Some(60),
        },
    };

    let profile = UserFitnessProfile {
        user_id: "user_789".to_string(),
        age: Some(35),
        gender: Some("M".to_string()),
        weight: Some(75.0),
        height: Some(180.0),
        fitness_level: FitnessLevel::Intermediate,
        primary_sports: vec!["running".to_string(), "cycling".to_string()],
        training_history_months: 24,
        preferences,
    };

    assert_eq!(profile.user_id, "user_789");
    assert_eq!(profile.age, Some(35));
    assert_eq!(profile.primary_sports.len(), 2);
    assert_eq!(profile.training_history_months, 24);

    match profile.fitness_level {
        FitnessLevel::Intermediate => (),
        _ => panic!("Expected Intermediate fitness level"),
    }

    assert!((profile.preferences.time_availability.hours_per_week - 8.0).abs() < f64::EPSILON);
    assert_eq!(
        profile.preferences.time_availability.preferred_days.len(),
        3
    );
}

#[test]
fn test_fitness_levels() {
    let levels = [
        FitnessLevel::Beginner,
        FitnessLevel::Intermediate,
        FitnessLevel::Advanced,
        FitnessLevel::Elite,
    ];

    assert_eq!(levels.len(), 4);

    // Test that all levels are distinct
    for (i, level1) in levels.iter().enumerate() {
        for (j, level2) in levels.iter().enumerate() {
            if i != j {
                // They should serialize to different values
                let json1 = serde_json::to_string(level1).unwrap();
                let json2 = serde_json::to_string(level2).unwrap();
                assert_ne!(json1, json2);
            }
        }
    }
}

// === Advanced Analytics Tests ===

#[test]
fn test_advanced_insight() {
    let mut metadata = HashMap::new();
    metadata.insert(
        "metric".to_string(),
        serde_json::Value::String("pace".to_string()),
    );
    metadata.insert(
        "value".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(5.5).unwrap()),
    );

    let insight = AdvancedInsight {
        insight_type: "pace_improvement".to_string(),
        message: "Your pace has improved by 5% over the last month".to_string(),
        confidence: Confidence::High,
        severity: InsightSeverity::Info,
        metadata,
    };

    assert_eq!(insight.insight_type, "pace_improvement");
    match insight.confidence {
        Confidence::High => (),
        _ => panic!("Expected High confidence"),
    }
    assert_eq!(insight.metadata.len(), 2);
    assert!(insight.metadata.contains_key("metric"));

    match insight.severity {
        InsightSeverity::Info => (),
        _ => panic!("Expected Info severity"),
    }
}

#[test]
fn test_anomaly() {
    let anomaly = Anomaly {
        anomaly_type: "heart_rate_spike".to_string(),
        description: "Unusual heart rate spike detected during easy run".to_string(),
        severity: InsightSeverity::Warning,
        confidence: Confidence::Medium,
        affected_metric: "average_heart_rate".to_string(),
        expected_value: Some(140.0),
        actual_value: Some(170.0),
    };

    assert_eq!(anomaly.anomaly_type, "heart_rate_spike");
    assert_eq!(anomaly.expected_value, Some(140.0));
    assert_eq!(anomaly.actual_value, Some(170.0));

    match anomaly.severity {
        InsightSeverity::Warning => (),
        _ => panic!("Expected Warning severity"),
    }
}

#[test]
fn test_trend_analysis() {
    let data_points = vec![
        TrendDataPoint {
            date: Utc::now() - chrono::Duration::days(30),
            value: 5.5,
            smoothed_value: Some(5.4),
        },
        TrendDataPoint {
            date: Utc::now() - chrono::Duration::days(15),
            value: 5.3,
            smoothed_value: Some(5.35),
        },
        TrendDataPoint {
            date: Utc::now(),
            value: 5.1,
            smoothed_value: Some(5.2),
        },
    ];

    let trend = TrendAnalysis {
        timeframe: TimeFrame::Month,
        metric: "average_pace".to_string(),
        trend_direction: TrendDirection::Improving,
        trend_strength: 0.8,
        statistical_significance: 0.95,
        data_points,
        insights: vec![],
    };

    assert_eq!(trend.metric, "average_pace");
    assert_eq!(trend.trend_direction, TrendDirection::Improving);
    assert!((trend.trend_strength - 0.8).abs() < f64::EPSILON);
    assert_eq!(trend.data_points.len(), 3);
    assert!(trend.statistical_significance > 0.9);
}

// === Contextual Factors Tests ===

#[test]
fn test_weather_conditions() {
    let weather = WeatherConditions {
        temperature_celsius: 22.0,
        humidity_percentage: Some(70.0),
        wind_speed_kmh: Some(12.0),
        conditions: "light rain".to_string(),
    };

    assert!((weather.temperature_celsius - 22.0).abs() < f32::EPSILON);
    assert_eq!(weather.humidity_percentage, Some(70.0));
    assert_eq!(weather.conditions, "light rain");
}

#[test]
fn test_location_context() {
    let location = LocationContext {
        city: Some("San Francisco".to_string()),
        region: Some("CA".to_string()),
        country: Some("USA".to_string()),
        trail_name: Some("Golden Gate Park Loop".to_string()),
        terrain_type: Some("mixed".to_string()),
        display_name: "Golden Gate Park Loop, San Francisco, CA".to_string(),
    };

    assert_eq!(location.city, Some("San Francisco".to_string()));
    assert_eq!(
        location.trail_name,
        Some("Golden Gate Park Loop".to_string())
    );
    assert_eq!(location.terrain_type, Some("mixed".to_string()));
    assert!(location.display_name.contains("Golden Gate Park"));
}

#[test]
fn test_time_of_day_variants() {
    let times = [
        TimeOfDay::EarlyMorning,
        TimeOfDay::Morning,
        TimeOfDay::Midday,
        TimeOfDay::Afternoon,
        TimeOfDay::Evening,
        TimeOfDay::Night,
    ];

    assert_eq!(times.len(), 6);

    // Test serialization produces different values
    for (i, time1) in times.iter().enumerate() {
        for (j, time2) in times.iter().enumerate() {
            if i != j {
                let json1 = serde_json::to_string(time1).unwrap();
                let json2 = serde_json::to_string(time2).unwrap();
                assert_ne!(json1, json2);
            }
        }
    }
}

#[test]
fn test_weekly_load() {
    let load = WeeklyLoad {
        total_distance_km: 50.0,
        total_duration_hours: 5.0,
        activity_count: 7,
        load_trend: TrendDirection::Improving,
    };

    assert!((load.total_distance_km - 50.0).abs() < f64::EPSILON);
    assert!((load.total_duration_hours - 5.0).abs() < f64::EPSILON);
    assert_eq!(load.activity_count, 7);
    assert_eq!(load.load_trend, TrendDirection::Improving);
}

// === Activity Analyzer Tests ===

#[tokio::test]
async fn test_advanced_activity_analyzer_creation() {
    let analyzer = AdvancedActivityAnalyzer::new();
    let _ = analyzer; // Just test creation

    let default_analyzer = AdvancedActivityAnalyzer::default();
    let _ = default_analyzer; // Test default creation
}

// === Performance Analyzer Tests ===

#[tokio::test]
async fn test_advanced_performance_analyzer_creation() {
    let analyzer = AdvancedPerformanceAnalyzer::new();
    let _ = analyzer; // Just test creation

    let default_analyzer = AdvancedPerformanceAnalyzer::default();
    let _ = default_analyzer; // Test default creation
}

// === Integration Tests ===

#[test]
fn test_activity_insights_serialization() {
    let insights = ActivityInsights {
        activity_id: "activity_123".to_string(),
        overall_score: 8.5,
        insights: vec![],
        metrics: AdvancedMetrics {
            trimp: Some(85.0),
            aerobic_efficiency: Some(1.2),
            power_to_weight_ratio: Some(3.5),
            training_stress_score: Some(75.0),
            intensity_factor: Some(0.8),
            variability_index: Some(1.1),
            efficiency_factor: Some(1.2),
            decoupling_percentage: Some(5.5),

            // Enhanced power metrics
            normalized_power: None,
            work: None,
            avg_power_to_weight: None,

            // Running-specific metrics
            running_effectiveness: None,
            stride_efficiency: None,
            ground_contact_balance: None,

            // Recovery and physiological metrics
            estimated_recovery_time: None,
            training_load: None,
            aerobic_contribution: None,

            // Environmental impact metrics
            temperature_stress: None,
            altitude_adjustment: None,

            custom_metrics: HashMap::new(),
        },
        recommendations: vec!["Focus on consistent pacing".to_string()],
        anomalies: vec![],
    };

    // Test serialization
    let json = serde_json::to_string(&insights).expect("Serialization should work");
    assert!(json.contains("activity_123"));
    assert!(json.contains("8.5"));

    // Test deserialization
    let deserialized: ActivityInsights =
        serde_json::from_str(&json).expect("Deserialization should work");
    assert_eq!(deserialized.activity_id, "activity_123");
    assert!((deserialized.overall_score - 8.5).abs() < f64::EPSILON);
}

#[test]
fn test_complete_contextual_factors() {
    let complete_context = ContextualFactors {
        weather: Some(WeatherConditions {
            temperature_celsius: 18.0,
            humidity_percentage: Some(80.0),
            wind_speed_kmh: Some(5.0),
            conditions: "overcast".to_string(),
        }),
        location: Some(LocationContext {
            city: Some("Portland".to_string()),
            region: Some("OR".to_string()),
            country: Some("USA".to_string()),
            trail_name: Some("Forest Park Trail".to_string()),
            terrain_type: Some("trail".to_string()),
            display_name: "Forest Park Trail, Portland, OR".to_string(),
        }),
        time_of_day: TimeOfDay::Afternoon,
        days_since_last_activity: Some(3),
        weekly_load: Some(WeeklyLoad {
            total_distance_km: 35.0,
            total_duration_hours: 3.5,
            activity_count: 4,
            load_trend: TrendDirection::Declining,
        }),
    };

    // Test all fields are populated correctly
    assert!(complete_context.weather.is_some());
    assert!(complete_context.location.is_some());
    assert_eq!(complete_context.days_since_last_activity, Some(3));
    assert!(complete_context.weekly_load.is_some());

    let weather = complete_context.weather.unwrap();
    assert!((weather.temperature_celsius - 18.0).abs() < f32::EPSILON);

    let location = complete_context.location.unwrap();
    assert_eq!(location.city, Some("Portland".to_string()));

    let load = complete_context.weekly_load.unwrap();
    assert_eq!(load.activity_count, 4);
}
