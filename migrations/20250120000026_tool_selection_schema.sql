-- ABOUTME: Tool selection schema for per-tenant MCP tool configuration
-- ABOUTME: Enables admins to customize which tools are exposed to MCP clients per tenant

-- Tool Catalog Table (master list of all available tools)
CREATE TABLE IF NOT EXISTS tool_catalog (
    id TEXT PRIMARY KEY,
    tool_name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN (
        'fitness', 'analysis', 'goals', 'nutrition',
        'recipes', 'sleep', 'configuration', 'connections'
    )),
    is_enabled_by_default INTEGER NOT NULL DEFAULT 1,
    requires_provider TEXT,  -- NULL or provider name if provider-specific
    min_plan TEXT NOT NULL DEFAULT 'starter' CHECK (min_plan IN ('starter', 'professional', 'enterprise')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Tenant Tool Overrides Table (per-tenant customization)
CREATE TABLE IF NOT EXISTS tenant_tool_overrides (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL REFERENCES tool_catalog(tool_name) ON DELETE CASCADE,
    is_enabled INTEGER NOT NULL,
    enabled_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(tenant_id, tool_name)
);

-- Indexes for Tool Selection Tables
CREATE INDEX IF NOT EXISTS idx_tool_catalog_category ON tool_catalog(category);
CREATE INDEX IF NOT EXISTS idx_tool_catalog_enabled ON tool_catalog(is_enabled_by_default);
CREATE INDEX IF NOT EXISTS idx_tool_catalog_min_plan ON tool_catalog(min_plan);
CREATE INDEX IF NOT EXISTS idx_tenant_tool_overrides_tenant ON tenant_tool_overrides(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_tool_overrides_tool ON tenant_tool_overrides(tool_name);

-- Seed tool_catalog with all ToolId variants
-- Fitness tools (core data access)
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-001', 'get_activities', 'Get Activities', 'Get user fitness activities with optional filtering and limits', 'fitness', 1, NULL, 'starter'),
('tc-002', 'get_athlete', 'Get Athlete Profile', 'Get user athlete profile and basic information', 'fitness', 1, NULL, 'starter'),
('tc-003', 'get_stats', 'Get Statistics', 'Get user performance statistics and metrics', 'fitness', 1, NULL, 'starter'),
('tc-004', 'analyze_activity', 'Analyze Activity', 'Analyze a specific activity with detailed performance insights', 'analysis', 1, NULL, 'starter'),
('tc-005', 'get_activity_intelligence', 'Activity Intelligence', 'Get AI-powered intelligence analysis for an activity', 'analysis', 1, NULL, 'starter');

-- Connection tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-006', 'get_connection_status', 'Connection Status', 'Check OAuth connection status for fitness providers', 'connections', 1, NULL, 'starter'),
('tc-007', 'connect_provider', 'Connect Provider', 'Connect to a fitness data provider via OAuth', 'connections', 1, NULL, 'starter'),
('tc-008', 'disconnect_provider', 'Disconnect Provider', 'Disconnect user from a fitness data provider', 'connections', 1, NULL, 'starter');

-- Goal and planning tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-009', 'set_goal', 'Set Goal', 'Set a new fitness goal for the user', 'goals', 1, NULL, 'starter'),
('tc-010', 'suggest_goals', 'Suggest Goals', 'Get AI-suggested fitness goals based on activity history', 'goals', 1, NULL, 'starter'),
('tc-011', 'analyze_goal_feasibility', 'Goal Feasibility', 'Analyze whether a goal is achievable given current fitness level', 'goals', 1, NULL, 'professional'),
('tc-012', 'track_progress', 'Track Progress', 'Track progress towards fitness goals', 'goals', 1, NULL, 'starter');

-- Analysis and intelligence tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-013', 'calculate_metrics', 'Calculate Metrics', 'Calculate custom fitness metrics and performance indicators', 'analysis', 1, NULL, 'starter'),
('tc-014', 'analyze_performance_trends', 'Performance Trends', 'Analyze performance trends over time', 'analysis', 1, NULL, 'professional'),
('tc-015', 'compare_activities', 'Compare Activities', 'Compare two activities for performance analysis', 'analysis', 1, NULL, 'starter'),
('tc-016', 'detect_patterns', 'Detect Patterns', 'Detect patterns and insights in activity data', 'analysis', 1, NULL, 'professional'),
('tc-017', 'generate_recommendations', 'Generate Recommendations', 'Generate personalized training recommendations', 'analysis', 1, NULL, 'professional'),
('tc-018', 'calculate_fitness_score', 'Fitness Score', 'Calculate overall fitness score based on recent activities', 'analysis', 1, NULL, 'starter'),
('tc-019', 'predict_performance', 'Predict Performance', 'Predict future performance based on training patterns', 'analysis', 1, NULL, 'enterprise'),
('tc-020', 'analyze_training_load', 'Training Load', 'Analyze training load and recovery metrics', 'analysis', 1, NULL, 'professional');

-- Configuration management tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-021', 'get_configuration_catalog', 'Configuration Catalog', 'Get the complete configuration catalog with all available parameters', 'configuration', 1, NULL, 'starter'),
('tc-022', 'get_configuration_profiles', 'Configuration Profiles', 'Get available configuration profiles (Research, Elite, Recreational, etc.)', 'configuration', 1, NULL, 'starter'),
('tc-023', 'get_user_configuration', 'Get User Config', 'Get current user configuration settings and overrides', 'configuration', 1, NULL, 'starter'),
('tc-024', 'update_user_configuration', 'Update User Config', 'Update user configuration parameters and session overrides', 'configuration', 1, NULL, 'starter'),
('tc-025', 'calculate_personalized_zones', 'Personalized Zones', 'Calculate personalized training zones based on user VO2 max', 'configuration', 1, NULL, 'starter'),
('tc-026', 'validate_configuration', 'Validate Config', 'Validate configuration parameters against safety rules', 'configuration', 1, NULL, 'starter');

-- Sleep and recovery analysis tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-027', 'analyze_sleep_quality', 'Sleep Quality', 'Analyze sleep quality from Fitbit/Garmin data using NSF/AASM guidelines', 'sleep', 1, NULL, 'professional'),
('tc-028', 'calculate_recovery_score', 'Recovery Score', 'Calculate holistic recovery score combining TSB, sleep quality, and HRV', 'sleep', 1, NULL, 'professional'),
('tc-029', 'suggest_rest_day', 'Rest Day Suggestion', 'AI-powered rest day recommendation based on recovery indicators', 'sleep', 1, NULL, 'professional'),
('tc-030', 'track_sleep_trends', 'Sleep Trends', 'Track sleep patterns and correlate with performance over time', 'sleep', 1, NULL, 'professional'),
('tc-031', 'optimize_sleep_schedule', 'Sleep Schedule', 'Optimize sleep duration based on training load and recovery needs', 'sleep', 1, NULL, 'enterprise');

-- Fitness configuration management tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-032', 'get_fitness_config', 'Get Fitness Config', 'Get user fitness configuration settings including heart rate zones', 'configuration', 1, NULL, 'starter'),
('tc-033', 'set_fitness_config', 'Set Fitness Config', 'Save user fitness configuration settings for zones and thresholds', 'configuration', 1, NULL, 'starter'),
('tc-034', 'list_fitness_configs', 'List Fitness Configs', 'List all available fitness configuration names for the user', 'configuration', 1, NULL, 'starter'),
('tc-035', 'delete_fitness_config', 'Delete Fitness Config', 'Delete a specific fitness configuration by name', 'configuration', 1, NULL, 'starter');

-- Nutrition analysis and USDA food database tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-036', 'calculate_daily_nutrition', 'Daily Nutrition', 'Calculate daily calorie and macronutrient needs using Mifflin-St Jeor BMR formula', 'nutrition', 1, NULL, 'starter'),
('tc-037', 'get_nutrient_timing', 'Nutrient Timing', 'Get optimal pre/post-workout nutrition recommendations following ISSN guidelines', 'nutrition', 1, NULL, 'professional'),
('tc-038', 'search_food', 'Search Food', 'Search USDA FoodData Central database for foods by name/description', 'nutrition', 1, NULL, 'starter'),
('tc-039', 'get_food_details', 'Food Details', 'Get detailed nutritional information for a specific food from USDA database', 'nutrition', 1, NULL, 'starter'),
('tc-040', 'analyze_meal_nutrition', 'Meal Nutrition', 'Analyze total calories and macronutrients for a meal of multiple foods', 'nutrition', 1, NULL, 'starter');

-- Recipe management tools
INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan) VALUES
('tc-041', 'get_recipe_constraints', 'Recipe Constraints', 'Get macro targets for LLM recipe generation by training phase', 'recipes', 1, NULL, 'starter'),
('tc-042', 'validate_recipe', 'Validate Recipe', 'Validate recipe nutrition against USDA and calculate macros', 'recipes', 1, NULL, 'starter'),
('tc-043', 'save_recipe', 'Save Recipe', 'Save validated recipe with cached nutrition data', 'recipes', 1, NULL, 'starter'),
('tc-044', 'list_recipes', 'List Recipes', 'List saved recipes with optional meal timing filter', 'recipes', 1, NULL, 'starter'),
('tc-045', 'get_recipe', 'Get Recipe', 'Get a specific recipe by ID', 'recipes', 1, NULL, 'starter'),
('tc-046', 'delete_recipe', 'Delete Recipe', 'Delete a recipe from collection', 'recipes', 1, NULL, 'starter'),
('tc-047', 'search_recipes', 'Search Recipes', 'Search recipes by name, tags, or description', 'recipes', 1, NULL, 'starter');
