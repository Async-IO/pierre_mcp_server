-- ABOUTME: User and tenant LLM API key credentials schema
-- ABOUTME: Stores encrypted API keys for Gemini, Groq, and other LLM providers with user/tenant isolation
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- User LLM Credentials Table
-- Stores per-user LLM provider API keys with encrypted secrets
-- Resolution order: user_id specific → tenant-level (user_id NULL) → environment fallback
CREATE TABLE IF NOT EXISTS user_llm_credentials (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,  -- NULL = tenant-level default
    provider TEXT NOT NULL CHECK (provider IN ('gemini', 'groq', 'openai', 'anthropic', 'local')),
    api_key_encrypted TEXT NOT NULL,  -- AES-256-GCM encrypted with AAD
    base_url TEXT,  -- For local/custom providers only
    default_model TEXT,  -- Optional model override
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(id),
    -- Unique constraint: one credential per provider per user (or tenant default)
    UNIQUE(tenant_id, user_id, provider)
);

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_user_llm_credentials_tenant ON user_llm_credentials(tenant_id);
CREATE INDEX IF NOT EXISTS idx_user_llm_credentials_user ON user_llm_credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_user_llm_credentials_provider ON user_llm_credentials(provider);
CREATE INDEX IF NOT EXISTS idx_user_llm_credentials_lookup ON user_llm_credentials(tenant_id, user_id, provider);
CREATE INDEX IF NOT EXISTS idx_user_llm_credentials_tenant_default ON user_llm_credentials(tenant_id, provider) WHERE user_id IS NULL;

-- Audit table for LLM credential changes (immutable log)
CREATE TABLE IF NOT EXISTS user_llm_credentials_audit (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('create', 'update', 'delete', 'rotate')),
    credential_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    user_id TEXT,
    provider TEXT NOT NULL,
    changed_by TEXT NOT NULL REFERENCES users(id),
    ip_address TEXT,
    user_agent TEXT,
    reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_user_llm_audit_timestamp ON user_llm_credentials_audit(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_user_llm_audit_tenant ON user_llm_credentials_audit(tenant_id);
CREATE INDEX IF NOT EXISTS idx_user_llm_audit_user ON user_llm_credentials_audit(user_id);
