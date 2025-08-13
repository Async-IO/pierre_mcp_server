-- Migration 007: Tenant Users and Role Management
-- Adds tenant_users table for role-based permissions and proper user-tenant relationships

-- Create tenant_users table for role-based permissions
CREATE TABLE IF NOT EXISTS tenant_users (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'billing', 'member')),
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Ensure unique user-tenant relationships
    UNIQUE(tenant_id, user_id)
);

-- Create indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_tenant_users_tenant ON tenant_users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_users_user ON tenant_users(user_id);
CREATE INDEX IF NOT EXISTS idx_tenant_users_role ON tenant_users(role);

-- Migrate existing user-tenant relationships from users table
-- For users with tenant_id, create tenant_users entries with 'member' role by default
INSERT OR IGNORE INTO tenant_users (tenant_id, user_id, role, joined_at)
SELECT 
    u.tenant_id,
    u.id,
    'member' as role,
    u.created_at as joined_at
FROM users u 
WHERE u.tenant_id IS NOT NULL;

-- For users who don't have a tenant_id but need to be associated with default tenant
-- First ensure default tenant exists
INSERT OR IGNORE INTO tenants (id, name, slug, owner_user_id, created_at) 
SELECT 
    'default-tenant' as id,
    'Default Tenant' as name,
    'default' as slug,
    (SELECT id FROM users WHERE email LIKE '%admin%' OR id = (SELECT MIN(id) FROM users) LIMIT 1) as owner_user_id,
    CURRENT_TIMESTAMP as created_at
WHERE NOT EXISTS (SELECT 1 FROM tenants WHERE id = 'default-tenant');

-- Associate users without tenant_id to default tenant
INSERT OR IGNORE INTO tenant_users (tenant_id, user_id, role, joined_at)
SELECT 
    'default-tenant' as tenant_id,
    u.id as user_id,
    'member' as role,
    u.created_at as joined_at
FROM users u 
WHERE u.tenant_id IS NULL;

-- Update users table to set tenant_id for users without one
UPDATE users 
SET tenant_id = 'default-tenant' 
WHERE tenant_id IS NULL;