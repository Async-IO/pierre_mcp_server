-- ABOUTME: Database schema for Mobility features (stretching exercises and yoga poses)
-- ABOUTME: Supports recovery and flexibility training with activity-aware recommendations

-- ============================================================================
-- Stretching Exercises Table
-- ============================================================================
-- Static reference data for stretching exercises, seeded by admin
-- Used by suggest_stretches_for_activity tool to recommend post-workout stretches

CREATE TABLE IF NOT EXISTS stretching_exercises (
    id TEXT PRIMARY KEY,

    -- Exercise identity
    name TEXT NOT NULL,
    description TEXT NOT NULL,

    -- Categorization
    category TEXT NOT NULL,  -- 'static', 'dynamic', 'pnf', 'ballistic'
    difficulty TEXT NOT NULL DEFAULT 'beginner',  -- 'beginner', 'intermediate', 'advanced'

    -- Muscle targeting (JSON arrays for flexibility)
    primary_muscles TEXT NOT NULL,    -- JSON array: ["quadriceps", "hip_flexors"]
    secondary_muscles TEXT,           -- JSON array: ["glutes", "lower_back"]

    -- Execution details
    duration_seconds INTEGER NOT NULL DEFAULT 30,
    repetitions INTEGER,              -- For dynamic stretches
    sets INTEGER DEFAULT 1,

    -- Activity association for smart recommendations
    recommended_for_activities TEXT,  -- JSON array: ["running", "cycling", "swimming"]
    contraindications TEXT,           -- JSON array: conditions where stretch should be avoided

    -- Instructions
    instructions TEXT NOT NULL,       -- JSON array of step-by-step instructions
    cues TEXT,                        -- JSON array: form cues and tips

    -- Media references (optional)
    image_url TEXT,
    video_url TEXT,

    -- Metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_stretching_category ON stretching_exercises(category);
CREATE INDEX IF NOT EXISTS idx_stretching_difficulty ON stretching_exercises(difficulty);
CREATE INDEX IF NOT EXISTS idx_stretching_name ON stretching_exercises(name);

-- ============================================================================
-- Yoga Poses Table
-- ============================================================================
-- Static reference data for yoga poses, seeded by admin
-- Used by suggest_yoga_sequence tool to create recovery sequences

CREATE TABLE IF NOT EXISTS yoga_poses (
    id TEXT PRIMARY KEY,

    -- Pose identity
    english_name TEXT NOT NULL,
    sanskrit_name TEXT,

    -- Description and benefits
    description TEXT NOT NULL,
    benefits TEXT NOT NULL,           -- JSON array of benefits

    -- Categorization
    category TEXT NOT NULL,           -- 'standing', 'seated', 'supine', 'prone', 'inversion', 'balance', 'twist'
    difficulty TEXT NOT NULL DEFAULT 'beginner',  -- 'beginner', 'intermediate', 'advanced'
    pose_type TEXT NOT NULL,          -- 'stretch', 'strength', 'balance', 'relaxation', 'breathing'

    -- Muscle and body targeting
    primary_muscles TEXT NOT NULL,    -- JSON array: ["hamstrings", "lower_back"]
    secondary_muscles TEXT,           -- JSON array
    chakras TEXT,                     -- JSON array: for traditional yoga context

    -- Execution details
    hold_duration_seconds INTEGER NOT NULL DEFAULT 30,
    breath_guidance TEXT,             -- e.g., "Inhale arms up, exhale fold forward"

    -- Activity association for recovery recommendations
    recommended_for_activities TEXT,  -- JSON array: ["running", "cycling"]
    recommended_for_recovery TEXT,    -- JSON array: ["post_cardio", "rest_day", "morning"]
    contraindications TEXT,           -- JSON array: conditions where pose should be avoided

    -- Instructions
    instructions TEXT NOT NULL,       -- JSON array of step-by-step instructions
    modifications TEXT,               -- JSON array: easier variations
    progressions TEXT,                -- JSON array: harder variations
    cues TEXT,                        -- JSON array: alignment cues

    -- Sequencing hints
    warmup_poses TEXT,                -- JSON array: pose IDs that should precede
    followup_poses TEXT,              -- JSON array: pose IDs that should follow

    -- Media references (optional)
    image_url TEXT,
    video_url TEXT,

    -- Metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_yoga_category ON yoga_poses(category);
CREATE INDEX IF NOT EXISTS idx_yoga_difficulty ON yoga_poses(difficulty);
CREATE INDEX IF NOT EXISTS idx_yoga_pose_type ON yoga_poses(pose_type);
CREATE INDEX IF NOT EXISTS idx_yoga_english_name ON yoga_poses(english_name);
CREATE INDEX IF NOT EXISTS idx_yoga_sanskrit_name ON yoga_poses(sanskrit_name);

-- ============================================================================
-- Activity-Muscle Mapping Table
-- ============================================================================
-- Maps activity types to muscle stress patterns for intelligent recommendations
-- This enables "suggest stretches for your running workout" functionality

CREATE TABLE IF NOT EXISTS activity_muscle_mapping (
    id TEXT PRIMARY KEY,

    -- Activity identification
    activity_type TEXT NOT NULL UNIQUE,  -- 'running', 'cycling', 'swimming', etc.

    -- Muscle stress levels (1-10 scale, stored as JSON)
    -- Enables TSS/intensity-weighted recommendations
    primary_muscles TEXT NOT NULL,       -- JSON: {"quadriceps": 8, "hamstrings": 7, "calves": 9}
    secondary_muscles TEXT,              -- JSON: {"hip_flexors": 5, "lower_back": 4}

    -- Recommended recovery focus
    recommended_stretch_categories TEXT, -- JSON array: ["static", "dynamic"]
    recommended_yoga_categories TEXT,    -- JSON array: ["standing", "supine"]

    -- Metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_activity_muscle_type ON activity_muscle_mapping(activity_type);
