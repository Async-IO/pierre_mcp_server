-- ABOUTME: Database migration for Coach Redesign v2 (ASY-144).
-- ABOUTME: Adds markdown-defined coach support with structured sections and relations.
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Add new columns to coaches table for markdown-defined coaches
-- ============================================================================

-- Unique slug identifier (matches markdown filename without .md)
ALTER TABLE coaches ADD COLUMN slug TEXT;

-- Purpose: one paragraph description (from ## Purpose section)
ALTER TABLE coaches ADD COLUMN purpose TEXT;

-- When to use: scenarios when user should consult this coach (not counted in tokens)
ALTER TABLE coaches ADD COLUMN when_to_use TEXT;

-- Instructions: core AI system prompt (from ## Instructions section)
-- Note: This complements existing system_prompt column for backward compatibility
ALTER TABLE coaches ADD COLUMN instructions TEXT;

-- Example inputs: sample questions users might ask
ALTER TABLE coaches ADD COLUMN example_inputs TEXT;

-- Example outputs: description of response style/format
ALTER TABLE coaches ADD COLUMN example_outputs TEXT;

-- Success criteria: what defines a successful coaching session
ALTER TABLE coaches ADD COLUMN success_criteria TEXT;

-- Prerequisites: JSON object with providers[], min_activities, activity_types[]
ALTER TABLE coaches ADD COLUMN prerequisites TEXT;

-- Source file path (relative to coaches/ directory, e.g., "training/marathon-coach.md")
ALTER TABLE coaches ADD COLUMN source_file TEXT;

-- Content hash for change detection (SHA-256 of markdown file content)
ALTER TABLE coaches ADD COLUMN content_hash TEXT;

-- Index for slug lookups (used by seeder and API)
CREATE UNIQUE INDEX IF NOT EXISTS idx_coaches_slug ON coaches(tenant_id, slug) WHERE slug IS NOT NULL;

-- Index for source file lookups (used by seeder to detect changed files)
CREATE INDEX IF NOT EXISTS idx_coaches_source_file ON coaches(source_file) WHERE source_file IS NOT NULL;

-- ============================================================================
-- Coach relations table: links between coaches with relationship types
-- ============================================================================

CREATE TABLE IF NOT EXISTS coach_relations (
    id TEXT PRIMARY KEY,

    -- Source coach (the one defining the relationship)
    coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,

    -- Target coach (the related coach)
    related_coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,

    -- Relationship type:
    -- 'related': general bidirectional relationship
    -- 'alternative': alternative coach for similar needs (bidirectional)
    -- 'prerequisite': must consult before this coach (directional)
    -- 'sequel': consult after this coach (directional)
    relation_type TEXT NOT NULL CHECK (relation_type IN ('related', 'alternative', 'prerequisite', 'sequel')),

    -- Metadata
    created_at TEXT NOT NULL,

    -- Prevent duplicate relationships
    UNIQUE(coach_id, related_coach_id, relation_type)
);

-- Index for finding related coaches
CREATE INDEX IF NOT EXISTS idx_coach_relations_coach ON coach_relations(coach_id);
CREATE INDEX IF NOT EXISTS idx_coach_relations_related ON coach_relations(related_coach_id);

-- ============================================================================
-- Coach versions table: tracks changes to coach definitions over time
-- ============================================================================

CREATE TABLE IF NOT EXISTS coach_versions (
    id TEXT PRIMARY KEY,

    -- Reference to the coach
    coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,

    -- Version number (incremented on each update)
    version INTEGER NOT NULL,

    -- Content hash at this version
    content_hash TEXT NOT NULL,

    -- Full content snapshot (JSON with all fields)
    content_snapshot TEXT NOT NULL,

    -- Change summary (what was modified)
    change_summary TEXT,

    -- Metadata
    created_at TEXT NOT NULL,
    created_by TEXT REFERENCES users(id) ON DELETE SET NULL,

    -- Unique version per coach
    UNIQUE(coach_id, version)
);

-- Index for version history queries
CREATE INDEX IF NOT EXISTS idx_coach_versions_coach ON coach_versions(coach_id, version DESC);
