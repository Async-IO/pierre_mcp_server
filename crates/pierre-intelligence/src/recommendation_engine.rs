// ABOUTME: Training recommendation engine for personalized fitness guidance and coaching
// ABOUTME: Generates custom workout plans, recovery suggestions, and training adaptations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Training recommendation engine for personalized insights
#![allow(clippy::cast_precision_loss)] // Safe: fitness data conversions
#![allow(clippy::cast_possible_truncation)] // Safe: controlled ranges
#![allow(clippy::cast_sign_loss)] // Safe: positive values only

use super::{
    Confidence, FitnessLevel, InsightSeverity, RecommendationPriority, RecommendationType,
    TrainingRecommendation, UserFitnessProfile,
};
use crate::config::intelligence::{
    DefaultStrategy, IntelligenceConfig, IntelligenceStrategy, RecommendationEngineConfig,
};
use crate::errors::AppResult;
use crate::physiological_constants::{
    consistency::CONSISTENCY_SCORE_THRESHOLD,
    frequency_targets::MAX_WEEKLY_FREQUENCY,
    heart_rate::HIGH_INTENSITY_HR_THRESHOLD,
    hr_estimation::{ASSUMED_MAX_HR, RECOVERY_HR_PERCENTAGE},
    intensity_balance::{
        HIGH_INTENSITY_UPPER_LIMIT, LOW_INTENSITY_LOWER_LIMIT, MODERATE_NUTRITION_HR_THRESHOLD,
    },
    nutrition::{
        DURING_EXERCISE_DURATION_THRESHOLD, POST_EXERCISE_DURATION_THRESHOLD,
        PRE_EXERCISE_DURATION_THRESHOLD,
    },
    time_periods::{
        LONG_TRAINING_GAP_DAYS, MAX_CONSECUTIVE_TRAINING_DAYS, RECOVERY_ANALYSIS_DAYS,
        SHORT_TRAINING_GAP_DAYS, TRAINING_PATTERN_ANALYSIS_WEEKS,
    },
    volume_thresholds::{
        HIGH_WEEKLY_LOAD_SECONDS, HIGH_WEEKLY_VOLUME_HOURS, MAX_HIGH_INTENSITY_SESSIONS_PER_WEEK,
        MIN_WEEKLY_VOLUME_HOURS,
    },
    zone_distributions::equipment::{SHOE_REPLACEMENT_MAX_KM, SHOE_REPLACEMENT_MIN_KM},
};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use chrono::{naive::Days, Utc};

use crate::models::Activity;

/// Trait for generating training recommendations
#[async_trait::async_trait]
pub trait RecommendationEngineTrait {
    /// Generate personalized training recommendations
    async fn generate_recommendations(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> AppResult<Vec<TrainingRecommendation>>;

    /// Generate recovery recommendations based on training load
    async fn generate_recovery_recommendations(
        &self,
        activities: &[Activity],
    ) -> AppResult<Vec<TrainingRecommendation>>;

    /// Generate nutrition recommendations for activities
    async fn generate_nutrition_recommendations(
        &self,
        activity: &Activity,
    ) -> AppResult<Vec<TrainingRecommendation>>;

    /// Generate equipment recommendations
    async fn generate_equipment_recommendations(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> AppResult<Vec<TrainingRecommendation>>;
}

/// Advanced recommendation engine implementation with configurable strategy
pub struct AdvancedRecommendationEngine<S: IntelligenceStrategy = DefaultStrategy> {
    strategy: S,
    config: RecommendationEngineConfig,
    user_profile: Option<UserFitnessProfile>,
}

impl Default for AdvancedRecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedRecommendationEngine {
    /// Create a new recommendation engine with default strategy
    #[must_use]
    pub fn new() -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy: DefaultStrategy,
            config: global_config.recommendation_engine.clone(),
            user_profile: None,
        }
    }
}

impl<S: IntelligenceStrategy> AdvancedRecommendationEngine<S> {
    /// Create a new recommendation engine with custom strategy
    #[must_use]
    pub fn with_strategy(strategy: S) -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy,
            config: global_config.recommendation_engine.clone(),
            user_profile: None,
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub const fn with_config(strategy: S, config: RecommendationEngineConfig) -> Self {
        Self {
            strategy,
            config,
            user_profile: None,
        }
    }

    /// Create engine with user profile using default strategy
    #[must_use]
    pub fn with_profile(profile: UserFitnessProfile) -> AdvancedRecommendationEngine {
        let global_config = IntelligenceConfig::global();
        AdvancedRecommendationEngine {
            strategy: DefaultStrategy,
            config: global_config.recommendation_engine.clone(),
            user_profile: Some(profile),
        }
    }

    /// Set user profile for this engine
    pub fn set_profile(&mut self, profile: UserFitnessProfile) {
        self.user_profile = Some(profile);
    }

    /// Analyze training patterns to identify areas for improvement
    fn analyze_training_patterns(&self, activities: &[Activity]) -> TrainingPatternAnalysis {
        let recent_activities: Vec<Activity> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date();
                let weeks_ago = (Utc::now() - activity_utc).num_weeks();
                weeks_ago <= TRAINING_PATTERN_ANALYSIS_WEEKS
            })
            .cloned()
            .collect();
        let mut sport_frequency: HashMap<String, usize> = HashMap::new();
        let mut weekly_load = 0.0;
        let mut high_intensity_count = 0;
        let mut _low_intensity_count = 0;
        let mut _total_duration = 0;

        for activity in &recent_activities {
            *sport_frequency
                .entry(format!("{:?}", activity.sport_type()))
                .or_insert(0) += 1;

            let duration = activity.duration_seconds();
            // Safe: duration represents time in seconds, precision loss acceptable for hour calculations
            {
                weekly_load += duration as f64 / 3600.0;
            } // Hours
            _total_duration += duration;

            if let Some(avg_hr) = activity.average_heart_rate() {
                // Use configurable heart rate thresholds
                let hr_config = &self.config.thresholds;
                // Safe: heart rate thresholds are small positive values (80-220 bpm)
                let intensity_threshold = {
                    u32::try_from((hr_config.intensity_threshold * ASSUMED_MAX_HR) as u64)
                        .unwrap_or(u32::MAX)
                };
                // Safe: heart rate thresholds are small positive values (60-150 bpm)
                let recovery_threshold = {
                    u32::try_from((RECOVERY_HR_PERCENTAGE * ASSUMED_MAX_HR) as u64)
                        .unwrap_or(u32::MAX)
                };

                if avg_hr > intensity_threshold {
                    high_intensity_count += 1;
                } else if avg_hr < recovery_threshold {
                    _low_intensity_count += 1;
                }
            }
        }

        weekly_load /= 4.0; // Average per week

        let intensity_balance = if recent_activities.is_empty() {
            0.0
        } else {
            // Safe: activity count precision loss acceptable for ratio calculations
            {
                f64::from(high_intensity_count) / recent_activities.len() as f64
            }
        };

        // Use configurable frequency thresholds for consistency scoring
        // Safe: frequency threshold is small positive value representing weekly activities (0-20)
        let high_freq = usize::try_from(
            (f64::from(self.config.thresholds.high_weekly_frequency) * 4.0).round() as i64,
        )
        .unwrap_or(usize::MAX); // 4 weeks
                                // Safe: frequency threshold is small positive value representing weekly activities (0-20)
        let low_freq = usize::try_from(
            (f64::from(self.config.thresholds.low_weekly_frequency) * 4.0).round() as i64,
        )
        .unwrap_or(usize::MAX);
        // Safe: frequency threshold is small positive value representing weekly activities (0-20)
        let ideal_freq = usize::try_from(
            (f64::midpoint(
                f64::from(self.config.thresholds.high_weekly_frequency),
                f64::from(self.config.thresholds.low_weekly_frequency),
            ) * 4.0)
                .round() as i64,
        )
        .unwrap_or(usize::MAX);

        let distance_weight = if self.config.weights.distance_weight > 0.0 {
            self.config.weights.distance_weight
        } else {
            1.0
        };
        let consistency_score = if recent_activities.len() >= high_freq {
            self.config.thresholds.consistency_threshold
        } else if recent_activities.len() >= ideal_freq {
            self.config.thresholds.consistency_threshold * self.config.weights.frequency_weight
                / distance_weight
        } else if recent_activities.len() >= low_freq {
            self.config.thresholds.pace_improvement_threshold
                * (self.config.weights.frequency_weight + self.config.weights.consistency_weight)
        } else {
            self.config.thresholds.pace_improvement_threshold
        };

        TrainingPatternAnalysis {
            weekly_load_hours: weekly_load,
            sport_diversity: sport_frequency.len(),
            intensity_balance,
            consistency_score,
            primary_sport: sport_frequency
                .iter()
                .max_by_key(|(_, &count)| count)
                .map_or_else(|| "Unknown".into(), |(sport, _)| sport.clone()),
            training_gaps: self.identify_training_gaps(&recent_activities),
        }
    }

    /// Identify gaps in training routine
    fn identify_training_gaps(&self, activities: &[Activity]) -> Vec<TrainingGap> {
        let mut gaps = Vec::new();

        // Check for long periods without activity
        if activities.len() < 2 {
            return gaps;
        }

        let mut sorted_activities = activities.to_vec();
        sorted_activities.sort_by(|a, b| {
            let date_a = Some(a.start_date());
            let date_b = Some(b.start_date());
            date_a.cmp(&date_b)
        });

        for i in 1..sorted_activities.len() {
            let prev_date = sorted_activities[i - 1].start_date();
            let curr_date = sorted_activities[i].start_date();
            let gap_days = (curr_date - prev_date).num_days();

            if gap_days > SHORT_TRAINING_GAP_DAYS {
                gaps.push(TrainingGap {
                    gap_type: GapType::LongRest,
                    duration_days: gap_days,
                    description: format!("{gap_days} days without training"),
                    severity: if gap_days > LONG_TRAINING_GAP_DAYS {
                        InsightSeverity::Warning
                    } else {
                        InsightSeverity::Info
                    },
                });
            }
        }

        // Check for missing training types
        let sports: HashSet<_> = activities.iter().map(Activity::sport_type).collect();

        if let Some(profile) = &self.user_profile {
            for primary_sport in &profile.primary_sports {
                // Convert string to SportType for comparison - this is simplified
                if sports.is_empty() {
                    // Check if sports exist in activity history
                    gaps.push(TrainingGap {
                        gap_type: GapType::MissingSport,
                        duration_days: 0,
                        description: format!(
                            "Missing {primary_sport} training in recent activities"
                        ),
                        severity: InsightSeverity::Info,
                    });
                }
            }
        }

        gaps
    }

    /// Generate strategy-based recommendations using the strategy field
    fn generate_strategy_based_recommendations(
        &self,
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        // Use strategy to determine if volume increase is recommended
        // Use config thresholds for conversion factor
        let conversion_factor = self.config.thresholds.volume_increase_threshold * 100.0; // Use volume_increase_threshold as conversion basis
        let current_volume_km = analysis.weekly_load_hours * conversion_factor;
        if self
            .strategy
            .should_recommend_volume_increase(current_volume_km)
        {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Volume,
                title: "Strategy-Based Volume Increase".into(),
                description: "Your current strategy suggests increasing training volume based on your profile.".into(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Strategic volume increases are tailored to your current fitness level and goals.".into(),
                actionable_steps: vec![
                    "Follow your strategy's volume progression guidelines".into(),
                    "Increase volume gradually as recommended by your training approach".into(),
                    "Monitor adaptation and adjust based on response".into(),
                ],
            });
        }

        // Use strategy frequency thresholds
        // Estimate weekly activities based on intensity threshold
        let intensity_threshold = if self.config.thresholds.intensity_threshold > 0.0 {
            self.config.thresholds.intensity_threshold
        } else {
            1.0
        };
        let weekly_activities =
            i32::try_from((analysis.weekly_load_hours / intensity_threshold).ceil() as i64)
                .unwrap_or(i32::MAX);
        if self.strategy.should_recommend_recovery(weekly_activities) {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Recovery,
                title: "Strategy-Based Recovery Focus".into(),
                description: "Your training strategy recommends focusing on recovery based on current frequency.".into(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Strategic recovery prevents overtraining and optimizes long-term performance gains.".into(),
                actionable_steps: vec![
                    "Reduce training frequency as per your strategy guidelines".into(),
                    "Include more recovery-focused activities".into(),
                    "Prioritize sleep and stress management".into(),
                ],
            });
        }

        recommendations
    }

    /// Generate sport diversity recommendations using `sport_diversity` and `primary_sport` fields
    fn generate_sport_diversity_recommendations(
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        // Check sport diversity - if low, recommend cross-training
        if analysis.sport_diversity <= 1 {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Strategy,
                title: "Increase Sport Diversity".into(),
                description: format!("You're primarily focused on {}. Consider adding cross-training activities.", analysis.primary_sport),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Sport diversity helps prevent overuse injuries and provides balanced fitness development.".into(),
                actionable_steps: vec![
                    "Add one complementary sport to your routine".into(),
                    "Try swimming, cycling, or running as cross-training".into(),
                    "Use different sports for active recovery days".into(),
                ],
            });
        } else if analysis.sport_diversity >= 4 {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Strategy,
                title: "Focus Training Specificity".into(),
                description: format!("You're training in {} different sports. Consider focusing more on your primary sport: {}.", analysis.sport_diversity, analysis.primary_sport),
                priority: RecommendationPriority::Low,
                confidence: Confidence::Medium,
                rationale: "While cross-training is beneficial, too much diversity may limit specific performance gains.".into(),
                actionable_steps: vec![
                    format!("Increase focus on {} training sessions", analysis.primary_sport),
                    "Maintain 1-2 complementary sports for variety".into(),
                    "Ensure primary sport gets 60-70% of training time".into(),
                ],
            });
        }

        recommendations
    }

    /// Generate intensity-based recommendations
    fn generate_intensity_recommendations(
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.intensity_balance > HIGH_INTENSITY_UPPER_LIMIT {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Intensity,
                title: "Add More Easy Training".into(),
                description: "Your training intensity is quite high. Consider adding more low-intensity, base-building sessions.".into(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "High-intensity training should typically make up only 20-30% of total training volume for optimal adaptation and recovery.".into(),
                actionable_steps: vec![
                    "Add 1-2 easy-paced sessions per week".into(),
                    "Keep heart rate below aerobic threshold (Zone 2)".into(),
                    "Focus on conversational pace".into(),
                ],
            });
        } else if analysis.intensity_balance < LOW_INTENSITY_LOWER_LIMIT {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Intensity,
                title: "Increase Training Intensity".into(),
                description: "Your training could benefit from more high-intensity sessions to improve performance.".into(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Including 20-30% high-intensity training can improve VO2 max, lactate threshold, and overall performance.".into(),
                actionable_steps: vec![
                    "Add 1 interval session per week".into(),
                    "Include tempo runs or threshold workouts".into(),
                    "Ensure proper recovery between hard sessions".into(),
                ],
            });
        }

        recommendations
    }

    /// Generate volume-based recommendations
    fn generate_volume_recommendations(
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.weekly_load_hours < MIN_WEEKLY_VOLUME_HOURS {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Volume,
                title: "Gradually Increase Training Volume".into(),
                description: "Your current training volume could be increased for better fitness gains.".into(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Gradual volume increases of 10% per week can lead to improved fitness while minimizing injury risk.".into(),
                actionable_steps: vec![
                    "Add 15-20 minutes to your longest session each week".into(),
                    "Include one additional short session per week".into(),
                    "Monitor for signs of overtraining".into(),
                ],
            });
        } else if analysis.weekly_load_hours > HIGH_WEEKLY_VOLUME_HOURS {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Volume,
                title: "Monitor Training Load".into(),
                description: "High training volume detected. Ensure adequate recovery and listen to your body.".into(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Very high training loads increase injury risk and may lead to overtraining if recovery is inadequate.".into(),
                actionable_steps: vec![
                    "Schedule regular recovery weeks".into(),
                    "Monitor heart rate variability".into(),
                    "Prioritize sleep and nutrition".into(),
                ],
            });
        }

        recommendations
    }

    /// Generate consistency recommendations
    fn generate_consistency_recommendations(
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.consistency_score < CONSISTENCY_SCORE_THRESHOLD {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Strategy,
                title: "Improve Training Consistency".into(),
                description: "Focus on building a more consistent training routine for better adaptations.".into(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Consistent training stimulus is more effective than sporadic high-intensity efforts for long-term fitness gains.".into(),
                actionable_steps: vec![
                    "Schedule fixed training days in your calendar".into(),
                    "Start with shorter, manageable sessions".into(),
                    "Find an accountability partner or group".into(),
                    "Track your progress to stay motivated".into(),
                ],
            });
        }

        for gap in &analysis.training_gaps {
            match gap.gap_type {
                GapType::LongRest => {
                    if gap.duration_days > (SHORT_TRAINING_GAP_DAYS + 3) {
                        // Use severity field to determine recommendation priority
                        let priority = match gap.severity {
                            InsightSeverity::Warning => RecommendationPriority::High,
                            InsightSeverity::Info => RecommendationPriority::Medium,
                            InsightSeverity::Critical => RecommendationPriority::Low,
                        };

                        recommendations.push(TrainingRecommendation {
                            recommendation_type: RecommendationType::Strategy,
                            title: "Avoid Long Training Breaks".into(),
                            description: format!("Recent {} gap detected. Try to maintain more consistent activity.", gap.description),
                            priority,
                            confidence: Confidence::Medium,
                            rationale: "Training breaks longer than a week can lead to fitness losses and increased injury risk when resuming.".into(),
                            actionable_steps: vec![
                                "Aim for at least one activity every 5-7 days".into(),
                                "Use easy sessions to maintain base fitness".into(),
                                "Plan ahead for busy periods".into(),
                            ],
                        });
                    }
                }
                GapType::MissingSport => {
                    // Use severity field to determine recommendation priority
                    let priority = match gap.severity {
                        InsightSeverity::Warning => RecommendationPriority::Medium,
                        InsightSeverity::Info | InsightSeverity::Critical => {
                            RecommendationPriority::Low
                        }
                    };

                    recommendations.push(TrainingRecommendation {
                        recommendation_type: RecommendationType::Strategy,
                        title: "Include Cross-Training".into(),
                        description: gap.description.clone(),
                        priority,
                        confidence: Confidence::Medium,
                        rationale: "Cross-training helps prevent overuse injuries and maintains overall fitness.".into(),
                        actionable_steps: vec![
                            "Add 1 cross-training session per week".into(),
                            "Choose activities that complement your primary sport".into(),
                            "Use cross-training for active recovery".into(),
                        ],
                    });
                }
            }
        }

        recommendations
    }
}

#[async_trait::async_trait]
impl RecommendationEngineTrait for AdvancedRecommendationEngine {
    async fn generate_recommendations(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> AppResult<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        // Analyze current training patterns
        let analysis = self.analyze_training_patterns(activities);

        // Generate different types of recommendations
        recommendations.extend(Self::generate_intensity_recommendations(&analysis));
        recommendations.extend(Self::generate_volume_recommendations(&analysis));
        recommendations.extend(Self::generate_consistency_recommendations(&analysis));

        // Add strategy-specific recommendations using the strategy field
        recommendations.extend(self.generate_strategy_based_recommendations(&analysis));

        // Add sport diversity recommendations using sport_diversity and primary_sport fields
        recommendations.extend(Self::generate_sport_diversity_recommendations(&analysis));

        // Fitness level specific recommendations
        match user_profile.fitness_level {
            FitnessLevel::Beginner => {
                recommendations.push(TrainingRecommendation {
                    recommendation_type: RecommendationType::Strategy,
                    title: "Focus on Building Base Fitness".into(),
                    description: "Prioritize consistency and gradual progression over intensity.".into(),
                    priority: RecommendationPriority::High,
                    confidence: Confidence::High,
                    rationale: "Building a strong aerobic base is crucial for beginners to support future training adaptations.".into(),
                    actionable_steps: vec![
                        "Start with 20-30 minute easy sessions".into(),
                        "Gradually increase duration by 10% each week".into(),
                        "Include rest days between sessions".into(),
                        "Focus on proper form and technique".into(),
                    ],
                });
            }
            FitnessLevel::Intermediate => {
                recommendations.push(TrainingRecommendation {
                    recommendation_type: RecommendationType::Strategy,
                    title: "Introduce Structured Training".into(),
                    description: "Add periodization and specific training phases to your routine.".into(),
                    priority: RecommendationPriority::Medium,
                    confidence: Confidence::High,
                    rationale: "Structured training helps intermediate athletes break through plateaus and continue improving.".into(),
                    actionable_steps: vec![
                        "Plan 4-6 week training blocks".into(),
                        "Include base, build, and peak phases".into(),
                        "Add sport-specific skill work".into(),
                        "Monitor training stress and recovery".into(),
                    ],
                });
            }
            FitnessLevel::Advanced | FitnessLevel::Elite => {
                recommendations.push(TrainingRecommendation {
                    recommendation_type: RecommendationType::Strategy,
                    title: "Optimize Training Specificity".into(),
                    description: "Fine-tune training to target specific performance limiters.".into(),
                    priority: RecommendationPriority::Medium,
                    confidence: Confidence::Medium,
                    rationale: "Advanced athletes benefit from highly specific training targeting individual weaknesses and performance goals.".into(),
                    actionable_steps: vec![
                        "Conduct regular performance testing".into(),
                        "Identify and target limiting factors".into(),
                        "Use advanced training metrics (power, pace zones)".into(),
                        "Include mental training and race tactics".into(),
                    ],
                });
            }
        }

        // Sort by priority and confidence
        recommendations.sort_by(|a, b| {
            let priority_order = |p: &RecommendationPriority| match p {
                RecommendationPriority::Critical => 4,
                RecommendationPriority::High => 3,
                RecommendationPriority::Medium => 2,
                RecommendationPriority::Low => 1,
            };

            priority_order(&b.priority)
                .cmp(&priority_order(&a.priority))
                .then_with(|| {
                    b.confidence
                        .as_score()
                        .partial_cmp(&a.confidence.as_score())
                        .unwrap_or(Ordering::Equal)
                })
        });

        Ok(recommendations.into_iter().take(8).collect()) // Return top 8 recommendations
    }

    async fn generate_recovery_recommendations(
        &self,
        activities: &[Activity],
    ) -> AppResult<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        // Analyze recent training load
        let recent_activities: Vec<Activity> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date();
                let days_ago = (Utc::now() - activity_utc).num_days();
                days_ago <= RECOVERY_ANALYSIS_DAYS
            })
            .cloned()
            .collect();
        let total_duration: u64 = recent_activities
            .iter()
            .map(Activity::duration_seconds)
            .sum();

        let high_intensity_sessions = recent_activities
            .iter()
            .filter(|a| a.average_heart_rate().unwrap_or(0) > HIGH_INTENSITY_HR_THRESHOLD)
            .count();

        // Check if recovery is needed
        if total_duration > HIGH_WEEKLY_LOAD_SECONDS
            || high_intensity_sessions > MAX_HIGH_INTENSITY_SESSIONS_PER_WEEK
        {
            // >5 hours or >3 hard sessions
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Recovery,
                title: "Prioritize Recovery This Week".into(),
                description: "High training load detected. Focus on recovery activities.".into(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Adequate recovery prevents overtraining and allows for training adaptations to occur.".into(),
                actionable_steps: vec![
                    "Include at least 2 complete rest days".into(),
                    "Add gentle yoga or stretching sessions".into(),
                    "Prioritize 8+ hours of sleep".into(),
                    "Consider massage or foam rolling".into(),
                    "Stay hydrated and eat adequate protein".into(),
                ],
            });
        }

        // Check for consecutive training days
        let consecutive_days = Self::count_consecutive_training_days(&recent_activities);
        if consecutive_days > MAX_CONSECUTIVE_TRAINING_DAYS {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Recovery,
                title: "Take a Rest Day".into(),
                description: format!("{consecutive_days} consecutive training days detected."),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Regular rest days are essential for physical and mental recovery."
                    .to_owned(),
                actionable_steps: vec![
                    "Schedule a complete rest day today".into(),
                    "Focus on nutrition and hydration".into(),
                    "Light walking or gentle stretching only".into(),
                ],
            });
        }

        Ok(recommendations)
    }

    async fn generate_nutrition_recommendations(
        &self,
        activity: &Activity,
    ) -> AppResult<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        // Safe: duration conversion to hours, precision loss acceptable for nutrition calculations
        let duration_hours = activity.duration_seconds() as f64 / 3600.0;
        let high_intensity =
            activity.average_heart_rate().unwrap_or(0) > MODERATE_NUTRITION_HR_THRESHOLD;

        // Pre-activity nutrition
        if duration_hours > PRE_EXERCISE_DURATION_THRESHOLD {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Nutrition,
                title: "Pre-Exercise Fueling".into(),
                description: "Proper pre-exercise nutrition for longer sessions.".into(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Adequate carbohydrate intake before longer sessions maintains energy levels and performance.".into(),
                actionable_steps: vec![
                    "Eat 30-60g carbohydrates 1-2 hours before exercise".into(),
                    "Include easily digestible foods (banana, oatmeal, toast)".into(),
                    "Avoid high fiber and fat before training".into(),
                    "Stay hydrated leading up to exercise".into(),
                ],
            });
        }

        // During-activity nutrition
        if duration_hours > DURING_EXERCISE_DURATION_THRESHOLD {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Nutrition,
                title: "In-Exercise Fueling".into(),
                description: "Maintain energy during long training sessions.".into(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Consuming carbohydrates during exercise >2 hours prevents glycogen depletion and maintains performance.".into(),
                actionable_steps: vec![
                    "Consume 30-60g carbohydrates per hour after the first hour".into(),
                    "Use sports drinks, gels, or easily digestible snacks".into(),
                    "Drink 150-250ml fluid every 15-20 minutes".into(),
                    "Practice fueling strategy during training".into(),
                ],
            });
        }

        // Post-activity recovery
        if duration_hours > POST_EXERCISE_DURATION_THRESHOLD || high_intensity {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Nutrition,
                title: "Post-Exercise Recovery Nutrition".into(),
                description: "Optimize recovery with proper post-exercise nutrition.".into(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Post-exercise nutrition within 30-60 minutes optimizes glycogen replenishment and muscle protein synthesis.".into(),
                actionable_steps: vec![
                    "Consume 1-1.2g carbohydrates per kg body weight within 30 minutes".into(),
                    "Include 20-25g high-quality protein".into(),
                    "Rehydrate with 150% of fluid losses".into(),
                    "Consider chocolate milk, recovery smoothie, or balanced meal".into(),
                ],
            });
        }

        Ok(recommendations)
    }

    async fn generate_equipment_recommendations(
        &self,
        _user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> AppResult<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        // Analyze primary sports
        let mut sport_counts: HashMap<String, usize> = HashMap::new();
        for activity in activities {
            *sport_counts
                .entry(format!("{:?}", activity.sport_type()))
                .or_insert(0) += 1;
        }

        // Running-specific equipment
        // Safe: weekly frequency is small positive value (1-20 activities)
        if sport_counts.get("Run").unwrap_or(&0) > &(MAX_WEEKLY_FREQUENCY as usize) {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Equipment,
                title: "Running Equipment Optimization".into(),
                description: "Optimize your running gear for better performance and injury prevention.".into(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Proper running equipment reduces injury risk and can improve performance and comfort.".into(),
                actionable_steps: vec![
                    "Get professional gait analysis and shoe fitting".into(),
                    format!(
                        "Replace running shoes every {}-{}km",
                        SHOE_REPLACEMENT_MIN_KM as u32,
                        SHOE_REPLACEMENT_MAX_KM as u32
                    ),
                    "Consider moisture-wicking clothing for longer runs".into(),
                    "Use GPS watch or smartphone app for pacing".into(),
                ],
            });
        }

        // Cycling-specific equipment
        // Safe: weekly frequency is small positive value (1-20 activities)
        if sport_counts.get("Ride").unwrap_or(&0) > &(MAX_WEEKLY_FREQUENCY as usize) {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Equipment,
                title: "Cycling Equipment Optimization".into(),
                description: "Enhance your cycling setup for efficiency and comfort.".into(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Proper bike fit and equipment can significantly improve cycling efficiency and reduce injury risk.".into(),
                actionable_steps: vec![
                    "Get professional bike fit assessment".into(),
                    "Ensure proper helmet fit and replacement schedule".into(),
                    "Consider power meter for training precision".into(),
                    "Maintain bike regularly for optimal performance".into(),
                ],
            });
        }

        // General monitoring equipment
        let has_hr_data = activities.iter().any(|a| a.average_heart_rate().is_some());
        // Safe: weekly frequency is small positive value (1-20 activities)
        if !has_hr_data && activities.len() > (MAX_WEEKLY_FREQUENCY as usize) {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Equipment,
                title: "Heart Rate Monitoring".into(),
                description: "Consider adding heart rate monitoring to your training.".into(),
                priority: RecommendationPriority::Low,
                confidence: Confidence::Medium,
                rationale: "Heart rate data provides valuable insights into training intensity, recovery, and overall fitness progress.".into(),
                actionable_steps: vec![
                    "Consider chest strap or wrist-based heart rate monitor".into(),
                    "Learn your heart rate zones".into(),
                    "Use HR data to guide training intensity".into(),
                    "Track resting heart rate for recovery monitoring".into(),
                ],
            });
        }

        Ok(recommendations)
    }
}

impl AdvancedRecommendationEngine {
    /// Count consecutive training days
    fn count_consecutive_training_days(activities: &[Activity]) -> usize {
        let mut consecutive = 0;
        let mut current_date = Utc::now().date_naive();

        // Sort activities by date (most recent first)
        let mut sorted_activities = activities.to_vec();
        sorted_activities.sort_by(|a, b| {
            let date_a = Some(a.start_date());
            let date_b = Some(b.start_date());
            date_b.cmp(&date_a) // Reverse order (newest first)
        });

        for activity in sorted_activities {
            let activity_naive = activity.start_date().naive_utc().date();

            if activity_naive == current_date || activity_naive == current_date - Days::new(1) {
                consecutive += 1;
                current_date = activity_naive - Days::new(1);
            } else {
                break;
            }
        }

        consecutive
    }
}

/// Training pattern analysis results
#[derive(Debug)]
struct TrainingPatternAnalysis {
    weekly_load_hours: f64,
    sport_diversity: usize,
    intensity_balance: f64,
    consistency_score: f64,
    primary_sport: String,
    training_gaps: Vec<TrainingGap>,
}

/// Identified gap in training
#[derive(Debug)]
struct TrainingGap {
    gap_type: GapType,
    duration_days: i64,
    description: String,
    severity: InsightSeverity,
}

/// Types of training gaps
#[non_exhaustive]
#[derive(Debug)]
enum GapType {
    LongRest,
    MissingSport,
}
