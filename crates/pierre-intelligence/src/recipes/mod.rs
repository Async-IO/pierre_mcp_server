// ABOUTME: Recipe management module for nutrition planning with training-aware suggestions
// ABOUTME: Provides recipe storage, USDA validation, and unit conversion utilities
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Recipes Module
//!
//! Provides recipe management capabilities integrated with the nutrition system.
//! Supports training-aware meal planning with USDA-validated nutrition data.
//!
//! ## Architecture: "Combat des Chefs"
//!
//! This module implements a cost-efficient hybrid approach:
//! - **LLM clients (Claude, ChatGPT)**: Use `get_recipe_constraints` to get context,
//!   generate recipes themselves, then call `validate_recipe` for USDA verification.
//! - **Non-LLM clients**: Use `suggest_recipe` which calls Pierre's internal LLM.
//!
//! ## Key Features
//!
//! - Recipe storage with per-user ownership
//! - Ingredient unit conversion (cups, tbsp, pieces → grams)
//! - Training-aware meal timing (pre-training, post-training, rest day)
//! - USDA-validated nutrition data
//!
//! ## Example Usage
//!
//! ```text
//! use pierre_mcp_server::intelligence::recipes::{
//!     Recipe, MealTiming, IngredientUnit,
//! };
//!
//! // Create a recipe with unit conversion
//! let ingredient = RecipeIngredient::new("chicken breast", 1.0, IngredientUnit::Cups);
//! let grams = ingredient.to_grams(); // Converts to ~140g
//! ```

/// Unit conversion utilities for recipe ingredients
pub mod conversion;
/// Core data models for recipes
pub mod models;

// Re-export main types for convenience
pub use conversion::{convert_to_grams, ConversionError, IngredientDensity};
pub use models::{
    DietaryRestriction, IngredientUnit, MacroTargets, MealTiming, Recipe, RecipeConstraints,
    RecipeIngredient, SkillLevel, ValidatedNutrition,
};
