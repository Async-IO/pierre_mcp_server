-- ABOUTME: OAuth notifications schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates table for storing OAuth completion events and MCP notification delivery tracking

-- OAuth Notifications Table
CREATE TABLE IF NOT EXISTS oauth_notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    success INTEGER NOT NULL DEFAULT 1,
    message TEXT NOT NULL,
    expires_at TEXT,
    created_at TEXT NOT NULL,
    read_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

-- Indexes for OAuth Notifications
CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_id
ON oauth_notifications (user_id);

CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_unread
ON oauth_notifications (user_id, read_at);
