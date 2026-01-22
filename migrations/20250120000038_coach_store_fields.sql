-- ABOUTME: Adds publishing and store fields to coaches table for Coach Store feature.
-- ABOUTME: Enables coach submission, admin review, and store discovery.
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Coach Store publishing fields
-- ============================================================================

-- Publishing status: draft → pending_review → published/rejected
-- Default 'draft' means coaches start private and must be submitted for review
ALTER TABLE coaches ADD COLUMN publish_status TEXT DEFAULT 'draft';

-- When the coach was published to the store (approved by admin)
ALTER TABLE coaches ADD COLUMN published_at TEXT;

-- When the coach was submitted for review
ALTER TABLE coaches ADD COLUMN review_submitted_at TEXT;

-- When the admin made a decision (approve/reject)
ALTER TABLE coaches ADD COLUMN review_decision_at TEXT;

-- Which admin made the review decision
ALTER TABLE coaches ADD COLUMN review_decision_by TEXT REFERENCES users(id) ON DELETE SET NULL;

-- Reason provided when coach is rejected (helps author improve)
ALTER TABLE coaches ADD COLUMN rejection_reason TEXT;

-- ============================================================================
-- Coach Store metrics and display
-- ============================================================================

-- Number of users who have installed this coach from the store
-- Denormalized for performance in sorting/filtering
ALTER TABLE coaches ADD COLUMN install_count INTEGER DEFAULT 0;

-- URL to coach icon/avatar image for store display
ALTER TABLE coaches ADD COLUMN icon_url TEXT;

-- ============================================================================
-- Indexes for store queries
-- ============================================================================

-- Primary index for browsing published coaches
CREATE INDEX IF NOT EXISTS idx_coaches_publish_status
    ON coaches(publish_status);

-- Fast filter for published coaches only (partial index)
CREATE INDEX IF NOT EXISTS idx_coaches_published
    ON coaches(publish_status, published_at DESC)
    WHERE publish_status = 'published';

-- Admin review queue: oldest pending first
CREATE INDEX IF NOT EXISTS idx_coaches_pending_review
    ON coaches(publish_status, review_submitted_at ASC)
    WHERE publish_status = 'pending_review';

-- Popular coaches sort (published only)
CREATE INDEX IF NOT EXISTS idx_coaches_install_count
    ON coaches(install_count DESC)
    WHERE publish_status = 'published';
