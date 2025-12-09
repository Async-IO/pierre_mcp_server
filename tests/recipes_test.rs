// ABOUTME: Integration tests for the recipes module
// ABOUTME: Tests MealTiming, unit conversion, and recipe functionality
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tests for the recipes module including:
//! - Meal timing and macro distribution
//! - Ingredient unit conversion
//! - Recipe scaling and validation

use chrono::Utc;
use pierre_mcp_server::intelligence::recipes::{
    convert_to_grams, ConversionError, IngredientUnit, MacroTargets, MealTiming, Recipe,
    RecipeIngredient, ValidatedNutrition,
};
use uuid::Uuid;

// ============================================================================
// Meal Timing Tests
// ============================================================================

#[test]
fn test_meal_timing_macro_distribution() {
    // Pre-training should have highest carbs
    let (p, c, f) = MealTiming::PreTraining.macro_distribution();
    assert_eq!(c, 55);
    assert_eq!(p + c + f, 100);

    // Post-training should have highest protein
    let (p, c, f) = MealTiming::PostTraining.macro_distribution();
    assert_eq!(p, 30);
    assert_eq!(p + c + f, 100);

    // Rest day should have lowest carbs
    let (p, c, f) = MealTiming::RestDay.macro_distribution();
    assert_eq!(c, 35);
    assert_eq!(p + c + f, 100);
}

#[test]
fn test_macro_targets_from_calories() {
    let targets = MacroTargets::from_calories_and_timing(500.0, MealTiming::PostTraining);

    // Post-training: 30% protein, 45% carbs, 25% fat
    // 500 * 0.30 / 4 = 37.5g protein
    // 500 * 0.45 / 4 = 56.25g carbs
    // 500 * 0.25 / 9 = 13.89g fat
    assert!(
        (targets.protein_g.unwrap() - 37.5).abs() < 0.1,
        "Expected ~37.5g protein"
    );
    assert!(
        (targets.carbs_g.unwrap() - 56.25).abs() < 0.1,
        "Expected ~56.25g carbs"
    );
    assert!(
        (targets.fat_g.unwrap() - 13.89).abs() < 0.1,
        "Expected ~13.89g fat"
    );
}

// ============================================================================
// Ingredient Unit Tests
// ============================================================================

#[test]
fn test_ingredient_unit_properties() {
    assert!(IngredientUnit::Cups.is_volume());
    assert!(IngredientUnit::Grams.is_weight());
    assert!(IngredientUnit::Pieces.is_count());

    assert!(!IngredientUnit::Grams.is_volume());
    assert!(!IngredientUnit::Cups.is_weight());
}

// ============================================================================
// Unit Conversion Tests
// ============================================================================

#[test]
fn test_weight_conversions() {
    // Grams (identity)
    let grams_result = convert_to_grams("anything", 100.0, IngredientUnit::Grams).unwrap();
    assert!(
        (grams_result - 100.0).abs() < 0.01,
        "100g should convert to 100g"
    );

    // Kilograms
    let kg_result = convert_to_grams("anything", 1.5, IngredientUnit::Kilograms).unwrap();
    assert!(
        (kg_result - 1500.0).abs() < 0.01,
        "1.5kg should convert to 1500g"
    );

    // Ounces
    let oz_result = convert_to_grams("anything", 1.0, IngredientUnit::Ounces).unwrap();
    assert!(
        (oz_result - 28.35).abs() < 0.01,
        "1oz should convert to ~28.35g"
    );

    // Pounds
    let lb_result = convert_to_grams("anything", 1.0, IngredientUnit::Pounds).unwrap();
    assert!(
        (lb_result - 453.6).abs() < 0.01,
        "1lb should convert to ~453.6g"
    );
}

#[test]
fn test_volume_conversions() {
    // Rice: 0.77 g/ml
    // 1 cup rice = 240ml * 0.77 = 184.8g
    let rice_grams = convert_to_grams("rice", 1.0, IngredientUnit::Cups).unwrap();
    assert!(
        (rice_grams - 184.8).abs() < 0.1,
        "1 cup rice should be ~184.8g"
    );

    // 1 tbsp olive oil = 15ml * 0.92 = 13.8g
    let oil_grams = convert_to_grams("olive oil", 1.0, IngredientUnit::Tablespoons).unwrap();
    assert!(
        (oil_grams - 13.8).abs() < 0.1,
        "1 tbsp olive oil should be ~13.8g"
    );

    // 1 tsp honey = 5ml * 1.42 = 7.1g
    let honey_grams = convert_to_grams("honey", 1.0, IngredientUnit::Teaspoons).unwrap();
    assert!(
        (honey_grams - 7.1).abs() < 0.1,
        "1 tsp honey should be ~7.1g"
    );
}

#[test]
fn test_count_conversions() {
    // 2 eggs = 2 * 50g = 100g
    let egg_grams = convert_to_grams("egg", 2.0, IngredientUnit::Pieces).unwrap();
    assert!((egg_grams - 100.0).abs() < 0.1, "2 eggs should be ~100g");

    // 1 banana = 120g
    let banana_grams = convert_to_grams("banana", 1.0, IngredientUnit::Pieces).unwrap();
    assert!(
        (banana_grams - 120.0).abs() < 0.1,
        "1 banana should be ~120g"
    );

    // 3 garlic cloves = 3 * 3g = 9g
    let garlic_grams = convert_to_grams("garlic clove", 3.0, IngredientUnit::Pieces).unwrap();
    assert!(
        (garlic_grams - 9.0).abs() < 0.1,
        "3 garlic cloves should be ~9g"
    );
}

#[test]
fn test_alias_matching() {
    // "chicken" should match "chicken breast"
    let grams = convert_to_grams("chicken", 1.0, IngredientUnit::Cups).unwrap();
    assert!(grams > 0.0, "Should find chicken via partial match");

    // "rolled oats" should match "oats"
    let grams = convert_to_grams("rolled oats", 1.0, IngredientUnit::Cups).unwrap();
    assert!(grams > 0.0, "Should find oats via alias");
}

#[test]
fn test_case_insensitivity() {
    let lower = convert_to_grams("rice", 1.0, IngredientUnit::Cups).unwrap();
    let upper = convert_to_grams("RICE", 1.0, IngredientUnit::Cups).unwrap();
    let mixed = convert_to_grams("Rice", 1.0, IngredientUnit::Cups).unwrap();

    assert!(
        (lower - upper).abs() < 0.01,
        "Case should not affect conversion"
    );
    assert!(
        (lower - mixed).abs() < 0.01,
        "Case should not affect conversion"
    );
}

#[test]
fn test_unknown_ingredient() {
    let result = convert_to_grams("unicorn meat", 1.0, IngredientUnit::Cups);
    assert!(
        matches!(result, Err(ConversionError::DensityNotFound(_))),
        "Unknown ingredient should return DensityNotFound"
    );
}

#[test]
fn test_invalid_amount() {
    let result = convert_to_grams("rice", -1.0, IngredientUnit::Cups);
    assert!(
        matches!(result, Err(ConversionError::InvalidAmount)),
        "Negative amount should return InvalidAmount"
    );
}

#[test]
fn test_unsupported_unit() {
    // Eggs don't have volume density, only piece count
    let result = convert_to_grams("egg", 1.0, IngredientUnit::Cups);
    assert!(
        matches!(result, Err(ConversionError::UnsupportedUnit(_))),
        "Eggs don't support volume conversion"
    );
}

#[test]
fn test_has_density() {
    use pierre_mcp_server::intelligence::recipes::conversion::has_density;

    assert!(
        has_density("chicken breast"),
        "Common ingredient should exist"
    );
    assert!(has_density("rice"), "Common ingredient should exist");
    assert!(
        has_density("OLIVE OIL"),
        "Case insensitive lookup should work"
    );
    assert!(
        !has_density("unicorn meat"),
        "Unknown ingredient should return false"
    );
}

// ============================================================================
// Recipe Tests
// ============================================================================

#[test]
fn test_recipe_scaling() {
    let user_id = Uuid::new_v4();
    let recipe = Recipe::new(user_id, "Test Recipe", 2)
        .with_ingredient(RecipeIngredient::in_grams("chicken", 200.0))
        .with_ingredient(RecipeIngredient::in_grams("rice", 100.0));

    // Scale from 2 to 4 servings (double)
    let scaled = recipe.scaled(4);

    assert_eq!(scaled.servings, 4);
    assert!(
        (scaled.ingredients[0].grams - 400.0).abs() < 0.01,
        "Chicken should double to 400g"
    );
    assert!(
        (scaled.ingredients[1].grams - 200.0).abs() < 0.01,
        "Rice should double to 200g"
    );
}

#[test]
fn test_validated_nutrition_meets_targets() {
    let nutrition = ValidatedNutrition {
        calories: 500.0,
        protein_g: 40.0,
        carbs_g: 50.0,
        fat_g: 15.0,
        fiber_g: Some(5.0),
        sodium_mg: None,
        sugar_g: None,
        validated_at: Utc::now(),
    };

    // Within 10% tolerance
    let targets = MacroTargets {
        calories: Some(480.0), // 4% under
        protein_g: Some(38.0), // 5% under
        carbs_g: Some(52.0),   // 4% over
        fat_g: Some(14.0),     // 7% under
        fiber_g: None,
    };

    assert!(
        nutrition.meets_targets(&targets),
        "Should meet targets within 10% tolerance"
    );

    // Outside tolerance
    let strict_targets = MacroTargets {
        calories: Some(400.0), // 25% under - too far
        protein_g: None,
        carbs_g: None,
        fat_g: None,
        fiber_g: None,
    };

    assert!(
        !nutrition.meets_targets(&strict_targets),
        "Should fail targets outside 10% tolerance"
    );
}

#[test]
fn test_recipe_total_time() {
    let user_id = Uuid::new_v4();

    let recipe1 = Recipe::new(user_id, "Quick", 1)
        .with_prep_time(10)
        .with_cook_time(20);
    assert_eq!(
        recipe1.total_time_mins(),
        Some(30),
        "Prep + cook should sum"
    );

    let recipe2 = Recipe::new(user_id, "Prep Only", 1).with_prep_time(15);
    assert_eq!(
        recipe2.total_time_mins(),
        Some(15),
        "Prep only should return prep time"
    );

    let recipe3 = Recipe::new(user_id, "No Time", 1);
    assert_eq!(
        recipe3.total_time_mins(),
        None,
        "No times should return None"
    );
}
