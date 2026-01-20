// ABOUTME: Module containing all MCP tool implementations organized by category.
// ABOUTME: Each submodule corresponds to a tool category with feature flag support.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Tool Implementations
//!
//! This module contains all MCP tool implementations, organized by category:
//!
//! - `connection` - Provider connection management (connect, disconnect, status)
//! - `data` - Data access tools (activities, athlete, stats)
//! - `analytics` - Analysis tools (trends, patterns, metrics)
//! - `goals` - Goal management tools
//! - `fitness_config` - Fitness configuration tools
//! - `nutrition` - Nutrition and meal planning tools
//! - `sleep` - Sleep and recovery tools
//! - `recipes` - Recipe management tools
//! - `coaches` - AI coach management tools
//! - `admin` - Admin-only tools
//! - `configuration` - User configuration tools
//! - `mobility` - Stretching exercises, yoga poses, mobility recommendations
//!
//! Each category is conditionally compiled based on feature flags,
//! allowing for reduced binary size in deployments that don't need all tools.

// Connection tools: connect_provider, get_connection_status, disconnect_provider
#[cfg(feature = "tools-connection")]
pub mod connection;

// Data tools: get_activities, get_athlete, get_stats, get_activity_intelligence
#[cfg(feature = "tools-data")]
pub mod data;

// Analytics tools: analyze_activity, calculate_metrics, analyze_performance_trends, etc.
#[cfg(feature = "tools-analytics")]
pub mod analytics;

// Goals tools: set_goal, track_progress, suggest_goals, analyze_goal_feasibility
#[cfg(feature = "tools-goals")]
pub mod goals;

// Config tools: get_fitness_config, set_fitness_config, list_fitness_configs, etc.
#[cfg(feature = "tools-config")]
pub mod fitness_config;

// Nutrition tools: calculate_daily_nutrition, get_nutrient_timing, search_food, etc.
#[cfg(feature = "tools-nutrition")]
pub mod nutrition;

// Sleep tools: analyze_sleep_quality, calculate_recovery_score, suggest_rest_day, etc.
#[cfg(feature = "tools-sleep")]
pub mod sleep;

// Recipe tools: get_recipe_constraints, validate_recipe, save_recipe, etc.
#[cfg(feature = "tools-recipes")]
pub mod recipes;

// Coach tools: list_coaches, create_coach, get_coach, update_coach, etc.
#[cfg(feature = "tools-coaches")]
pub mod coaches;

// Admin tools: admin_create_system_coach, admin_list_system_coaches, etc.
#[cfg(feature = "tools-admin")]
pub mod admin;

// Configuration tools: get_configuration_catalog, get_user_configuration, etc.
#[cfg(feature = "tools-config")]
pub mod configuration;

// Mobility tools: stretching exercises, yoga poses, mobility recommendations
#[cfg(feature = "tools-mobility")]
pub mod mobility;
