// ABOUTME: Recipe management tools for meal planning and nutrition.
// ABOUTME: Implements validate_recipe, save_recipe, list_recipes, search_recipes, etc.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Recipe Management Tools
//!
//! This module provides tools for recipe management with direct business logic:
//! - `GetRecipeConstraintsTool` - Get macro targets for recipe generation
//! - `ValidateRecipeTool` - Validate recipe nutrition via USDA
//! - `SaveRecipeTool` - Save a new recipe
//! - `ListRecipesTool` - List user's recipes
//! - `GetRecipeTool` - Get recipe details
//! - `DeleteRecipeTool` - Delete a recipe
//! - `SearchRecipesTool` - Search recipes

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::IntelligenceConfig;
use crate::database::recipes::RecipeManager;
use crate::errors::{AppError, AppResult};
use crate::external::{UsdaClient, UsdaClientConfig};
use crate::intelligence::recipes::{
    convert_to_grams, DietaryRestriction, IngredientUnit, MacroTargets, MealTiming, Recipe,
    RecipeConstraints, RecipeIngredient, SkillLevel,
};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::models::TenantId;
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions
// ============================================================================

/// Get `RecipeManager` from context resources
fn get_recipe_manager(ctx: &ToolExecutionContext) -> AppResult<RecipeManager> {
    let pool = ctx
        .resources
        .database
        .sqlite_pool()
        .ok_or_else(|| AppError::internal("SQLite database required for recipes"))?;
    Ok(RecipeManager::new(pool.clone()))
}

/// Get tenant ID from context
fn get_tenant_id(ctx: &ToolExecutionContext) -> TenantId {
    ctx.tenant_id
        .map_or_else(|| TenantId::from(ctx.user_id), TenantId::from)
}

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
        _ => IngredientUnit::Grams,
    }
}

fn parse_dietary_restrictions(arr: Option<&Vec<Value>>) -> Vec<DietaryRestriction> {
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

// ============================================================================
// GetRecipeConstraintsTool
// ============================================================================

/// Tool for getting recipe constraints and macro targets.
pub struct GetRecipeConstraintsTool;

#[async_trait]
impl McpTool for GetRecipeConstraintsTool {
    fn name(&self) -> &'static str {
        "get_recipe_constraints"
    }

    fn description(&self) -> &'static str {
        "Get macro targets and constraints for LLM recipe generation"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "calories".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Target calories for the meal".to_owned()),
            },
        );
        properties.insert(
            "tdee".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("User's Total Daily Energy Expenditure".to_owned()),
            },
        );
        properties.insert(
            "meal_timing".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("pre_training, post_training, rest_day, or general".to_owned()),
            },
        );
        properties.insert(
            "dietary_restrictions".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Dietary restrictions like gluten_free, vegan".to_owned()),
            },
        );
        properties.insert(
            "max_prep_time_mins".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum preparation time".to_owned()),
            },
        );
        properties.insert(
            "max_cook_time_mins".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum cooking time".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, _ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let meal_timing = args
            .get("meal_timing")
            .and_then(Value::as_str)
            .map_or(MealTiming::General, parse_meal_timing);

        let tdee = args.get("tdee").and_then(Value::as_f64);
        let config = IntelligenceConfig::global();
        let tdee_proportions = &config.nutrition.meal_tdee_proportions;

        let (calories, tdee_based) = args.get("calories").and_then(Value::as_f64).map_or_else(
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

        let tdee_info = if tdee_based {
            let proportion = tdee_proportions.proportion_for_timing(meal_timing);
            format!(
                " (Based on TDEE of {:.0} kcal, {:.1}% of daily calories)",
                tdee.unwrap_or(0.0),
                proportion * 100.0
            )
        } else {
            String::new()
        };

        let prompt_hint = format!(
            "Create a {} recipe (~{:.0} kcal){} with approximately {:.0}g protein, {:.0}g carbs, {:.0}g fat. \
             Macro distribution: {}% protein, {}% carbs, {}% fat.",
            meal_timing.description(),
            calories,
            tdee_info,
            macro_targets.protein_g.unwrap_or(0.0),
            macro_targets.carbs_g.unwrap_or(0.0),
            macro_targets.fat_g.unwrap_or(0.0),
            protein_pct,
            carbs_pct,
            fat_pct
        );

        #[allow(clippy::cast_possible_truncation)]
        let max_prep = args
            .get("max_prep_time_mins")
            .and_then(Value::as_u64)
            .map(|v| v.min(480) as u16);
        #[allow(clippy::cast_possible_truncation)]
        let max_cook = args
            .get("max_cook_time_mins")
            .and_then(Value::as_u64)
            .map(|v| v.min(480) as u16);

        let constraints = RecipeConstraints {
            macro_targets,
            dietary_restrictions: parse_dietary_restrictions(
                args.get("dietary_restrictions").and_then(Value::as_array),
            ),
            cuisine_preferences: Vec::new(),
            excluded_ingredients: Vec::new(),
            max_prep_time_mins: max_prep,
            max_cook_time_mins: max_cook,
            skill_level: SkillLevel::default(),
            meal_timing,
            prompt_hint: Some(prompt_hint.clone()),
        };

        let mut result = json!({
            "calories": calories,
            "protein_g": constraints.macro_targets.protein_g,
            "carbs_g": constraints.macro_targets.carbs_g,
            "fat_g": constraints.macro_targets.fat_g,
            "meal_timing": format!("{meal_timing:?}").to_lowercase(),
            "meal_timing_description": meal_timing.description(),
            "prompt_hint": prompt_hint,
            "max_prep_time_mins": max_prep,
            "max_cook_time_mins": max_cook,
            "tdee_based": tdee_based,
        });

        if let Some(user_tdee) = tdee {
            result["tdee"] = json!(user_tdee);
            result["tdee_proportion"] = json!(tdee_proportions.proportion_for_timing(meal_timing));
        }

        Ok(ToolResult::ok(result))
    }
}

// ============================================================================
// ValidateRecipeTool
// ============================================================================

/// Tool for validating recipe nutrition via USDA.
pub struct ValidateRecipeTool;

#[async_trait]
impl McpTool for ValidateRecipeTool {
    fn name(&self) -> &'static str {
        "validate_recipe"
    }

    fn description(&self) -> &'static str {
        "Validate recipe nutrition using USDA FoodData Central"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Recipe name".to_owned()),
            },
        );
        properties.insert(
            "servings".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Number of servings".to_owned()),
            },
        );
        properties.insert(
            "ingredients".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Array of {name, amount, unit}".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["servings".to_owned(), "ingredients".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    #[allow(clippy::too_many_lines)]
    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let servings_val = args
            .get("servings")
            .and_then(Value::as_u64)
            .ok_or_else(|| AppError::invalid_input("servings is required"))?;
        if servings_val == 0 {
            return Err(AppError::invalid_input("servings must be at least 1"));
        }
        #[allow(clippy::cast_possible_truncation)]
        let servings = servings_val.min(255) as u8;

        let ingredients_json = args
            .get("ingredients")
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::invalid_input("ingredients array is required"))?;

        let api_key = ctx
            .resources
            .config
            .usda_api_key
            .clone()
            .unwrap_or_default();

        if api_key.is_empty() {
            return Err(AppError::internal("USDA API key not configured"));
        }

        let usda_config = UsdaClientConfig {
            api_key,
            ..UsdaClientConfig::default()
        };
        let client = UsdaClient::new(usda_config);

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
                .ok_or_else(|| AppError::invalid_input("Each ingredient must have 'name'"))?;

            let amount = ingredient_value
                .get("amount")
                .and_then(Value::as_f64)
                .ok_or_else(|| AppError::invalid_input("Each ingredient must have 'amount'"))?;

            let unit_str = ingredient_value
                .get("unit")
                .and_then(Value::as_str)
                .unwrap_or("grams");

            let unit = parse_ingredient_unit(unit_str);
            let grams = match convert_to_grams(name, amount, unit) {
                Ok(g) => g,
                Err(e) => {
                    warnings.push(format!("Could not convert {name}: {e}"));
                    if unit.is_volume() {
                        amount * 100.0
                    } else if unit.is_count() {
                        amount * 50.0
                    } else {
                        amount
                    }
                }
            };

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
                                "usda_match": Value::Null,
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
                        "usda_match": Value::Null,
                    }));
                }
                Err(e) => {
                    warnings.push(format!("USDA search failed for {name}: {e}"));
                    validated_ingredients.push(json!({
                        "name": name,
                        "amount": amount,
                        "unit": unit_str,
                        "grams": grams,
                        "usda_match": Value::Null,
                    }));
                }
            }
        }

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

        #[allow(clippy::cast_precision_loss)]
        let total_ingredients = validated_ingredients.len() as f64;
        let validation_completeness = if total_ingredients > 0.0 {
            (f64::from(usda_matched_count) / total_ingredients * 100.0).round() / 100.0
        } else {
            0.0
        };

        Ok(ToolResult::ok(json!({
            "validated": true,
            "servings": servings,
            "nutrition_per_serving": nutrition_per_serving,
            "ingredients": validated_ingredients,
            "warnings": warnings,
            "validated_at": Utc::now().to_rfc3339(),
            "validation_completeness": validation_completeness,
            "usda_matched_count": usda_matched_count,
            "total_ingredients": validated_ingredients.len(),
        })))
    }
}

// ============================================================================
// SaveRecipeTool
// ============================================================================

/// Tool for saving a recipe.
pub struct SaveRecipeTool;

#[async_trait]
impl McpTool for SaveRecipeTool {
    fn name(&self) -> &'static str {
        "save_recipe"
    }

    fn description(&self) -> &'static str {
        "Save a validated recipe to your collection"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Recipe name".to_owned()),
            },
        );
        properties.insert(
            "servings".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Number of servings".to_owned()),
            },
        );
        properties.insert(
            "instructions".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Array of instruction steps".to_owned()),
            },
        );
        properties.insert(
            "ingredients".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Array of {name, amount, unit}".to_owned()),
            },
        );
        properties.insert(
            "description".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Recipe description".to_owned()),
            },
        );
        properties.insert(
            "meal_timing".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("pre_training, post_training, rest_day, or general".to_owned()),
            },
        );
        properties.insert(
            "tags".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Tags for organization".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec![
                "name".to_owned(),
                "servings".to_owned(),
                "instructions".to_owned(),
                "ingredients".to_owned(),
            ]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let params: SaveRecipeParams = serde_json::from_value(args)
            .map_err(|e| AppError::invalid_input(format!("Invalid recipe parameters: {e}")))?;

        let tenant_id = get_tenant_id(ctx);
        let meal_timing = params
            .meal_timing
            .as_deref()
            .map_or(MealTiming::General, parse_meal_timing);

        let mut recipe = Recipe::new(ctx.user_id, &params.name, params.servings)
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

        let manager = get_recipe_manager(ctx)?;
        let recipe_id = manager
            .create_recipe(ctx.user_id, tenant_id, &recipe)
            .await?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "recipe_id": recipe_id,
            "name": params.name,
            "servings": params.servings,
            "meal_timing": format!("{meal_timing:?}").to_lowercase(),
            "created_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// ListRecipesTool
// ============================================================================

/// Tool for listing user's recipes.
pub struct ListRecipesTool;

#[async_trait]
impl McpTool for ListRecipesTool {
    fn name(&self) -> &'static str {
        "list_recipes"
    }

    fn description(&self) -> &'static str {
        "List your saved recipes"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "meal_timing".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Filter by meal timing".to_owned()),
            },
        );
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum results (default: 20)".to_owned()),
            },
        );
        properties.insert(
            "offset".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Pagination offset".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let tenant_id = get_tenant_id(ctx);
        let meal_timing = args
            .get("meal_timing")
            .and_then(Value::as_str)
            .map(parse_meal_timing);

        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(20_u32, |v| v.min(100) as u32);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = args.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let manager = get_recipe_manager(ctx)?;
        let recipes = manager
            .list_recipes(ctx.user_id, tenant_id, meal_timing, Some(limit), offset)
            .await?;

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

        let returned_count = recipe_summaries.len();
        #[allow(clippy::cast_possible_truncation)]
        let has_more = returned_count == limit as usize;

        Ok(ToolResult::ok(json!({
            "recipes": recipe_summaries,
            "count": returned_count,
            "offset": offset.unwrap_or(0),
            "limit": limit,
            "has_more": has_more,
        })))
    }
}

// ============================================================================
// GetRecipeTool
// ============================================================================

/// Tool for getting a specific recipe.
pub struct GetRecipeTool;

#[async_trait]
impl McpTool for GetRecipeTool {
    fn name(&self) -> &'static str {
        "get_recipe"
    }

    fn description(&self) -> &'static str {
        "Get details of a specific recipe"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "recipe_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Recipe ID".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["recipe_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let recipe_id = args
            .get("recipe_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("recipe_id is required"))?;

        let tenant_id = get_tenant_id(ctx);
        let manager = get_recipe_manager(ctx)?;
        let recipe = manager
            .get_recipe(recipe_id, ctx.user_id, tenant_id)
            .await?;

        match recipe {
            Some(r) => Ok(ToolResult::ok(json!({
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
            }))),
            None => Err(AppError::not_found(format!("Recipe {recipe_id}"))),
        }
    }
}

// ============================================================================
// DeleteRecipeTool
// ============================================================================

/// Tool for deleting a recipe.
pub struct DeleteRecipeTool;

#[async_trait]
impl McpTool for DeleteRecipeTool {
    fn name(&self) -> &'static str {
        "delete_recipe"
    }

    fn description(&self) -> &'static str {
        "Delete a recipe from your collection"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "recipe_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Recipe ID to delete".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["recipe_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let recipe_id = args
            .get("recipe_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("recipe_id is required"))?;

        let tenant_id = get_tenant_id(ctx);
        let manager = get_recipe_manager(ctx)?;
        let deleted = manager
            .delete_recipe(recipe_id, ctx.user_id, tenant_id)
            .await?;

        if deleted {
            Ok(ToolResult::ok(json!({
                "success": true,
                "deleted": true,
                "recipe_id": recipe_id,
            })))
        } else {
            Err(AppError::not_found(format!("Recipe {recipe_id}")))
        }
    }
}

// ============================================================================
// SearchRecipesTool
// ============================================================================

/// Tool for searching recipes.
pub struct SearchRecipesTool;

#[async_trait]
impl McpTool for SearchRecipesTool {
    fn name(&self) -> &'static str {
        "search_recipes"
    }

    fn description(&self) -> &'static str {
        "Search your recipes by name, tags, or description"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "query".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Search query".to_owned()),
            },
        );
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum results (default: 10)".to_owned()),
            },
        );
        properties.insert(
            "offset".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Pagination offset".to_owned()),
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
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("query is required"))?;

        let tenant_id = get_tenant_id(ctx);

        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(10_u32, |v| v.min(100) as u32);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = args.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let manager = get_recipe_manager(ctx)?;
        let recipes = manager
            .search_recipes(ctx.user_id, tenant_id, query, Some(limit), offset)
            .await?;

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

        let returned_count = results.len();
        #[allow(clippy::cast_possible_truncation)]
        let has_more = returned_count == limit as usize;

        Ok(ToolResult::ok(json!({
            "query": query,
            "results": results,
            "count": returned_count,
            "offset": offset.unwrap_or(0),
            "limit": limit,
            "has_more": has_more,
        })))
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all recipe tools for registration
#[must_use]
pub fn create_recipe_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(GetRecipeConstraintsTool),
        Box::new(ValidateRecipeTool),
        Box::new(SaveRecipeTool),
        Box::new(ListRecipesTool),
        Box::new(GetRecipeTool),
        Box::new(DeleteRecipeTool),
        Box::new(SearchRecipesTool),
    ]
}
