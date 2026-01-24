-- ABOUTME: Fixes datetime format in social feature triggers to use RFC3339.
-- ABOUTME: Replaces datetime('now') with strftime('%Y-%m-%dT%H:%M:%SZ', 'now') for consistency.
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Fix triggers to use RFC3339 datetime format
-- ============================================================================
-- The original triggers used datetime('now') which produces format like:
-- "2026-01-24 00:22:18"
-- But the Rust code expects RFC3339 format like:
-- "2026-01-24T00:22:18Z"
--
-- This migration drops and recreates the triggers with the correct format.

-- Drop existing triggers
DROP TRIGGER IF EXISTS trg_insight_reactions_insert;
DROP TRIGGER IF EXISTS trg_insight_reactions_delete;
DROP TRIGGER IF EXISTS trg_adapted_insights_insert;
DROP TRIGGER IF EXISTS trg_adapted_insights_delete;

-- Recreate trigger for reaction insert with RFC3339 format
CREATE TRIGGER IF NOT EXISTS trg_insight_reactions_insert
AFTER INSERT ON insight_reactions
BEGIN
    UPDATE shared_insights
    SET reaction_count = reaction_count + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = NEW.insight_id;
END;

-- Recreate trigger for reaction delete with RFC3339 format
CREATE TRIGGER IF NOT EXISTS trg_insight_reactions_delete
AFTER DELETE ON insight_reactions
BEGIN
    UPDATE shared_insights
    SET reaction_count = reaction_count - 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = OLD.insight_id;
END;

-- Recreate trigger for adapted insight insert with RFC3339 format
CREATE TRIGGER IF NOT EXISTS trg_adapted_insights_insert
AFTER INSERT ON adapted_insights
BEGIN
    UPDATE shared_insights
    SET adapt_count = adapt_count + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = NEW.source_insight_id;
END;

-- Recreate trigger for adapted insight delete with RFC3339 format
CREATE TRIGGER IF NOT EXISTS trg_adapted_insights_delete
AFTER DELETE ON adapted_insights
BEGIN
    UPDATE shared_insights
    SET adapt_count = adapt_count - 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = OLD.source_insight_id;
END;
