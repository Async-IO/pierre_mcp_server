// ABOUTME: Nutrition tracking models for food intake analysis
// ABOUTME: NutritionLog, MealEntry, MealType, and FoodItem definitions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of meal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MealType {
    /// Breakfast meal
    Breakfast,
    /// Lunch meal
    Lunch,
    /// Dinner meal
    Dinner,
    /// Snack between meals
    Snack,
    /// Unspecified or other meal type
    Other,
}

impl MealType {
    /// Parse meal type from string
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "breakfast" => Self::Breakfast,
            "lunch" => Self::Lunch,
            "dinner" => Self::Dinner,
            "snack" => Self::Snack,
            _ => Self::Other,
        }
    }
}

/// Individual food item within a meal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodItem {
    /// Food name
    pub name: String,
    /// Brand name (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    /// Serving size amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_size: Option<f64>,
    /// Serving unit (g, oz, cup, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_unit: Option<String>,
    /// Number of servings consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servings: Option<f64>,
    /// Calories per serving
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<f64>,
    /// Protein per serving (grams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    /// Carbohydrates per serving (grams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbohydrates_g: Option<f64>,
    /// Fat per serving (grams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g: Option<f64>,
}

/// Individual meal entry within a nutrition log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealEntry {
    /// Meal name/type (breakfast, lunch, dinner, snack)
    pub meal_type: MealType,
    /// Timestamp when meal was logged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    /// Meal description or name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Calories for this meal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<f64>,
    /// Protein in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    /// Carbohydrates in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbohydrates_g: Option<f64>,
    /// Fat in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g: Option<f64>,
    /// Individual food items (if available)
    pub food_items: Vec<FoodItem>,
}

/// Nutrition log entry for tracking food intake
///
/// Represents daily or per-meal nutrition data from wearable integrations
/// like `MyFitnessPal` (via Terra) or other nutrition tracking apps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutritionLog {
    /// Unique identifier for this nutrition log entry
    pub id: String,
    /// Date of the nutrition log
    pub date: DateTime<Utc>,
    /// Total calories consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_calories: Option<f64>,
    /// Total protein in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    /// Total carbohydrates in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbohydrates_g: Option<f64>,
    /// Total fat in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g: Option<f64>,
    /// Fiber in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiber_g: Option<f64>,
    /// Sugar in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sugar_g: Option<f64>,
    /// Sodium in mg
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sodium_mg: Option<f64>,
    /// Water intake in mL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_ml: Option<f64>,
    /// Individual meals/entries
    pub meals: Vec<MealEntry>,
    /// Provider of this nutrition data
    pub provider: String,
}
