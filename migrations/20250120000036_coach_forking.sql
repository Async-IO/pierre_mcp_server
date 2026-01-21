-- ABOUTME: Adds forked_from column to coaches for tracking forked system coaches
-- ABOUTME: Supports the coach forking feature where users can customize system coaches
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Coach forking support
-- ============================================================================

-- Add forked_from column to track the origin of forked coaches
-- This is NULL for original coaches, contains the source coach ID for forks
ALTER TABLE coaches ADD COLUMN forked_from TEXT REFERENCES coaches(id) ON DELETE SET NULL;

-- Index for finding all forks of a specific coach
CREATE INDEX IF NOT EXISTS idx_coaches_forked_from ON coaches(forked_from) WHERE forked_from IS NOT NULL;
