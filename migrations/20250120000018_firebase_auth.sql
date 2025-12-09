-- ABOUTME: Migration to add Firebase Authentication support
-- ABOUTME: Adds firebase_uid and auth_provider columns to users table
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- Add Firebase UID column (nullable for existing email/password users)
-- Note: SQLite doesn't support ADD COLUMN with UNIQUE constraint directly,
-- uniqueness is enforced via the index below
ALTER TABLE users ADD COLUMN firebase_uid TEXT;

-- Add auth provider column to track how user authenticated
-- 'email' = traditional email/password
-- 'google.com' = Google Sign-In via Firebase
-- 'apple.com' = Apple Sign-In via Firebase
ALTER TABLE users ADD COLUMN auth_provider TEXT NOT NULL DEFAULT 'email';

-- Create unique index for Firebase UID lookups (enforces uniqueness)
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_firebase_uid ON users(firebase_uid) WHERE firebase_uid IS NOT NULL;

-- Create index for auth provider queries
CREATE INDEX IF NOT EXISTS idx_users_auth_provider ON users(auth_provider);
