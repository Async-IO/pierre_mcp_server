// ABOUTME: Tests to ensure in-memory databases don't create physical files
// ABOUTME: Validates SQLite memory database isolation and cleanup behavior
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Tests to ensure in-memory databases don't create physical files

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};
use std::fs;

#[tokio::test]
async fn test_memory_database_no_physical_files() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();

    // Create in-memory database - this should NOT create any physical files
    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key).await?;

    // Verify no physical files are created with memory database patterns
    let current_dir = std::env::current_dir()?;
    let entries = fs::read_dir(&current_dir)?;

    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Check for problematic files that shouldn't exist
        assert!(
            !filename_str.starts_with(":memory:test_"),
            "Found physical file that should be in-memory: {filename_str}"
        );

        assert!(
            !filename_str.starts_with("sqlite::memory:"),
            "Found physical file with memory database URL: {filename_str}"
        );
    }

    // Test basic database functionality to ensure it works
    let user = pierre_mcp_server::models::User::new(
        "test@memory.test".to_owned(),
        "password_hash".to_owned(),
        Some("Memory Test User".to_owned()),
    );

    let user_id = database.create_user(&user).await?;
    let retrieved_user = database.get_user(user_id).await?.unwrap();

    assert_eq!(retrieved_user.email, "test@memory.test");
    assert_eq!(
        retrieved_user.display_name,
        Some("Memory Test User".to_owned())
    );

    Ok(())
}

#[tokio::test]
async fn test_multiple_memory_databases_isolated() -> Result<()> {
    let encryption_key1 = generate_encryption_key().to_vec();
    let encryption_key2 = generate_encryption_key().to_vec();

    // Create two separate in-memory databases
    #[cfg(feature = "postgresql")]
    let database1 = Database::new(
        "sqlite::memory:",
        encryption_key1,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database1 = Database::new("sqlite::memory:", encryption_key1).await?;

    #[cfg(feature = "postgresql")]
    let database2 = Database::new(
        "sqlite::memory:",
        encryption_key2,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database2 = Database::new("sqlite::memory:", encryption_key2).await?;

    // Create users in each database
    let user1 = pierre_mcp_server::models::User::new(
        "user1@test.com".to_owned(),
        "hash1".to_owned(),
        Some("User 1".to_owned()),
    );

    let user2 = pierre_mcp_server::models::User::new(
        "user2@test.com".to_owned(),
        "hash2".to_owned(),
        Some("User 2".to_owned()),
    );

    let user1_id = database1.create_user(&user1).await?;
    let user2_id = database2.create_user(&user2).await?;

    // Verify isolation - each database only contains its own user
    assert!(database1.get_user(user1_id).await?.is_some());
    assert!(database2.get_user(user2_id).await?.is_some());

    // User1 should not exist in database2 and vice versa
    assert!(database2.get_user(user1_id).await?.is_none());
    assert!(database1.get_user(user2_id).await?.is_none());

    Ok(())
}
