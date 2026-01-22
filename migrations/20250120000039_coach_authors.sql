-- ABOUTME: Database schema for coach authors (creator profiles for Store).
-- ABOUTME: Tracks author info, stats, and verification status.
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Coach Authors table: public profile for coach creators
-- ============================================================================

CREATE TABLE IF NOT EXISTS coach_authors (
    id TEXT PRIMARY KEY,

    -- Reference to the user who is the author
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Tenant isolation
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

    -- Author display info
    display_name TEXT NOT NULL,
    bio TEXT,
    avatar_url TEXT,
    website_url TEXT,

    -- Verification and trust
    is_verified INTEGER NOT NULL DEFAULT 0,
    verified_at TEXT,
    verified_by TEXT REFERENCES users(id) ON DELETE SET NULL,

    -- Denormalized stats for performance
    published_coach_count INTEGER NOT NULL DEFAULT 0,
    total_install_count INTEGER NOT NULL DEFAULT 0,

    -- Metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    -- One author profile per user per tenant
    UNIQUE(user_id, tenant_id)
);

-- Index for looking up author by user
CREATE INDEX IF NOT EXISTS idx_coach_authors_user
    ON coach_authors(user_id, tenant_id);

-- Index for listing verified authors
CREATE INDEX IF NOT EXISTS idx_coach_authors_verified
    ON coach_authors(is_verified, total_install_count DESC)
    WHERE is_verified = 1;

-- Index for popular authors (most installed)
CREATE INDEX IF NOT EXISTS idx_coach_authors_popular
    ON coach_authors(total_install_count DESC)
    WHERE published_coach_count > 0;

-- ============================================================================
-- Add author_id foreign key to coaches table
-- ============================================================================

ALTER TABLE coaches ADD COLUMN author_id TEXT REFERENCES coach_authors(id) ON DELETE SET NULL;

-- Index for finding coaches by author
CREATE INDEX IF NOT EXISTS idx_coaches_author
    ON coaches(author_id)
    WHERE author_id IS NOT NULL;
