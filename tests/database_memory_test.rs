// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tests to ensure in-memory databases don't create physical files

use anyhow::Result;
use pierre_mcp_server::database::{Database, generate_encryption_key};
use std::fs;

#[tokio::test]
async fn test_memory_database_no_physical_files() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    
    // Create in-memory database - this should NOT create any physical files
    let database = Database::new("sqlite::memory:", encryption_key).await?;
    
    // Verify no physical files are created with memory database patterns
    let current_dir = std::env::current_dir()?;
    let entries = fs::read_dir(&current_dir)?;
    
    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();
        
        // Check for problematic files that shouldn't exist
        if filename_str.starts_with(":memory:test_") {
            panic!("Found physical file that should be in-memory: {}", filename_str);
        }
        
        if filename_str.starts_with("sqlite::memory:") {
            panic!("Found physical file with memory database URL: {}", filename_str);
        }
    }
    
    // Test basic database functionality to ensure it works
    let user = pierre_mcp_server::models::User::new(
        "test@memory.test".to_string(),
        "password_hash".to_string(),
        Some("Memory Test User".to_string()),
    );
    
    let user_id = database.create_user(&user).await?;
    let retrieved_user = database.get_user(user_id).await?.unwrap();
    
    assert_eq!(retrieved_user.email, "test@memory.test");
    assert_eq!(retrieved_user.display_name, Some("Memory Test User".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_memory_databases_isolated() -> Result<()> {
    let encryption_key1 = generate_encryption_key().to_vec();
    let encryption_key2 = generate_encryption_key().to_vec();
    
    // Create two separate in-memory databases
    let database1 = Database::new("sqlite::memory:", encryption_key1).await?;
    let database2 = Database::new("sqlite::memory:", encryption_key2).await?;
    
    // Create users in each database
    let user1 = pierre_mcp_server::models::User::new(
        "user1@test.com".to_string(),
        "hash1".to_string(),
        Some("User 1".to_string()),
    );
    
    let user2 = pierre_mcp_server::models::User::new(
        "user2@test.com".to_string(),
        "hash2".to_string(),
        Some("User 2".to_string()),
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