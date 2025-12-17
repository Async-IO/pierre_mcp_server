-- ABOUTME: Admin configuration schema for runtime parameter management
-- ABOUTME: Supports system-wide and per-tenant config overrides with full audit logging
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- Admin Configuration Overrides Table
-- Stores runtime configuration overrides that take precedence over environment defaults
-- tenant_id NULL = system-wide override, non-NULL = tenant-specific override
CREATE TABLE IF NOT EXISTS admin_config_overrides (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    config_key TEXT NOT NULL,
    config_value TEXT NOT NULL,
    data_type TEXT NOT NULL CHECK (data_type IN ('float', 'integer', 'boolean', 'string', 'enum')),
    tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    reason TEXT,
    UNIQUE(category, config_key, tenant_id)
);

-- Admin Configuration Audit Log Table
-- Immutable record of all configuration changes for compliance and debugging
CREATE TABLE IF NOT EXISTS admin_config_audit (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    admin_user_id TEXT NOT NULL REFERENCES users(id),
    admin_email TEXT NOT NULL,
    category TEXT NOT NULL,
    config_key TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT NOT NULL,
    data_type TEXT NOT NULL,
    reason TEXT,
    tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
    ip_address TEXT,
    user_agent TEXT
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_admin_config_overrides_tenant ON admin_config_overrides(tenant_id);
CREATE INDEX IF NOT EXISTS idx_admin_config_overrides_category ON admin_config_overrides(category);
CREATE INDEX IF NOT EXISTS idx_admin_config_overrides_key ON admin_config_overrides(config_key);
CREATE INDEX IF NOT EXISTS idx_admin_config_overrides_category_key ON admin_config_overrides(category, config_key);

CREATE INDEX IF NOT EXISTS idx_admin_config_audit_timestamp ON admin_config_audit(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_admin_config_audit_category ON admin_config_audit(category);
CREATE INDEX IF NOT EXISTS idx_admin_config_audit_key ON admin_config_audit(config_key);
CREATE INDEX IF NOT EXISTS idx_admin_config_audit_admin ON admin_config_audit(admin_user_id);
CREATE INDEX IF NOT EXISTS idx_admin_config_audit_tenant ON admin_config_audit(tenant_id);

-- Configuration Categories (reference data for UI organization)
CREATE TABLE IF NOT EXISTS admin_config_categories (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT,
    display_order INTEGER NOT NULL DEFAULT 0,
    icon TEXT,
    is_active INTEGER NOT NULL DEFAULT 1
);

-- Insert default categories
INSERT OR IGNORE INTO admin_config_categories (id, name, display_name, description, display_order, icon) VALUES
    ('cat_rate_limiting', 'rate_limiting', 'Rate Limiting', 'API rate limits and throttling settings', 10, 'gauge'),
    ('cat_feature_flags', 'feature_flags', 'Feature Flags', 'Enable or disable system features', 20, 'toggle'),
    ('cat_heart_rate', 'heart_rate_zones', 'Heart Rate Zones', 'Heart rate zone thresholds based on sports science', 30, 'heart'),
    ('cat_training_zones', 'training_zones', 'Training Zones', 'Training zone configurations for different sports', 40, 'activity'),
    ('cat_effort', 'effort_thresholds', 'Effort Thresholds', 'RPE and effort level thresholds', 50, 'flame'),
    ('cat_recommendation', 'recommendation_engine', 'Recommendations', 'Recommendation engine thresholds and limits', 60, 'lightbulb'),
    ('cat_sleep', 'sleep_recovery', 'Sleep & Recovery', 'Sleep and recovery analysis thresholds', 70, 'moon'),
    ('cat_tsb', 'training_stress', 'Training Stress', 'Training stress balance (TSB) thresholds', 80, 'trending'),
    ('cat_weather', 'weather_analysis', 'Weather Analysis', 'Weather impact analysis thresholds', 90, 'cloud'),
    ('cat_nutrition', 'nutrition', 'Nutrition', 'Nutrition and macronutrient recommendations', 100, 'utensils'),
    ('cat_algorithms', 'algorithms', 'Algorithms', 'Algorithm selection for physiological calculations', 110, 'calculator'),
    ('cat_tokio_runtime', 'tokio_runtime', 'Tokio Runtime', 'Async runtime worker threads and stack settings', 120, 'cpu'),
    ('cat_sqlx_config', 'sqlx_config', 'Database Pool', 'SQLx connection pool configuration', 130, 'database');

-- Index for category ordering
CREATE INDEX IF NOT EXISTS idx_admin_config_categories_order ON admin_config_categories(display_order);
