// ABOUTME: Nutrition configuration for dietary analysis and recommendations
// ABOUTME: Configures BMR calculation, macronutrient targets, meal timing, and USDA API settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Nutrition Analysis Configuration
//!
//! Provides configuration for nutrition analysis and recommendations including
//! BMR calculation, macronutrient targets, and meal timing.
//!
//! # Scientific References
//!
//! - BMR: Mifflin et al. (1990) DOI: 10.1093/ajcn/51.2.241
//! - Protein: Phillips & Van Loon (2011) DOI: 10.1080/02640414.2011.619204
//! - Carbs: Burke et al. (2011) DOI: 10.1080/02640414.2011.585473
//! - Timing: Kerksick et al. (2017) DOI: 10.1186/s12970-017-0189-4

use crate::config::intelligence::error::ConfigError;
use crate::intelligence::recipes::MealTiming;
use serde::{Deserialize, Serialize};

/// Nutrition Analysis Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NutritionConfig {
    /// Basal Metabolic Rate (BMR) calculation settings
    pub bmr: BmrConfig,
    /// Activity factor multipliers for TDEE calculation
    pub activity_factors: ActivityFactorsConfig,
    /// Macronutrient distribution targets
    pub macronutrients: MacronutrientConfig,
    /// Nutrient timing recommendations
    pub nutrient_timing: NutrientTimingConfig,
    /// USDA `FoodData` Central API configuration
    pub usda_api: UsdaApiConfig,
    /// Meal timing macro distribution configuration
    pub meal_timing_macros: MealTimingMacrosConfig,
    /// Meal TDEE proportion configuration (calories per meal based on daily TDEE)
    pub meal_tdee_proportions: MealTdeeProportionsConfig,
}

/// BMR (Basal Metabolic Rate) calculation configuration
///
/// Reference: Mifflin, M.D., et al. (1990). A new predictive equation for resting energy expenditure.
/// American Journal of Clinical Nutrition, 51(2), 241-247. DOI: 10.1093/ajcn/51.2.241
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BmrConfig {
    /// Mifflin-St Jeor formula enabled (recommended)
    pub use_mifflin_st_jeor: bool,
    /// Harris-Benedict formula enabled (1919 original formula)
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

/// Macro distribution for a single meal timing (protein%, carbs%, fat%)
///
/// All percentages must sum to 100.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MacroDistribution {
    /// Protein percentage (0-100)
    pub protein_pct: u8,
    /// Carbohydrate percentage (0-100)
    pub carbs_pct: u8,
    /// Fat percentage (0-100)
    pub fat_pct: u8,
}

impl MacroDistribution {
    /// Create a new macro distribution
    ///
    /// # Panics
    ///
    /// Panics in debug mode if percentages don't sum to 100
    #[must_use]
    pub const fn new(protein_pct: u8, carbs_pct: u8, fat_pct: u8) -> Self {
        debug_assert!(
            protein_pct
                .saturating_add(carbs_pct)
                .saturating_add(fat_pct)
                == 100,
            "Macro percentages must sum to 100"
        );
        Self {
            protein_pct,
            carbs_pct,
            fat_pct,
        }
    }

    /// Get as a tuple (protein, carbs, fat)
    #[must_use]
    pub const fn as_tuple(&self) -> (u8, u8, u8) {
        (self.protein_pct, self.carbs_pct, self.fat_pct)
    }
}

/// Meal timing macro distribution configuration
///
/// Configures the macro (protein/carbs/fat) percentages for each meal timing context.
/// Defaults are based on ISSN sports nutrition position stands.
///
/// # Scientific References
///
/// - Pre-training: Kerksick CM et al. (2017) DOI: 10.1186/s12970-017-0189-4
/// - Post-training: Jäger R et al. (2017) DOI: 10.1186/s12970-017-0177-8
/// - Rest day: Impey SG et al. (2018) DOI: 10.1007/s40279-018-0867-7
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealTimingMacrosConfig {
    /// Pre-training macro distribution: high carbs for glycogen loading
    /// Default: (20% protein, 55% carbs, 25% fat)
    pub pre_training: MacroDistribution,
    /// Post-training macro distribution: high protein for muscle protein synthesis
    /// Default: (30% protein, 45% carbs, 25% fat)
    pub post_training: MacroDistribution,
    /// Rest day macro distribution: carb periodization with lower glycogen needs
    /// Default: (30% protein, 35% carbs, 35% fat)
    pub rest_day: MacroDistribution,
    /// General meal macro distribution: balanced everyday eating
    /// Default: (25% protein, 45% carbs, 30% fat)
    pub general: MacroDistribution,
}

impl Default for MealTimingMacrosConfig {
    fn default() -> Self {
        Self {
            // ISSN Position Stand: Nutrient Timing (Kerksick et al., 2017)
            // High carbs for pre-training glycogen optimization
            pre_training: MacroDistribution::new(20, 55, 25),
            // ISSN Position Stand: Protein and Exercise (Jäger et al., 2017)
            // High protein for post-training muscle protein synthesis
            post_training: MacroDistribution::new(30, 45, 25),
            // Carbohydrate periodization for rest days (Impey et al., 2018)
            // Lower carbs when glycogen demands are reduced
            rest_day: MacroDistribution::new(30, 35, 35),
            // Balanced distribution for general eating
            general: MacroDistribution::new(25, 45, 30),
        }
    }
}

impl MealTimingMacrosConfig {
    /// Validate that all macro distributions sum to 100%
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::InvalidWeights` if any meal timing's macro percentages
    /// (protein + carbs + fat) do not sum to exactly 100.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let configs = [
            ("pre_training", &self.pre_training),
            ("post_training", &self.post_training),
            ("rest_day", &self.rest_day),
            ("general", &self.general),
        ];

        for (name, config) in configs {
            let sum = config
                .protein_pct
                .saturating_add(config.carbs_pct)
                .saturating_add(config.fat_pct);
            if sum != 100 {
                return Err(ConfigError::InvalidWeights(Box::leak(
                    format!("{name} macro percentages must sum to 100, got {sum}").into_boxed_str(),
                )));
            }
        }

        Ok(())
    }

    /// Get macro distribution for a specific meal timing
    #[must_use]
    pub const fn get_distribution(&self, timing: MealTiming) -> (u8, u8, u8) {
        match timing {
            MealTiming::PreTraining => self.pre_training.as_tuple(),
            MealTiming::PostTraining => self.post_training.as_tuple(),
            MealTiming::RestDay => self.rest_day.as_tuple(),
            MealTiming::General => self.general.as_tuple(),
        }
    }
}

/// Meal TDEE proportion configuration
///
/// Defines what percentage of daily TDEE each meal timing should target.
/// When a user provides their TDEE, meal calories are calculated as: TDEE × proportion.
///
/// # Scientific Basis
///
/// Proportions based on athletic nutrition timing research:
/// - Pre-training: Lighter meal (15-20% of TDEE) for glycogen without gut distress
/// - Post-training: Recovery meal (25-30% of TDEE) for muscle protein synthesis and glycogen replenishment
/// - Rest day/General: Standard meal (25% of TDEE) for balanced daily eating
///
/// # References
///
/// - Kerksick CM et al. (2017) ISSN position stand: nutrient timing. DOI: 10.1186/s12970-017-0189-4
/// - Aragon AA, Schoenfeld BJ (2013) Nutrient timing revisited. DOI: 10.1186/1550-2783-10-5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealTdeeProportionsConfig {
    /// Pre-training meal as percentage of daily TDEE (0.0-1.0)
    /// Default: 0.175 (17.5%) - light meal before workout
    pub pre_training: f64,
    /// Post-training meal as percentage of daily TDEE (0.0-1.0)
    /// Default: 0.275 (27.5%) - recovery meal
    pub post_training: f64,
    /// Rest day meal as percentage of daily TDEE (0.0-1.0)
    /// Default: 0.25 (25%) - standard meal
    pub rest_day: f64,
    /// General meal as percentage of daily TDEE (0.0-1.0)
    /// Default: 0.25 (25%) - standard meal
    pub general: f64,
    /// Fallback calories when TDEE is not provided
    pub fallback_calories: MealFallbackCaloriesConfig,
}

/// Fallback calorie values when user TDEE is not available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealFallbackCaloriesConfig {
    /// Pre-training fallback (kcal)
    pub pre_training: f64,
    /// Post-training fallback (kcal)
    pub post_training: f64,
    /// Rest day fallback (kcal)
    pub rest_day: f64,
    /// General fallback (kcal)
    pub general: f64,
}

impl Default for MealFallbackCaloriesConfig {
    fn default() -> Self {
        Self {
            pre_training: 400.0,  // Light meal before workout
            post_training: 600.0, // Recovery meal
            rest_day: 500.0,      // Moderate meal
            general: 500.0,       // Default meal
        }
    }
}

impl Default for MealTdeeProportionsConfig {
    fn default() -> Self {
        Self {
            // 17.5% - light meal 1-3 hours before workout
            pre_training: 0.175,
            // 27.5% - larger recovery meal for muscle protein synthesis
            post_training: 0.275,
            // 25% - standard meal distribution for rest days
            rest_day: 0.25,
            // 25% - balanced meal for general eating
            general: 0.25,
            fallback_calories: MealFallbackCaloriesConfig::default(),
        }
    }
}

impl MealTdeeProportionsConfig {
    /// Validate that proportions are within valid range (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ValueOutOfRange` if any proportion is outside 0.0-1.0
    pub fn validate(&self) -> Result<(), ConfigError> {
        let configs = [
            ("pre_training", self.pre_training),
            ("post_training", self.post_training),
            ("rest_day", self.rest_day),
            ("general", self.general),
        ];

        for (name, value) in configs {
            if !(0.0..=1.0).contains(&value) {
                return Err(ConfigError::ValueOutOfRange(Box::leak(
                    format!("{name} proportion must be between 0.0 and 1.0, got {value}")
                        .into_boxed_str(),
                )));
            }
        }

        Ok(())
    }

    /// Calculate meal calories from TDEE and meal timing
    ///
    /// If TDEE is provided, returns TDEE × proportion for the timing.
    /// Otherwise, returns the fallback calorie value.
    #[must_use]
    pub fn calories_for_timing(&self, timing: MealTiming, tdee: Option<f64>) -> f64 {
        tdee.map_or_else(
            || self.fallback_calories_for_timing(timing),
            |daily_tdee| (daily_tdee * self.proportion_for_timing(timing)).round(),
        )
    }

    /// Get the TDEE proportion for a specific meal timing
    #[must_use]
    pub const fn proportion_for_timing(&self, timing: MealTiming) -> f64 {
        match timing {
            MealTiming::PreTraining => self.pre_training,
            MealTiming::PostTraining => self.post_training,
            MealTiming::RestDay => self.rest_day,
            MealTiming::General => self.general,
        }
    }

    /// Get fallback calories for a specific meal timing when TDEE is not provided
    #[must_use]
    pub const fn fallback_calories_for_timing(&self, timing: MealTiming) -> f64 {
        match timing {
            MealTiming::PreTraining => self.fallback_calories.pre_training,
            MealTiming::PostTraining => self.fallback_calories.post_training,
            MealTiming::RestDay => self.fallback_calories.rest_day,
            MealTiming::General => self.fallback_calories.general,
        }
    }
}

impl Default for BmrConfig {
    fn default() -> Self {
        Self {
            use_mifflin_st_jeor: true,
            use_harris_benedict: false,
            msj_weight_coef: 10.0,
            msj_height_coef: 6.25,
            msj_age_coef: -5.0,
            msj_male_constant: 5.0,
            msj_female_constant: -161.0,
        }
    }
}

impl Default for ActivityFactorsConfig {
    fn default() -> Self {
        Self {
            sedentary: 1.2,
            lightly_active: 1.375,
            moderately_active: 1.55,
            very_active: 1.725,
            extra_active: 1.9,
        }
    }
}

impl Default for MacronutrientConfig {
    fn default() -> Self {
        Self {
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
}

impl Default for NutrientTimingConfig {
    fn default() -> Self {
        Self {
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
}

impl Default for UsdaApiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.nal.usda.gov/fdc/v1".to_owned(),
            timeout_secs: 10,
            cache_ttl_hours: 24,
            max_cache_items: 1000,
            rate_limit_per_minute: 30,
        }
    }
}
