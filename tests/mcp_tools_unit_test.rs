// ABOUTME: Comprehensive unit tests for all McpTool implementations.
// ABOUTME: Tests tool metadata (name, description, schema, capabilities) for 40 tools.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

//! # MCP Tool Unit Tests
//!
//! This module contains unit tests for all `McpTool` implementations:
//! - Tool metadata tests (name, description, `input_schema`, capabilities)
//! - Parameter validation tests
//! - Factory function tests
//!
//! ## Test Categories (67 tools total)
//!
//! - Coaches (13 tools)
//! - Configuration (6 tools)
//! - Fitness Config (4 tools)
//! - Nutrition (5 tools)
//! - Recipes (7 tools)
//! - Sleep (5 tools)
//! - Data (3 tools)
//! - Analytics (3 tools)
//! - Goals (4 tools)
//! - Connection (3 tools)
//! - Admin (8 tools)
//! - Mobility (6 tools)
//!
//! ## Test Coverage Requirements
//!
//! Each tool should have tests for:
//! 1. `name()` returns correct static string
//! 2. `description()` is non-empty
//! 3. `input_schema()` returns valid JSON schema with correct required fields
//! 4. `capabilities()` returns expected capability flags

use pierre_mcp_server::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// COACHES TOOLS TESTS (13 tools)
// ============================================================================

mod coaches_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::coaches::{
        ActivateCoachTool, CreateCoachTool, DeactivateCoachTool, DeleteCoachTool,
        GetActiveCoachTool, GetCoachTool, HideCoachTool, ListCoachesTool, ListHiddenCoachesTool,
        SearchCoachesTool, ShowCoachTool, ToggleCoachFavoriteTool, UpdateCoachTool,
    };

    #[test]
    fn test_list_coaches_tool_metadata() {
        let tool = ListCoachesTool;
        assert_eq!(tool.name(), "list_coaches");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        assert_eq!(schema.schema_type, "object");

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_coach_tool_metadata() {
        let tool = CreateCoachTool;
        assert_eq!(tool.name(), "create_coach");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"title".to_owned()));
        assert!(required.contains(&"system_prompt".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_get_coach_tool_metadata() {
        let tool = GetCoachTool;
        assert_eq!(tool.name(), "get_coach");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"coach_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_update_coach_tool_metadata() {
        let tool = UpdateCoachTool;
        assert_eq!(tool.name(), "update_coach");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"coach_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_delete_coach_tool_metadata() {
        let tool = DeleteCoachTool;
        assert_eq!(tool.name(), "delete_coach");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"coach_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_toggle_coach_favorite_tool_metadata() {
        let tool = ToggleCoachFavoriteTool;
        assert_eq!(tool.name(), "toggle_coach_favorite");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"coach_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_search_coaches_tool_metadata() {
        let tool = SearchCoachesTool;
        assert_eq!(tool.name(), "search_coaches");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_activate_coach_tool_metadata() {
        let tool = ActivateCoachTool;
        assert_eq!(tool.name(), "activate_coach");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"coach_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_deactivate_coach_tool_metadata() {
        let tool = DeactivateCoachTool;
        assert_eq!(tool.name(), "deactivate_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_get_active_coach_tool_metadata() {
        let tool = GetActiveCoachTool;
        assert_eq!(tool.name(), "get_active_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_hide_coach_tool_metadata() {
        let tool = HideCoachTool;
        assert_eq!(tool.name(), "hide_coach");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"coach_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_show_coach_tool_metadata() {
        let tool = ShowCoachTool;
        assert_eq!(tool.name(), "show_coach");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"coach_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_list_hidden_coaches_tool_metadata() {
        let tool = ListHiddenCoachesTool;
        assert_eq!(tool.name(), "list_hidden_coaches");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_coach_tools_factory() {
        use pierre_mcp_server::tools::implementations::coaches::create_coach_tools;

        let tools = create_coach_tools();
        assert_eq!(tools.len(), 13, "Expected 13 coach tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "list_coaches",
            "create_coach",
            "get_coach",
            "update_coach",
            "delete_coach",
            "toggle_coach_favorite",
            "search_coaches",
            "activate_coach",
            "deactivate_coach",
            "get_active_coach",
            "hide_coach",
            "show_coach",
            "list_hidden_coaches",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_coach_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::coaches::create_coach_tools;

        let tools = create_coach_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// CONFIGURATION TOOLS TESTS (6 tools)
// ============================================================================

mod configuration_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::configuration::{
        CalculatePersonalizedZonesTool, GetConfigurationCatalogTool, GetConfigurationProfilesTool,
        GetUserConfigurationTool, UpdateUserConfigurationTool, ValidateConfigurationTool,
    };

    #[test]
    fn test_get_configuration_catalog_tool_metadata() {
        let tool = GetConfigurationCatalogTool;
        assert_eq!(tool.name(), "get_configuration_catalog");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_get_configuration_profiles_tool_metadata() {
        let tool = GetConfigurationProfilesTool;
        assert_eq!(tool.name(), "get_configuration_profiles");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_get_user_configuration_tool_metadata() {
        let tool = GetUserConfigurationTool;
        assert_eq!(tool.name(), "get_user_configuration");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_update_user_configuration_tool_metadata() {
        let tool = UpdateUserConfigurationTool;
        assert_eq!(tool.name(), "update_user_configuration");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_calculate_personalized_zones_tool_metadata() {
        let tool = CalculatePersonalizedZonesTool;
        assert_eq!(tool.name(), "calculate_personalized_zones");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_validate_configuration_tool_metadata() {
        let tool = ValidateConfigurationTool;
        assert_eq!(tool.name(), "validate_configuration");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_configuration_tools_factory() {
        use pierre_mcp_server::tools::implementations::configuration::create_configuration_tools;

        let tools = create_configuration_tools();
        assert_eq!(tools.len(), 6, "Expected 6 configuration tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "get_configuration_catalog",
            "get_configuration_profiles",
            "get_user_configuration",
            "update_user_configuration",
            "calculate_personalized_zones",
            "validate_configuration",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_configuration_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::configuration::create_configuration_tools;

        let tools = create_configuration_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// FITNESS CONFIG TOOLS TESTS (4 tools)
// ============================================================================

mod fitness_config_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::fitness_config::{
        DeleteFitnessConfigTool, GetFitnessConfigTool, ListFitnessConfigsTool, SetFitnessConfigTool,
    };

    #[test]
    fn test_get_fitness_config_tool_metadata() {
        let tool = GetFitnessConfigTool;
        assert_eq!(tool.name(), "get_fitness_config");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_set_fitness_config_tool_metadata() {
        let tool = SetFitnessConfigTool;
        assert_eq!(tool.name(), "set_fitness_config");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_list_fitness_configs_tool_metadata() {
        let tool = ListFitnessConfigsTool;
        assert_eq!(tool.name(), "list_fitness_configs");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_delete_fitness_config_tool_metadata() {
        let tool = DeleteFitnessConfigTool;
        assert_eq!(tool.name(), "delete_fitness_config");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_create_fitness_config_tools_factory() {
        use pierre_mcp_server::tools::implementations::fitness_config::create_fitness_config_tools;

        let tools = create_fitness_config_tools();
        assert_eq!(tools.len(), 4, "Expected 4 fitness config tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "get_fitness_config",
            "set_fitness_config",
            "list_fitness_configs",
            "delete_fitness_config",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_fitness_config_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::fitness_config::create_fitness_config_tools;

        let tools = create_fitness_config_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// NUTRITION TOOLS TESTS (5 tools)
// ============================================================================

mod nutrition_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::nutrition::{
        AnalyzeMealNutritionTool, CalculateDailyNutritionTool, GetFoodDetailsTool,
        GetNutrientTimingTool, SearchFoodTool,
    };

    #[test]
    fn test_calculate_daily_nutrition_tool_metadata() {
        let tool = CalculateDailyNutritionTool;
        assert_eq!(tool.name(), "calculate_daily_nutrition");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"weight_kg".to_owned()));
        assert!(required.contains(&"height_cm".to_owned()));
        assert!(required.contains(&"age".to_owned()));
        assert!(required.contains(&"gender".to_owned()));
        assert!(required.contains(&"activity_level".to_owned()));
        assert!(required.contains(&"training_goal".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_get_nutrient_timing_tool_metadata() {
        let tool = GetNutrientTimingTool;
        assert_eq!(tool.name(), "get_nutrient_timing");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"workout_intensity".to_owned()));
        assert!(required.contains(&"weight_kg".to_owned()));
        assert!(required.contains(&"daily_protein_g".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_search_food_tool_metadata() {
        let tool = SearchFoodTool;
        assert_eq!(tool.name(), "search_food");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"query".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_get_food_details_tool_metadata() {
        let tool = GetFoodDetailsTool;
        assert_eq!(tool.name(), "get_food_details");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"fdc_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_analyze_meal_nutrition_tool_metadata() {
        let tool = AnalyzeMealNutritionTool;
        assert_eq!(tool.name(), "analyze_meal_nutrition");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"ingredients".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_nutrition_tools_factory() {
        use pierre_mcp_server::tools::implementations::nutrition::create_nutrition_tools;

        let tools = create_nutrition_tools();
        assert_eq!(tools.len(), 5, "Expected 5 nutrition tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "calculate_daily_nutrition",
            "get_nutrient_timing",
            "search_food",
            "get_food_details",
            "analyze_meal_nutrition",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_nutrition_tools_read_only() {
        use pierre_mcp_server::tools::implementations::nutrition::create_nutrition_tools;

        let tools = create_nutrition_tools();
        for tool in &tools {
            assert!(
                !tool.capabilities().contains(ToolCapabilities::WRITES_DATA),
                "Tool {} should not have WRITES_DATA",
                tool.name()
            );
        }
    }
}

// ============================================================================
// RECIPES TOOLS TESTS (7 tools)
// ============================================================================

mod recipes_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::recipes::{
        DeleteRecipeTool, GetRecipeConstraintsTool, GetRecipeTool, ListRecipesTool, SaveRecipeTool,
        SearchRecipesTool, ValidateRecipeTool,
    };

    #[test]
    fn test_get_recipe_constraints_tool_metadata() {
        let tool = GetRecipeConstraintsTool;
        assert_eq!(tool.name(), "get_recipe_constraints");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_validate_recipe_tool_metadata() {
        let tool = ValidateRecipeTool;
        assert_eq!(tool.name(), "validate_recipe");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_save_recipe_tool_metadata() {
        let tool = SaveRecipeTool;
        assert_eq!(tool.name(), "save_recipe");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_list_recipes_tool_metadata() {
        let tool = ListRecipesTool;
        assert_eq!(tool.name(), "list_recipes");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_get_recipe_tool_metadata() {
        let tool = GetRecipeTool;
        assert_eq!(tool.name(), "get_recipe");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"recipe_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_delete_recipe_tool_metadata() {
        let tool = DeleteRecipeTool;
        assert_eq!(tool.name(), "delete_recipe");
        assert!(!tool.description().is_empty());

        let schema = tool.input_schema();
        let required = schema
            .required
            .as_ref()
            .expect("Should have required fields");
        assert!(required.contains(&"recipe_id".to_owned()));

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_search_recipes_tool_metadata() {
        let tool = SearchRecipesTool;
        assert_eq!(tool.name(), "search_recipes");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_recipe_tools_factory() {
        use pierre_mcp_server::tools::implementations::recipes::create_recipe_tools;

        let tools = create_recipe_tools();
        assert_eq!(tools.len(), 7, "Expected 7 recipe tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "get_recipe_constraints",
            "validate_recipe",
            "save_recipe",
            "list_recipes",
            "get_recipe",
            "delete_recipe",
            "search_recipes",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_recipe_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::recipes::create_recipe_tools;

        let tools = create_recipe_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// SLEEP TOOLS TESTS (5 tools)
// ============================================================================

mod sleep_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::sleep::{
        AnalyzeSleepQualityTool, CalculateRecoveryScoreTool, OptimizeSleepScheduleTool,
        SuggestRestDayTool, TrackSleepTrendsTool,
    };

    #[test]
    fn test_analyze_sleep_quality_tool_metadata() {
        let tool = AnalyzeSleepQualityTool;
        assert_eq!(tool.name(), "analyze_sleep_quality");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_calculate_recovery_score_tool_metadata() {
        let tool = CalculateRecoveryScoreTool;
        assert_eq!(tool.name(), "calculate_recovery_score");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_suggest_rest_day_tool_metadata() {
        let tool = SuggestRestDayTool;
        assert_eq!(tool.name(), "suggest_rest_day");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_track_sleep_trends_tool_metadata() {
        let tool = TrackSleepTrendsTool;
        assert_eq!(tool.name(), "track_sleep_trends");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_optimize_sleep_schedule_tool_metadata() {
        let tool = OptimizeSleepScheduleTool;
        assert_eq!(tool.name(), "optimize_sleep_schedule");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_sleep_tools_factory() {
        use pierre_mcp_server::tools::implementations::sleep::create_sleep_tools;

        let tools = create_sleep_tools();
        assert_eq!(tools.len(), 5, "Expected 5 sleep tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "analyze_sleep_quality",
            "calculate_recovery_score",
            "suggest_rest_day",
            "track_sleep_trends",
            "optimize_sleep_schedule",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_sleep_tools_are_read_only() {
        use pierre_mcp_server::tools::implementations::sleep::create_sleep_tools;

        let tools = create_sleep_tools();
        for tool in &tools {
            assert!(
                !tool.capabilities().contains(ToolCapabilities::WRITES_DATA),
                "Tool {} should not have WRITES_DATA (sleep tools analyze, not write)",
                tool.name()
            );
        }
    }

    #[test]
    fn test_all_sleep_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::sleep::create_sleep_tools;

        let tools = create_sleep_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// DATA TOOLS TESTS (3 tools)
// ============================================================================

mod data_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::data::{
        GetActivitiesTool, GetAthleteTool, GetStatsTool,
    };

    #[test]
    fn test_get_activities_tool_metadata() {
        let tool = GetActivitiesTool;
        assert_eq!(tool.name(), "get_activities");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_get_athlete_tool_metadata() {
        let tool = GetAthleteTool;
        assert_eq!(tool.name(), "get_athlete");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_get_stats_tool_metadata() {
        let tool = GetStatsTool;
        assert_eq!(tool.name(), "get_stats");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_data_tools_factory() {
        use pierre_mcp_server::tools::implementations::data::create_data_tools;

        let tools = create_data_tools();
        assert_eq!(tools.len(), 3, "Expected 3 data tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = ["get_activities", "get_athlete", "get_stats"];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_data_tools_are_read_only() {
        use pierre_mcp_server::tools::implementations::data::create_data_tools;

        let tools = create_data_tools();
        for tool in &tools {
            assert!(
                !tool.capabilities().contains(ToolCapabilities::WRITES_DATA),
                "Tool {} should not have WRITES_DATA",
                tool.name()
            );
        }
    }
}

// ============================================================================
// ANALYTICS TOOLS TESTS (3 tools)
// ============================================================================

mod analytics_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::analytics::{
        AnalyzeTrainingLoadTool, CalculateFitnessScoreTool, DetectPatternsTool,
    };

    #[test]
    fn test_analyze_training_load_tool_metadata() {
        let tool = AnalyzeTrainingLoadTool;
        assert_eq!(tool.name(), "analyze_training_load");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_detect_patterns_tool_metadata() {
        let tool = DetectPatternsTool;
        assert_eq!(tool.name(), "detect_patterns");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_calculate_fitness_score_tool_metadata() {
        let tool = CalculateFitnessScoreTool;
        assert_eq!(tool.name(), "calculate_fitness_score");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_analytics_tools_factory() {
        use pierre_mcp_server::tools::implementations::analytics::create_analytics_tools;

        let tools = create_analytics_tools();
        assert_eq!(tools.len(), 3, "Expected 3 analytics tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "analyze_training_load",
            "detect_patterns",
            "calculate_fitness_score",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_analytics_tools_are_read_only() {
        use pierre_mcp_server::tools::implementations::analytics::create_analytics_tools;

        let tools = create_analytics_tools();
        for tool in &tools {
            assert!(
                !tool.capabilities().contains(ToolCapabilities::WRITES_DATA),
                "Tool {} should not have WRITES_DATA (analytics tools analyze, not write)",
                tool.name()
            );
        }
    }
}

// ============================================================================
// GOALS TOOLS TESTS (4 tools)
// ============================================================================

mod goals_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::goals::{
        AnalyzeGoalFeasibilityTool, SetGoalTool, SuggestGoalsTool, TrackProgressTool,
    };

    #[test]
    fn test_set_goal_tool_metadata() {
        let tool = SetGoalTool;
        assert_eq!(tool.name(), "set_goal");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_suggest_goals_tool_metadata() {
        let tool = SuggestGoalsTool;
        assert_eq!(tool.name(), "suggest_goals");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_track_progress_tool_metadata() {
        let tool = TrackProgressTool;
        assert_eq!(tool.name(), "track_progress");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_analyze_goal_feasibility_tool_metadata() {
        let tool = AnalyzeGoalFeasibilityTool;
        assert_eq!(tool.name(), "analyze_goal_feasibility");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_goal_tools_factory() {
        use pierre_mcp_server::tools::implementations::goals::create_goal_tools;

        let tools = create_goal_tools();
        assert_eq!(tools.len(), 4, "Expected 4 goal tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "set_goal",
            "suggest_goals",
            "track_progress",
            "analyze_goal_feasibility",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_goal_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::goals::create_goal_tools;

        let tools = create_goal_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// CONNECTION TOOLS TESTS (3 tools)
// ============================================================================

mod connection_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::connection::{
        ConnectProviderTool, DisconnectProviderTool, GetConnectionStatusTool,
    };

    #[test]
    fn test_connect_provider_tool_metadata() {
        let tool = ConnectProviderTool;
        assert_eq!(tool.name(), "connect_provider");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::REQUIRES_TENANT));
    }

    #[test]
    fn test_get_connection_status_tool_metadata() {
        let tool = GetConnectionStatusTool;
        assert_eq!(tool.name(), "get_connection_status");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_disconnect_provider_tool_metadata() {
        let tool = DisconnectProviderTool;
        assert_eq!(tool.name(), "disconnect_provider");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_create_connection_tools_factory() {
        use pierre_mcp_server::tools::implementations::connection::create_connection_tools;

        let tools = create_connection_tools();
        assert_eq!(tools.len(), 3, "Expected 3 connection tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "connect_provider",
            "get_connection_status",
            "disconnect_provider",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_connection_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::connection::create_connection_tools;

        let tools = create_connection_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// ADMIN TOOLS TESTS (8 tools)
// ============================================================================

mod admin_tests {
    use super::*;
    use pierre_mcp_server::tools::implementations::admin::{
        AdminAssignCoachTool, AdminCreateSystemCoachTool, AdminDeleteSystemCoachTool,
        AdminGetSystemCoachTool, AdminListCoachAssignmentsTool, AdminListSystemCoachesTool,
        AdminUnassignCoachTool, AdminUpdateSystemCoachTool,
    };

    #[test]
    fn test_admin_list_system_coaches_tool_metadata() {
        let tool = AdminListSystemCoachesTool;
        assert_eq!(tool.name(), "admin_list_system_coaches");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_admin_create_system_coach_tool_metadata() {
        let tool = AdminCreateSystemCoachTool;
        assert_eq!(tool.name(), "admin_create_system_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_admin_get_system_coach_tool_metadata() {
        let tool = AdminGetSystemCoachTool;
        assert_eq!(tool.name(), "admin_get_system_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_admin_update_system_coach_tool_metadata() {
        let tool = AdminUpdateSystemCoachTool;
        assert_eq!(tool.name(), "admin_update_system_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_admin_delete_system_coach_tool_metadata() {
        let tool = AdminDeleteSystemCoachTool;
        assert_eq!(tool.name(), "admin_delete_system_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_admin_assign_coach_tool_metadata() {
        let tool = AdminAssignCoachTool;
        assert_eq!(tool.name(), "admin_assign_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_admin_unassign_coach_tool_metadata() {
        let tool = AdminUnassignCoachTool;
        assert_eq!(tool.name(), "admin_unassign_coach");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::WRITES_DATA));
    }

    #[test]
    fn test_admin_list_coach_assignments_tool_metadata() {
        let tool = AdminListCoachAssignmentsTool;
        assert_eq!(tool.name(), "admin_list_coach_assignments");
        assert!(!tool.description().is_empty());

        let caps = tool.capabilities();
        assert!(caps.contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(caps.contains(ToolCapabilities::READS_DATA));
    }

    #[test]
    fn test_create_admin_tools_factory() {
        use pierre_mcp_server::tools::implementations::admin::create_admin_tools;

        let tools = create_admin_tools();
        assert_eq!(tools.len(), 8, "Expected 8 admin tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let expected_names = [
            "admin_list_system_coaches",
            "admin_create_system_coach",
            "admin_get_system_coach",
            "admin_update_system_coach",
            "admin_delete_system_coach",
            "admin_assign_coach",
            "admin_unassign_coach",
            "admin_list_coach_assignments",
        ];

        for expected in expected_names {
            assert!(names.contains(&expected), "Missing: {expected}");
        }
    }

    #[test]
    fn test_all_admin_tools_require_auth() {
        use pierre_mcp_server::tools::implementations::admin::create_admin_tools;

        let tools = create_admin_tools();
        for tool in &tools {
            assert!(
                tool.capabilities()
                    .contains(ToolCapabilities::REQUIRES_AUTH),
                "Tool {} should have REQUIRES_AUTH",
                tool.name()
            );
        }
    }
}

// ============================================================================
// CROSS-CATEGORY TESTS
// ============================================================================

#[test]
fn test_total_tool_count() {
    use pierre_mcp_server::tools::implementations::{
        admin::create_admin_tools, analytics::create_analytics_tools, coaches::create_coach_tools,
        configuration::create_configuration_tools, connection::create_connection_tools,
        data::create_data_tools, fitness_config::create_fitness_config_tools,
        goals::create_goal_tools, mobility::create_mobility_tools,
        nutrition::create_nutrition_tools, recipes::create_recipe_tools, sleep::create_sleep_tools,
    };

    let coaches = create_coach_tools();
    let configuration = create_configuration_tools();
    let fitness_config = create_fitness_config_tools();
    let nutrition = create_nutrition_tools();
    let recipes = create_recipe_tools();
    let sleep = create_sleep_tools();
    let data = create_data_tools();
    let analytics = create_analytics_tools();
    let goals = create_goal_tools();
    let connection = create_connection_tools();
    let admin = create_admin_tools();
    let mobility = create_mobility_tools();

    let total = coaches.len()
        + configuration.len()
        + fitness_config.len()
        + nutrition.len()
        + recipes.len()
        + sleep.len()
        + data.len()
        + analytics.len()
        + goals.len()
        + connection.len()
        + admin.len()
        + mobility.len();

    assert_eq!(total, 67, "Expected 67 tools across all categories");
}

#[test]
fn test_all_tools_have_valid_schemas() {
    use pierre_mcp_server::tools::implementations::{
        admin::create_admin_tools, analytics::create_analytics_tools, coaches::create_coach_tools,
        configuration::create_configuration_tools, connection::create_connection_tools,
        data::create_data_tools, fitness_config::create_fitness_config_tools,
        goals::create_goal_tools, nutrition::create_nutrition_tools, recipes::create_recipe_tools,
        sleep::create_sleep_tools,
    };

    let all_tools: Vec<Box<dyn McpTool>> = create_coach_tools()
        .into_iter()
        .chain(create_configuration_tools())
        .chain(create_fitness_config_tools())
        .chain(create_nutrition_tools())
        .chain(create_recipe_tools())
        .chain(create_sleep_tools())
        .chain(create_data_tools())
        .chain(create_analytics_tools())
        .chain(create_goal_tools())
        .chain(create_connection_tools())
        .chain(create_admin_tools())
        .collect();

    for tool in &all_tools {
        let schema = tool.input_schema();

        // All tools should have object schema
        assert_eq!(
            schema.schema_type,
            "object",
            "Tool {} should have object schema",
            tool.name()
        );

        // If tool has required fields, they should exist in properties
        if let Some(required) = &schema.required {
            if let Some(properties) = &schema.properties {
                for field in required {
                    assert!(
                        properties.contains_key(field),
                        "Tool {} requires field '{}' but it's not in properties",
                        tool.name(),
                        field
                    );
                }
            }
        }
    }
}

#[test]
fn test_all_tool_names_are_unique() {
    use pierre_mcp_server::tools::implementations::{
        admin::create_admin_tools, analytics::create_analytics_tools, coaches::create_coach_tools,
        configuration::create_configuration_tools, connection::create_connection_tools,
        data::create_data_tools, fitness_config::create_fitness_config_tools,
        goals::create_goal_tools, nutrition::create_nutrition_tools, recipes::create_recipe_tools,
        sleep::create_sleep_tools,
    };

    let all_tools: Vec<Box<dyn McpTool>> = create_coach_tools()
        .into_iter()
        .chain(create_configuration_tools())
        .chain(create_fitness_config_tools())
        .chain(create_nutrition_tools())
        .chain(create_recipe_tools())
        .chain(create_sleep_tools())
        .chain(create_data_tools())
        .chain(create_analytics_tools())
        .chain(create_goal_tools())
        .chain(create_connection_tools())
        .chain(create_admin_tools())
        .collect();

    let names: Vec<&str> = all_tools.iter().map(|t| t.name()).collect();
    let mut unique_names = names.clone();
    unique_names.sort_unstable();
    unique_names.dedup();

    assert_eq!(
        names.len(),
        unique_names.len(),
        "All tool names should be unique across categories"
    );
}

#[test]
fn test_all_tools_have_descriptions() {
    use pierre_mcp_server::tools::implementations::{
        admin::create_admin_tools, analytics::create_analytics_tools, coaches::create_coach_tools,
        configuration::create_configuration_tools, connection::create_connection_tools,
        data::create_data_tools, fitness_config::create_fitness_config_tools,
        goals::create_goal_tools, nutrition::create_nutrition_tools, recipes::create_recipe_tools,
        sleep::create_sleep_tools,
    };

    let all_tools: Vec<Box<dyn McpTool>> = create_coach_tools()
        .into_iter()
        .chain(create_configuration_tools())
        .chain(create_fitness_config_tools())
        .chain(create_nutrition_tools())
        .chain(create_recipe_tools())
        .chain(create_sleep_tools())
        .chain(create_data_tools())
        .chain(create_analytics_tools())
        .chain(create_goal_tools())
        .chain(create_connection_tools())
        .chain(create_admin_tools())
        .collect();

    for tool in &all_tools {
        assert!(
            !tool.description().is_empty(),
            "Tool {} should have a description",
            tool.name()
        );
        assert!(
            tool.description().len() >= 10,
            "Tool {} description should be meaningful (>= 10 chars)",
            tool.name()
        );
    }
}
