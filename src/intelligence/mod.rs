// ABOUTME: Intelligence module re-exports from pierre-intelligence crate
// ABOUTME: Preserves all existing import paths while delegating to the extracted crate
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Intelligence Module
//!
//! Advanced analytics and intelligence for fitness data analysis.
//! Provides sophisticated analysis tools for Claude/LLM integration via MCP.
//!
//! This module re-exports from the `pierre-intelligence` crate and includes
//! local modules that depend on main crate features (HTTP, LLM, etc.).

// Re-export all public items from pierre-intelligence
pub use pierre_intelligence::*;

// Re-export submodules for path-based access (e.g., crate::intelligence::algorithms::FtpAlgorithm)
pub use pierre_intelligence::{
    activity_analyzer, algorithms, analysis_config, analyzer, friend_activity_cache, goal_engine,
    insight_adapter, insights, metrics, metrics_extractor, nutrition_calculator, pattern_detection,
    performance_analyzer, performance_analyzer_v2, performance_prediction, physiological_constants,
    recipes, recommendation_engine, recovery_calculator, sleep_analysis, statistical_analysis,
    training_load, visitor,
};

// Local submodules that remain in the main crate (external deps: HTTP, LLM, etc.)

/// Location and geographic context
pub mod location;
/// Weather data integration and analysis
pub mod weather;

/// LLM-powered insight quality validation for social sharing
pub mod insight_validation;
/// Generates shareable, privacy-preserving insights
pub mod social_insights;

// Re-export types from local modules (same as before the extraction)
pub use insight_validation::{
    contains_metrics, detect_metrics, quick_reject_check, redact_content,
    validate_insight_with_policy, validate_insight_with_quick_check, DetectedMetric,
    InsightMetricType, InsightSharingPolicy, InsightValidationResult, RedactionInfo,
    ValidationVerdict,
};
pub use social_insights::{calculate_milestone_relevance, capitalize_first};
pub use social_insights::{InsightContextBuilder, InsightGenerationContext, InsightSuggestion};
pub use social_insights::{PersonalRecord as SocialPersonalRecord, SharedInsightGenerator};
