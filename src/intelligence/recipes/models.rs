// ABOUTME: Data models for recipe management with training-aware nutrition planning
// ABOUTME: Defines Recipe, RecipeIngredient, MealTiming, and related types
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Meal timing context for training-aware recipe suggestions
///
/// Adjusts macro recommendations based on when the meal is consumed
/// relative to training sessions.
///
/// # Scientific Basis
///
/// Macro distributions are based on peer-reviewed sports nutrition research:
///
/// - **Pre-training (20% protein, 55% carbs, 25% fat)**: High carbohydrate availability
///   maximizes muscle glycogen stores for energy. The ISSN recommends 1-4 g/kg of
///   high-glycemic carbohydrates 1-4 hours before exercise for glycogen optimization.
///   Lower fat aids gastric emptying and reduces GI distress.
///
///   *Reference: Kerksick CM et al. (2017) "ISSN Position Stand: Nutrient Timing"
///   Journal of the International Society of Sports Nutrition 14:33.
///   DOI: [10.1186/s12970-017-0189-4](https://doi.org/10.1186/s12970-017-0189-4)*
///
/// - **Post-training (30% protein, 45% carbs, 25% fat)**: Elevated protein (0.25-0.4 g/kg)
///   within 2 hours maximizes muscle protein synthesis (MPS). Moderate carbohydrates
///   (0.8-1.2 g/kg) accelerate glycogen resynthesis when combined with protein.
///
///   *Reference: Jäger R et al. (2017) "ISSN Position Stand: Protein and Exercise"
///   Journal of the International Society of Sports Nutrition 14:20.
///   DOI: [10.1186/s12970-017-0177-8](https://doi.org/10.1186/s12970-017-0177-8)*
///
/// - **Rest day (30% protein, 35% carbs, 35% fat)**: Carbohydrate periodization reduces
///   intake on non-training days when glycogen demands are lower. Training with reduced
///   glycogen availability stimulates mitochondrial biogenesis and oxidative capacity.
///   Higher fat compensates for reduced carb calories while maintaining satiety.
///
///   *Reference: Impey SG et al. (2018) "Fuel for the Work Required: A Theoretical
///   Framework for Carbohydrate Periodization" Sports Medicine 48(5):1031-1048.
///   DOI: [10.1007/s40279-018-0867-7](https://doi.org/10.1007/s40279-018-0867-7)*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MealTiming {
    /// 2-3 hours before workout: higher carbs, moderate protein, low fat
    /// Focus on easily digestible energy sources
    PreTraining,
    /// Within 2 hours after workout: high protein, moderate-fast carbs
    /// Focus on recovery and glycogen replenishment
    PostTraining,
    /// Rest day meal: lower carbs, moderate protein and fat
    /// Reduced glycogen needs, maintenance focus
    RestDay,
    /// General meal with no specific timing context
    /// Uses user's standard macro targets
    #[default]
    General,
}

impl MealTiming {
    /// Get recommended macro distribution percentages for this timing
    ///
    /// Returns (`protein_pct`, `carbs_pct`, `fat_pct`) tuple that sums to 100
    #[must_use]
    pub const fn macro_distribution(&self) -> (u8, u8, u8) {
        match self {
            // Pre-training: prioritize carbs for energy
            Self::PreTraining => (20, 55, 25),
            // Post-training: prioritize protein for recovery
            Self::PostTraining => (30, 45, 25),
            // Rest day: balanced with lower carbs
            Self::RestDay => (30, 35, 35),
            // General: balanced distribution
            Self::General => (25, 45, 30),
        }
    }

    /// Get human-readable description of this timing
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::PreTraining => "Pre-training meal (2-3h before workout)",
            Self::PostTraining => "Post-training meal (within 2h after workout)",
            Self::RestDay => "Rest day meal",
            Self::General => "General meal",
        }
    }
}

/// Ingredient measurement unit with conversion support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IngredientUnit {
    /// Weight in grams (base unit)
    #[default]
    Grams,
    /// Volume in milliliters
    Milliliters,
    /// US cups (240ml)
    Cups,
    /// Tablespoons (15ml)
    Tablespoons,
    /// Teaspoons (5ml)
    Teaspoons,
    /// Count of whole items (eggs, bananas, etc.)
    Pieces,
    /// Weight in ounces (28.35g)
    Ounces,
    /// Weight in pounds (453.6g)
    Pounds,
    /// Weight in kilograms (1000g)
    Kilograms,
}

impl IngredientUnit {
    /// Check if this unit is a volume measurement
    #[must_use]
    pub const fn is_volume(&self) -> bool {
        matches!(
            self,
            Self::Milliliters | Self::Cups | Self::Tablespoons | Self::Teaspoons
        )
    }

    /// Check if this unit is a weight measurement
    #[must_use]
    pub const fn is_weight(&self) -> bool {
        matches!(
            self,
            Self::Grams | Self::Ounces | Self::Pounds | Self::Kilograms
        )
    }

    /// Check if this unit is a count
    #[must_use]
    pub const fn is_count(&self) -> bool {
        matches!(self, Self::Pieces)
    }

    /// Get the abbreviation for display
    #[must_use]
    pub const fn abbreviation(&self) -> &'static str {
        match self {
            Self::Grams => "g",
            Self::Milliliters => "ml",
            Self::Cups => "cup",
            Self::Tablespoons => "tbsp",
            Self::Teaspoons => "tsp",
            Self::Pieces => "pc",
            Self::Ounces => "oz",
            Self::Pounds => "lb",
            Self::Kilograms => "kg",
        }
    }
}

/// Dietary restriction for filtering recipes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DietaryRestriction {
    /// No gluten-containing ingredients
    GlutenFree,
    /// No dairy products
    DairyFree,
    /// No animal products
    Vegan,
    /// No meat or fish
    Vegetarian,
    /// No nuts
    NutFree,
    /// Low sodium (< 600mg per serving)
    LowSodium,
    /// Low sugar (< 10g per serving)
    LowSugar,
    /// Ketogenic (< 20g carbs per serving)
    Keto,
    /// Paleo-compliant
    Paleo,
    /// Custom restriction with description
    Custom(String),
}

/// Cooking skill level for recipe complexity filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillLevel {
    /// Simple recipes, basic techniques
    Beginner,
    /// Moderate complexity, some techniques required
    #[default]
    Intermediate,
    /// Complex recipes, advanced techniques
    Advanced,
}

/// Macro nutrient targets for recipe suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroTargets {
    /// Target calories (optional, will be calculated if not set)
    pub calories: Option<f64>,
    /// Target protein in grams
    pub protein_g: Option<f64>,
    /// Target carbohydrates in grams
    pub carbs_g: Option<f64>,
    /// Target fat in grams
    pub fat_g: Option<f64>,
    /// Target fiber in grams (optional)
    pub fiber_g: Option<f64>,
}

impl MacroTargets {
    /// Create empty targets (no constraints)
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            calories: None,
            protein_g: None,
            carbs_g: None,
            fat_g: None,
            fiber_g: None,
        }
    }

    /// Create targets from calorie goal and meal timing
    ///
    /// Uses configurable macro distribution percentages from the global intelligence config.
    /// Defaults are based on ISSN sports nutrition position stands.
    #[must_use]
    pub fn from_calories_and_timing(calories: f64, timing: MealTiming) -> Self {
        // Use configurable macro distribution from global config
        let config = crate::config::IntelligenceConfig::global();
        let (protein_pct, carbs_pct, fat_pct) =
            config.nutrition.meal_timing_macros.get_distribution(timing);

        // Calculate grams from percentages
        // Protein: 4 cal/g, Carbs: 4 cal/g, Fat: 9 cal/g
        let protein_g = (calories * f64::from(protein_pct) / 100.0) / 4.0;
        let carbs_g = (calories * f64::from(carbs_pct) / 100.0) / 4.0;
        let fat_g = (calories * f64::from(fat_pct) / 100.0) / 9.0;

        Self {
            calories: Some(calories),
            protein_g: Some(protein_g),
            carbs_g: Some(carbs_g),
            fat_g: Some(fat_g),
            fiber_g: None,
        }
    }
}

/// Constraints for recipe suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeConstraints {
    /// Target macronutrients
    pub macro_targets: MacroTargets,
    /// Dietary restrictions to respect
    pub dietary_restrictions: Vec<DietaryRestriction>,
    /// Preferred cuisines (Mediterranean, Asian, etc.)
    pub cuisine_preferences: Vec<String>,
    /// Ingredients to exclude
    pub excluded_ingredients: Vec<String>,
    /// Maximum preparation time in minutes
    pub max_prep_time_mins: Option<u16>,
    /// Maximum cooking time in minutes
    pub max_cook_time_mins: Option<u16>,
    /// Required skill level
    pub skill_level: SkillLevel,
    /// Meal timing context for macro adjustments
    pub meal_timing: MealTiming,
    /// Prompt hint for LLM clients (generated by Pierre)
    pub prompt_hint: Option<String>,
}

impl Default for RecipeConstraints {
    fn default() -> Self {
        Self {
            macro_targets: MacroTargets::empty(),
            dietary_restrictions: Vec::new(),
            cuisine_preferences: Vec::new(),
            excluded_ingredients: Vec::new(),
            max_prep_time_mins: None,
            max_cook_time_mins: None,
            skill_level: SkillLevel::default(),
            meal_timing: MealTiming::default(),
            prompt_hint: None,
        }
    }
}

/// USDA-validated nutrition data for a recipe (per serving)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedNutrition {
    /// Total calories per serving
    pub calories: f64,
    /// Protein in grams per serving
    pub protein_g: f64,
    /// Carbohydrates in grams per serving
    pub carbs_g: f64,
    /// Fat in grams per serving
    pub fat_g: f64,
    /// Fiber in grams per serving (if available)
    pub fiber_g: Option<f64>,
    /// Sodium in milligrams per serving (if available)
    pub sodium_mg: Option<f64>,
    /// Sugar in grams per serving (if available)
    pub sugar_g: Option<f64>,
    /// When the nutrition was last validated against USDA
    pub validated_at: DateTime<Utc>,
}

impl ValidatedNutrition {
    /// Check if this nutrition meets the given targets (within 10% tolerance)
    #[must_use]
    pub fn meets_targets(&self, targets: &MacroTargets) -> bool {
        let tolerance = 0.10; // 10% tolerance

        let check = |actual: f64, target: Option<f64>| -> bool {
            target.is_none_or(|t| {
                let diff = (actual - t).abs();
                diff <= t * tolerance
            })
        };

        check(self.calories, targets.calories)
            && check(self.protein_g, targets.protein_g)
            && check(self.carbs_g, targets.carbs_g)
            && check(self.fat_g, targets.fat_g)
    }
}

/// Single ingredient in a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    /// USDA `FoodData` Central ID (if validated)
    pub fdc_id: Option<i64>,
    /// Human-readable ingredient name
    pub name: String,
    /// Amount in the specified unit
    pub amount: f64,
    /// Measurement unit
    pub unit: IngredientUnit,
    /// Normalized weight in grams (for nutrition calculation)
    pub grams: f64,
    /// Optional preparation notes (diced, minced, etc.)
    pub preparation: Option<String>,
}

impl RecipeIngredient {
    /// Create a new ingredient with grams already calculated
    #[must_use]
    pub fn new(name: impl Into<String>, amount: f64, unit: IngredientUnit, grams: f64) -> Self {
        Self {
            fdc_id: None,
            name: name.into(),
            amount,
            unit,
            grams,
            preparation: None,
        }
    }

    /// Create an ingredient with grams as the unit
    #[must_use]
    pub fn in_grams(name: impl Into<String>, grams: f64) -> Self {
        Self {
            fdc_id: None,
            name: name.into(),
            amount: grams,
            unit: IngredientUnit::Grams,
            grams,
            preparation: None,
        }
    }

    /// Add preparation instructions
    #[must_use]
    pub fn with_preparation(mut self, prep: impl Into<String>) -> Self {
        self.preparation = Some(prep.into());
        self
    }

    /// Set the USDA FDC ID after validation
    #[must_use]
    pub const fn with_fdc_id(mut self, fdc_id: i64) -> Self {
        self.fdc_id = Some(fdc_id);
        self
    }
}

/// A complete recipe with ingredients and instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    /// Unique recipe identifier
    pub id: Uuid,
    /// Owner user ID (per-user recipes)
    pub user_id: Uuid,
    /// Recipe name
    pub name: String,
    /// Recipe description
    pub description: Option<String>,
    /// Number of servings this recipe makes
    pub servings: u8,
    /// Preparation time in minutes
    pub prep_time_mins: Option<u16>,
    /// Cooking time in minutes
    pub cook_time_mins: Option<u16>,
    /// List of ingredients
    pub ingredients: Vec<RecipeIngredient>,
    /// Cooking instructions (ordered steps)
    pub instructions: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Cached USDA-validated nutrition (per serving)
    pub nutrition: Option<ValidatedNutrition>,
    /// Intended meal timing
    pub meal_timing: MealTiming,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Recipe {
    /// Create a new recipe with basic information
    #[must_use]
    pub fn new(user_id: Uuid, name: impl Into<String>, servings: u8) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            name: name.into(),
            description: None,
            servings,
            prep_time_mins: None,
            cook_time_mins: None,
            ingredients: Vec::new(),
            instructions: Vec::new(),
            tags: Vec::new(),
            nutrition: None,
            meal_timing: MealTiming::General,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a description
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set preparation time
    #[must_use]
    pub const fn with_prep_time(mut self, mins: u16) -> Self {
        self.prep_time_mins = Some(mins);
        self
    }

    /// Set cooking time
    #[must_use]
    pub const fn with_cook_time(mut self, mins: u16) -> Self {
        self.cook_time_mins = Some(mins);
        self
    }

    /// Add an ingredient
    #[must_use]
    pub fn with_ingredient(mut self, ingredient: RecipeIngredient) -> Self {
        self.ingredients.push(ingredient);
        self
    }

    /// Add multiple ingredients
    #[must_use]
    pub fn with_ingredients(mut self, ingredients: Vec<RecipeIngredient>) -> Self {
        self.ingredients.extend(ingredients);
        self
    }

    /// Add an instruction step
    #[must_use]
    pub fn with_instruction(mut self, step: impl Into<String>) -> Self {
        self.instructions.push(step.into());
        self
    }

    /// Add multiple instruction steps
    #[must_use]
    pub fn with_instructions(mut self, steps: Vec<String>) -> Self {
        self.instructions.extend(steps);
        self
    }

    /// Add a tag
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set meal timing context
    #[must_use]
    pub const fn with_meal_timing(mut self, timing: MealTiming) -> Self {
        self.meal_timing = timing;
        self
    }

    /// Set validated nutrition
    #[must_use]
    pub const fn with_nutrition(mut self, nutrition: ValidatedNutrition) -> Self {
        self.nutrition = Some(nutrition);
        self
    }

    /// Get total time (prep + cook)
    #[must_use]
    pub const fn total_time_mins(&self) -> Option<u16> {
        match (self.prep_time_mins, self.cook_time_mins) {
            (Some(prep), Some(cook)) => Some(prep.saturating_add(cook)),
            (Some(prep), None) => Some(prep),
            (None, Some(cook)) => Some(cook),
            (None, None) => None,
        }
    }

    /// Get total weight of all ingredients in grams
    #[must_use]
    pub fn total_weight_grams(&self) -> f64 {
        self.ingredients.iter().map(|i| i.grams).sum()
    }

    /// Scale recipe to different number of servings
    #[must_use]
    pub fn scaled(&self, new_servings: u8) -> Self {
        if new_servings == self.servings || self.servings == 0 {
            return self.clone();
        }

        let scale_factor = f64::from(new_servings) / f64::from(self.servings);

        let scaled_ingredients = self
            .ingredients
            .iter()
            .map(|i| RecipeIngredient {
                fdc_id: i.fdc_id,
                name: i.name.clone(),
                amount: i.amount * scale_factor,
                unit: i.unit,
                grams: i.grams * scale_factor,
                preparation: i.preparation.clone(),
            })
            .collect();

        Self {
            id: self.id,
            user_id: self.user_id,
            name: self.name.clone(),
            description: self.description.clone(),
            servings: new_servings,
            prep_time_mins: self.prep_time_mins,
            cook_time_mins: self.cook_time_mins,
            ingredients: scaled_ingredients,
            instructions: self.instructions.clone(),
            tags: self.tags.clone(),
            nutrition: self.nutrition.clone(), // Nutrition per serving stays the same
            meal_timing: self.meal_timing,
            created_at: self.created_at,
            updated_at: Utc::now(),
        }
    }
}
