-- ABOUTME: System prompts schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates table for storing the LLM system prompt per tenant

-- System Prompts Table
-- Stores the system prompt (instructions) for the LLM assistant per tenant
-- This allows tenants to customize the AI assistant's behavior and communication style
CREATE TABLE IF NOT EXISTS system_prompts (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL UNIQUE REFERENCES tenants(id) ON DELETE CASCADE,
    prompt_text TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Index for System Prompts
CREATE INDEX IF NOT EXISTS idx_system_prompts_tenant ON system_prompts(tenant_id);
