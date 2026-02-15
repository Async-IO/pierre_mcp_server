// ABOUTME: Recipe management tool handlers for MCP protocol ("Combat des Chefs" architecture)
// ABOUTME: Implements 7 tools: get_recipe_constraints, validate_recipe, save_recipe, list_recipes, get_recipe, delete_recipe, search_recipes
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::{IntelligenceConfig, MealTdeeProportionsConfig};
use crate::database::recipes::RecipeManager;
use crate::external::{UsdaClient, UsdaClientConfig};
use crate::intelligence::recipes::{
    convert_to_grams, DietaryRestriction, IngredientUnit, MacroTargets, MealTiming, Recipe,
    RecipeConstraints, RecipeIngredient, SkillLevel,
};
use crate::models::TenantId;
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::utils::uuid::parse_user_id_for_protocol;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

use super::{apply_format_to_response, extract_output_format};

/// TDEE context for calorie calculation in recipe constraints
/// Bundles TDEE-related parameters to reduce function argument count
struct TdeeContext<'a> {
    /// Whether the calories were calculated from TDEE (vs explicit or fallback)
    tdee_based: bool,
    /// User's daily TDEE if provided
    tdee: Option<f64>,
    /// Reference to the TDEE proportion configuration
    proportions: &'a MealTdeeProportionsConfig,
}

/// Input parameters for saving a recipe
#[derive(Debug, Deserialize)]
struct SaveRecipeParams {
    name: String,
    description: Option<String>,
    servings: u8,
    prep_time_mins: Option<u16>,
    cook_time_mins: Option<u16>,
    instructions: Vec<String>,
    ingredients: Vec<IngredientInput>,
    tags: Option<Vec<String>>,
    meal_timing: Option<String>,
}

/// Input format for recipe ingredients
#[derive(Debug, Deserialize)]
struct IngredientInput {
    name: String,
    amount: f64,
    unit: String,
    preparation: Option<String>,
}

/// Handle `get_recipe_constraints` tool - get macro targets for LLM recipe generation
///
/// Returns personalized macro targets and constraints based on daily calorie budget,
/// user's TDEE for personalized calculation, meal timing, and dietary restrictions.
///
/// # Parameters
/// - `calories`: Target calories for the meal (optional, calculated from TDEE/timing if not provided)
/// - `tdee`: User's Total Daily Energy Expenditure in kcal (optional, enables personalized meal calories)
/// - `meal_timing`: `pre_training`, `post_training`, `rest_day`, or `general` (optional, default: `general`)
/// - `dietary_restrictions`: Array of restrictions like `gluten_free`, `vegan` (optional)
/// - `max_prep_time_mins`: Maximum preparation time (optional)
/// - `max_cook_time_mins`: Maximum cooking time (optional)
///
/// # Returns
/// JSON object with macro targets and a prompt hint for LLM recipe generation.
/// When TDEE is provided, includes `tdee_based: true` and `tdee_proportion` in response.
///
/// # Errors
/// Returns `ProtocolError` if parameters are invalid
#[must_use]
pub fn handle_get_recipe_constraints(
    _executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_recipe_constraints cancelled".to_owned(),
                ));
            }
        }

        let meal_timing = request
            .parameters
            .get("meal_timing")
            .and_then(Value::as_str)
            .map_or(MealTiming::General, parse_meal_timing);

        let tdee = request.parameters.get("tdee").and_then(Value::as_f64);
        let config = IntelligenceConfig::global();
        let tdee_proportions = &config.nutrition.meal_tdee_proportions;

        // Get calories: explicit > TDEE-based > defaults
        let (calories, tdee_based) = request
            .parameters
            .get("calories")
            .and_then(Value::as_f64)
            .map_or_else(
                || {
                    (
                        tdee_proportions.calories_for_timing(meal_timing, tdee),
                        tdee.is_some(),
                    )
                },
                |explicit_cals| (explicit_cals, false),
            );

        let macro_targets = MacroTargets::from_calories_and_timing(calories, meal_timing);
        let (protein_pct, carbs_pct, fat_pct) = meal_timing.macro_distribution();

        let tdee_ctx = TdeeContext {
            tdee_based,
            tdee,
            proportions: tdee_proportions,
        };

        let prompt_hint = build_recipe_prompt_hint(
            meal_timing,
            calories,
            &macro_targets,
            protein_pct,
            carbs_pct,
            fat_pct,
            &tdee_ctx,
        );

        let constraints =
            build_recipe_constraints(macro_targets, meal_timing, &prompt_hint, &request);

        let result = build_constraints_response(
            &constraints,
            calories,
            meal_timing,
            &prompt_hint,
            &tdee_ctx,
        );

        Ok(UniversalResponse {
            success: true,
            result: Some(result),
            error: None,
            metadata: None,
        })
    })
}

/// Build prompt hint for LLM recipe generation
fn build_recipe_prompt_hint(
    timing: MealTiming,
    calories: f64,
    macros: &MacroTargets,
    protein_pct: u8,
    carbs_pct: u8,
    fat_pct: u8,
    tdee_ctx: &TdeeContext<'_>,
) -> String {
    let tdee_info = if tdee_ctx.tdee_based {
        let proportion = tdee_ctx.proportions.proportion_for_timing(timing);
        format!(
            " (Based on TDEE of {:.0} kcal, {:.1}% of daily calories)",
            tdee_ctx.tdee.unwrap_or(0.0),
            proportion * 100.0
        )
    } else {
        String::new()
    };

    format!(
        "Create a {} recipe (~{:.0} kcal){} with approximately {:.0}g protein, {:.0}g carbs, {:.0}g fat. \
         Macro distribution: {}% protein, {}% carbs, {}% fat.",
        timing.description(),
        calories,
        tdee_info,
        macros.protein_g.unwrap_or(0.0),
        macros.carbs_g.unwrap_or(0.0),
        macros.fat_g.unwrap_or(0.0),
        protein_pct,
        carbs_pct,
        fat_pct
    )
}

/// Build recipe constraints struct from request parameters
fn build_recipe_constraints(
    macro_targets: MacroTargets,
    meal_timing: MealTiming,
    prompt_hint: &str,
    request: &UniversalRequest,
) -> RecipeConstraints {
    RecipeConstraints {
        macro_targets,
        dietary_restrictions: parse_dietary_restrictions(
            request
                .parameters
                .get("dietary_restrictions")
                .and_then(Value::as_array),
        ),
        cuisine_preferences: Vec::new(),
        excluded_ingredients: Vec::new(),
        max_prep_time_mins: parse_time_mins(&request.parameters, "max_prep_time_mins"),
        max_cook_time_mins: parse_time_mins(&request.parameters, "max_cook_time_mins"),
        skill_level: SkillLevel::default(),
        meal_timing,
        prompt_hint: Some(prompt_hint.to_owned()),
    }
}

/// Parse time in minutes from request parameters with capping
fn parse_time_mins(params: &Value, key: &str) -> Option<u16> {
    params.get(key).and_then(Value::as_u64).map(|v| {
        #[allow(clippy::cast_possible_truncation)]
        let capped = v.min(480) as u16;
        capped
    })
}

/// Build the JSON response for recipe constraints
fn build_constraints_response(
    constraints: &RecipeConstraints,
    calories: f64,
    meal_timing: MealTiming,
    prompt_hint: &str,
    tdee_ctx: &TdeeContext<'_>,
) -> Value {
    let mut result = json!({
        "calories": calories,
        "protein_g": constraints.macro_targets.protein_g,
        "carbs_g": constraints.macro_targets.carbs_g,
        "fat_g": constraints.macro_targets.fat_g,
        "meal_timing": format!("{meal_timing:?}").to_lowercase(),
        "meal_timing_description": meal_timing.description(),
        "prompt_hint": prompt_hint,
        "max_prep_time_mins": constraints.max_prep_time_mins,
        "max_cook_time_mins": constraints.max_cook_time_mins,
        "tdee_based": tdee_ctx.tdee_based,
    });

    if let Some(user_tdee) = tdee_ctx.tdee {
        result["tdee"] = json!(user_tdee);
        result["tdee_proportion"] = json!(tdee_ctx.proportions.proportion_for_timing(meal_timing));
    }

    result
}

/// Handle `validate_recipe` tool - validate recipe nutrition via USDA
///
/// Validates a recipe's ingredients against the USDA `FoodData` Central database
/// and calculates accurate nutrition information per serving.
///
/// # Parameters
/// - `name`: Recipe name (required)
/// - `servings`: Number of servings (required)
/// - `ingredients`: Array of {name, amount, unit, preparation?} (required)
///
/// # Returns
/// JSON object with:
/// - `nutrition_per_serving`: Calculated macros and micronutrients per serving
/// - `validation_completeness`: Percentage of ingredients matched (0.0 to 1.0)
/// - `usda_matched_count`: Number of ingredients with successful USDA lookups
/// - `total_ingredients`: Total number of ingredients in the recipe
/// - `warnings`: Any issues encountered during validation
///
/// # Errors
/// Returns `ProtocolError` if required parameters missing or USDA API fails
#[must_use]
#[allow(clippy::too_many_lines)] // Complex validation logic with USDA API calls
pub fn handle_validate_recipe(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "validate_recipe cancelled".to_owned(),
                ));
            }
        }

        let servings_val = request
            .parameters
            .get("servings")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: servings".to_owned())
            })?;
        if servings_val == 0 {
            return Err(ProtocolError::InvalidRequest(
                "servings must be at least 1".to_owned(),
            ));
        }
        #[allow(clippy::cast_possible_truncation)]
        let servings = servings_val.min(255) as u8;

        let ingredients_json = request
            .parameters
            .get("ingredients")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(
                    "Missing required parameter: ingredients (must be array)".to_owned(),
                )
            })?;

        // Get USDA client
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
                error: Some("USDA API key not configured".to_owned()),
                metadata: None,
            });
        }

        let usda_config = UsdaClientConfig {
            api_key,
            ..UsdaClientConfig::default()
        };
        let client = UsdaClient::new(usda_config);

        // Process ingredients and calculate nutrition
        let mut total_calories = 0.0;
        let mut total_protein = 0.0;
        let mut total_carbs = 0.0;
        let mut total_fat = 0.0;
        let mut total_fiber = 0.0;
        let mut total_sodium = 0.0;
        let mut total_sugar = 0.0;
        let mut warnings: Vec<String> = Vec::new();
        let mut validated_ingredients: Vec<Value> = Vec::new();
        let mut usda_matched_count: u32 = 0;

        for ingredient_value in ingredients_json {
            let name = ingredient_value
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ProtocolError::InvalidRequest("Each ingredient must have 'name'".to_owned())
                })?;

            let amount = ingredient_value
                .get("amount")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    ProtocolError::InvalidRequest("Each ingredient must have 'amount'".to_owned())
                })?;

            let unit_str = ingredient_value
                .get("unit")
                .and_then(Value::as_str)
                .unwrap_or("grams");

            // Convert to grams
            let unit = parse_ingredient_unit(unit_str);
            let grams = match convert_to_grams(name, amount, unit) {
                Ok(g) => g,
                Err(e) => {
                    warnings.push(format!("Could not convert {name}: {e}"));
                    // Estimate 100g per cup as fallback
                    if unit.is_volume() {
                        amount * 100.0
                    } else if unit.is_count() {
                        amount * 50.0
                    } else {
                        amount
                    }
                }
            };

            // Search USDA for ingredient (page_size=1, page_number=1 for first match)
            match client.search_foods(name, 1, 1).await {
                Ok(result) if !result.foods.is_empty() => {
                    let food = &result.foods[0];
                    match client.get_food_details(food.fdc_id).await {
                        Ok(details) => {
                            let multiplier = grams / 100.0;
                            for nutrient in &details.food_nutrients {
                                match nutrient.nutrient_name.as_str() {
                                    "Energy" => total_calories += nutrient.amount * multiplier,
                                    "Protein" => total_protein += nutrient.amount * multiplier,
                                    "Carbohydrate, by difference" => {
                                        total_carbs += nutrient.amount * multiplier;
                                    }
                                    "Total lipid (fat)" => {
                                        total_fat += nutrient.amount * multiplier;
                                    }
                                    "Fiber, total dietary" => {
                                        total_fiber += nutrient.amount * multiplier;
                                    }
                                    "Sodium, Na" => total_sodium += nutrient.amount * multiplier,
                                    "Sugars, total including NLEA" => {
                                        total_sugar += nutrient.amount * multiplier;
                                    }
                                    _ => {}
                                }
                            }
                            validated_ingredients.push(json!({
                                "name": name,
                                "amount": amount,
                                "unit": unit_str,
                                "grams": grams,
                                "fdc_id": food.fdc_id,
                                "usda_match": food.description,
                            }));
                            usda_matched_count += 1;
                        }
                        Err(e) => {
                            warnings.push(format!("USDA lookup failed for {name}: {e}"));
                            validated_ingredients.push(json!({
                                "name": name,
                                "amount": amount,
                                "unit": unit_str,
                                "grams": grams,
                                "usda_match": null,
                            }));
                        }
                    }
                }
                Ok(_) => {
                    warnings.push(format!("No USDA match found for: {name}"));
                    validated_ingredients.push(json!({
                        "name": name,
                        "amount": amount,
                        "unit": unit_str,
                        "grams": grams,
                        "usda_match": null,
                    }));
                }
                Err(e) => {
                    warnings.push(format!("USDA search failed for {name}: {e}"));
                    validated_ingredients.push(json!({
                        "name": name,
                        "amount": amount,
                        "unit": unit_str,
                        "grams": grams,
                        "usda_match": null,
                    }));
                }
            }
        }

        // Calculate per-serving values
        let servings_f64 = f64::from(servings);
        let nutrition_per_serving = json!({
            "calories": (total_calories / servings_f64).round(),
            "protein_g": (total_protein / servings_f64 * 10.0).round() / 10.0,
            "carbs_g": (total_carbs / servings_f64 * 10.0).round() / 10.0,
            "fat_g": (total_fat / servings_f64 * 10.0).round() / 10.0,
            "fiber_g": (total_fiber / servings_f64 * 10.0).round() / 10.0,
            "sodium_mg": (total_sodium / servings_f64).round(),
            "sugar_g": (total_sugar / servings_f64 * 10.0).round() / 10.0,
        });

        // Calculate validation completeness (percentage of ingredients with USDA matches)
        #[allow(clippy::cast_precision_loss)]
        let total_ingredients = validated_ingredients.len() as f64;
        let validation_completeness = if total_ingredients > 0.0 {
            (f64::from(usda_matched_count) / total_ingredients * 100.0).round() / 100.0
        } else {
            0.0
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "validated": true,
                "servings": servings,
                "nutrition_per_serving": nutrition_per_serving,
                "ingredients": validated_ingredients,
                "warnings": warnings,
                "validated_at": Utc::now().to_rfc3339(),
                "validation_completeness": validation_completeness,
                "usda_matched_count": usda_matched_count,
                "total_ingredients": validated_ingredients.len(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `save_recipe` tool - save a validated recipe to user's collection
///
/// Saves a recipe with validated nutrition information to the user's personal recipe collection.
///
/// # Parameters
/// - `name`: Recipe name (required)
/// - `description`: Recipe description (optional)
/// - `servings`: Number of servings (required)
/// - `prep_time_mins`: Preparation time in minutes (optional)
/// - `cook_time_mins`: Cooking time in minutes (optional)
/// - `instructions`: Array of instruction steps (required)
/// - `ingredients`: Array of {name, amount, unit, preparation?} (required)
/// - `tags`: Array of tags (optional)
/// - `meal_timing`: `pre_training`, `post_training`, `rest_day`, or `general` (optional)
///
/// # Returns
/// JSON object with the saved recipe ID
///
/// # Errors
/// Returns `ProtocolError` if required parameters missing or save fails
#[must_use]
pub fn handle_save_recipe(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "save_recipe cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

        // Parse recipe parameters
        let params: SaveRecipeParams =
            serde_json::from_value(request.parameters.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid recipe parameters: {e}"))
            })?;

        // Build recipe
        let meal_timing = params
            .meal_timing
            .as_deref()
            .map_or(MealTiming::General, parse_meal_timing);

        let mut recipe = Recipe::new(user_id, &params.name, params.servings)
            .with_meal_timing(meal_timing)
            .with_instructions(params.instructions);

        if let Some(desc) = params.description {
            recipe = recipe.with_description(desc);
        }

        if let Some(prep) = params.prep_time_mins {
            recipe = recipe.with_prep_time(prep);
        }

        if let Some(cook) = params.cook_time_mins {
            recipe = recipe.with_cook_time(cook);
        }

        if let Some(tags) = params.tags {
            for tag in tags {
                recipe = recipe.with_tag(tag);
            }
        }

        // Convert ingredients
        let mut ingredients = Vec::new();
        for ing in params.ingredients {
            let unit = parse_ingredient_unit(&ing.unit);
            let grams = convert_to_grams(&ing.name, ing.amount, unit).unwrap_or(ing.amount);
            let mut ingredient = RecipeIngredient::new(&ing.name, ing.amount, unit, grams);
            if let Some(prep) = ing.preparation {
                ingredient = ingredient.with_preparation(prep);
            }
            ingredients.push(ingredient);
        }
        recipe = recipe.with_ingredients(ingredients);

        // Save to database
        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("Database not available".to_owned()))?;

        let manager = RecipeManager::new(pool.clone());
        let recipe_id = manager
            .create_recipe(user_id, tenant_id, &recipe)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to save recipe: {e}")))?;

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "recipe_id": recipe_id,
                "name": params.name,
                "servings": params.servings,
                "meal_timing": format!("{meal_timing:?}").to_lowercase(),
                "created_at": Utc::now().to_rfc3339(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `list_recipes` tool - list user's saved recipes
///
/// # Parameters
/// - `meal_timing`: Filter by `pre_training`, `post_training`, `rest_day`, `general` (optional)
/// - `limit`: Maximum number of recipes to return (default: 50)
/// - `offset`: Pagination offset (default: 0)
///
/// # Returns
/// JSON array of recipe summaries
#[must_use]
pub fn handle_list_recipes(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

        let meal_timing = request
            .parameters
            .get("meal_timing")
            .and_then(Value::as_str)
            .map(parse_meal_timing);

        // Pagination: default 20, max 100
        let limit = request
            .parameters
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(20_u32, |v| {
                #[allow(clippy::cast_possible_truncation)]
                let capped = v.min(100) as u32;
                capped
            });

        // Handle offset from both integer and float JSON numbers (MCP clients may send floats)
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = request.parameters.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("Database not available".to_owned()))?;

        let manager = RecipeManager::new(pool.clone());
        let recipes = manager
            .list_recipes(user_id, tenant_id, meal_timing, Some(limit), offset)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to list recipes: {e}")))?;

        let recipe_summaries: Vec<Value> = recipes
            .iter()
            .map(|r| {
                json!({
                    "id": r.id.to_string(),
                    "name": r.name,
                    "servings": r.servings,
                    "meal_timing": format!("{:?}", r.meal_timing).to_lowercase(),
                    "total_time_mins": r.total_time_mins(),
                    "tags": r.tags,
                    "has_nutrition": r.nutrition.is_some(),
                    "calories_per_serving": r.nutrition.as_ref().map(|n| n.calories.round()),
                    "updated_at": r.updated_at.to_rfc3339(),
                })
            })
            .collect();

        // Pagination: has_more is true if we got exactly limit results (more may exist)
        let returned_count = recipe_summaries.len();
        #[allow(clippy::cast_possible_truncation)]
        let has_more = returned_count == limit as usize;
        let offset_val = offset.unwrap_or(0);

        let result = UniversalResponse {
            success: true,
            result: Some(json!({
                "recipes": recipe_summaries,
                "count": returned_count,
                "offset": offset_val,
                "limit": limit,
                "has_more": has_more,
            })),
            error: None,
            metadata: None,
        };

        // Apply format transformation
        Ok(apply_format_to_response(result, "recipes", output_format))
    })
}

/// Handle `get_recipe` tool - get a specific recipe by ID
///
/// # Parameters
/// - `recipe_id`: Recipe UUID (required)
///
/// # Returns
/// Full recipe details including ingredients and nutrition
#[must_use]
pub fn handle_get_recipe(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

        let recipe_id = request
            .parameters
            .get("recipe_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: recipe_id".to_owned())
            })?;

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("Database not available".to_owned()))?;

        let manager = RecipeManager::new(pool.clone());
        let recipe = manager
            .get_recipe(recipe_id, user_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to get recipe: {e}")))?;

        match recipe {
            Some(r) => {
                let result = UniversalResponse {
                    success: true,
                    result: Some(json!({
                        "id": r.id.to_string(),
                        "name": r.name,
                        "description": r.description,
                        "servings": r.servings,
                        "prep_time_mins": r.prep_time_mins,
                        "cook_time_mins": r.cook_time_mins,
                        "total_time_mins": r.total_time_mins(),
                        "meal_timing": format!("{:?}", r.meal_timing).to_lowercase(),
                        "ingredients": r.ingredients.iter().map(|i| json!({
                            "name": i.name,
                            "amount": i.amount,
                            "unit": format!("{:?}", i.unit).to_lowercase(),
                            "grams": i.grams,
                            "preparation": i.preparation,
                            "fdc_id": i.fdc_id,
                        })).collect::<Vec<_>>(),
                        "instructions": r.instructions,
                        "tags": r.tags,
                        "nutrition_per_serving": r.nutrition.map(|n| json!({
                            "calories": n.calories.round(),
                            "protein_g": (n.protein_g * 10.0).round() / 10.0,
                            "carbs_g": (n.carbs_g * 10.0).round() / 10.0,
                            "fat_g": (n.fat_g * 10.0).round() / 10.0,
                            "fiber_g": n.fiber_g.map(|v| (v * 10.0).round() / 10.0),
                            "sodium_mg": n.sodium_mg.map(f64::round),
                            "sugar_g": n.sugar_g.map(|v| (v * 10.0).round() / 10.0),
                            "validated_at": n.validated_at.to_rfc3339(),
                        })),
                        "created_at": r.created_at.to_rfc3339(),
                        "updated_at": r.updated_at.to_rfc3339(),
                    })),
                    error: None,
                    metadata: None,
                };
                // Apply format transformation
                Ok(apply_format_to_response(result, "recipe", output_format))
            }
            None => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Recipe not found: {recipe_id}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `delete_recipe` tool - delete a recipe from user's collection
///
/// # Parameters
/// - `recipe_id`: Recipe UUID (required)
///
/// # Returns
/// Success confirmation
#[must_use]
pub fn handle_delete_recipe(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

        let recipe_id = request
            .parameters
            .get("recipe_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: recipe_id".to_owned())
            })?;

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("Database not available".to_owned()))?;

        let manager = RecipeManager::new(pool.clone());
        let deleted = manager
            .delete_recipe(recipe_id, user_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to delete recipe: {e}")))?;

        if deleted {
            Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "deleted": true,
                    "recipe_id": recipe_id,
                })),
                error: None,
                metadata: None,
            })
        } else {
            Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Recipe not found: {recipe_id}")),
                metadata: None,
            })
        }
    })
}

/// Handle `search_recipes` tool - search user's recipes by name, tags, or description
///
/// # Parameters
/// - `query`: Search query string (required)
/// - `limit`: Maximum results (default: 20)
///
/// # Returns
/// JSON array of matching recipes
#[must_use]
pub fn handle_search_recipes(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

        let query = request
            .parameters
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: query".to_owned())
            })?;

        // Pagination: default 10, max 100
        #[allow(clippy::cast_possible_truncation)]
        let limit = request
            .parameters
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(10_u32, |v| v.min(100) as u32);

        // Handle offset from both integer and float JSON numbers (MCP clients may send floats)
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = request.parameters.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("Database not available".to_owned()))?;

        let manager = RecipeManager::new(pool.clone());
        let recipes = manager
            .search_recipes(user_id, tenant_id, query, Some(limit), offset)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to search recipes: {e}")))?;

        let results: Vec<Value> = recipes
            .iter()
            .map(|r| {
                json!({
                    "id": r.id.to_string(),
                    "name": r.name,
                    "description": r.description,
                    "servings": r.servings,
                    "meal_timing": format!("{:?}", r.meal_timing).to_lowercase(),
                    "tags": r.tags,
                    "calories_per_serving": r.nutrition.as_ref().map(|n| n.calories.round()),
                })
            })
            .collect();

        // Pagination: has_more is true if we got exactly limit results (more may exist)
        let returned_count = results.len();
        #[allow(clippy::cast_possible_truncation)]
        let has_more = returned_count == limit as usize;
        let offset_val = offset.unwrap_or(0);

        let result = UniversalResponse {
            success: true,
            result: Some(json!({
                "query": query,
                "results": results,
                "count": returned_count,
                "offset": offset_val,
                "limit": limit,
                "has_more": has_more,
            })),
            error: None,
            metadata: None,
        };

        // Apply format transformation
        Ok(apply_format_to_response(result, "results", output_format))
    })
}

// Helper functions

fn parse_meal_timing(s: &str) -> MealTiming {
    match s.to_lowercase().as_str() {
        "pre_training" => MealTiming::PreTraining,
        "post_training" => MealTiming::PostTraining,
        "rest_day" => MealTiming::RestDay,
        _ => MealTiming::General,
    }
}

fn parse_ingredient_unit(s: &str) -> IngredientUnit {
    match s.to_lowercase().as_str() {
        "milliliters" | "ml" => IngredientUnit::Milliliters,
        "cups" | "cup" => IngredientUnit::Cups,
        "tablespoons" | "tbsp" => IngredientUnit::Tablespoons,
        "teaspoons" | "tsp" => IngredientUnit::Teaspoons,
        "pieces" | "piece" | "pc" => IngredientUnit::Pieces,
        "ounces" | "oz" => IngredientUnit::Ounces,
        "pounds" | "lb" => IngredientUnit::Pounds,
        "kilograms" | "kg" => IngredientUnit::Kilograms,
        // Default to grams for "grams", "g", or any unrecognized unit
        _ => IngredientUnit::Grams,
    }
}

fn parse_dietary_restrictions(arr: Option<&Vec<Value>>) -> Vec<DietaryRestriction> {
    use DietaryRestriction;

    let Some(values) = arr else {
        return Vec::new();
    };

    values
        .iter()
        .filter_map(|v| v.as_str())
        .filter_map(|s| match s.to_lowercase().as_str() {
            "gluten_free" => Some(DietaryRestriction::GlutenFree),
            "dairy_free" => Some(DietaryRestriction::DairyFree),
            "vegan" => Some(DietaryRestriction::Vegan),
            "vegetarian" => Some(DietaryRestriction::Vegetarian),
            "nut_free" => Some(DietaryRestriction::NutFree),
            "low_sodium" => Some(DietaryRestriction::LowSodium),
            "low_sugar" => Some(DietaryRestriction::LowSugar),
            "keto" => Some(DietaryRestriction::Keto),
            "paleo" => Some(DietaryRestriction::Paleo),
            _ => None,
        })
        .collect()
}
