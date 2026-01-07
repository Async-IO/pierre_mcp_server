<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 22: Nutrition System & USDA Integration

This chapter covers Pierre's nutrition system including daily calorie/macro calculations, USDA food database integration, meal analysis, and nutrient timing for athletes. You'll learn about energy expenditure estimation, protein requirements, and post-workout nutrition windows.

## What You'll Learn

- Daily calorie and macro calculation
- USDA FoodData Central API integration
- Meal nutrition analysis
- Nutrient timing for performance
- Energy balance for weight management
- Protein requirements by sport
- Carbohydrate periodization
- Hydration recommendations
- Training-aware recipe management (Combat des Chefs)

## Daily Nutrition Calculation

Pierre calculates personalized daily nutrition needs based on training load.

**Total Daily Energy Expenditure (TDEE)**:
```
TDEE = BMR + Activity Calories + Exercise Calories + TEF

Where:
- BMR (Basal Metabolic Rate): Resting energy expenditure
- Activity Calories: Daily lifestyle activity
- Exercise Calories: Planned training
- TEF (Thermic Effect of Food): Digestion cost (~10% of intake)
```

**BMR calculation** (Mifflin-St Jeor equation):
```
Men:   BMR = 10 × weight_kg + 6.25 × height_cm - 5 × age + 5
Women: BMR = 10 × weight_kg + 6.25 × height_cm - 5 × age - 161
```

**Activity multipliers**:
- **Sedentary**: 1.2 (desk job, minimal activity)
- **Lightly active**: 1.375 (light exercise 1-3 days/week)
- **Moderately active**: 1.55 (moderate exercise 3-5 days/week)
- **Very active**: 1.725 (hard exercise 6-7 days/week)
- **Extremely active**: 1.9 (athlete, 2x/day training)

**Exercise calories** (from activity data):
```
Calories = TSS × 1.0 (approximation: 1 TSS ≈ 1 kcal for cycling)

Or use heart rate-based:
Calories = duration_min × (0.6309 × HR + 0.1988 × weight_kg + 0.2017 × age - 55.0969) / 4.184
```

## Macronutrient Targets

Pierre recommends macros based on sport and training phase.

**Protein requirements** (g/kg body weight/day):
- **Endurance athletes**: 1.2-1.6 g/kg
- **Strength athletes**: 1.6-2.2 g/kg
- **Ultra-endurance**: 1.6-2.0 g/kg
- **Recovery day**: 1.2-1.4 g/kg

**Carbohydrate requirements** (g/kg/day):
- **Low intensity**: 3-5 g/kg
- **Moderate training (1hr/day)**: 5-7 g/kg
- **High volume (1-3hr/day)**: 6-10 g/kg
- **Extreme volume (4-5hr/day)**: 8-12 g/kg

**Fat requirements**:
- **Minimum**: 0.8-1.0 g/kg (hormone production, vitamin absorption)
- **Typical**: 20-35% of total calories
- **Low-carb athletes**: Up to 60-70% of calories (fat-adapted)

**Example calculation** (70kg cyclist, moderate training):
```
TDEE: 2800 kcal
Protein: 70kg × 1.4 g/kg = 98g (392 kcal)
Carbs: 70kg × 6 g/kg = 420g (1680 kcal)
Fat: Remaining = (2800 - 392 - 1680) / 9 = 81g (728 kcal)

Macros: 14% protein / 60% carbs / 26% fat
```

## USDA FoodData Central Integration

Pierre integrates with USDA's food database for nutrition data.

**USDA API endpoints**:
- `/foods/search`: Search food database
- `/food/{fdcId}`: Get detailed nutrition data
- `/foods/list`: Browse food categories

**Food search** (conceptual):
```rust
async fn search_food(query: &str) -> Result<Vec<FoodSearchResult>> {
    let url = format!(
        "https://api.nal.usda.gov/fdc/v1/foods/search?query={}&api_key={}",
        query, api_key
    );

    let response: UsdaSearchResponse = client.get(url).send().await?.json().await?;

    Ok(response.foods.into_iter().map(|food| FoodSearchResult {
        fdc_id: food.fdc_id,
        description: food.description,
        brand_name: food.brand_name,
        serving_size: food.serving_size,
        serving_unit: food.serving_unit,
    }).collect())
}
```

**Food nutrition details**:
```json
{
  "fdcId": 171705,
  "description": "Banana, raw",
  "foodNutrients": [
    {
      "nutrientName": "Protein",
      "value": 1.09,
      "unitName": "G"
    },
    {
      "nutrientName": "Total lipid (fat)",
      "value": 0.33,
      "unitName": "G"
    },
    {
      "nutrientName": "Carbohydrate, by difference",
      "value": 22.84,
      "unitName": "G"
    },
    {
      "nutrientName": "Energy",
      "value": 89,
      "unitName": "KCAL"
    }
  ]
}
```

## Meal Nutrition Analysis

Pierre analyzes complete meals from multiple foods.

**Meal analysis input**:
```json
{
  "foods": [
    {"fdc_id": 171705, "servings": 1, "description": "Banana"},
    {"fdc_id": 174608, "servings": 2, "description": "Peanut butter, 2 tbsp"},
    {"fdc_id": 173757, "servings": 2, "description": "Whole wheat bread, 2 slices"}
  ]
}
```

**Meal analysis output**:
```json
{
  "total_calories": 450,
  "total_protein_g": 16,
  "total_carbs_g": 58,
  "total_fat_g": 18,
  "macro_percentages": {
    "protein": 14,
    "carbs": 52,
    "fat": 34
  },
  "micronutrients": {
    "vitamin_b6_mg": 0.8,
    "potassium_mg": 850,
    "fiber_g": 10
  }
}
```

## Nutrient Timing

Pierre provides timing recommendations for optimal performance and recovery.

**Pre-workout nutrition** (1-3 hours before):
- **Carbs**: 1-4 g/kg body weight (fuel glycogen stores)
- **Protein**: 0.15-0.25 g/kg (reduce muscle breakdown)
- **Fat**: Minimal (slows digestion)
- **Example**: Oatmeal (60g) + banana + protein shake

**During workout** (>90 minutes):
- **Carbs**: 30-60 g/hour (maintain blood glucose)
- **Electrolytes**: Sodium 500-700 mg/L (prevent hyponatremia)
- **Fluid**: 400-800 ml/hour (depends on sweat rate)

**Post-workout nutrition** (within 30-60 min):
- **Carbs**: 1.0-1.2 g/kg (replenish glycogen)
- **Protein**: 0.25-0.3 g/kg (muscle protein synthesis)
- **Ratio**: 3:1 to 4:1 carb:protein optimal
- **Example (70kg athlete)**: 70-84g carbs + 18-21g protein

**Anabolic window**:
- **0-2 hours post-exercise**: Glycogen synthesis rate 2-3× higher
- **Protein synthesis**: Elevated 24-48 hours (not just 30min window)
- **Practical**: Eat within 2 hours, total daily intake matters most

### Cross-Provider Intensity Inference

The `get_nutrient_timing` tool supports cross-provider activity data to auto-infer workout intensity:

```json
{
  "tool": "get_nutrient_timing",
  "parameters": {
    "weight_kg": 70,
    "daily_protein_g": 140,
    "activity_provider": "strava",
    "days_back": 7
  }
}
```

**How intensity is inferred**:
- Fetches recent activities from the specified provider
- Analyzes training volume (hours/day) and heart rate patterns
- Returns `intensity_source: "inferred"` in the response

**Inference thresholds**:
| Intensity | Training Volume | Avg Heart Rate |
|-----------|-----------------|----------------|
| High | >2 hours/day | >150 bpm |
| Moderate | 1-2 hours/day | 130-150 bpm |
| Low | <1 hour/day | <130 bpm |

**Fallback behavior**: If activity fetch fails and `workout_intensity` is also provided, falls back to the explicit value. If neither succeeds, returns an error.

## Carbohydrate Periodization

Pierre adjusts carb intake based on training intensity.

**Daily carb adjustment**:
```
Rest day:       3-4 g/kg (maintenance)
Easy day:       4-5 g/kg (light recovery)
Moderate day:   5-7 g/kg (typical training)
Hard day:       7-9 g/kg (high intensity)
Race day:       8-12 g/kg (maximum fueling)
```

**Benefits of periodization**:
1. **Metabolic flexibility**: Trains fat oxidation on low-carb days
2. **Glycogen supercompensation**: Maximizes storage for key workouts
3. **Body composition**: Reduces excess carbs on easy days
4. **Performance**: Fuels hard sessions adequately

**Example week** (70kg cyclist):
```
Monday (rest):      70kg × 3g = 210g carbs
Tuesday (easy):     70kg × 5g = 350g carbs
Wednesday (hard):   70kg × 8g = 560g carbs
Thursday (moderate): 70kg × 6g = 420g carbs
Friday (easy):      70kg × 5g = 350g carbs
Saturday (long):    70kg × 9g = 630g carbs
Sunday (race):      70kg × 10g = 700g carbs
```

## Hydration Recommendations

Pierre calculates sweat rate and hydration needs.

**Sweat rate calculation**:
```
Sweat Rate (L/hr) = (Pre-Weight - Post-Weight + Fluid Consumed - Urine Output) / Duration

Example:
Pre: 70.0 kg
Post: 69.2 kg
Fluid: 0.5 L
Duration: 1 hour
Sweat Rate = (70.0 - 69.2 + 0.5 - 0) / 1 = 1.3 L/hr
```

**Hydration guidelines**:
- **Daily baseline**: 30-35 ml/kg body weight
- **Pre-exercise**: 5-7 ml/kg 2-4 hours before
- **During exercise**: Replace 60-80% of sweat losses
- **Post-exercise**: 150% of fluid deficit (1.5L for each kg lost)

**Electrolyte needs** (sodium):
- **Low sweaters**: 300-500 mg/L
- **Average**: 500-800 mg/L
- **Heavy/salty sweaters**: 800-1200 mg/L

## Recipe Management (Combat des Chefs)

Pierre provides training-aware recipe management using the "Combat des Chefs" architecture:
- **LLM clients generate recipes** (cost-efficient, creative)
- **Pierre validates nutrition via USDA** (accurate, authoritative)
- **Per-user storage** (private recipe collections)

### Meal Timing & Macro Targets

Recipes are categorized by training timing with specific macro distributions:

| Meal Timing | Protein | Carbs | Fat | Use Case |
|-------------|---------|-------|-----|----------|
| `pre_training` | 20% | 55% | 25% | 1-3 hours before workout |
| `post_training` | 30% | 45% | 25% | Within 60 min after workout |
| `rest_day` | 30% | 35% | 35% | Recovery days, lower carb |
| `general` | 25% | 45% | 30% | Balanced everyday meals |

### Recipe Workflow

**Step 1: Get Constraints**
```json
{
  "tool": "get_recipe_constraints",
  "parameters": {
    "meal_timing": "post_training",
    "target_calories": 600
  }
}
```

Returns macro targets, guidelines, and example ingredients for the LLM to use.

**Step 2: LLM Generates Recipe**

The AI assistant creates a recipe based on constraints and user preferences.

**Step 3: Validate with Pierre**
```json
{
  "tool": "validate_recipe",
  "parameters": {
    "name": "Recovery Protein Bowl",
    "meal_timing": "post_training",
    "target_calories": 600,
    "ingredients": [
      {"name": "chicken breast", "quantity": 200, "unit": "grams"},
      {"name": "brown rice", "quantity": 1, "unit": "cup"},
      {"name": "broccoli", "quantity": 150, "unit": "grams"}
    ]
  }
}
```

Pierre validates nutrition via USDA and returns:
- Actual calories and macros
- Compliance score vs targets
- Suggestions for improvements

**Step 4: Save if Valid**
```json
{
  "tool": "save_recipe",
  "parameters": {
    "name": "Recovery Protein Bowl",
    "meal_timing": "post_training",
    "ingredients": [...],
    "instructions": ["Cook rice", "Grill chicken", "Steam broccoli", "Combine and serve"],
    "tags": ["high-protein", "post-workout", "quick"]
  }
}
```

### Unit Conversion

Pierre automatically converts common units to grams for accurate nutrition lookup:

| Category | Units | Example Conversion |
|----------|-------|-------------------|
| Weight | oz, lb, kg | 1 oz → 28.35g |
| Volume | cups, tbsp, tsp, ml | 1 cup → ~240g (varies by ingredient) |
| Count | pieces, whole | 1 banana → ~118g |

### Recipe Tools Summary

| Tool | Purpose |
|------|---------|
| `get_recipe_constraints` | Get macro targets for meal timing |
| `validate_recipe` | Validate nutrition via USDA |
| `save_recipe` | Store in user's collection |
| `list_recipes` | Browse saved recipes |
| `get_recipe` | Retrieve specific recipe |
| `delete_recipe` | Remove from collection |
| `search_recipes` | Find by name/ingredients/tags |

## Key Takeaways

1. **TDEE calculation**: BMR + activity + exercise + TEF determines daily calorie needs.

2. **Protein**: 1.2-2.2 g/kg depending on sport and training phase.

3. **Carbs**: 3-12 g/kg based on training volume and intensity.

4. **USDA integration**: 800,000+ foods with detailed nutrition data via FoodData Central API.

5. **Meal analysis**: Sum nutrition from multiple foods for complete meal breakdown.

6. **Nutrient timing**: Pre (1-3hr), during (>90min), post (30-60min) windows optimize performance.

7. **Carb periodization**: Match carb intake to training intensity for metabolic flexibility.

8. **Sweat rate**: Measure weight before/after to calculate individual fluid needs.

9. **Post-workout ratio**: 3:1 to 4:1 carb:protein ratio optimizes recovery.

10. **Total daily intake**: 24-hour totals matter more than strict timing windows.

11. **Combat des Chefs**: LLM generates recipes, Pierre validates via USDA for accuracy.

12. **Meal timing macros**: Pre-training (high carb), post-training (high protein), rest day (balanced).

---

**End of Part VI: Tools & Intelligence**

You've completed the tools and intelligence section. You now understand:
- All 47 MCP tools and their usage (Chapter 19)
- Sports science algorithms (Chapter 20)
- Recovery and sleep analysis (Chapter 21)
- Nutrition system, USDA integration, and recipe management (Chapter 22)

**Next Chapter**: [Chapter 23: Testing Framework](./chapter-23-testing.md) - Begin Part VII by learning about Pierre's testing infrastructure including synthetic data generation, E2E tests, tools-to-types validation, and test organization.
