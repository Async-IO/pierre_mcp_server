-- ABOUTME: Migration to add insight_sharing_policy field to user_social_settings
-- ABOUTME: Enables per-user configuration of how data is shared in social feed
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Add insight_sharing_policy column to user_social_settings
-- ============================================================================
-- This field controls what level of data detail users allow in their shared insights:
--   - data_rich: Allow all metrics (times, paces, HR, power)
--   - sanitized: Auto-redact specific numbers to ranges
--   - general_only: Only allow insights without specific metrics
--   - disabled: No sharing allowed
--
-- Default is 'data_rich' to preserve existing behavior

ALTER TABLE user_social_settings
ADD COLUMN insight_sharing_policy TEXT NOT NULL DEFAULT 'data_rich'
CHECK (insight_sharing_policy IN ('data_rich', 'sanitized', 'general_only', 'disabled'));
