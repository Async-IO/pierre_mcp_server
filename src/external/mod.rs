// ABOUTME: External API client modules (USDA FoodData Central)
// ABOUTME: Provides nutritional data integration and caching

// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! External API Clients
//!
//! This module contains clients for external APIs used by the pierre MCP server.

pub mod usda_client;

// Re-export commonly used types
pub use usda_client::{FoodDetails, FoodNutrient, FoodSearchResult, UsdaClient, UsdaClientConfig};
