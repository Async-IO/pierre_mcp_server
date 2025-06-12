//! Training recommendation engine for personalized insights

use super::*;
use crate::models::Activity;
use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

/// Trait for generating training recommendations
#[async_trait::async_trait]
pub trait RecommendationEngineTrait {
    /// Generate personalized training recommendations
    async fn generate_recommendations(&self, user_profile: &UserFitnessProfile, activities: &[Activity]) -> Result<Vec<TrainingRecommendation>>;
    
    /// Generate recovery recommendations based on training load
    async fn generate_recovery_recommendations(&self, activities: &[Activity]) -> Result<Vec<TrainingRecommendation>>;
    
    /// Generate nutrition recommendations for activities
    async fn generate_nutrition_recommendations(&self, activity: &Activity) -> Result<Vec<TrainingRecommendation>>;
    
    /// Generate equipment recommendations
    async fn generate_equipment_recommendations(&self, user_profile: &UserFitnessProfile, activities: &[Activity]) -> Result<Vec<TrainingRecommendation>>;
}

/// Advanced recommendation engine implementation
pub struct AdvancedRecommendationEngine {
    user_profile: Option<UserFitnessProfile>,
}

impl AdvancedRecommendationEngine {
    /// Create a new recommendation engine
    pub fn new() -> Self {
        Self {
            user_profile: None,
        }
    }

    /// Create engine with user profile
    pub fn with_profile(profile: UserFitnessProfile) -> Self {
        Self {
            user_profile: Some(profile),
        }
    }

    /// Analyze training patterns to identify areas for improvement
    fn analyze_training_patterns(&self, activities: &[Activity]) -> TrainingPatternAnalysis {
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                if let Some(start_time) = &a.start_date_local {
                    if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                        let activity_utc = activity_date.with_timezone(&Utc);
                        let weeks_ago = (Utc::now() - activity_utc).num_weeks();
                        return weeks_ago <= 4; // Last 4 weeks
                    }
                }
                false
            })
            .collect();

        let mut sport_frequency: HashMap<String, usize> = HashMap::new();
        let mut weekly_load = 0.0;
        let mut high_intensity_count = 0;
        let mut low_intensity_count = 0;
        let mut total_duration = 0;

        for activity in &recent_activities {
            *sport_frequency.entry(activity.sport_type.clone()).or_insert(0) += 1;
            
            if let Some(duration) = activity.moving_time {
                weekly_load += duration as f64 / 3600.0; // Hours
                total_duration += duration;
            }
            
            if let Some(avg_hr) = activity.average_heartrate {
                if avg_hr > 160.0 {
                    high_intensity_count += 1;
                } else if avg_hr < 130.0 {
                    low_intensity_count += 1;
                }
            }
        }

        weekly_load /= 4.0; // Average per week
        
        let intensity_balance = if recent_activities.len() > 0 {
            high_intensity_count as f64 / recent_activities.len() as f64
        } else {
            0.0
        };

        let consistency_score = if recent_activities.len() >= 12 { // 3+ per week
            1.0
        } else if recent_activities.len() >= 8 { // 2+ per week
            0.75
        } else if recent_activities.len() >= 4 { // 1+ per week
            0.5
        } else {
            0.25
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
            let date_a = a.start_date_local.as_ref().and_then(|d| DateTime::parse_from_rfc3339(d).ok());
            let date_b = b.start_date_local.as_ref().and_then(|d| DateTime::parse_from_rfc3339(d).ok());
            date_a.cmp(&date_b)
        });

        for i in 1..sorted_activities.len() {
            if let (Some(prev_date_str), Some(curr_date_str)) = 
                (&sorted_activities[i-1].start_date_local, &sorted_activities[i].start_date_local) {
                if let (Ok(prev_date), Ok(curr_date)) = 
                    (DateTime::parse_from_rfc3339(prev_date_str), DateTime::parse_from_rfc3339(curr_date_str)) {
                    let gap_days = (curr_date - prev_date).num_days();
                    
                    if gap_days > 7 {
                        gaps.push(TrainingGap {
                            gap_type: GapType::LongRest,
                            duration_days: gap_days,
                            description: format!("{} days without training", gap_days),
                            severity: if gap_days > 14 { InsightSeverity::Warning } else { InsightSeverity::Info },
                        });
                    }
                }
            }
        }

        // Check for missing training types
        let sports: std::collections::HashSet<_> = activities.iter().map(|a| &a.sport_type).collect();
        
        if let Some(profile) = &self.user_profile {
            for primary_sport in &profile.primary_sports {
                if !sports.contains(&primary_sport) {
                    gaps.push(TrainingGap {
                        gap_type: GapType::MissingSport,
                        duration_days: 0,
                        description: format!("Missing {} training in recent activities", primary_sport),
                        severity: InsightSeverity::Info,
                    });
                }
            }
        }

        gaps
    }

    /// Generate intensity-based recommendations
    fn generate_intensity_recommendations(&self, analysis: &TrainingPatternAnalysis) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.intensity_balance > 0.6 {
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
        } else if analysis.intensity_balance < 0.2 {
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
    fn generate_volume_recommendations(&self, analysis: &TrainingPatternAnalysis) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.weekly_load_hours < 3.0 {
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
        } else if analysis.weekly_load_hours > 15.0 {
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
    fn generate_consistency_recommendations(&self, analysis: &TrainingPatternAnalysis) -> Vec<TrainingRecommendation> {
        let mut recommendations = Vec::new();

        if analysis.consistency_score < 0.5 {
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
                    if gap.duration_days > 10 {
                        recommendations.push(TrainingRecommendation {
                            recommendation_type: RecommendationType::Strategy,
                            title: "Avoid Long Training Breaks".to_string(),
                            description: format!("Recent {} gap detected. Try to maintain more consistent activity.", gap.description),
                            priority: RecommendationPriority::Medium,
                            confidence: Confidence::Medium,
                            rationale: "Training breaks longer than a week can lead to fitness losses and increased injury risk when resuming.".to_string(),
                            actionable_steps: vec![
                                "Aim for at least one activity every 5-7 days".to_string(),
                                "Use easy sessions to maintain base fitness".to_string(),
                                "Plan ahead for busy periods".to_string(),
                            ],
                        });
                    }
                },
                GapType::MissingSport => {
                    recommendations.push(TrainingRecommendation {
                        recommendation_type: RecommendationType::Strategy,
                        title: "Include Cross-Training".to_string(),
                        description: gap.description.clone(),
                        priority: RecommendationPriority::Low,
                        confidence: Confidence::Medium,
                        rationale: "Cross-training helps prevent overuse injuries and maintains overall fitness.".to_string(),
                        actionable_steps: vec![
                            "Add 1 cross-training session per week".to_string(),
                            "Choose activities that complement your primary sport".to_string(),
                            "Use cross-training for active recovery".to_string(),
                        ],
                    });
                },
            }
        }

        recommendations
    }
}

#[async_trait::async_trait]
impl RecommendationEngineTrait for AdvancedRecommendationEngine {
    async fn generate_recommendations(&self, user_profile: &UserFitnessProfile, activities: &[Activity]) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();
        
        // Analyze current training patterns
        let analysis = self.analyze_training_patterns(activities);
        
        // Generate different types of recommendations
        recommendations.extend(self.generate_intensity_recommendations(&analysis));
        recommendations.extend(self.generate_volume_recommendations(&analysis));
        recommendations.extend(self.generate_consistency_recommendations(&analysis));
        
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
            },
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
            },
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
            },
        }

        // Sort by priority and confidence
        recommendations.sort_by(|a, b| {
            let priority_order = |p: &RecommendationPriority| match p {
                RecommendationPriority::Critical => 4,
                RecommendationPriority::High => 3,
                RecommendationPriority::Medium => 2,
                RecommendationPriority::Low => 1,
            };
            
            priority_order(&b.priority).cmp(&priority_order(&a.priority))
                .then_with(|| b.confidence.as_score().partial_cmp(&a.confidence.as_score()).unwrap_or(std::cmp::Ordering::Equal))
        });

        Ok(recommendations.into_iter().take(8).collect()) // Return top 8 recommendations
    }

    async fn generate_recovery_recommendations(&self, activities: &[Activity]) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();
        
        // Analyze recent training load
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                if let Some(start_time) = &a.start_date_local {
                    if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                        let activity_utc = activity_date.with_timezone(&Utc);
                        let days_ago = (Utc::now() - activity_utc).num_days();
                        return days_ago <= 7; // Last week
                    }
                }
                false
            })
            .collect();

        let total_duration: i32 = recent_activities
            .iter()
            .filter_map(|a| a.moving_time)
            .sum();

        let high_intensity_sessions = recent_activities
            .iter()
            .filter(|a| a.average_heartrate.unwrap_or(0.0) > 160.0)
            .count();

        // Check if recovery is needed
        if total_duration > 18000 || high_intensity_sessions > 3 { // >5 hours or >3 hard sessions
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
        let consecutive_days = self.count_consecutive_training_days(&recent_activities);
        if consecutive_days > 5 {
            recommendations.push(TrainingRecommendation {
                recommendation_type: RecommendationType::Recovery,
                title: "Take a Rest Day".to_string(),
                description: format!("{} consecutive training days detected.", consecutive_days),
                priority: RecommendationPriority::Medium,
                confidence: Confidence::High,
                rationale: "Regular rest days are essential for physical and mental recovery.".to_string(),
                actionable_steps: vec![
                    "Schedule a complete rest day today".to_string(),
                    "Focus on nutrition and hydration".to_string(),
                    "Light walking or gentle stretching only".to_string(),
                ],
            });
        }

        Ok(recommendations)
    }

    async fn generate_nutrition_recommendations(&self, activity: &Activity) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();
        
        let duration_hours = activity.moving_time.unwrap_or(0) as f64 / 3600.0;
        let high_intensity = activity.average_heartrate.unwrap_or(0.0) > 150.0;

        // Pre-activity nutrition
        if duration_hours > 1.5 {
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
        if duration_hours > 2.0 {
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
        if duration_hours > 1.0 || high_intensity {
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

    async fn generate_equipment_recommendations(&self, user_profile: &UserFitnessProfile, activities: &[Activity]) -> Result<Vec<TrainingRecommendation>> {
        let mut recommendations = Vec::new();
        
        // Analyze primary sports
        let mut sport_counts: HashMap<String, usize> = HashMap::new();
        for activity in activities {
            *sport_counts.entry(activity.sport_type.clone()).or_insert(0) += 1;
        }

        // Running-specific equipment
        if sport_counts.get("Run").unwrap_or(&0) > &5 {
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
        if sport_counts.get("Ride").unwrap_or(&0) > &5 {
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
        let has_hr_data = activities.iter().any(|a| a.average_heartrate.is_some());
        if !has_hr_data && activities.len() > 5 {
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
            let date_a = a.start_date_local.as_ref().and_then(|d| DateTime::parse_from_rfc3339(d).ok());
            let date_b = b.start_date_local.as_ref().and_then(|d| DateTime::parse_from_rfc3339(d).ok());
            date_b.cmp(&date_a) // Reverse order (newest first)
        });

        for activity in sorted_activities {
            if let Some(start_time) = &activity.start_date_local {
                if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                    let activity_naive = activity_date.naive_utc().date();
                    
                    if activity_naive == current_date || activity_naive == current_date - chrono::naive::Days::new(1) {
                        consecutive += 1;
                        current_date = activity_naive - chrono::naive::Days::new(1);
                    } else {
                        break;
                    }
                }
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
                    preferred_days: vec!["Monday".to_string(), "Wednesday".to_string(), "Friday".to_string()],
                    preferred_duration_minutes: Some(60),
                },
            },
        };
        
        let engine = AdvancedRecommendationEngine::with_profile(profile.clone());
        
        // Create sample activities with high intensity
        let mut activities = Vec::new();
        for i in 0..10 {
            let mut activity = Activity::default();
            activity.sport_type = "Run".to_string();
            activity.average_heartrate = Some(170.0); // High intensity
            activity.moving_time = Some(3600); // 1 hour
            activity.start_date_local = Some(
                (Utc::now() - Duration::days(i * 2)).to_rfc3339()
            );
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
            let mut activity = Activity::default();
            activity.average_heartrate = Some(170.0);
            activity.moving_time = Some(7200); // 2 hours each
            activity.start_date_local = Some(
                (Utc::now() - Duration::days(i)).to_rfc3339()
            );
            activities.push(activity);
        }
        
        let result = engine.generate_recovery_recommendations(&activities).await;
        assert!(result.is_ok());
        
        let recommendations = result.unwrap();
        assert!(!recommendations.is_empty());
        
        // Should recommend recovery due to high load
        assert!(recommendations.iter().any(|r| r.recommendation_type == RecommendationType::Recovery));
    }
}