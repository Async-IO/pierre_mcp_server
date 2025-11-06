// ABOUTME: Community-contributed plugins for Pierre MCP Server
// ABOUTME: Example plugins demonstrating the plugin system and providing additional functionality
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Basic activity analysis plugin
pub mod basic_analysis;
/// Weather integration plugin
pub mod weather_integration;

// Re-export community plugins

/// Basic analysis plugin for activity insights
pub use basic_analysis::BasicAnalysisPlugin;
/// Weather integration plugin for weather data
pub use weather_integration::WeatherIntegrationPlugin;
