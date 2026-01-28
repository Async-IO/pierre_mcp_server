-- ABOUTME: Schema for synthetic provider activities stored in database
-- ABOUTME: Allows testing without OAuth by reading seeded activities from DB

-- Synthetic activities table for synthetic provider
-- This table is ONLY used by the synthetic provider, not by real providers (Strava, Garmin, etc.)
CREATE TABLE IF NOT EXISTS synthetic_activities (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,

    -- Core activity fields
    name TEXT NOT NULL,
    sport_type TEXT NOT NULL,
    start_date TEXT NOT NULL,  -- ISO 8601 format
    duration_seconds INTEGER NOT NULL,

    -- Distance and elevation
    distance_meters REAL,
    elevation_gain REAL,

    -- Heart rate
    average_heart_rate INTEGER,
    max_heart_rate INTEGER,

    -- Speed
    average_speed REAL,
    max_speed REAL,

    -- Other metrics
    calories INTEGER,

    -- Location
    city TEXT,
    region TEXT,
    country TEXT,
    start_latitude REAL,
    start_longitude REAL,

    -- Environmental
    temperature REAL,
    humidity REAL,

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Foreign keys
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_synthetic_activities_user_id ON synthetic_activities(user_id);
CREATE INDEX IF NOT EXISTS idx_synthetic_activities_tenant_id ON synthetic_activities(tenant_id);
CREATE INDEX IF NOT EXISTS idx_synthetic_activities_start_date ON synthetic_activities(start_date DESC);
CREATE INDEX IF NOT EXISTS idx_synthetic_activities_sport_type ON synthetic_activities(sport_type);

-- Trigger to update updated_at on modification
CREATE TRIGGER IF NOT EXISTS synthetic_activities_updated_at
    AFTER UPDATE ON synthetic_activities
    FOR EACH ROW
BEGIN
    UPDATE synthetic_activities SET updated_at = datetime('now') WHERE id = NEW.id;
END;
