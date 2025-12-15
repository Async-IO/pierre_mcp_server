// ABOUTME: Unit conversion utilities for recipe ingredients
// ABOUTME: Converts volume/count units to grams using ingredient-specific densities
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::LazyLock;

use super::models::IngredientUnit;

/// Conversion error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    /// Density not found for ingredient
    DensityNotFound(String),
    /// Invalid conversion (e.g., negative amount)
    InvalidAmount,
    /// Unit not supported for this ingredient type
    UnsupportedUnit(IngredientUnit),
}

impl Display for ConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::DensityNotFound(name) => {
                write!(f, "Density not found for ingredient: {name}")
            }
            Self::InvalidAmount => write!(f, "Invalid amount (must be positive)"),
            Self::UnsupportedUnit(unit) => {
                write!(f, "Unit {unit:?} not supported for this conversion")
            }
        }
    }
}

impl Error for ConversionError {}

/// Density information for an ingredient
///
/// Used to convert volume measurements to weight (grams).
/// Density is expressed as grams per milliliter for liquids/powders,
/// or grams per piece for countable items.
#[derive(Debug, Clone)]
pub struct IngredientDensity {
    /// Grams per milliliter (for volume conversions)
    pub grams_per_ml: Option<f64>,
    /// Grams per piece (for count conversions)
    pub grams_per_piece: Option<f64>,
    /// Common names/aliases for this ingredient
    pub aliases: &'static [&'static str],
}

impl IngredientDensity {
    /// Create density for a liquid/powder ingredient
    const fn liquid(grams_per_ml: f64) -> Self {
        Self {
            grams_per_ml: Some(grams_per_ml),
            grams_per_piece: None,
            aliases: &[],
        }
    }

    /// Create density for a countable ingredient
    const fn countable(grams_per_piece: f64) -> Self {
        Self {
            grams_per_ml: None,
            grams_per_piece: Some(grams_per_piece),
            aliases: &[],
        }
    }

    /// Create density for an ingredient with both volume and count options
    const fn both(grams_per_ml: f64, grams_per_piece: f64) -> Self {
        Self {
            grams_per_ml: Some(grams_per_ml),
            grams_per_piece: Some(grams_per_piece),
            aliases: &[],
        }
    }

    /// Add aliases for matching
    const fn with_aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases;
        self
    }
}

/// Volume conversion constants (to milliliters)
const ML_PER_CUP: f64 = 240.0;
const ML_PER_TBSP: f64 = 15.0;
const ML_PER_TSP: f64 = 5.0;

/// Weight conversion constants (to grams)
const GRAMS_PER_OZ: f64 = 28.35;
const GRAMS_PER_LB: f64 = 453.6;
const GRAMS_PER_KG: f64 = 1000.0;

/// Common ingredient densities database
///
/// Densities are approximate averages. For precise nutrition calculations,
/// USDA FDC data should be used when available.
static INGREDIENT_DENSITIES: LazyLock<HashMap<&'static str, IngredientDensity>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();

        // === Proteins ===
        m.insert(
            "chicken breast",
            IngredientDensity::liquid(0.58).with_aliases(&["chicken", "boneless chicken"]),
        );
        m.insert(
            "ground beef",
            IngredientDensity::liquid(0.54).with_aliases(&["beef mince", "minced beef"]),
        );
        m.insert(
            "salmon",
            IngredientDensity::liquid(0.50).with_aliases(&["salmon fillet"]),
        );
        m.insert("tuna", IngredientDensity::liquid(0.55));
        m.insert(
            "egg",
            IngredientDensity::countable(50.0).with_aliases(&["eggs", "large egg"]),
        );
        m.insert(
            "egg white",
            IngredientDensity::both(1.03, 33.0).with_aliases(&["egg whites"]),
        );
        m.insert("tofu", IngredientDensity::liquid(0.52));
        m.insert(
            "greek yogurt",
            IngredientDensity::liquid(1.05).with_aliases(&["yogurt", "plain yogurt"]),
        );
        m.insert(
            "cottage cheese",
            IngredientDensity::liquid(0.96).with_aliases(&["low fat cottage cheese"]),
        );

        // === Grains & Carbs ===
        m.insert(
            "rice",
            IngredientDensity::liquid(0.77).with_aliases(&["white rice", "uncooked rice"]),
        );
        m.insert(
            "cooked rice",
            IngredientDensity::liquid(0.72).with_aliases(&["steamed rice"]),
        );
        m.insert(
            "oats",
            IngredientDensity::liquid(0.36).with_aliases(&[
                "rolled oats",
                "oatmeal",
                "old fashioned oats",
            ]),
        );
        m.insert(
            "quinoa",
            IngredientDensity::liquid(0.71).with_aliases(&["uncooked quinoa"]),
        );
        m.insert(
            "pasta",
            IngredientDensity::liquid(0.45).with_aliases(&["dry pasta", "uncooked pasta"]),
        );
        m.insert(
            "bread",
            IngredientDensity::countable(30.0).with_aliases(&["slice of bread", "bread slice"]),
        );
        m.insert(
            "flour",
            IngredientDensity::liquid(0.50).with_aliases(&[
                "all purpose flour",
                "all-purpose flour",
                "white flour",
            ]),
        );
        m.insert(
            "whole wheat flour",
            IngredientDensity::liquid(0.51).with_aliases(&["wholemeal flour"]),
        );

        // === Fruits ===
        m.insert(
            "banana",
            IngredientDensity::countable(120.0).with_aliases(&["bananas", "medium banana"]),
        );
        m.insert(
            "apple",
            IngredientDensity::countable(180.0).with_aliases(&["apples", "medium apple"]),
        );
        m.insert(
            "orange",
            IngredientDensity::countable(130.0).with_aliases(&["oranges", "medium orange"]),
        );
        m.insert(
            "blueberries",
            IngredientDensity::liquid(0.64).with_aliases(&["blueberry"]),
        );
        m.insert(
            "strawberries",
            IngredientDensity::liquid(0.53).with_aliases(&["strawberry"]),
        );
        m.insert(
            "avocado",
            IngredientDensity::countable(150.0).with_aliases(&["avocados", "medium avocado"]),
        );

        // === Vegetables ===
        m.insert(
            "spinach",
            IngredientDensity::liquid(0.12).with_aliases(&["fresh spinach", "baby spinach"]),
        );
        m.insert("broccoli", IngredientDensity::liquid(0.36));
        m.insert(
            "carrot",
            IngredientDensity::both(0.50, 60.0).with_aliases(&["carrots", "medium carrot"]),
        );
        m.insert(
            "onion",
            IngredientDensity::both(0.63, 110.0).with_aliases(&["onions", "medium onion"]),
        );
        m.insert(
            "garlic clove",
            IngredientDensity::countable(3.0).with_aliases(&["garlic", "clove of garlic"]),
        );
        m.insert(
            "tomato",
            IngredientDensity::both(0.60, 150.0).with_aliases(&["tomatoes", "medium tomato"]),
        );
        m.insert(
            "bell pepper",
            IngredientDensity::countable(120.0).with_aliases(&["pepper", "capsicum"]),
        );
        m.insert(
            "sweet potato",
            IngredientDensity::countable(130.0).with_aliases(&["sweet potatoes"]),
        );
        m.insert(
            "potato",
            IngredientDensity::countable(150.0).with_aliases(&["potatoes", "medium potato"]),
        );

        // === Dairy & Alternatives ===
        m.insert(
            "milk",
            IngredientDensity::liquid(1.03).with_aliases(&["whole milk", "skim milk"]),
        );
        m.insert(
            "almond milk",
            IngredientDensity::liquid(1.02).with_aliases(&["unsweetened almond milk"]),
        );
        m.insert(
            "butter",
            IngredientDensity::liquid(0.91).with_aliases(&["unsalted butter"]),
        );
        m.insert(
            "cheese",
            IngredientDensity::liquid(0.45).with_aliases(&["shredded cheese", "cheddar"]),
        );
        m.insert(
            "parmesan",
            IngredientDensity::liquid(0.42).with_aliases(&["parmesan cheese", "grated parmesan"]),
        );

        // === Fats & Oils ===
        m.insert(
            "olive oil",
            IngredientDensity::liquid(0.92).with_aliases(&["extra virgin olive oil", "evoo"]),
        );
        m.insert(
            "coconut oil",
            IngredientDensity::liquid(0.92).with_aliases(&["virgin coconut oil"]),
        );
        m.insert(
            "vegetable oil",
            IngredientDensity::liquid(0.92).with_aliases(&["canola oil", "cooking oil"]),
        );

        // === Nuts & Seeds ===
        m.insert(
            "almonds",
            IngredientDensity::liquid(0.56).with_aliases(&["almond", "whole almonds"]),
        );
        m.insert(
            "peanut butter",
            IngredientDensity::liquid(1.07).with_aliases(&["natural peanut butter"]),
        );
        m.insert(
            "almond butter",
            IngredientDensity::liquid(1.06).with_aliases(&["natural almond butter"]),
        );
        m.insert(
            "walnuts",
            IngredientDensity::liquid(0.46).with_aliases(&["walnut"]),
        );
        m.insert(
            "chia seeds",
            IngredientDensity::liquid(0.65).with_aliases(&["chia"]),
        );
        m.insert(
            "flax seeds",
            IngredientDensity::liquid(0.54).with_aliases(&["flaxseed", "ground flax"]),
        );

        // === Sweeteners ===
        m.insert(
            "sugar",
            IngredientDensity::liquid(0.85).with_aliases(&[
                "white sugar",
                "granulated sugar",
                "cane sugar",
            ]),
        );
        m.insert(
            "brown sugar",
            IngredientDensity::liquid(0.93).with_aliases(&["packed brown sugar"]),
        );
        m.insert(
            "honey",
            IngredientDensity::liquid(1.42).with_aliases(&["raw honey"]),
        );
        m.insert(
            "maple syrup",
            IngredientDensity::liquid(1.33).with_aliases(&["pure maple syrup"]),
        );

        // === Liquids ===
        m.insert(
            "water",
            IngredientDensity::liquid(1.0).with_aliases(&["cold water", "warm water"]),
        );
        m.insert(
            "broth",
            IngredientDensity::liquid(1.0).with_aliases(&["stock", "chicken broth", "beef broth"]),
        );
        m.insert(
            "soy sauce",
            IngredientDensity::liquid(1.15).with_aliases(&["shoyu", "tamari"]),
        );

        // === Legumes ===
        m.insert(
            "black beans",
            IngredientDensity::liquid(0.72).with_aliases(&["canned black beans"]),
        );
        m.insert(
            "chickpeas",
            IngredientDensity::liquid(0.72).with_aliases(&["garbanzo beans", "canned chickpeas"]),
        );
        m.insert(
            "lentils",
            IngredientDensity::liquid(0.77).with_aliases(&["dried lentils", "red lentils"]),
        );

        // === Protein Powders ===
        m.insert(
            "whey protein",
            IngredientDensity::liquid(0.42).with_aliases(&[
                "protein powder",
                "whey",
                "whey protein powder",
            ]),
        );
        m.insert(
            "casein protein",
            IngredientDensity::liquid(0.40).with_aliases(&["casein", "casein powder"]),
        );

        m
    });

/// Look up density for an ingredient by name
///
/// Performs case-insensitive matching and checks aliases.
fn lookup_density(ingredient_name: &str) -> Option<&'static IngredientDensity> {
    let normalized = ingredient_name.to_lowercase();

    // Direct match
    if let Some(density) = INGREDIENT_DENSITIES.get(normalized.as_str()) {
        return Some(density);
    }

    // Check aliases
    for (_, density) in INGREDIENT_DENSITIES.iter() {
        for alias in density.aliases {
            if alias.eq_ignore_ascii_case(&normalized) {
                return Some(density);
            }
        }
    }

    // Partial match (ingredient name contains a known ingredient)
    for (key, density) in INGREDIENT_DENSITIES.iter() {
        if normalized.contains(key) || key.contains(normalized.as_str()) {
            return Some(density);
        }
    }

    None
}

/// Convert an ingredient amount to grams
///
/// # Arguments
/// * `ingredient_name` - Name of the ingredient (used for density lookup)
/// * `amount` - Amount in the specified unit
/// * `unit` - The unit of measurement
///
/// # Returns
/// * `Ok(grams)` - The equivalent weight in grams
/// * `Err(ConversionError)` - If conversion is not possible
///
/// # Errors
///
/// Returns `ConversionError::InvalidAmount` if amount is negative.
/// Returns `ConversionError::DensityNotFound` if ingredient density is unknown (for volume/count units).
/// Returns `ConversionError::UnsupportedUnit` if the unit is incompatible with the ingredient.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::intelligence::recipes::conversion::{convert_to_grams, ConversionError};
/// use pierre_mcp_server::intelligence::recipes::IngredientUnit;
///
/// // Convert 2 cups of rice to grams
/// let grams = convert_to_grams("rice", 2.0, IngredientUnit::Cups);
/// assert!(grams.is_ok());
///
/// // Weight units don't need density lookup
/// let grams = convert_to_grams("anything", 100.0, IngredientUnit::Grams);
/// assert_eq!(grams, Ok(100.0));
/// ```
pub fn convert_to_grams(
    ingredient_name: &str,
    amount: f64,
    unit: IngredientUnit,
) -> Result<f64, ConversionError> {
    if amount < 0.0 {
        return Err(ConversionError::InvalidAmount);
    }

    // Weight units don't need density lookup
    match unit {
        IngredientUnit::Grams => return Ok(amount),
        IngredientUnit::Kilograms => return Ok(amount * GRAMS_PER_KG),
        IngredientUnit::Ounces => return Ok(amount * GRAMS_PER_OZ),
        IngredientUnit::Pounds => return Ok(amount * GRAMS_PER_LB),
        _ => {} // Continue to density lookup
    }

    // Look up density for volume/count conversions
    let density = lookup_density(ingredient_name)
        .ok_or_else(|| ConversionError::DensityNotFound(ingredient_name.to_owned()))?;

    match unit {
        IngredientUnit::Pieces => {
            let grams_per_piece = density
                .grams_per_piece
                .ok_or(ConversionError::UnsupportedUnit(unit))?;
            Ok(amount * grams_per_piece)
        }
        IngredientUnit::Milliliters => {
            let grams_per_ml = density
                .grams_per_ml
                .ok_or(ConversionError::UnsupportedUnit(unit))?;
            Ok(amount * grams_per_ml)
        }
        IngredientUnit::Cups => {
            let grams_per_ml = density
                .grams_per_ml
                .ok_or(ConversionError::UnsupportedUnit(unit))?;
            Ok(amount * ML_PER_CUP * grams_per_ml)
        }
        IngredientUnit::Tablespoons => {
            let grams_per_ml = density
                .grams_per_ml
                .ok_or(ConversionError::UnsupportedUnit(unit))?;
            Ok(amount * ML_PER_TBSP * grams_per_ml)
        }
        IngredientUnit::Teaspoons => {
            let grams_per_ml = density
                .grams_per_ml
                .ok_or(ConversionError::UnsupportedUnit(unit))?;
            Ok(amount * ML_PER_TSP * grams_per_ml)
        }
        // Already handled above
        IngredientUnit::Grams
        | IngredientUnit::Kilograms
        | IngredientUnit::Ounces
        | IngredientUnit::Pounds => unreachable!(),
    }
}

/// Check if an ingredient has a known density
#[must_use]
pub fn has_density(ingredient_name: &str) -> bool {
    lookup_density(ingredient_name).is_some()
}

/// Get all known ingredient names
#[must_use]
pub fn known_ingredients() -> Vec<&'static str> {
    INGREDIENT_DENSITIES.keys().copied().collect()
}
