-- PostgreSQL initialization script for Pierre MCP Server
-- This script sets up additional extensions and configurations

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Enable crypto functions for hashing (if needed)
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Set timezone
SET timezone = 'UTC';

-- Grant permissions to pierre user
GRANT ALL PRIVILEGES ON DATABASE pierre_mcp_server TO pierre;
GRANT ALL PRIVILEGES ON SCHEMA public TO pierre;

-- Set default permissions for future tables
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO pierre;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO pierre;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO pierre;