-- ABOUTME: Database migration to add startup_query column to coaches table.
-- ABOUTME: Enables coaches to automatically fetch context when a conversation starts.
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Add startup_query column for automatic context fetching
-- ============================================================================

-- Query to automatically execute when starting a conversation with this coach.
-- Example: "Fetch my last 25 running activities and analyze my weekly mileage"
ALTER TABLE coaches ADD COLUMN startup_query TEXT;

-- Index for efficient lookup of coaches with startup queries
-- This helps when we need to find if a coach has a startup query by its system_prompt
CREATE INDEX IF NOT EXISTS idx_coaches_startup_query
ON coaches(id) WHERE startup_query IS NOT NULL;
