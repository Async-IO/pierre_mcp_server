// ABOUTME: Query parameter binding helpers
// ABOUTME: Reduces repetitive .bind() chains across PostgreSQL and SQLite
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// This module is currently minimal as the query parameter binding helpers
// were deferred to a later phase. The existing database implementations
// use direct .bind() chains which, while verbose, are explicit and type-safe.
