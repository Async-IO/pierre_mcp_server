// ABOUTME: Nutrition calculation algorithms using peer-reviewed scientific formulas
// ABOUTME: BMR, TDEE, macronutrient distribution, and meal timing calculations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Nutrition Calculator Module
//!
//! This module implements evidence-based nutrition calculations for athletes and active individuals.
//! All formulas are based on peer-reviewed research with citations.
//!
//! # Scientific References
//!
//! - Mifflin, M.D., et al. (1990). A new predictive equation for resting energy expenditure.
//!   *American Journal of Clinical Nutrition*, 51(2), 241-247.
//!   <https://doi.org/10.1093/ajcn/51.2.241>
//!
//! - Phillips, S.M., & Van Loon, L.J. (2011). Dietary protein for athletes.
//!   *Journal of Sports Sciences*, 29(sup1), S29-S38.
//!   <https://doi.org/10.1080/02640414.2011.619204>
//!
//! - Burke, L.M., et al. (2011). Carbohydrates for training and competition.
//!   *Journal of Sports Sciences*, 29(sup1), S17-S27.
//!   <https://doi.org/10.1080/02640414.2011.585473>
//!
//! - Kerksick, C.M., et al. (2017). Nutrient timing position stand.
//!   *Journal of the International Society of Sports Nutrition*, 14, 33.
//!   <https://doi.org/10.1186/s12970-017-0189-4>

use crate::config::intelligence_config::{
    ActivityFactorsConfig, BmrConfig, MacronutrientConfig, NutrientTimingConfig,
};
use crate::errors::AppError;
use serde::{Deserialize, Serialize};

/// Gender for BMR calculations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Gender {
    /// Male gender (higher BMR)
    Male,
    /// Female gender (lower BMR)
    Female,
}

/// Activity level for TDEE calculation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActivityLevel {
    /// Sedentary (little/no exercise)
    Sedentary,
    /// Lightly active (1-3 days/week)
    LightlyActive,
    /// Moderately active (3-5 days/week)
    ModeratelyActive,
    /// Very active (6-7 days/week)
    VeryActive,
    /// Extra active (hard training 2x/day)
    ExtraActive,
}

/// Training goal for macronutrient distribution
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrainingGoal {
    /// Weight loss (caloric deficit)
    WeightLoss,
    /// Maintenance (caloric balance)
    Maintenance,
    /// Muscle gain (caloric surplus)
    MuscleGain,
    /// Endurance performance (high carb)
    EndurancePerformance,
    /// Strength performance (high protein)
    StrengthPerformance,
}

/// Workout intensity for nutrient timing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkoutIntensity {
    /// Low intensity (<50% max HR)
    Low,
    /// Moderate intensity (50-75% max HR)
    Moderate,
    /// High intensity (>75% max HR)
    High,
}

/// Complete daily nutrition needs calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyNutritionNeeds {
    /// Basal Metabolic Rate (BMR) in kcal/day
    pub bmr: f64,

    /// Total Daily Energy Expenditure (TDEE) in kcal/day
    pub tdee: f64,

    /// Recommended daily protein (grams)
    pub protein_g: f64,

    /// Recommended daily carbohydrates (grams)
    pub carbs_g: f64,

    /// Recommended daily fats (grams)
    pub fat_g: f64,

    /// Macronutrient percentages
    pub macro_percentages: MacroPercentages,

    /// Calculation method used
    pub method: String,

    /// Activity level used
    pub activity_level: ActivityLevel,

    /// Training goal used
    pub training_goal: TrainingGoal,
}

/// Macronutrient percentage breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroPercentages {
    /// Protein as percentage of total calories
    pub protein_percent: f64,
    /// Carbohydrates as percentage of total calories
    pub carbs_percent: f64,
    /// Fat as percentage of total calories
    pub fat_percent: f64,
}

/// Nutrient timing recommendations for workout nutrition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrientTimingPlan {
    /// Pre-workout nutrition recommendations
    pub pre_workout: PreWorkoutNutrition,
    /// Post-workout nutrition recommendations
    pub post_workout: PostWorkoutNutrition,
    /// Daily protein distribution strategy
    pub daily_protein_distribution: ProteinDistribution,
}

/// Pre-workout nutrition recommendations based on workout intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreWorkoutNutrition {
    /// Carbohydrates (grams)
    pub carbs_g: f64,
    /// Timing before workout (hours)
    pub timing_hours_before: f64,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Post-workout nutrition recommendations for recovery and adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostWorkoutNutrition {
    /// Protein (grams)
    pub protein_g: f64,
    /// Carbohydrates (grams)
    pub carbs_g: f64,
    /// Timing after workout (hours)
    pub timing_hours_after: f64,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Daily protein distribution strategy for optimal muscle protein synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinDistribution {
    /// Recommended meals per day
    pub meals_per_day: u8,
    /// Protein per meal (grams)
    pub protein_per_meal_g: f64,
    /// Distribution strategy
    pub strategy: String,
}

/// Calculate Basal Metabolic Rate using Mifflin-St Jeor equation (1990)
///
/// Formula: BMR = (10 x `weight_kg`) + (6.25 x `height_cm`) - (5 x age) + `gender_offset`
/// - Men: +5
/// - Women: -161
///
/// # Arguments
/// * `weight_kg` - Body weight in kilograms
/// * `height_cm` - Height in centimeters
/// * `age` - Age in years
/// * `gender` - Male or Female
/// * `config` - BMR configuration with formula coefficients
///
/// # Reference
/// Mifflin et al. (1990) DOI: 10.1093/ajcn/51.2.241
///
/// # Errors
///
/// Returns an error if input values are out of valid ranges
pub fn calculate_mifflin_st_jeor(
    weight_kg: f64,
    height_cm: f64,
    age: u32,
    gender: Gender,
    config: &BmrConfig,
) -> Result<f64, AppError> {
    // Validate inputs
    if weight_kg <= 0.0 || weight_kg > 300.0 {
        return Err(AppError::invalid_input(
            "Weight must be between 0 and 300 kg",
        ));
    }
    if height_cm <= 0.0 || height_cm > 300.0 {
        return Err(AppError::invalid_input(
            "Height must be between 0 and 300 cm",
        ));
    }
    if !(10..=120).contains(&age) {
        return Err(AppError::invalid_input(
            "Age must be between 10 and 120 years (Mifflin-St Jeor formula validated for ages 10+)",
        ));
    }

    // Mifflin-St Jeor formula components
    let weight_component = config.msj_weight_coef * weight_kg;
    let height_component = config.msj_height_coef * height_cm;
    let age_component = config.msj_age_coef * f64::from(age);

    let gender_constant = match gender {
        Gender::Male => config.msj_male_constant,
        Gender::Female => config.msj_female_constant,
    };

    let bmr = weight_component + height_component + age_component + gender_constant;

    // Minimum 1000 kcal/day safety check
    Ok(bmr.max(1000.0))
}

/// Calculate Total Daily Energy Expenditure (TDEE)
///
/// Formula: TDEE = BMR x Activity Factor
///
/// Activity factors based on `McArdle` et al. (2010):
/// - Sedentary: 1.2 (little/no exercise)
/// - Lightly active: 1.375 (1-3 days/week)
/// - Moderately active: 1.55 (3-5 days/week)
/// - Very active: 1.725 (6-7 days/week)
/// - Extra active: 1.9 (hard training 2x/day)
///
/// # Arguments
/// * `bmr` - Basal Metabolic Rate (kcal/day)
/// * `activity_level` - Activity level category
/// * `config` - Activity factor configuration
///
/// # Reference
/// `McArdle` et al. (2010) - Exercise Physiology
///
/// # Errors
///
/// Returns an error if BMR is not positive
pub fn calculate_tdee(
    bmr: f64,
    activity_level: ActivityLevel,
    config: &ActivityFactorsConfig,
) -> Result<f64, AppError> {
    if bmr <= 0.0 {
        return Err(AppError::invalid_input("BMR must be positive"));
    }

    let activity_factor = match activity_level {
        ActivityLevel::Sedentary => config.sedentary,
        ActivityLevel::LightlyActive => config.lightly_active,
        ActivityLevel::ModeratelyActive => config.moderately_active,
        ActivityLevel::VeryActive => config.very_active,
        ActivityLevel::ExtraActive => config.extra_active,
    };

    Ok(bmr * activity_factor)
}

/// Calculate recommended daily protein intake
///
/// Formula: Protein (g) = `weight_kg` x `protein_factor`
///
/// Protein recommendations (Phillips & Van Loon 2011):
/// - Sedentary: 0.8 g/kg (DRI minimum)
/// - Moderate activity: 1.2-1.4 g/kg
/// - Athletes: 1.6-2.2 g/kg
///   - Endurance: 1.6-2.0 g/kg
///   - Strength: 1.8-2.2 g/kg
///
/// # Arguments
/// * `weight_kg` - Body weight in kilograms
/// * `activity_level` - Activity level category
/// * `training_goal` - Training goal (affects protein needs)
/// * `config` - Macronutrient configuration
///
/// # Reference
/// Phillips & Van Loon (2011) DOI: 10.1080/02640414.2011.619204
///
/// # Errors
///
/// Returns an error if weight is not positive
pub fn calculate_protein_needs(
    weight_kg: f64,
    activity_level: ActivityLevel,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    if weight_kg <= 0.0 {
        return Err(AppError::invalid_input("Weight must be positive"));
    }

    let protein_g_per_kg = match (activity_level, training_goal) {
        // Sedentary baseline (DRI minimum)
        (ActivityLevel::Sedentary, _) => config.protein_min_g_per_kg,

        // Moderate activity
        (ActivityLevel::LightlyActive | ActivityLevel::ModeratelyActive, _) => {
            config.protein_moderate_g_per_kg
        }

        // Athletic - goal-specific
        (
            ActivityLevel::VeryActive | ActivityLevel::ExtraActive,
            TrainingGoal::EndurancePerformance,
        ) => config.protein_endurance_max_g_per_kg,
        (
            ActivityLevel::VeryActive | ActivityLevel::ExtraActive,
            TrainingGoal::StrengthPerformance | TrainingGoal::MuscleGain,
        ) => config.protein_strength_max_g_per_kg,

        // Weight loss: higher protein for muscle preservation
        (_, TrainingGoal::WeightLoss) => config.protein_athlete_g_per_kg,

        // Default for very/extra active
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, _) => {
            config.protein_athlete_g_per_kg
        }
    };

    Ok(weight_kg * protein_g_per_kg)
}

/// Calculate recommended daily carbohydrate intake
///
/// Formula: Carbs (g) = `weight_kg` x `carb_factor`
///
/// Carbohydrate recommendations (Burke et al. 2011):
/// - Low activity: 3 g/kg
/// - Moderate activity: 5-7 g/kg
/// - High endurance: 8-12 g/kg
///
/// # Arguments
/// * `weight_kg` - Body weight in kilograms
/// * `activity_level` - Activity level category
/// * `training_goal` - Training goal (affects carb needs)
/// * `config` - Macronutrient configuration
///
/// # Reference
/// Burke et al. (2011) DOI: 10.1080/02640414.2011.585473
///
/// # Errors
///
/// Returns an error if weight is not positive
pub fn calculate_carb_needs(
    weight_kg: f64,
    activity_level: ActivityLevel,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    if weight_kg <= 0.0 {
        return Err(AppError::invalid_input("Weight must be positive"));
    }

    let carbs_g_per_kg = match (activity_level, training_goal) {
        // Low activity
        (ActivityLevel::Sedentary | ActivityLevel::LightlyActive, _) => {
            config.carbs_low_activity_g_per_kg
        }

        // Endurance athletes need high carbs
        (_, TrainingGoal::EndurancePerformance) => config.carbs_high_endurance_g_per_kg,

        // Moderate activity
        (ActivityLevel::ModeratelyActive, _) => config.carbs_moderate_activity_g_per_kg,

        // Very/extra active (non-endurance) - slightly higher than moderate
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, _) => {
            config.carbs_moderate_activity_g_per_kg * 1.2
        }
    };

    Ok(weight_kg * carbs_g_per_kg)
}

/// Calculate recommended daily fat intake
///
/// Formula: Fat (g) = (TDEE x `fat_percentage`) / 9 kcal/g
///
/// Fat recommendations (DRI guidelines):
/// - Minimum: 20% of TDEE
/// - Optimal: 25-30% of TDEE
/// - Maximum: 35% of TDEE
///
/// Fat is calculated as the remaining calories after protein and carbs,
/// with bounds enforcement to meet DRI guidelines.
///
/// # Arguments
/// * `tdee` - Total Daily Energy Expenditure (kcal/day)
/// * `protein_g` - Daily protein intake (grams)
/// * `carbs_g` - Daily carbohydrate intake (grams)
/// * `training_goal` - Training goal (affects fat targeting)
/// * `config` - Macronutrient configuration
///
/// # Reference
/// DRI (Dietary Reference Intakes) - Institute of Medicine
///
/// # Errors
///
/// Returns an error if TDEE is not positive
pub fn calculate_fat_needs(
    tdee: f64,
    protein_g: f64,
    carbs_g: f64,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    if tdee <= 0.0 {
        return Err(AppError::invalid_input("TDEE must be positive"));
    }

    // Calculate remaining calories after protein and carbs
    // Protein: 4 kcal/g, Carbs: 4 kcal/g
    let protein_kcal = protein_g * 4.0;
    let carbs_kcal = carbs_g * 4.0;
    let fat_kcal_available = tdee - protein_kcal - carbs_kcal;

    // Goal-specific fat targeting
    let target_fat_percent = match training_goal {
        TrainingGoal::WeightLoss => config.fat_min_percent_tdee, // Lower fat for calorie deficit
        TrainingGoal::MuscleGain | TrainingGoal::StrengthPerformance => {
            config.fat_optimal_percent_tdee - 2.5 // Slightly lower, more protein/carbs
        }
        TrainingGoal::EndurancePerformance | TrainingGoal::Maintenance => {
            config.fat_optimal_percent_tdee
        }
    };

    // Calculate from available calories or target percentage (whichever is higher)
    // Fat: 9 kcal/g
    let fat_from_remainder = fat_kcal_available / 9.0;
    let fat_from_target = (tdee * target_fat_percent / 100.0) / 9.0;

    let fat_g = fat_from_remainder.max(fat_from_target);

    // Enforce minimum and maximum
    let min_fat = (tdee * config.fat_min_percent_tdee / 100.0) / 9.0;
    let max_fat = (tdee * config.fat_max_percent_tdee / 100.0) / 9.0;

    Ok(fat_g.clamp(min_fat, max_fat))
}

/// User parameters for daily nutrition calculation
pub struct DailyNutritionParams {
    /// Body weight in kilograms
    pub weight_kg: f64,
    /// Height in centimeters
    pub height_cm: f64,
    /// Age in years
    pub age: u32,
    /// Biological gender for BMR calculation
    pub gender: Gender,
    /// Activity level for TDEE multiplier
    pub activity_level: ActivityLevel,
    /// Training goal for macro distribution
    pub training_goal: TrainingGoal,
}

/// Calculate complete daily nutrition needs
///
/// This is the main entry point combining BMR, TDEE, and macronutrient calculations.
///
/// # Arguments
/// * `params` - User biometric and lifestyle parameters
/// * `bmr_config` - BMR configuration
/// * `activity_config` - Activity factor configuration
/// * `macro_config` - Macronutrient configuration
///
/// # Errors
///
/// Returns an error if any input validation fails
pub fn calculate_daily_nutrition_needs(
    params: &DailyNutritionParams,
    bmr_config: &BmrConfig,
    activity_config: &ActivityFactorsConfig,
    macro_config: &MacronutrientConfig,
) -> Result<DailyNutritionNeeds, AppError> {
    // Step 1: Calculate BMR
    let bmr = calculate_mifflin_st_jeor(
        params.weight_kg,
        params.height_cm,
        params.age,
        params.gender,
        bmr_config,
    )?;

    // Step 2: Calculate TDEE
    let tdee = calculate_tdee(bmr, params.activity_level, activity_config)?;

    // Step 3: Calculate protein needs
    let protein_g = calculate_protein_needs(
        params.weight_kg,
        params.activity_level,
        params.training_goal,
        macro_config,
    )?;

    // Step 4: Calculate carb needs
    let carbs_g = calculate_carb_needs(
        params.weight_kg,
        params.activity_level,
        params.training_goal,
        macro_config,
    )?;

    // Step 5: Calculate fat needs (uses remaining calories)
    let fat_g = calculate_fat_needs(tdee, protein_g, carbs_g, params.training_goal, macro_config)?;

    // Calculate actual macro percentages
    let total_kcal = fat_g.mul_add(9.0, protein_g.mul_add(4.0, carbs_g * 4.0));
    let macro_percentages = MacroPercentages {
        protein_percent: (protein_g * 4.0 / total_kcal) * 100.0,
        carbs_percent: (carbs_g * 4.0 / total_kcal) * 100.0,
        fat_percent: (fat_g * 9.0 / total_kcal) * 100.0,
    };

    Ok(DailyNutritionNeeds {
        bmr,
        tdee,
        protein_g,
        carbs_g,
        fat_g,
        macro_percentages,
        method: "Mifflin-St Jeor + Activity Factor".to_owned(),
        activity_level: params.activity_level,
        training_goal: params.training_goal,
    })
}

/// Calculate nutrient timing recommendations for workout days
///
/// Provides pre/post-workout nutrition timing based on:
/// - Kerksick et al. (2017) - ISSN nutrient timing position stand
/// - Aragon & Schoenfeld (2013) - Anabolic window meta-analysis
///
/// Key findings:
/// - Pre-workout: 1-3 hours before, focus on carbs
/// - Post-workout: 2-hour window (flexible, not 30 minutes)
/// - Protein distribution: 3-4 meals per day optimal
///
/// # Arguments
/// * `weight_kg` - Body weight in kilograms
/// * `daily_protein_g` - Total daily protein target (grams)
/// * `workout_intensity` - Workout intensity level
/// * `config` - Nutrient timing configuration
///
/// # Reference
/// Kerksick et al. (2017) DOI: 10.1186/s12970-017-0189-4
/// Aragon & Schoenfeld (2013) DOI: 10.1186/1550-2783-10-5
///
/// # Errors
///
/// Returns an error if weight is not positive
pub fn calculate_nutrient_timing(
    weight_kg: f64,
    daily_protein_g: f64,
    workout_intensity: WorkoutIntensity,
    config: &NutrientTimingConfig,
) -> Result<NutrientTimingPlan, AppError> {
    if weight_kg <= 0.0 {
        return Err(AppError::invalid_input("Weight must be positive"));
    }

    // Pre-workout carbs based on intensity
    let pre_workout_carbs = match workout_intensity {
        WorkoutIntensity::Low => weight_kg * config.pre_workout_carbs_g_per_kg * 0.5,
        WorkoutIntensity::Moderate => weight_kg * config.pre_workout_carbs_g_per_kg,
        WorkoutIntensity::High => weight_kg * config.pre_workout_carbs_g_per_kg * 1.3,
    };

    // Post-workout protein (20-40g optimal range)
    let post_workout_protein = config
        .post_workout_protein_g_min
        .max((daily_protein_g / 5.0).min(config.post_workout_protein_g_max));

    // Post-workout carbs
    let post_workout_carbs = weight_kg * config.post_workout_carbs_g_per_kg;

    // Protein distribution across day
    let meals_per_day = config.protein_meals_per_day_optimal;
    let protein_per_meal = daily_protein_g / f64::from(meals_per_day);

    Ok(NutrientTimingPlan {
        pre_workout: PreWorkoutNutrition {
            carbs_g: pre_workout_carbs,
            timing_hours_before: config.pre_workout_window_hours,
            recommendations: vec![
                format!(
                    "Consume {:.0}g carbs 1-3 hours before workout",
                    pre_workout_carbs
                ),
                "Focus on easily digestible carbs (banana, oatmeal, toast)".to_owned(),
                "Small amount of protein (10-20g) can be beneficial".to_owned(),
            ],
        },
        post_workout: PostWorkoutNutrition {
            protein_g: post_workout_protein,
            carbs_g: post_workout_carbs,
            timing_hours_after: config.post_workout_window_hours,
            recommendations: vec![
                format!(
                    "Consume {:.0}g protein + {:.0}g carbs within 2 hours",
                    post_workout_protein, post_workout_carbs
                ),
                "Window is flexible - total daily intake matters most".to_owned(),
                "Quality protein sources: whey, chicken, fish, eggs".to_owned(),
                "Carbs restore glycogen - more important after high-intensity".to_owned(),
            ],
        },
        daily_protein_distribution: ProteinDistribution {
            meals_per_day,
            protein_per_meal_g: protein_per_meal,
            strategy: format!(
                "Distribute {daily_protein_g:.0}g protein across {meals_per_day} meals (~{protein_per_meal:.0}g each) for optimal muscle protein synthesis"
            ),
        },
    })
}
