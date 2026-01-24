-- ABOUTME: Database schema for coach-mediated social features.
-- ABOUTME: Creates friend_connections, user_social_settings, shared_insights, insight_reactions, adapted_insights tables.
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- ============================================================================
-- Friend Connections table: bidirectional friend relationships
-- ============================================================================
-- Design: Cross-tenant friendships where both users must accept.
-- The initiator_id is the user who sent the request, receiver_id accepts/declines.

CREATE TABLE IF NOT EXISTS friend_connections (
    id TEXT PRIMARY KEY,

    -- The user who initiated the friend request
    initiator_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- The user who receives the friend request
    receiver_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Connection status: pending, accepted, declined, blocked
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'declined', 'blocked')),

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    accepted_at TEXT,

    -- Prevent duplicate connections (in either direction checked by application logic)
    UNIQUE(initiator_id, receiver_id),

    -- Prevent self-friendship
    CHECK (initiator_id != receiver_id)
);

-- Index for finding all friends of a user (both directions)
CREATE INDEX IF NOT EXISTS idx_friend_connections_initiator
    ON friend_connections(initiator_id, status);

CREATE INDEX IF NOT EXISTS idx_friend_connections_receiver
    ON friend_connections(receiver_id, status);

-- Index for pending requests (for notifications)
CREATE INDEX IF NOT EXISTS idx_friend_connections_pending
    ON friend_connections(receiver_id, created_at DESC)
    WHERE status = 'pending';

-- Index for accepted friends (most common query)
CREATE INDEX IF NOT EXISTS idx_friend_connections_accepted
    ON friend_connections(initiator_id)
    WHERE status = 'accepted';

-- ============================================================================
-- User Social Settings table: privacy and sharing preferences
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_social_settings (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    -- Discovery settings
    -- Whether user can be found in friend search
    discoverable INTEGER NOT NULL DEFAULT 1,

    -- Default visibility for new shared insights: friends_only, public
    default_visibility TEXT NOT NULL DEFAULT 'friends_only' CHECK (default_visibility IN ('friends_only', 'public')),

    -- What activity types to auto-suggest sharing
    -- Stored as JSON array: ["run", "ride", "swim", "strength"]
    share_activity_types TEXT NOT NULL DEFAULT '["run", "ride", "swim"]',

    -- Notification preferences
    notify_friend_requests INTEGER NOT NULL DEFAULT 1,
    notify_insight_reactions INTEGER NOT NULL DEFAULT 1,
    notify_adapted_insights INTEGER NOT NULL DEFAULT 1,

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ============================================================================
-- Shared Insights table: coach insights users share with friends
-- ============================================================================
-- The coach mediates sharing - users share coach-generated insights, not raw data.
-- Privacy-first: no GPS, routes, exact paces. Only relative/contextual info.

CREATE TABLE IF NOT EXISTS shared_insights (
    id TEXT PRIMARY KEY,

    -- The user who shared this insight
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Visibility: friends_only or public
    visibility TEXT NOT NULL DEFAULT 'friends_only' CHECK (visibility IN ('friends_only', 'public')),

    -- Insight classification
    -- Types: achievement, milestone, training_tip, recovery, motivation
    insight_type TEXT NOT NULL CHECK (insight_type IN ('achievement', 'milestone', 'training_tip', 'recovery', 'motivation')),

    -- Sport context (optional): run, ride, swim, strength, etc.
    sport_type TEXT,

    -- The shareable (sanitized) insight content - no private data
    -- This is the coach-generated summary safe for sharing
    content TEXT NOT NULL,

    -- Optional title for the insight
    title TEXT,

    -- Training phase context (optional): base, build, peak, recovery
    training_phase TEXT,

    -- Denormalized reaction counts for performance
    reaction_count INTEGER NOT NULL DEFAULT 0,
    adapt_count INTEGER NOT NULL DEFAULT 0,

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    -- Optional expiry (for time-sensitive insights)
    expires_at TEXT
);

-- Index for user's shared insights
CREATE INDEX IF NOT EXISTS idx_shared_insights_user
    ON shared_insights(user_id, created_at DESC);

-- Index for feed queries (friends_only visibility)
CREATE INDEX IF NOT EXISTS idx_shared_insights_friends_feed
    ON shared_insights(visibility, created_at DESC)
    WHERE visibility = 'friends_only';

-- Index for public feed
CREATE INDEX IF NOT EXISTS idx_shared_insights_public_feed
    ON shared_insights(created_at DESC)
    WHERE visibility = 'public';

-- Index for filtering by insight type
CREATE INDEX IF NOT EXISTS idx_shared_insights_type
    ON shared_insights(insight_type, created_at DESC);

-- Index for filtering by sport
CREATE INDEX IF NOT EXISTS idx_shared_insights_sport
    ON shared_insights(sport_type, created_at DESC)
    WHERE sport_type IS NOT NULL;

-- ============================================================================
-- Insight Reactions table: reactions to shared insights
-- ============================================================================

CREATE TABLE IF NOT EXISTS insight_reactions (
    id TEXT PRIMARY KEY,

    -- The shared insight being reacted to
    insight_id TEXT NOT NULL REFERENCES shared_insights(id) ON DELETE CASCADE,

    -- The user who reacted
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Reaction type: like, celebrate, inspire, support
    reaction_type TEXT NOT NULL CHECK (reaction_type IN ('like', 'celebrate', 'inspire', 'support')),

    -- Timestamp
    created_at TEXT NOT NULL,

    -- One reaction per user per insight
    UNIQUE(insight_id, user_id)
);

-- Index for counting reactions on an insight
CREATE INDEX IF NOT EXISTS idx_insight_reactions_insight
    ON insight_reactions(insight_id, reaction_type);

-- Index for user's reactions (for feed highlighting)
CREATE INDEX IF NOT EXISTS idx_insight_reactions_user
    ON insight_reactions(user_id, created_at DESC);

-- ============================================================================
-- Adapted Insights table: "Adapt to My Training" personalized insights
-- ============================================================================
-- When a user taps "Adapt to My Training" on a friend's insight,
-- Pierre generates a personalized version for their training context.

CREATE TABLE IF NOT EXISTS adapted_insights (
    id TEXT PRIMARY KEY,

    -- The original shared insight this was adapted from
    source_insight_id TEXT NOT NULL REFERENCES shared_insights(id) ON DELETE CASCADE,

    -- The user who requested the adaptation
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- The personalized content generated by the coach
    adapted_content TEXT NOT NULL,

    -- Context used for adaptation (stored as JSON)
    -- Contains: user's training phase, fitness level, recent activities (sanitized)
    adaptation_context TEXT,

    -- Whether the user found this adaptation helpful
    was_helpful INTEGER,

    -- Timestamps
    created_at TEXT NOT NULL,

    -- One adaptation per user per source insight
    UNIQUE(source_insight_id, user_id)
);

-- Index for user's adapted insights
CREATE INDEX IF NOT EXISTS idx_adapted_insights_user
    ON adapted_insights(user_id, created_at DESC);

-- Index for tracking adaptations of a shared insight
CREATE INDEX IF NOT EXISTS idx_adapted_insights_source
    ON adapted_insights(source_insight_id);

-- ============================================================================
-- Triggers for maintaining denormalized counts
-- ============================================================================

-- Trigger to update reaction_count on shared_insights when reactions change
CREATE TRIGGER IF NOT EXISTS trg_insight_reactions_insert
AFTER INSERT ON insight_reactions
BEGIN
    UPDATE shared_insights
    SET reaction_count = reaction_count + 1,
        updated_at = datetime('now')
    WHERE id = NEW.insight_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_insight_reactions_delete
AFTER DELETE ON insight_reactions
BEGIN
    UPDATE shared_insights
    SET reaction_count = reaction_count - 1,
        updated_at = datetime('now')
    WHERE id = OLD.insight_id;
END;

-- Trigger to update adapt_count on shared_insights when adaptations are created
CREATE TRIGGER IF NOT EXISTS trg_adapted_insights_insert
AFTER INSERT ON adapted_insights
BEGIN
    UPDATE shared_insights
    SET adapt_count = adapt_count + 1,
        updated_at = datetime('now')
    WHERE id = NEW.source_insight_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_adapted_insights_delete
AFTER DELETE ON adapted_insights
BEGIN
    UPDATE shared_insights
    SET adapt_count = adapt_count - 1,
        updated_at = datetime('now')
    WHERE id = OLD.source_insight_id;
END;
