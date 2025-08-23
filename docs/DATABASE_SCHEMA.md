# Pierre MCP Server Database Schema

This document describes the complete database schema for Pierre MCP Server as implemented in the codebase.

## Schema Overview

Pierre MCP Server uses both PostgreSQL and SQLite database implementations through a plugin architecture. The schema supports:
- Multi-tenant architecture with tenant isolation
- OAuth integration for multiple providers
- API key management with rate limiting  
- A2A (Agent-to-Agent) protocol support
- Comprehensive usage tracking and analytics

## Core Tables

### Users Table
```sql
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    display_name TEXT,
    password_hash TEXT NOT NULL,
    tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
    strava_access_token TEXT,
    strava_refresh_token TEXT,
    strava_expires_at TIMESTAMPTZ,
    strava_scope TEXT,
    strava_nonce TEXT,
    is_approved BOOLEAN NOT NULL DEFAULT false,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: Core user management with approval workflow and legacy Strava integration.

### Tenants Table
```sql
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    domain VARCHAR(255) UNIQUE,
    subscription_tier VARCHAR(50) DEFAULT 'starter' CHECK (subscription_tier IN ('starter', 'professional', 'enterprise')),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: Multi-tenant organization support with domain-based isolation.

### Tenant Users Table
```sql
CREATE TABLE IF NOT EXISTS tenant_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'billing', 'member')),
    joined_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, user_id)
)
```

**Purpose**: Maps users to tenants with role-based access control.

## OAuth and Authentication

### User OAuth Tokens Table
```sql
CREATE TABLE IF NOT EXISTS user_oauth_tokens (
    id VARCHAR(255) PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id VARCHAR(255) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    token_type VARCHAR(50) DEFAULT 'bearer',
    expires_at TIMESTAMPTZ,
    scope TEXT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, tenant_id, provider)
)
```

**Purpose**: Encrypted OAuth tokens per user-tenant-provider combination.

### Tenant OAuth Apps Table
```sql
CREATE TABLE IF NOT EXISTS tenant_oauth_apps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    client_id VARCHAR(255) NOT NULL,
    client_secret_encrypted BYTEA NOT NULL,
    client_secret_nonce BYTEA NOT NULL,
    redirect_uri VARCHAR(500) NOT NULL,
    scopes TEXT[] DEFAULT '{}',
    rate_limit_per_day INTEGER DEFAULT 15000,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, provider)
)
```

**Purpose**: OAuth app configurations per tenant with encrypted secrets.

### OAuth Apps Table (Server-Wide)
```sql
CREATE TABLE IF NOT EXISTS oauth_apps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id VARCHAR(255) UNIQUE NOT NULL,
    client_secret VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    redirect_uris TEXT[] NOT NULL DEFAULT '{}',
    scopes TEXT[] NOT NULL DEFAULT '{}',
    app_type VARCHAR(50) DEFAULT 'web' CHECK (app_type IN ('desktop', 'web', 'mobile', 'server')),
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: Server-wide OAuth app registrations for AI assistants.

### Authorization Codes Table
```sql
CREATE TABLE IF NOT EXISTS authorization_codes (
    code VARCHAR(255) PRIMARY KEY,
    client_id VARCHAR(255) NOT NULL REFERENCES oauth_apps(client_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redirect_uri VARCHAR(500) NOT NULL,
    scope VARCHAR(500) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL
)
```

**Purpose**: Temporary authorization codes for OAuth flow.

## API Key Management

### API Keys Table
```sql
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    description TEXT,
    tier TEXT NOT NULL CHECK (tier IN ('trial', 'starter', 'professional', 'enterprise')),
    is_active BOOLEAN NOT NULL DEFAULT true,
    rate_limit_requests INTEGER NOT NULL,
    rate_limit_window_seconds INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMPTZ
)
```

**Purpose**: User API keys with tier-based rate limiting.

### API Key Usage Table
```sql
CREATE TABLE IF NOT EXISTS api_key_usage (
    id SERIAL PRIMARY KEY,
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    endpoint TEXT NOT NULL,
    response_time_ms INTEGER,
    status_code SMALLINT NOT NULL,
    method TEXT,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address INET,
    user_agent TEXT,
    error_message TEXT
)
```

**Purpose**: Detailed API usage analytics and monitoring.

## A2A (Agent-to-Agent) Protocol

### A2A Clients Table
```sql
CREATE TABLE IF NOT EXISTS a2a_clients (
    client_id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    client_secret_hash TEXT NOT NULL,
    api_key_hash TEXT NOT NULL,
    capabilities TEXT[] NOT NULL DEFAULT '{}',
    redirect_uris TEXT[] NOT NULL DEFAULT '{}',
    contact_email TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    rate_limit_requests INTEGER DEFAULT 100,
    rate_limit_window_seconds INTEGER DEFAULT 3600,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: A2A client registrations with capabilities and rate limits.

### A2A Sessions Table
```sql
CREATE TABLE IF NOT EXISTS a2a_sessions (
    session_token TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES a2a_clients(client_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    granted_scopes TEXT[] NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL,
    last_active_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: Active A2A authentication sessions.

### A2A Tasks Table
```sql
CREATE TABLE IF NOT EXISTS a2a_tasks (
    task_id TEXT PRIMARY KEY,
    session_token TEXT NOT NULL REFERENCES a2a_sessions(session_token) ON DELETE CASCADE,
    task_type TEXT NOT NULL,
    parameters JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    result JSONB,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: A2A task execution tracking with async results.

### A2A Usage Table
```sql
CREATE TABLE IF NOT EXISTS a2a_usage (
    id SERIAL PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES a2a_clients(client_id) ON DELETE CASCADE,
    session_token TEXT REFERENCES a2a_sessions(session_token) ON DELETE SET NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    endpoint TEXT NOT NULL,
    response_time_ms INTEGER,
    status_code SMALLINT NOT NULL,
    method TEXT,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address INET,
    user_agent TEXT
)
```

**Purpose**: A2A protocol usage analytics.

## Administration

### Admin Tokens Table
```sql
CREATE TABLE IF NOT EXISTS admin_tokens (
    id TEXT PRIMARY KEY,
    service_name TEXT NOT NULL,
    service_description TEXT,
    token_hash TEXT NOT NULL,
    token_prefix TEXT NOT NULL,
    jwt_secret_hash TEXT NOT NULL,
    permissions TEXT NOT NULL DEFAULT '[]',
    is_super_admin BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: System administration tokens with permission-based access.

### Admin Token Usage Table
```sql
CREATE TABLE IF NOT EXISTS admin_token_usage (
    id SERIAL PRIMARY KEY,
    admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action TEXT NOT NULL,
    target_resource TEXT,
    ip_address INET,
    user_agent TEXT,
    request_size_bytes INTEGER,
    success BOOLEAN NOT NULL,
    method TEXT,
    response_time_ms INTEGER
)
```

**Purpose**: Audit trail for admin operations.

### Admin Provisioned Keys Table
```sql
CREATE TABLE IF NOT EXISTS admin_provisioned_keys (
    id SERIAL PRIMARY KEY,
    admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
    api_key_id TEXT NOT NULL,
    user_email TEXT NOT NULL,
    requested_tier TEXT NOT NULL,
    provisioned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    provisioned_by_service TEXT NOT NULL,
    rate_limit_requests INTEGER NOT NULL,
    rate_limit_period TEXT NOT NULL,
    key_status TEXT NOT NULL DEFAULT 'active',
    notes TEXT
)
```

**Purpose**: Track admin-provisioned API keys for compliance.

## User Data and Analytics

### User Profiles Table
```sql
CREATE TABLE IF NOT EXISTS user_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    profile_data JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: Extended user profile data in JSON format.

### User Configurations Table  
```sql
CREATE TABLE IF NOT EXISTS user_configurations (
    user_id TEXT PRIMARY KEY,
    config_data TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: User-specific configuration settings.

### Goals Table
```sql
CREATE TABLE IF NOT EXISTS goals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    goal_data JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: User fitness goals stored as JSON.

### Insights Table
```sql
CREATE TABLE IF NOT EXISTS insights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    insight_type TEXT NOT NULL,
    content JSONB NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
)
```

**Purpose**: AI-generated fitness insights.

### JWT Usage Table
```sql
CREATE TABLE IF NOT EXISTS jwt_usage (
    id SERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    endpoint TEXT NOT NULL,
    response_time_ms INTEGER,
    status_code INTEGER NOT NULL,
    method TEXT,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address INET,
    user_agent TEXT
)
```

**Purpose**: JWT-authenticated request analytics.

## Multi-Tenant Usage Tracking

### Tenant Provider Usage Table
```sql
CREATE TABLE IF NOT EXISTS tenant_provider_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    usage_date DATE NOT NULL,
    request_count INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, provider, usage_date)
)
```

**Purpose**: Daily usage statistics per tenant-provider combination.

## Database Implementations

### PostgreSQL Implementation
Located in `src/database_plugins/postgres.rs` - Production database with full feature support.

### SQLite Implementation  
Located in `src/database/` modules - Development and testing database with subset of features.

## Security Features

### Encryption
- OAuth client secrets encrypted with AES-256-GCM
- Separate encryption keys per tenant using KEK/DEK pattern
- Database-level encryption for sensitive columns

### Data Isolation
- Tenant-level data isolation through foreign key constraints
- Row-level security policies (PostgreSQL)
- API-level tenant context validation

### Audit Trail
- Comprehensive usage tracking across all endpoints
- Admin operation logging
- Rate limiting enforcement with detailed metrics

## Indexes and Performance

Key indexes are created for:
- User email lookups
- API key authentication
- Tenant-based queries  
- Usage analytics time-series queries
- OAuth token provider lookups

All timestamp columns use TIMESTAMPTZ for proper timezone handling.