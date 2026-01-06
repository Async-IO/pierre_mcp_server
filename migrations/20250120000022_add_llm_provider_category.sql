-- ABOUTME: Add LLM provider configuration category for local LLM support
-- ABOUTME: Enables configuration of Groq, Gemini, and local LLM providers
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- Add LLM provider category for AI model configuration
INSERT OR IGNORE INTO admin_config_categories (id, name, display_name, description, display_order, icon) VALUES
    ('cat_llm_provider', 'llm_provider', 'LLM Provider', 'AI model provider configuration for chat and recipe generation', 95, 'brain');
