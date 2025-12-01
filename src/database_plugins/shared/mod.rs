// ABOUTME: Shared database logic for PostgreSQL and SQLite implementations
// ABOUTME: Eliminates duplication by extracting common business logic
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Enum conversion utilities (`UserTier`, `UserStatus`, `TaskStatus`, etc.)
pub mod enums;

/// Input validation logic (email, tenant ownership, expiration, scopes)
pub mod validation;

/// Model ↔ SQL row conversion helpers (row parsing, struct construction)
pub mod mappers;

/// Encryption/decryption utilities for OAuth tokens and sensitive data
pub mod encryption;

/// Query parameter binding helpers (reduce repetitive `.bind()` chains)
pub mod builders;

/// Transaction retry patterns (deadlock handling, exponential backoff)
pub mod transactions;
