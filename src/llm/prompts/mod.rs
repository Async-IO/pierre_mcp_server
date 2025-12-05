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

/// Get the system prompt for the Pierre fitness assistant
///
/// This is the default system prompt used when starting a new conversation.
/// It includes tool definitions that match the MCP server's capabilities.
#[must_use]
pub const fn get_pierre_system_prompt() -> &'static str {
    PIERRE_SYSTEM_PROMPT
}
