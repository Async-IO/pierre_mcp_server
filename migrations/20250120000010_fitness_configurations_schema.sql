-- ABOUTME: Fitness configurations schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates table for storing fitness-related configurations per tenant and user

-- Fitness Configurations Table
CREATE TABLE IF NOT EXISTS fitness_configurations (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id TEXT,
    configuration_name TEXT NOT NULL DEFAULT 'default',
    config_data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(tenant_id, user_id, configuration_name)
);

-- Indexes for Fitness Configurations
CREATE INDEX IF NOT EXISTS idx_fitness_configs_tenant ON fitness_configurations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_fitness_configs_user ON fitness_configurations(user_id);
CREATE INDEX IF NOT EXISTS idx_fitness_configs_tenant_user ON fitness_configurations(tenant_id, user_id);
