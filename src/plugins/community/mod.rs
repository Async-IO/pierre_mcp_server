// ABOUTME: Community-contributed plugins for Pierre MCP Server
// ABOUTME: Example plugins demonstrating the plugin system and providing additional functionality
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

pub mod basic_analysis;
pub mod weather_integration;

// Re-export community plugins
pub use basic_analysis::BasicAnalysisPlugin;
pub use weather_integration::WeatherIntegrationPlugin;
