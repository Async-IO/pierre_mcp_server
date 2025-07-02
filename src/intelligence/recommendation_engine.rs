// ABOUTME: Training recommendation engine for personalized fitness guidance and coaching
// ABOUTME: Generates custom workout plans, recovery suggestions, and training adaptations
//! Training recommendation engine for personalized insights

use super::*;
use crate::config::intelligence_config::{
    IntelligenceConfig, IntelligenceStrategy, RecommendationEngineConfig,
};
use crate::intelligence::physiological_constants::{
    consistency::*, frequency_targets::*, hr_estimation::*, intensity_balance::*, nutrition::*,
    time_periods::*, volume_thresholds::*,
};
use crate::models::Activity;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;

/// Trait for generating training recommendations
#[async_trait::async_trait]
pub trait RecommendationEngineTrait {
    /// Generate personalized training recommendations
    async fn generate_recommendations(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> Result<Vec<TrainingRecommendation>>;

    /// Generate recovery recommendations based on training load
    async fn generate_recovery_recommendations(
        &self,
        activities: &[Activity],
    ) -> Result<Vec<TrainingRecommendation>>;

    /// Generate nutrition recommendations for activities
    async fn generate_nutrition_recommendations(
        &self,
        activity: &Activity,
    ) -> Result<Vec<TrainingRecommendation>>;

    /// Generate equipment recommendations
    async fn generate_equipment_recommendations(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> Result<Vec<TrainingRecommendation>>;
}

/// Advanced recommendation engine implementation with configurable strategy
pub struct AdvancedRecommendationEngine<
    S: IntelligenceStrategy = crate::config::intelligence_config::DefaultStrategy,
> {
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
    pub fn new() -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy: crate::config::intelligence_config::DefaultStrategy,
            config: global_config.recommendation_engine.clone(),
            user_profile: None,
        }
    }
}

impl<S: IntelligenceStrategy> AdvancedRecommendationEngine<S> {
    /// Create a new recommendation engine with custom strategy
    pub fn with_strategy(strategy: S) -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy,
            config: global_config.recommendation_engine.clone(),
            user_profile: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(strategy: S, config: RecommendationEngineConfig) -> Self {
        Self {
            strategy,
            config,
            user_profile: None,
        }
    }

    /// Create engine with user profile using default strategy
    pub fn with_profile(profile: UserFitnessProfile) -> AdvancedRecommendationEngine {
        let global_config = IntelligenceConfig::global();
        AdvancedRecommendationEngine {
            strategy: crate::config::intelligence_config::DefaultStrategy,
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
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date;
                let weeks_ago = (Utc::now() - activity_utc).num_weeks();
                weeks_ago <= TRAINING_PATTERN_ANALYSIS_WEEKS
            })
            .collect();

        let owned_activities: Vec<Activity> =
            recent_activities.iter().map(|a| (*a).clone()).collect();
        let mut sport_frequency: HashMap<String, usize> = HashMap::new();
        let mut weekly_load = 0.0;
        let mut high_intensity_count = 0;
        let mut _low_intensity_count = 0;
        let mut _total_duration = 0;

        for activity in &recent_activities {
            *sport_frequency
                .entry(format!("{:?}", activity.sport_type))
                .or_insert(0) += 1;

            let duration = activity.duration_seconds;
            weekly_load += duration as f64 / 3600.0; // Hours
            _total_duration += duration;

            if let Some(avg_hr) = activity.average_heart_rate {
                // Use configurable heart rate thresholds
                let hr_config = &self.config.thresholds;
                let intensity_threshold = (hr_config.intensity_threshold * ASSUMED_MAX_HR) as u32;
                let recovery_threshold = (RECOVERY_HR_PERCENTAGE * ASSUMED_MAX_HR) as u32;

                if avg_hr > intensity_threshold {
                    high_intensity_count += 1;
                } else if avg_hr < recovery_threshold {
                    _low_intensity_count += 1;
                }
            }
        }

        weekly_load /= 4.0; // Average per week

        let intensity_balance = if !recent_activities.is_empty() {
            high_intensity_count as f64 / recent_activities.len() as f64
        } else {
            0.0
        };

        // Use configurable frequency thresholds for consistency scoring
        let high_freq = (self.config.thresholds.high_weekly_frequency * 4) as usize; // 4 weeks
        let low_freq = (self.config.thresholds.low_weekly_frequency * 4) as usize;
        let ideal_freq = ((self.config.thresholds.high_weekly_frequency
            + self.config.thresholds.low_weekly_frequency)
            / 2
            * 4) as usize;

        let consistency_score = if recent_activities.len() >= high_freq {
            self.config.thresholds.consistency_threshold
        } else if recent_activities.len() >= ideal_freq {
            self.config.thresholds.consistency_threshold * self.config.weights.frequency_weight
                / self.config.weights.distance_weight
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
                .map(|(sport, _)| sport.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            training_gaps: self.identify_training_gaps(&owned_activities),
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
            let date_a = Some(a.start_date);
            let date_b = Some(b.start_date);
            date_a.cmp(&date_b)
        });

        for i in 1..sorted_activities.len() {
            let prev_date = sorted_activities[i - 1].start_date;
            let curr_date = sorted_activities[i].start_date;
            let gap_days = (curr_date - prev_date).num_days();

            if gap_days > SHORT_TRAINING_GAP_DAYS {
                gaps.push(TrainingGap {
                    gap_type: GapType::LongRest,
                    duration_days: gap_days,
                    description: format!("{} days without training", gap_days),
                    severity: if gap_days > LONG_TRAINING_GAP_DAYS {
                        InsightSeverity::Warning
                    } else {
                        InsightSeverity::Info
                    },
                });
            }
        }

        // Check for missing training types
        let sports: std::collections::HashSet<_> =
            activities.iter().map(|a| &a.sport_type).collect();

        if let Some(profile) = &self.user_profile {
            for primary_sport in &profile.primary_sports {
                // Convert string to SportType for comparison - this is simplified
                if sports.is_empty() {
                    // Just check if sports exist for now
                    gaps.push(TrainingGap {
                        gap_type: GapType::MissingSport,
                        duration_days: 0,
                        description: format!(
                            "Missing {} training in recent activities",
                            primary_sport
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
                title: "Strategy-Based Volume Increase".to_string(),
                description: "Your current strategy suggests increasing training volume based on your profile.".to_string(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Strategic volume increases are tailored to your current fitness level and goals.".to_string(),
                actionable_steps: vec![
                    "Follow your strategy's volume progression guidelines".to_string(),
                    "Increase volume gradually as recommended by your training approach".to_string(),
                    "Monitor adaptation and adjust based on response".to_string(),
                ],
            });
        }

        // Use strategy frequency thresholds
        // Estimate weekly activities based on intensity threshold
        let weekly_activities =
            (analysis.weekly_load_hours / self.config.thresholds.intensity_threshold).ceil() as i32;
        if self.strategy.should_recommend_recovery(weekly_activities) {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Recovery,
                title: "Strategy-Based Recovery Focus".to_string(),
                description: "Your training strategy recommends focusing on recovery based on current frequency.".to_string(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Strategic recovery prevents overtraining and optimizes long-term performance gains.".to_string(),
                actionable_steps: vec![
                    "Reduce training frequency as per your strategy guidelines".to_string(),
                    "Include more recovery-focused activities".to_string(),
                    "Prioritize sleep and stress management".to_string(),
                ],
            });
        }

        recommendations
    }

    /// Generate sport diversity recommendations using sport_diversity and primary_sport fields
    fn generate_sport_diversity_recommendations(
        &self,
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        // Check sport diversity - if low, recommend cross-training
        if analysis.sport_diversity <= 1 {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Strategy,
                title: "Increase Sport Diversity".to_string(),
                description: format!("You're primarily focused on {}. Consider adding cross-training activities.", analysis.primary_sport),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Sport diversity helps prevent overuse injuries and provides balanced fitness development.".to_string(),
                actionable_steps: vec![
                    "Add one complementary sport to your routine".to_string(),
                    "Try swimming, cycling, or running as cross-training".to_string(),
                    "Use different sports for active recovery days".to_string(),
                ],
            });
        } else if analysis.sport_diversity >= 4 {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Strategy,
                title: "Focus Training Specificity".to_string(),
                description: format!("You're training in {} different sports. Consider focusing more on your primary sport: {}.", analysis.sport_diversity, analysis.primary_sport),
                priority: RecommendationPriority::Low,
                confidence: Confidence::Medium,
                rationale: "While cross-training is beneficial, too much diversity may limit specific performance gains.".to_string(),
                actionable_steps: vec![
                    format!("Increase focus on {} training sessions", analysis.primary_sport),
                    "Maintain 1-2 complementary sports for variety".to_string(),
                    "Ensure primary sport gets 60-70% of training time".to_string(),
                ],
            });
        }

        recommendations
    }

    /// Generate intensity-based recommendations
    fn generate_intensity_recommendations(
        &self,
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.intensity_balance > HIGH_INTENSITY_UPPER_LIMIT {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Intensity,
                title: "Add More Easy Training".to_string(),
                description: "Your training intensity is quite high. Consider adding more low-intensity, base-building sessions.".to_string(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "High-intensity training should typically make up only 20-30% of total training volume for optimal adaptation and recovery.".to_string(),
                actionable_steps: vec![
                    "Add 1-2 easy-paced sessions per week".to_string(),
                    "Keep heart rate below aerobic threshold (Zone 2)".to_string(),
                    "Focus on conversational pace".to_string(),
                ],
            });
        } else if analysis.intensity_balance < LOW_INTENSITY_LOWER_LIMIT {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Intensity,
                title: "Increase Training Intensity".to_string(),
                description: "Your training could benefit from more high-intensity sessions to improve performance.".to_string(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Including 20-30% high-intensity training can improve VO2 max, lactate threshold, and overall performance.".to_string(),
                actionable_steps: vec![
                    "Add 1 interval session per week".to_string(),
                    "Include tempo runs or threshold workouts".to_string(),
                    "Ensure proper recovery between hard sessions".to_string(),
                ],
            });
        }

        recommendations
    }

    /// Generate volume-based recommendations
    fn generate_volume_recommendations(
        &self,
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.weekly_load_hours < MIN_WEEKLY_VOLUME_HOURS {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Volume,
                title: "Gradually Increase Training Volume".to_string(),
                description: "Your current training volume could be increased for better fitness gains.".to_string(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Gradual volume increases of 10% per week can lead to improved fitness while minimizing injury risk.".to_string(),
                actionable_steps: vec![
                    "Add 15-20 minutes to your longest session each week".to_string(),
                    "Include one additional short session per week".to_string(),
                    "Monitor for signs of overtraining".to_string(),
                ],
            });
        } else if analysis.weekly_load_hours > HIGH_WEEKLY_VOLUME_HOURS {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Volume,
                title: "Monitor Training Load".to_string(),
                description: "High training volume detected. Ensure adequate recovery and listen to your body.".to_string(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Very high training loads increase injury risk and may lead to overtraining if recovery is inadequate.".to_string(),
                actionable_steps: vec![
                    "Schedule regular recovery weeks".to_string(),
                    "Monitor heart rate variability".to_string(),
                    "Prioritize sleep and nutrition".to_string(),
                ],
            });
        }

        recommendations
    }

    /// Generate consistency recommendations
    fn generate_consistency_recommendations(
        &self,
        analysis: &TrainingPatternAnalysis,
    ) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.consistency_score < CONSISTENCY_SCORE_THRESHOLD {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Strategy,
                title: "Improve Training Consistency".to_string(),
                description: "Focus on building a more consistent training routine for better adaptations.".to_string(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Consistent training stimulus is more effective than sporadic high-intensity efforts for long-term fitness gains.".to_string(),
                actionable_steps: vec![
                    "Schedule fixed training days in your calendar".to_string(),
                    "Start with shorter, manageable sessions".to_string(),
                    "Find an accountability partner or group".to_string(),
                    "Track your progress to stay motivated".to_string(),
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
                            _ => RecommendationPriority::Low,
                        };

                        recommendations.push(TrainingRecommendation {
                            recommendation_type: RecommendationType::Strategy,
                            title: "Avoid Long Training Breaks".to_string(),
                            description: format!("Recent {} gap detected. Try to maintain more consistent activity.", gap.description),
                            priority,
                            confidence: Confidence::Medium,
                            rationale: "Training breaks longer than a week can lead to fitness losses and increased injury risk when resuming.".to_string(),
                            actionable_steps: vec![
                                "Aim for at least one activity every 5-7 days".to_string(),
                                "Use easy sessions to maintain base fitness".to_string(),
                                "Plan ahead for busy periods".to_string(),
                            ],
                        });
                    }
                }
                GapType::MissingSport => {
                    // Use severity field to determine recommendation priority
                    let priority = match gap.severity {
                        InsightSeverity::Warning => RecommendationPriority::Medium,
                        InsightSeverity::Info => RecommendationPriority::Low,
                        _ => RecommendationPriority::Low,
                    };

                    recommendations.push(TrainingRecommendation {
                        recommendation_type: RecommendationType::Strategy,
                        title: "Include Cross-Training".to_string(),
                        description: gap.description.clone(),
                        priority,
                        confidence: Confidence::Medium,
                        rationale: "Cross-training helps prevent overuse injuries and maintains overall fitness.".to_string(),
                        actionable_steps: vec![
                            "Add 1 cross-training session per week".to_string(),
                            "Choose activities that complement your primary sport".to_string(),
                            "Use cross-training for active recovery".to_string(),
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
    ) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        // Analyze current training patterns
        let analysis = self.analyze_training_patterns(activities);

        // Generate different types of recommendations
        recommendations.extend(self.generate_intensity_recommendations(&analysis));
        recommendations.extend(self.generate_volume_recommendations(&analysis));
        recommendations.extend(self.generate_consistency_recommendations(&analysis));

        // Add strategy-specific recommendations using the strategy field
        recommendations.extend(self.generate_strategy_based_recommendations(&analysis));

        // Add sport diversity recommendations using sport_diversity and primary_sport fields
        recommendations.extend(self.generate_sport_diversity_recommendations(&analysis));

        // Fitness level specific recommendations
        match user_profile.fitness_level {
            FitnessLevel::Beginner => {
                recommendations.push(TrainingRecommendation {
                    recommendation_type: RecommendationType::Strategy,
                    title: "Focus on Building Base Fitness".to_string(),
                    description: "Prioritize consistency and gradual progression over intensity.".to_string(),
                    priority: RecommendationPriority::High,
                    confidence: Confidence::High,
                    rationale: "Building a strong aerobic base is crucial for beginners to support future training adaptations.".to_string(),
                    actionable_steps: vec![
                        "Start with 20-30 minute easy sessions".to_string(),
                        "Gradually increase duration by 10% each week".to_string(),
                        "Include rest days between sessions".to_string(),
                        "Focus on proper form and technique".to_string(),
                    ],
                });
            }
            FitnessLevel::Intermediate => {
                recommendations.push(TrainingRecommendation {
                    recommendation_type: RecommendationType::Strategy,
                    title: "Introduce Structured Training".to_string(),
                    description: "Add periodization and specific training phases to your routine.".to_string(),
                    priority: RecommendationPriority::Medium,
                    confidence: Confidence::High,
                    rationale: "Structured training helps intermediate athletes break through plateaus and continue improving.".to_string(),
                    actionable_steps: vec![
                        "Plan 4-6 week training blocks".to_string(),
                        "Include base, build, and peak phases".to_string(),
                        "Add sport-specific skill work".to_string(),
                        "Monitor training stress and recovery".to_string(),
                    ],
                });
            }
            FitnessLevel::Advanced | FitnessLevel::Elite => {
                recommendations.push(TrainingRecommendation {
                    recommendation_type: RecommendationType::Strategy,
                    title: "Optimize Training Specificity".to_string(),
                    description: "Fine-tune training to target specific performance limiters.".to_string(),
                    priority: RecommendationPriority::Medium,
                    confidence: Confidence::Medium,
                    rationale: "Advanced athletes benefit from highly specific training targeting individual weaknesses and performance goals.".to_string(),
                    actionable_steps: vec![
                        "Conduct regular performance testing".to_string(),
                        "Identify and target limiting factors".to_string(),
                        "Use advanced training metrics (power, pace zones)".to_string(),
                        "Include mental training and race tactics".to_string(),
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
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        Ok(recommendations.into_iter().take(8).collect()) // Return top 8 recommendations
    }

    async fn generate_recovery_recommendations(
        &self,
        activities: &[Activity],
    ) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        // Analyze recent training load
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date;
                let days_ago = (Utc::now() - activity_utc).num_days();
                days_ago <= RECOVERY_ANALYSIS_DAYS
            })
            .collect();

        let owned_activities: Vec<Activity> =
            recent_activities.iter().map(|a| (*a).clone()).collect();
        let total_duration: u64 = recent_activities.iter().map(|a| a.duration_seconds).sum();

        let high_intensity_sessions = recent_activities
            .iter()
            .filter(|a| a.average_heart_rate.unwrap_or(0) > crate::intelligence::physiological_constants::heart_rate::HIGH_INTENSITY_HR_THRESHOLD)
            .count();

        // Check if recovery is needed
        if total_duration > HIGH_WEEKLY_LOAD_SECONDS
            || high_intensity_sessions > MAX_HIGH_INTENSITY_SESSIONS_PER_WEEK
        {
            // >5 hours or >3 hard sessions
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Recovery,
                title: "Prioritize Recovery This Week".to_string(),
                description: "High training load detected. Focus on recovery activities.".to_string(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Adequate recovery prevents overtraining and allows for training adaptations to occur.".to_string(),
                actionable_steps: vec![
                    "Include at least 2 complete rest days".to_string(),
                    "Add gentle yoga or stretching sessions".to_string(),
                    "Prioritize 8+ hours of sleep".to_string(),
                    "Consider massage or foam rolling".to_string(),
                    "Stay hydrated and eat adequate protein".to_string(),
                ],
            });
        }

        // Check for consecutive training days
        let consecutive_days = self.count_consecutive_training_days(&owned_activities);
        if consecutive_days > MAX_CONSECUTIVE_TRAINING_DAYS {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Recovery,
                title: "Take a Rest Day".to_string(),
                description: format!("{} consecutive training days detected.", consecutive_days),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Regular rest days are essential for physical and mental recovery."
                    .to_string(),
                actionable_steps: vec![
                    "Schedule a complete rest day today".to_string(),
                    "Focus on nutrition and hydration".to_string(),
                    "Light walking or gentle stretching only".to_string(),
                ],
            });
        }

        Ok(recommendations)
    }

    async fn generate_nutrition_recommendations(
        &self,
        activity: &Activity,
    ) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        let duration_hours = activity.duration_seconds as f64 / 3600.0;
        let high_intensity =
            activity.average_heart_rate.unwrap_or(0) > MODERATE_NUTRITION_HR_THRESHOLD;

        // Pre-activity nutrition
        if duration_hours > PRE_EXERCISE_DURATION_THRESHOLD {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Nutrition,
                title: "Pre-Exercise Fueling".to_string(),
                description: "Proper pre-exercise nutrition for longer sessions.".to_string(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Adequate carbohydrate intake before longer sessions maintains energy levels and performance.".to_string(),
                actionable_steps: vec![
                    "Eat 30-60g carbohydrates 1-2 hours before exercise".to_string(),
                    "Include easily digestible foods (banana, oatmeal, toast)".to_string(),
                    "Avoid high fiber and fat before training".to_string(),
                    "Stay hydrated leading up to exercise".to_string(),
                ],
            });
        }

        // During-activity nutrition
        if duration_hours > DURING_EXERCISE_DURATION_THRESHOLD {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Nutrition,
                title: "In-Exercise Fueling".to_string(),
                description: "Maintain energy during long training sessions.".to_string(),
                priority: RecommendationPriority::High,
                confidence: Confidence::High,
                rationale: "Consuming carbohydrates during exercise >2 hours prevents glycogen depletion and maintains performance.".to_string(),
                actionable_steps: vec![
                    "Consume 30-60g carbohydrates per hour after the first hour".to_string(),
                    "Use sports drinks, gels, or easily digestible snacks".to_string(),
                    "Drink 150-250ml fluid every 15-20 minutes".to_string(),
                    "Practice fueling strategy during training".to_string(),
                ],
            });
        }

        // Post-activity recovery
        if duration_hours > POST_EXERCISE_DURATION_THRESHOLD || high_intensity {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Nutrition,
                title: "Post-Exercise Recovery Nutrition".to_string(),
                description: "Optimize recovery with proper post-exercise nutrition.".to_string(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Post-exercise nutrition within 30-60 minutes optimizes glycogen replenishment and muscle protein synthesis.".to_string(),
                actionable_steps: vec![
                    "Consume 1-1.2g carbohydrates per kg body weight within 30 minutes".to_string(),
                    "Include 20-25g high-quality protein".to_string(),
                    "Rehydrate with 150% of fluid losses".to_string(),
                    "Consider chocolate milk, recovery smoothie, or balanced meal".to_string(),
                ],
            });
        }

        Ok(recommendations)
    }

    async fn generate_equipment_recommendations(
        &self,
        _user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();

        // Analyze primary sports
        let mut sport_counts: HashMap<String, usize> = HashMap::new();
        for activity in activities {
            *sport_counts
                .entry(format!("{:?}", activity.sport_type))
                .or_insert(0) += 1;
        }

        // Running-specific equipment
        if sport_counts.get("Run").unwrap_or(&0) > &(MAX_WEEKLY_FREQUENCY as usize) {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Equipment,
                title: "Running Equipment Optimization".to_string(),
                description: "Optimize your running gear for better performance and injury prevention.".to_string(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Proper running equipment reduces injury risk and can improve performance and comfort.".to_string(),
                actionable_steps: vec![
                    "Get professional gait analysis and shoe fitting".to_string(),
                    "Replace running shoes every 500-800km".to_string(),
                    "Consider moisture-wicking clothing for longer runs".to_string(),
                    "Use GPS watch or smartphone app for pacing".to_string(),
                ],
            });
        }

        // Cycling-specific equipment
        if sport_counts.get("Ride").unwrap_or(&0) > &(MAX_WEEKLY_FREQUENCY as usize) {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Equipment,
                title: "Cycling Equipment Optimization".to_string(),
                description: "Enhance your cycling setup for efficiency and comfort.".to_string(),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::Medium,
                rationale: "Proper bike fit and equipment can significantly improve cycling efficiency and reduce injury risk.".to_string(),
                actionable_steps: vec![
                    "Get professional bike fit assessment".to_string(),
                    "Ensure proper helmet fit and replacement schedule".to_string(),
                    "Consider power meter for training precision".to_string(),
                    "Maintain bike regularly for optimal performance".to_string(),
                ],
            });
        }

        // General monitoring equipment
        let has_hr_data = activities.iter().any(|a| a.average_heart_rate.is_some());
        if !has_hr_data && activities.len() > MAX_WEEKLY_FREQUENCY as usize {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Equipment,
                title: "Heart Rate Monitoring".to_string(),
                description: "Consider adding heart rate monitoring to your training.".to_string(),
                priority: RecommendationPriority::Low,
                confidence: Confidence::Medium,
                rationale: "Heart rate data provides valuable insights into training intensity, recovery, and overall fitness progress.".to_string(),
                actionable_steps: vec![
                    "Consider chest strap or wrist-based heart rate monitor".to_string(),
                    "Learn your heart rate zones".to_string(),
                    "Use HR data to guide training intensity".to_string(),
                    "Track resting heart rate for recovery monitoring".to_string(),
                ],
            });
        }

        Ok(recommendations)
    }
}

impl AdvancedRecommendationEngine {
    /// Count consecutive training days
    fn count_consecutive_training_days(&self, activities: &[Activity]) -> usize {
        let mut consecutive = 0;
        let mut current_date = Utc::now().date_naive();

        // Sort activities by date (most recent first)
        let mut sorted_activities = activities.to_vec();
        sorted_activities.sort_by(|a, b| {
            let date_a = Some(a.start_date);
            let date_b = Some(b.start_date);
            date_b.cmp(&date_a) // Reverse order (newest first)
        });

        for activity in sorted_activities {
            let activity_naive = activity.start_date.naive_utc().date();

            if activity_naive == current_date
                || activity_naive == current_date - chrono::naive::Days::new(1)
            {
                consecutive += 1;
                current_date = activity_naive - chrono::naive::Days::new(1);
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
#[derive(Debug)]
enum GapType {
    LongRest,
    MissingSport,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_training_recommendations() {
        let profile = UserFitnessProfile {
            user_id: "test_user".to_string(),
            age: Some(30),
            gender: Some("M".to_string()),
            weight: Some(70.0),
            height: Some(175.0),
            fitness_level: FitnessLevel::Intermediate,
            primary_sports: vec!["Run".to_string()],
            training_history_months: 12,
            preferences: UserPreferences {
                preferred_units: "metric".to_string(),
                training_focus: vec!["endurance".to_string()],
                injury_history: vec![],
                time_availability: TimeAvailability {
                    hours_per_week: 5.0,
                    preferred_days: vec![
                        "Monday".to_string(),
                        "Wednesday".to_string(),
                        "Friday".to_string(),
                    ],
                    preferred_duration_minutes: Some(60),
                },
            },
        };

        let engine = AdvancedRecommendationEngine::<
            crate::config::intelligence_config::DefaultStrategy,
        >::with_profile(profile.clone());

        // Create sample activities with high intensity
        let mut activities = Vec::new();
        for i in 0..10 {
            let activity = Activity {
                sport_type: crate::models::SportType::Run,
                average_heart_rate: Some(170), // High intensity
                duration_seconds: 3600,        // 1 hour
                start_date: Utc::now() - Duration::days(i * 2),
                ..Activity::default()
            };
            activities.push(activity);
        }

        let result = engine.generate_recommendations(&profile, &activities).await;
        assert!(result.is_ok());

        let recommendations = result.unwrap();
        assert!(!recommendations.is_empty());

        // Should recommend adding easy training due to high intensity
        assert!(recommendations.iter().any(|r| r.title.contains("Easy")));
    }

    #[tokio::test]
    async fn test_recovery_recommendations() {
        let engine = AdvancedRecommendationEngine::new();

        // Create high load activities
        let mut activities = Vec::new();
        for i in 0..7 {
            let activity = Activity {
                average_heart_rate: Some(170),
                duration_seconds: 7200, // 2 hours each
                start_date: Utc::now() - Duration::days(i),
                ..Activity::default()
            };
            activities.push(activity);
        }

        let result = engine.generate_recovery_recommendations(&activities).await;
        assert!(result.is_ok());

        let recommendations = result.unwrap();
        assert!(!recommendations.is_empty());

        // Should recommend recovery due to high load
        assert!(recommendations
            .iter()
            .any(|r| r.recommendation_type == RecommendationType::Recovery));
    }
}
