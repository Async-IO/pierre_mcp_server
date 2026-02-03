// ABOUTME: Nutrition tools for meal planning and nutrient tracking.
// ABOUTME: Implements calculate_daily_nutrition, get_nutrient_timing, search_food, get_food_details, analyze_meal_nutrition.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Nutrition Tools
//!
//! This module provides tools for nutrition management with direct business logic:
//! - `CalculateDailyNutritionTool` - Calculate daily calorie and macronutrient needs
//! - `GetNutrientTimingTool` - Optimal nutrient timing recommendations
//! - `SearchFoodTool` - Search USDA food database
//! - `GetFoodDetailsTool` - Get detailed food information
//! - `AnalyzeMealNutritionTool` - Analyze meal nutritional content
//!
//! All tools use direct intelligence module access.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

use crate::config::IntelligenceConfig;
use crate::errors::{AppError, AppResult};
use crate::external::{FoodNutrient, UsdaClient, UsdaClientConfig};
use crate::intelligence::{
    calculate_daily_nutrition_needs, calculate_nutrient_timing, ActivityLevel,
    DailyNutritionParams, Gender, TrainingGoal, WorkoutIntensity,
};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions
// ============================================================================

/// Parse gender from string
fn parse_gender(gender_str: &str) -> AppResult<Gender> {
    match gender_str.to_lowercase().as_str() {
        "male" => Ok(Gender::Male),
        "female" => Ok(Gender::Female),
        other => Err(AppError::invalid_input(format!(
            "Invalid gender '{other}'. Must be 'male' or 'female'"
        ))),
    }
}

/// Parse activity level from string
fn parse_activity_level(activity_str: &str) -> AppResult<ActivityLevel> {
    match activity_str.to_lowercase().as_str() {
        "sedentary" => Ok(ActivityLevel::Sedentary),
        "lightly_active" => Ok(ActivityLevel::LightlyActive),
        "moderately_active" => Ok(ActivityLevel::ModeratelyActive),
        "very_active" => Ok(ActivityLevel::VeryActive),
        "extra_active" => Ok(ActivityLevel::ExtraActive),
        other => Err(AppError::invalid_input(format!(
            "Invalid activity_level '{other}'. Must be: sedentary, lightly_active, moderately_active, very_active, extra_active"
        ))),
    }
}

/// Parse training goal from string
fn parse_training_goal(goal_str: &str) -> AppResult<TrainingGoal> {
    match goal_str.to_lowercase().as_str() {
        "maintenance" => Ok(TrainingGoal::Maintenance),
        "weight_loss" => Ok(TrainingGoal::WeightLoss),
        "muscle_gain" => Ok(TrainingGoal::MuscleGain),
        "endurance_performance" => Ok(TrainingGoal::EndurancePerformance),
        other => Err(AppError::invalid_input(format!(
            "Invalid training_goal '{other}'. Must be: maintenance, weight_loss, muscle_gain, endurance_performance"
        ))),
    }
}

/// Parse workout intensity from string
fn parse_workout_intensity(intensity_str: &str) -> AppResult<WorkoutIntensity> {
    match intensity_str.to_lowercase().as_str() {
        "low" | "easy" => Ok(WorkoutIntensity::Low),
        "moderate" | "medium" => Ok(WorkoutIntensity::Moderate),
        "high" | "hard" => Ok(WorkoutIntensity::High),
        other => Err(AppError::invalid_input(format!(
            "Invalid workout_intensity '{other}'. Must be: low, moderate, high"
        ))),
    }
}

/// Get USDA client from context
fn get_usda_client(ctx: &ToolExecutionContext) -> AppResult<UsdaClient> {
    let api_key = ctx.resources.config.usda_api_key.clone().ok_or_else(|| {
        AppError::internal("USDA API key not configured. Set USDA_API_KEY environment variable.")
    })?;

    Ok(UsdaClient::new(UsdaClientConfig {
        api_key,
        ..UsdaClientConfig::default()
    }))
}

// ============================================================================
// CalculateDailyNutritionTool
// ============================================================================

/// Tool for calculating daily calorie and macronutrient needs.
pub struct CalculateDailyNutritionTool;

#[async_trait]
impl McpTool for CalculateDailyNutritionTool {
    fn name(&self) -> &'static str {
        "calculate_daily_nutrition"
    }

    fn description(&self) -> &'static str {
        "Calculate daily calorie and macronutrient needs based on biometrics and goals"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "weight_kg".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Body weight in kilograms".to_owned()),
            },
        );
        properties.insert(
            "height_cm".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Height in centimeters".to_owned()),
            },
        );
        properties.insert(
            "age".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Age in years".to_owned()),
            },
        );
        properties.insert(
            "gender".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Gender: male or female".to_owned()),
            },
        );
        properties.insert(
            "activity_level".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Activity level: sedentary, lightly_active, moderately_active, very_active, extra_active".to_owned(),
                ),
            },
        );
        properties.insert(
            "training_goal".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Training goal: maintenance, weight_loss, muscle_gain, endurance_performance"
                        .to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec![
                "weight_kg".to_owned(),
                "height_cm".to_owned(),
                "age".to_owned(),
                "gender".to_owned(),
                "activity_level".to_owned(),
                "training_goal".to_owned(),
            ]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Calculating daily nutrition needs");

        // Parse required parameters
        let weight_kg = args
            .get("weight_kg")
            .and_then(Value::as_f64)
            .ok_or_else(|| AppError::invalid_input("weight_kg is required"))?;

        let height_cm = args
            .get("height_cm")
            .and_then(Value::as_f64)
            .ok_or_else(|| AppError::invalid_input("height_cm is required"))?;

        let age_u64 = args
            .get("age")
            .and_then(Value::as_u64)
            .ok_or_else(|| AppError::invalid_input("age is required"))?;

        #[allow(clippy::cast_possible_truncation)]
        let age = if age_u64 <= 150 {
            age_u64 as u32
        } else {
            return Err(AppError::invalid_input(
                "Age must be between 0 and 150 years",
            ));
        };

        let gender_str = args
            .get("gender")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("gender is required"))?;
        let gender = parse_gender(gender_str)?;

        let activity_str = args
            .get("activity_level")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("activity_level is required"))?;
        let activity_level = parse_activity_level(activity_str)?;

        let goal_str = args
            .get("training_goal")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("training_goal is required"))?;
        let training_goal = parse_training_goal(goal_str)?;

        let params = DailyNutritionParams {
            weight_kg,
            height_cm,
            age,
            gender,
            activity_level,
            training_goal,
        };

        let nutrition_config = &IntelligenceConfig::global().nutrition;

        let nutrition = calculate_daily_nutrition_needs(
            &params,
            &nutrition_config.bmr,
            &nutrition_config.activity_factors,
            &nutrition_config.macronutrients,
        )
        .map_err(|e| AppError::internal(format!("Nutrition calculation failed: {e}")))?;

        Ok(ToolResult::ok(json!({
            "bmr": nutrition.bmr,
            "tdee": nutrition.tdee,
            "protein_g": nutrition.protein_g,
            "carbs_g": nutrition.carbs_g,
            "fat_g": nutrition.fat_g,
            "macro_percentages": {
                "protein_percent": nutrition.macro_percentages.protein_percent,
                "carbs_percent": nutrition.macro_percentages.carbs_percent,
                "fat_percent": nutrition.macro_percentages.fat_percent,
            },
            "calculation_method": nutrition.method,
            "input_parameters": {
                "weight_kg": weight_kg,
                "height_cm": height_cm,
                "age": age,
                "gender": gender_str,
                "activity_level": activity_str,
                "training_goal": goal_str,
            },
            "calculated_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// GetNutrientTimingTool
// ============================================================================

/// Tool for optimal nutrient timing recommendations.
pub struct GetNutrientTimingTool;

#[async_trait]
impl McpTool for GetNutrientTimingTool {
    fn name(&self) -> &'static str {
        "get_nutrient_timing"
    }

    fn description(&self) -> &'static str {
        "Get optimal nutrient timing recommendations around workouts"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "workout_intensity".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Workout intensity: low, moderate, high".to_owned()),
            },
        );
        properties.insert(
            "weight_kg".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Body weight in kilograms".to_owned()),
            },
        );
        properties.insert(
            "daily_protein_g".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Daily protein target in grams".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec![
                "workout_intensity".to_owned(),
                "weight_kg".to_owned(),
                "daily_protein_g".to_owned(),
            ]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Getting nutrient timing recommendations");

        let intensity_str = args
            .get("workout_intensity")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("workout_intensity is required"))?;
        let intensity = parse_workout_intensity(intensity_str)?;

        let weight_kg = args
            .get("weight_kg")
            .and_then(Value::as_f64)
            .ok_or_else(|| AppError::invalid_input("weight_kg is required"))?;

        let daily_protein_g = args
            .get("daily_protein_g")
            .and_then(Value::as_f64)
            .ok_or_else(|| AppError::invalid_input("daily_protein_g is required"))?;

        let nutrition_config = &IntelligenceConfig::global().nutrition;

        let timing = calculate_nutrient_timing(
            weight_kg,
            daily_protein_g,
            intensity,
            &nutrition_config.nutrient_timing,
        )
        .map_err(|e| AppError::internal(format!("Nutrient timing calculation failed: {e}")))?;

        Ok(ToolResult::ok(json!({
            "pre_workout": {
                "timing_hours_before": timing.pre_workout.timing_hours_before,
                "carbs_g": timing.pre_workout.carbs_g,
                "recommendations": timing.pre_workout.recommendations,
            },
            "post_workout": {
                "timing_hours_after": timing.post_workout.timing_hours_after,
                "protein_g": timing.post_workout.protein_g,
                "carbs_g": timing.post_workout.carbs_g,
                "recommendations": timing.post_workout.recommendations,
            },
            "daily_protein_distribution": {
                "meals_per_day": timing.daily_protein_distribution.meals_per_day,
                "protein_per_meal_g": timing.daily_protein_distribution.protein_per_meal_g,
                "strategy": timing.daily_protein_distribution.strategy,
            },
            "input_parameters": {
                "workout_intensity": intensity_str,
                "weight_kg": weight_kg,
                "daily_protein_g": daily_protein_g,
            },
            "calculated_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// SearchFoodTool
// ============================================================================

/// Tool for searching the USDA food database.
pub struct SearchFoodTool;

#[async_trait]
impl McpTool for SearchFoodTool {
    fn name(&self) -> &'static str {
        "search_food"
    }

    fn description(&self) -> &'static str {
        "Search USDA FoodData Central database for foods. Returns up to 10 results by default. Check the `has_more` field before requesting additional pages."
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "query".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Search query for food items".to_owned()),
            },
        );
        properties.insert(
            "page_size".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Number of results per page (default: 10, max: 50)".to_owned()),
            },
        );
        properties.insert(
            "page_number".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Page number (1-indexed, default: 1). Only use if previous response had has_more=true".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["query".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Searching food database");

        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("query is required"))?;

        #[allow(clippy::cast_possible_truncation)]
        let page_size = args
            .get("page_size")
            .and_then(Value::as_u64)
            .map_or(10_u32, |s| s.min(50) as u32);

        #[allow(clippy::cast_possible_truncation)]
        let page_number = args
            .get("page_number")
            .and_then(Value::as_u64)
            .map_or(1_u32, |p| p.clamp(1, 100) as u32);

        let client = get_usda_client(ctx)?;

        let results = client
            .search_foods(query, page_size, page_number)
            .await
            .map_err(|e| AppError::internal(format!("USDA search failed: {e}")))?;

        let foods: Vec<_> = results
            .foods
            .iter()
            .map(|food| {
                json!({
                    "fdc_id": food.fdc_id,
                    "description": food.description,
                    "brand_owner": food.brand_owner,
                    "data_type": food.data_type,
                })
            })
            .collect();

        let has_more = results.current_page < results.total_pages;

        Ok(ToolResult::ok(json!({
            "query": query,
            "foods": foods,
            "returned_count": foods.len(),
            "total_hits": results.total_hits,
            "page_number": results.current_page,
            "page_size": page_size,
            "total_pages": results.total_pages,
            "has_more": has_more,
            "searched_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// GetFoodDetailsTool
// ============================================================================

/// Tool for getting detailed food information.
pub struct GetFoodDetailsTool;

#[async_trait]
impl McpTool for GetFoodDetailsTool {
    fn name(&self) -> &'static str {
        "get_food_details"
    }

    fn description(&self) -> &'static str {
        "Get detailed nutritional information for a specific food item"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "fdc_id".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("USDA FoodData Central ID of the food item".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["fdc_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Getting food details");

        let fdc_id = args
            .get("fdc_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| AppError::invalid_input("fdc_id is required"))?;

        let client = get_usda_client(ctx)?;

        let details = client
            .get_food_details(fdc_id)
            .await
            .map_err(|e| AppError::internal(format!("USDA food details fetch failed: {e}")))?;

        // Format nutrients for response
        let nutrients: Vec<_> = details
            .food_nutrients
            .iter()
            .map(|n| {
                json!({
                    "name": n.nutrient_name,
                    "amount": n.amount,
                    "unit": n.unit_name,
                })
            })
            .collect();

        Ok(ToolResult::ok(json!({
            "fdc_id": details.fdc_id,
            "description": details.description,
            "data_type": details.data_type,
            "serving_size": details.serving_size,
            "serving_size_unit": details.serving_size_unit,
            "nutrients": nutrients,
            "fetched_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// AnalyzeMealNutritionTool
// ============================================================================

/// Tool for analyzing meal nutritional content.
pub struct AnalyzeMealNutritionTool;

#[async_trait]
impl McpTool for AnalyzeMealNutritionTool {
    fn name(&self) -> &'static str {
        "analyze_meal_nutrition"
    }

    fn description(&self) -> &'static str {
        "Analyze nutritional content of a meal from its ingredients"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "ingredients".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some(
                    "Array of ingredients with fdc_id and amount_g fields".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["ingredients".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Analyzing meal nutrition");

        let ingredients = args
            .get("ingredients")
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::invalid_input("ingredients array is required"))?;

        if ingredients.is_empty() {
            return Err(AppError::invalid_input(
                "At least one ingredient is required",
            ));
        }

        let client = get_usda_client(ctx)?;

        let mut total_calories = 0.0;
        let mut total_protein = 0.0;
        let mut total_carbs = 0.0;
        let mut total_fat = 0.0;
        let mut total_fiber = 0.0;
        let mut analyzed_ingredients = Vec::new();

        for ingredient in ingredients {
            let fdc_id = ingredient
                .get("fdc_id")
                .and_then(Value::as_u64)
                .ok_or_else(|| AppError::invalid_input("Each ingredient needs fdc_id"))?;

            let amount_g = ingredient
                .get("amount_g")
                .and_then(Value::as_f64)
                .ok_or_else(|| AppError::invalid_input("Each ingredient needs amount_g"))?;

            let details = client
                .get_food_details(fdc_id)
                .await
                .map_err(|e| AppError::internal(format!("Failed to fetch food {fdc_id}: {e}")))?;

            // Calculate scaled nutrients (USDA provides per 100g)
            let scale = amount_g / 100.0;

            let calories = find_nutrient_amount(&details.food_nutrients, "Energy") * scale;
            let protein = find_nutrient_amount(&details.food_nutrients, "Protein") * scale;
            let carbs =
                find_nutrient_amount(&details.food_nutrients, "Carbohydrate, by difference")
                    * scale;
            let fat = find_nutrient_amount(&details.food_nutrients, "Total lipid (fat)") * scale;
            let fiber =
                find_nutrient_amount(&details.food_nutrients, "Fiber, total dietary") * scale;

            total_calories += calories;
            total_protein += protein;
            total_carbs += carbs;
            total_fat += fat;
            total_fiber += fiber;

            analyzed_ingredients.push(json!({
                "fdc_id": fdc_id,
                "description": details.description,
                "amount_g": amount_g,
                "calories": round_to_1(calories),
                "protein_g": round_to_1(protein),
                "carbs_g": round_to_1(carbs),
                "fat_g": round_to_1(fat),
            }));
        }

        Ok(ToolResult::ok(json!({
            "meal_totals": {
                "calories": round_to_1(total_calories),
                "protein_g": round_to_1(total_protein),
                "carbs_g": round_to_1(total_carbs),
                "fat_g": round_to_1(total_fat),
                "fiber_g": round_to_1(total_fiber),
            },
            "macro_breakdown": {
                "protein_percent": calculate_macro_percent(total_protein * 4.0, total_calories),
                "carbs_percent": calculate_macro_percent(total_carbs * 4.0, total_calories),
                "fat_percent": calculate_macro_percent(total_fat * 9.0, total_calories),
            },
            "ingredients": analyzed_ingredients,
            "ingredient_count": analyzed_ingredients.len(),
            "analyzed_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// Nutrient helpers
// ============================================================================

/// Find a nutrient amount by name
fn find_nutrient_amount(nutrients: &[FoodNutrient], name: &str) -> f64 {
    nutrients
        .iter()
        .find(|n| n.nutrient_name == name)
        .map_or(0.0, |n| n.amount)
}

/// Calculate macro percentage
fn calculate_macro_percent(macro_calories: f64, total_calories: f64) -> f64 {
    if total_calories > 0.0 {
        round_to_1((macro_calories / total_calories) * 100.0)
    } else {
        0.0
    }
}

/// Round to 1 decimal place
fn round_to_1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all nutrition tools for registration
#[must_use]
pub fn create_nutrition_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(CalculateDailyNutritionTool),
        Box::new(GetNutrientTimingTool),
        Box::new(SearchFoodTool),
        Box::new(GetFoodDetailsTool),
        Box::new(AnalyzeMealNutritionTool),
    ]
}
