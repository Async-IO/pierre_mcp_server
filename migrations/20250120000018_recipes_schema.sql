-- ABOUTME: Recipe storage schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates tables for user recipes with training-aware nutrition planning

-- Recipes Table
-- Stores recipe metadata and cached nutrition information
CREATE TABLE IF NOT EXISTS recipes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    servings INTEGER NOT NULL DEFAULT 1 CHECK (servings > 0),
    prep_time_mins INTEGER CHECK (prep_time_mins >= 0),
    cook_time_mins INTEGER CHECK (cook_time_mins >= 0),
    instructions TEXT NOT NULL,  -- JSON array of instruction steps
    tags TEXT NOT NULL DEFAULT '[]',  -- JSON array of tags
    meal_timing TEXT NOT NULL DEFAULT 'general' CHECK (meal_timing IN ('pre_training', 'post_training', 'rest_day', 'general')),
    -- Cached nutrition per serving (validated against USDA)
    cached_calories REAL,
    cached_protein_g REAL,
    cached_carbs_g REAL,
    cached_fat_g REAL,
    cached_fiber_g REAL,
    cached_sodium_mg REAL,
    cached_sugar_g REAL,
    nutrition_validated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Recipe Ingredients Table
-- Stores individual ingredients for each recipe
CREATE TABLE IF NOT EXISTS recipe_ingredients (
    id TEXT PRIMARY KEY,
    recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    fdc_id INTEGER,  -- USDA FoodData Central ID (if validated)
    name TEXT NOT NULL,
    amount REAL NOT NULL CHECK (amount > 0),
    unit TEXT NOT NULL DEFAULT 'grams' CHECK (unit IN ('grams', 'milliliters', 'cups', 'tablespoons', 'teaspoons', 'pieces', 'ounces', 'pounds', 'kilograms')),
    grams REAL NOT NULL CHECK (grams >= 0),  -- Normalized weight for nutrition calc
    preparation TEXT,  -- Optional prep notes (diced, minced, etc.)
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- Indexes for Recipes
CREATE INDEX IF NOT EXISTS idx_recipes_user ON recipes(user_id);
CREATE INDEX IF NOT EXISTS idx_recipes_tenant ON recipes(tenant_id);
CREATE INDEX IF NOT EXISTS idx_recipes_user_tenant ON recipes(user_id, tenant_id);
CREATE INDEX IF NOT EXISTS idx_recipes_meal_timing ON recipes(meal_timing);
CREATE INDEX IF NOT EXISTS idx_recipes_updated ON recipes(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_recipes_created ON recipes(created_at DESC);

-- Indexes for Recipe Ingredients
CREATE INDEX IF NOT EXISTS idx_recipe_ingredients_recipe ON recipe_ingredients(recipe_id);
CREATE INDEX IF NOT EXISTS idx_recipe_ingredients_fdc ON recipe_ingredients(fdc_id) WHERE fdc_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_recipe_ingredients_order ON recipe_ingredients(recipe_id, sort_order);
