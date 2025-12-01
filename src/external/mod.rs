// ABOUTME: External API client modules (USDA FoodData Central)
// ABOUTME: Provides nutritional data integration and caching

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! External API Clients
//!
//! This module contains clients for external APIs used by the pierre MCP server.

/// USDA FoodData Central API client for nutritional data
pub mod usda_client;

/// Re-export commonly used types from USDA client
pub use usda_client::{FoodDetails, FoodNutrient, FoodSearchResult, UsdaClient, UsdaClientConfig};
