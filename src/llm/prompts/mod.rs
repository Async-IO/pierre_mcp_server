// ABOUTME: System prompts for LLM interactions loaded at compile time
// ABOUTME: Provides the Pierre fitness assistant system prompt for Gemini function calling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # System Prompts
//!
//! This module provides system prompts for LLM interactions.
//! Prompts are loaded at compile time from markdown files for easy maintenance.

/// Pierre Fitness Intelligence Assistant system prompt
///
/// Contains instructions for the AI assistant including:
/// - Role and communication style
/// - Available MCP tools with parameters
/// - Guidelines for data handling
/// - Example interaction patterns
pub const PIERRE_SYSTEM_PROMPT: &str = include_str!("pierre_system.md");

/// Coach generation system prompt
///
/// Contains instructions for the LLM to analyze a conversation and generate
/// a specialized coach profile with title, description, system prompt, and tags.
pub const COACH_GENERATION_PROMPT: &str = include_str!("coach_generation.md");

/// Insight validation system prompt
///
/// Contains instructions for the LLM to evaluate fitness content quality
/// before sharing to social feed. Returns valid, improved, or rejected verdict.
pub const INSIGHT_VALIDATION_PROMPT: &str = include_str!("insight_validation.md");

/// Insight generation system prompt
///
/// Contains instructions for the LLM to transform a fitness analysis into
/// a shareable social post with hashtags, ready for direct copying.
pub const INSIGHT_GENERATION_PROMPT: &str = include_str!("insight_generation.md");

/// Get the system prompt for the Pierre fitness assistant
///
/// This is the default system prompt used when starting a new conversation.
/// It includes tool definitions that match the MCP server's capabilities.
#[must_use]
pub const fn get_pierre_system_prompt() -> &'static str {
    PIERRE_SYSTEM_PROMPT
}

/// Get the system prompt for coach generation from conversations
///
/// This prompt instructs the LLM to analyze a conversation and generate
/// a structured coach profile in JSON format.
#[must_use]
pub const fn get_coach_generation_prompt() -> &'static str {
    COACH_GENERATION_PROMPT
}

/// Get the system prompt for insight quality validation
///
/// This prompt instructs the LLM to evaluate fitness content quality
/// before sharing to social feed, returning a verdict with optional improvements.
#[must_use]
pub const fn get_insight_validation_prompt() -> &'static str {
    INSIGHT_VALIDATION_PROMPT
}

/// Get the system prompt for insight generation
///
/// This prompt instructs the LLM to transform a fitness analysis into
/// a shareable social post with hashtags, ready for direct copying.
#[must_use]
pub const fn get_insight_generation_prompt() -> &'static str {
    INSIGHT_GENERATION_PROMPT
}
