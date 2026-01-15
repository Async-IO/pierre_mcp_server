-- ABOUTME: Database schema for user-created Coaches feature.
-- ABOUTME: Coaches are custom AI personas with system prompts that shape Pierre's responses.

-- Coaches table: stores user-created custom coaches
CREATE TABLE IF NOT EXISTS coaches (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,

    -- Coach content
    title TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL,

    -- Categorization
    category TEXT NOT NULL DEFAULT 'custom',  -- training/nutrition/recovery/recipes/custom
    tags TEXT,  -- JSON array of tags, e.g., ["running", "marathon"]

    -- Token tracking (transparency for users about context usage)
    token_count INTEGER NOT NULL DEFAULT 0,

    -- User preferences
    is_favorite INTEGER NOT NULL DEFAULT 0,  -- SQLite boolean: 0 = false, 1 = true
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,  -- ISO 8601 timestamp

    -- Metadata
    created_at TEXT NOT NULL,  -- ISO 8601 timestamp
    updated_at TEXT NOT NULL,  -- ISO 8601 timestamp

    -- Foreign keys
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Index for listing coaches by user (most common query)
CREATE INDEX IF NOT EXISTS idx_coaches_user ON coaches(user_id);

-- Index for tenant-level queries (admin operations)
CREATE INDEX IF NOT EXISTS idx_coaches_tenant ON coaches(tenant_id);

-- Index for filtering by category
CREATE INDEX IF NOT EXISTS idx_coaches_category ON coaches(user_id, category);

-- Index for favorites filter (user's favorite coaches)
CREATE INDEX IF NOT EXISTS idx_coaches_favorite ON coaches(user_id, is_favorite) WHERE is_favorite = 1;

-- Index for recently used (sorting by last_used_at)
CREATE INDEX IF NOT EXISTS idx_coaches_recent ON coaches(user_id, last_used_at DESC) WHERE last_used_at IS NOT NULL;
