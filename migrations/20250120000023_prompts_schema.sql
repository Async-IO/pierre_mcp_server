-- ABOUTME: Prompt suggestions schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates tables for storing AI chat prompt suggestions per tenant

-- Prompt Suggestions Table
-- Stores categorized prompt suggestions for the chat interface
CREATE TABLE IF NOT EXISTS prompt_suggestions (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    category_key TEXT NOT NULL,
    category_title TEXT NOT NULL,
    category_icon TEXT NOT NULL,
    pillar TEXT NOT NULL CHECK (pillar IN ('activity', 'nutrition', 'recovery')),
    prompts TEXT NOT NULL,  -- JSON array of prompt strings
    display_order INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(tenant_id, category_key)
);

-- Indexes for Prompt Suggestions
CREATE INDEX IF NOT EXISTS idx_prompt_suggestions_tenant ON prompt_suggestions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_prompt_suggestions_active ON prompt_suggestions(tenant_id, is_active);
CREATE INDEX IF NOT EXISTS idx_prompt_suggestions_order ON prompt_suggestions(tenant_id, display_order);

-- Welcome Prompts Table
-- Stores the welcome/featured prompt shown to first-time connected users
CREATE TABLE IF NOT EXISTS welcome_prompts (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL UNIQUE REFERENCES tenants(id) ON DELETE CASCADE,
    prompt_text TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Index for Welcome Prompts
CREATE INDEX IF NOT EXISTS idx_welcome_prompts_tenant ON welcome_prompts(tenant_id);
