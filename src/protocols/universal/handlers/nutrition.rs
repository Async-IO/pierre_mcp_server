// ABOUTME: Nutrition analysis tool handlers for MCP protocol
// ABOUTME: Implements 5 tools: calculate_daily_nutrition, get_nutrient_timing, search_food, get_food_details, analyze_meal_nutrition
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::external::{UsdaClient, UsdaClientConfig};
use crate::intelligence::{
    calculate_daily_nutrition_needs, calculate_nutrient_timing, ActivityLevel,
    DailyNutritionParams, Gender, TrainingGoal, WorkoutIntensity,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

/// Fetch food details from USDA API
async fn fetch_food_details(
    fdc_id: u64,
    executor: &crate::protocols::universal::UniversalToolExecutor,
) -> Result<crate::external::FoodDetails, UniversalResponse> {
    let api_key = executor
        .resources
        .config
        .usda_api_key
        .clone()
        .unwrap_or_default();

    if api_key.is_empty() {
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(
                "USDA API key not configured. Set USDA_API_KEY environment variable.".to_string(),
            ),
            metadata: None,
        });
    }

    let usda_config = UsdaClientConfig {
        api_key,
        ..UsdaClientConfig::default()
    };

    let client = UsdaClient::new(usda_config);
    client
        .get_food_details(fdc_id)
        .await
        .map_err(|e| UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("USDA API request failed: {e}")),
            metadata: None,
        })
}

/// Parse gender from string parameter
fn parse_gender(gender_str: &str) -> Result<Gender, UniversalResponse> {
    match gender_str.to_lowercase().as_str() {
        "male" => Ok(Gender::Male),
        "female" => Ok(Gender::Female),
        _ => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some("Gender must be 'male' or 'female'".to_string()),
            metadata: None,
        }),
    }
}

/// Parse activity level from string parameter
fn parse_activity_level(activity_str: &str) -> Result<ActivityLevel, UniversalResponse> {
    match activity_str.to_lowercase().as_str() {
        "sedentary" => Ok(ActivityLevel::Sedentary),
        "lightly_active" => Ok(ActivityLevel::LightlyActive),
        "moderately_active" => Ok(ActivityLevel::ModeratelyActive),
        "very_active" => Ok(ActivityLevel::VeryActive),
        "extra_active" => Ok(ActivityLevel::ExtraActive),
        _ => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some("Invalid activity_level. Must be one of: sedentary, lightly_active, moderately_active, very_active, extra_active".to_string()),
            metadata: None,
        }),
    }
}

/// Parse training goal from string parameter
fn parse_training_goal(goal_str: &str) -> Result<TrainingGoal, UniversalResponse> {
    match goal_str.to_lowercase().as_str() {
        "maintenance" => Ok(TrainingGoal::Maintenance),
        "weight_loss" => Ok(TrainingGoal::WeightLoss),
        "muscle_gain" => Ok(TrainingGoal::MuscleGain),
        "endurance_performance" => Ok(TrainingGoal::EndurancePerformance),
        _ => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some("Invalid training_goal. Must be one of: maintenance, weight_loss, muscle_gain, endurance_performance".to_string()),
            metadata: None,
        }),
    }
}

/// Parse nutrition parameters from request
fn parse_nutrition_params(
    request: &UniversalRequest,
) -> Result<DailyNutritionParams, UniversalResponse> {
    let weight_kg = request
        .parameters
        .get("weight_kg")
        .and_then(Value::as_f64)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some("Missing or invalid required parameter: weight_kg".to_string()),
            metadata: None,
        })?;

    let height_cm = request
        .parameters
        .get("height_cm")
        .and_then(Value::as_f64)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some("Missing or invalid required parameter: height_cm".to_string()),
            metadata: None,
        })?;

    let age_u64 = request
        .parameters
        .get("age")
        .and_then(Value::as_u64)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some("Missing or invalid required parameter: age".to_string()),
            metadata: None,
        })?;

    #[allow(clippy::cast_possible_truncation)] // Age validated to be <= 150
    let age = if age_u64 <= 150 {
        age_u64 as u32
    } else {
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some("Age must be between 0 and 150 years".to_string()),
            metadata: None,
        });
    };

    let gender_str = request
        .parameters
        .get("gender")
        .and_then(Value::as_str)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some("Missing or invalid required parameter: gender".to_string()),
            metadata: None,
        })?;

    let gender = parse_gender(gender_str)?;

    let activity_level_str = request
        .parameters
        .get("activity_level")
        .and_then(Value::as_str)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some("Missing or invalid required parameter: activity_level".to_string()),
            metadata: None,
        })?;

    let activity_level = parse_activity_level(activity_level_str)?;

    let training_goal_str = request
        .parameters
        .get("training_goal")
        .and_then(Value::as_str)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some("Missing or invalid required parameter: training_goal".to_string()),
            metadata: None,
        })?;

    let training_goal = parse_training_goal(training_goal_str)?;

    Ok(DailyNutritionParams {
        weight_kg,
        height_cm,
        age,
        gender,
        activity_level,
        training_goal,
    })
}

/// Handle `calculate_daily_nutrition` tool - calculate daily calorie and macronutrient needs
///
/// Calculates BMR, TDEE, and macronutrient distribution based on:
/// - Athlete biometrics (weight, height, age, gender)
/// - Activity level
/// - Training goal (maintenance, weight loss, muscle gain, endurance)
///
/// # Parameters
/// - `weight_kg`: Body weight in kilograms (required)
/// - `height_cm`: Height in centimeters (required)
/// - `age`: Age in years (required)
/// - `gender`: "male" or "female" (required)
/// - `activity_level`: "sedentary", "`lightly_active`", "`moderately_active`", "`very_active`", or "`extra_active`" (required)
/// - `training_goal`: "maintenance", "`weight_loss`", "`muscle_gain`", or "`endurance_performance`" (required)
///
/// # Returns
/// JSON object with:
/// - `bmr`: Basal Metabolic Rate (kcal/day)
/// - `tdee`: Total Daily Energy Expenditure (kcal/day)
/// - `target_calories`: Adjusted calories for goal (kcal/day)
/// - `protein_g`: Daily protein target (grams)
/// - `carbs_g`: Daily carbohydrate target (grams)
/// - `fat_g`: Daily fat target (grams)
/// - `protein_percent`: Protein percentage of total calories
/// - `carbs_percent`: Carbs percentage of total calories
/// - `fat_percent`: Fat percentage of total calories
///
/// # Errors
/// Returns `ProtocolError` if required parameters are missing or invalid
#[must_use]
pub fn handle_calculate_daily_nutrition(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Executor required by handler signature but config accessed via global singleton
        let _ = executor;

        // Parse user parameters
        let params = match parse_nutrition_params(&request) {
            Ok(p) => p,
            Err(response) => return Ok(response),
        };

        // Get nutrition config
        let nutrition_config =
            &crate::config::intelligence_config::IntelligenceConfig::global().nutrition;

        // Calculate daily nutrition needs
        let nutrition_result = calculate_daily_nutrition_needs(
            &params,
            &nutrition_config.bmr,
            &nutrition_config.activity_factors,
            &nutrition_config.macronutrients,
        );

        match nutrition_result {
            Ok(nutrition) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "bmr": nutrition.bmr,
                    "tdee": nutrition.tdee,
                    "tdee": nutrition.tdee,
                    "protein_g": nutrition.protein_g,
                    "carbs_g": nutrition.carbs_g,
                    "fat_g": nutrition.fat_g,
                    "protein_percent": nutrition.macro_percentages.protein_percent,
                    "carbs_percent": nutrition.macro_percentages.carbs_percent,
                    "fat_percent": nutrition.macro_percentages.fat_percent,
                    "goal": format!("{:?}", params.training_goal),
                })),
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Calculation error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `get_nutrient_timing` tool - get pre/post-workout nutrition recommendations
///
/// Provides optimal nutrient timing for workouts based on:
/// - Athlete weight
/// - Daily protein target
/// - Workout intensity
///
/// # Parameters
/// - `weight_kg`: Body weight in kilograms (required)
/// - `daily_protein_g`: Daily protein target in grams (required)
/// - `workout_intensity`: "low", "moderate", or "high" (required)
///
/// # Returns
/// JSON object with:
/// - `pre_workout`: Object with `timing_minutes`, `carbs_g`, `protein_g`
/// - `post_workout`: Object with `timing_minutes`, `protein_g`, `carbs_g`
/// - `protein_distribution`: Object with `meals_per_day`, `protein_per_meal_g`, `breakfast_g`, `lunch_g`, `dinner_g`, `snacks_g`
///
/// # Errors
/// Returns `ProtocolError` if required parameters are missing or invalid
#[must_use]
pub fn handle_get_nutrient_timing(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Executor required by handler signature but config accessed via global singleton
        let _ = executor;

        let weight_kg = request
            .parameters
            .get("weight_kg")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(
                    "Missing or invalid required parameter: weight_kg".to_string(),
                )
            })?;

        let daily_protein_g = request
            .parameters
            .get("daily_protein_g")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(
                    "Missing or invalid required parameter: daily_protein_g".to_string(),
                )
            })?;

        let workout_intensity_str = request
            .parameters
            .get("workout_intensity")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(
                    "Missing or invalid required parameter: workout_intensity".to_string(),
                )
            })?;

        let workout_intensity = match workout_intensity_str.to_lowercase().as_str() {
            "low" => WorkoutIntensity::Low,
            "moderate" => WorkoutIntensity::Moderate,
            "high" => WorkoutIntensity::High,
            _ => {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "Invalid workout_intensity. Must be one of: low, moderate, high"
                            .to_string(),
                    ),
                    metadata: None,
                })
            }
        };

        let config = &crate::config::intelligence_config::IntelligenceConfig::global().nutrition;

        let timing_result = calculate_nutrient_timing(
            weight_kg,
            daily_protein_g,
            workout_intensity,
            &config.nutrient_timing,
        );

        match timing_result {
            Ok(timing) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
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
                })),
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Calculation error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `search_food` tool - search USDA `FoodData` Central database
///
/// Searches for foods by name/description in the USDA database.
/// Uses free USDA `FoodData` Central API with 24-hour caching.
///
/// # Parameters
/// - `query`: Search query (e.g., "apple", "chicken breast") (required)
/// - `page_size`: Number of results to return (1-200, default: 10) (optional)
///
/// # Returns
/// JSON array of foods with:
/// - `fdc_id`: `FoodData` Central ID
/// - `description`: Food description
/// - `data_type`: Data source type
/// - `brand_owner`: Brand name (if applicable)
///
/// # Errors
/// Returns `ProtocolError` if query is missing or API request fails
#[must_use]
pub fn handle_search_food(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let query = request
            .parameters
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(
                    "Missing or invalid required parameter: query".to_string(),
                )
            })?;

        let page_size_u64 = request
            .parameters
            .get("page_size")
            .and_then(Value::as_u64)
            .unwrap_or(10);

        #[allow(clippy::cast_possible_truncation)] // Page size validated to be <= 200
        let page_size = if page_size_u64 <= 200 {
            page_size_u64 as u32
        } else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("Page size must be between 1 and 200".to_string()),
                metadata: None,
            });
        };

        // Search foods using USDA API
        let api_key = executor
            .resources
            .config
            .usda_api_key
            .clone()
            .unwrap_or_default();

        if api_key.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    "USDA API key not configured. Set USDA_API_KEY environment variable."
                        .to_string(),
                ),
                metadata: None,
            });
        }

        let usda_config = UsdaClientConfig {
            api_key,
            ..UsdaClientConfig::default()
        };

        let client = UsdaClient::new(usda_config);
        let search_result = client.search_foods(query, page_size).await;

        match search_result {
            Ok(foods) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "foods": foods,
                    "total": foods.len(),
                })),
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Search error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `get_food_details` tool - get detailed nutritional information for a food
///
/// Retrieves complete nutritional data for a specific food from USDA database.
///
/// # Parameters
/// - `fdc_id`: `FoodData` Central ID (required)
///
/// # Returns
/// JSON object with:
/// - `fdc_id`: `FoodData` Central ID
/// - `description`: Food description
/// - `nutrients`: Array of nutrients with name, amount, and unit
/// - `serving_size`: Serving size (grams)
///
/// # Errors
/// Returns `ProtocolError` if `fdc_id` is missing or food not found
#[must_use]
pub fn handle_get_food_details(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let fdc_id = request
            .parameters
            .get("fdc_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(
                    "Missing or invalid required parameter: fdc_id".to_string(),
                )
            })?;

        // Get food details using USDA API
        let api_key = executor
            .resources
            .config
            .usda_api_key
            .clone()
            .unwrap_or_default();

        if api_key.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    "USDA API key not configured. Set USDA_API_KEY environment variable."
                        .to_string(),
                ),
                metadata: None,
            });
        }

        let usda_config = UsdaClientConfig {
            api_key,
            ..UsdaClientConfig::default()
        };

        let client = UsdaClient::new(usda_config);
        let details_result = client.get_food_details(fdc_id).await;

        match details_result {
            Ok(food) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "fdc_id": food.fdc_id,
                    "description": food.description,
                    "data_type": food.data_type,
                    "nutrients": food.food_nutrients.iter().map(|n| json!({
                        "nutrient_id": n.nutrient_id,
                        "name": n.nutrient_name,
                        "amount": n.amount,
                        "unit": n.unit_name,
                    })).collect::<Vec<_>>(),
                    "serving_size": food.serving_size,
                    "serving_size_unit": food.serving_size_unit,
                })),
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Food not found: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `analyze_meal_nutrition` tool - analyze total nutrition for a meal
///
/// Calculates total calories and macronutrients for a meal composed of multiple foods.
///
/// # Parameters
/// - `foods`: Array of food items with `fdc_id` and `grams` (required)
///
/// # Example
/// ```json
/// {
///   "foods": [
///     {"fdc_id": 171477, "grams": 150},
///     {"fdc_id": 171688, "grams": 182}
///   ]
/// }
/// ```
///
/// # Returns
/// JSON object with:
/// - `total_calories`: Total calories (kcal)
/// - `total_protein_g`: Total protein (grams)
/// - `total_carbs_g`: Total carbohydrates (grams)
/// - `total_fat_g`: Total fat (grams)
/// - `foods`: Array of food details with amounts
///
/// # Errors
/// Returns `ProtocolError` if foods array is missing or invalid
#[must_use]
pub fn handle_analyze_meal_nutrition(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let foods_array = request
            .parameters
            .get("foods")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(
                    "Missing or invalid required parameter: foods (must be array)".to_string(),
                )
            })?;

        // Parse food items
        let mut meal_foods = Vec::new();
        for food_item in foods_array {
            let fdc_id = food_item
                .get("fdc_id")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    ProtocolError::InvalidRequest("Each food must have fdc_id".to_string())
                })?;

            let grams = food_item
                .get("grams")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    ProtocolError::InvalidRequest("Each food must have grams".to_string())
                })?;

            meal_foods.push((fdc_id, grams));
        }

        // Get food details for each item
        let mut total_calories = 0.0;
        let mut total_protein = 0.0;
        let mut total_carbs = 0.0;
        let mut total_fat = 0.0;
        let mut food_details = Vec::new();

        for (fdc_id, grams) in meal_foods {
            let food = match fetch_food_details(fdc_id, executor).await {
                Ok(f) => f,
                Err(response) => return Ok(response),
            };

            // Calculate nutrition per gram (USDA data is per 100g)
            let multiplier = grams / 100.0;

            // Find key nutrients
            for nutrient in &food.food_nutrients {
                match nutrient.nutrient_name.as_str() {
                    "Energy" => total_calories += nutrient.amount * multiplier,
                    "Protein" => total_protein += nutrient.amount * multiplier,
                    "Carbohydrate, by difference" => {
                        total_carbs += nutrient.amount * multiplier;
                    }
                    "Total lipid (fat)" => total_fat += nutrient.amount * multiplier,
                    _ => {}
                }
            }

            food_details.push(json!({
                "fdc_id": fdc_id,
                "description": food.description,
                "grams": grams,
            }));
        }

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "total_calories": total_calories.round(),
                "total_protein_g": total_protein.round(),
                "total_carbs_g": total_carbs.round(),
                "total_fat_g": total_fat.round(),
                "foods": food_details,
            })),
            error: None,
            metadata: None,
        })
    })
}
