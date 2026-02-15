// ABOUTME: Unit tests for the chat database module
// ABOUTME: Tests conversation and message CRUD operations with multi-tenant isolation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Test files: allow missing_docs (rustc lint) and unwrap (valid in tests per CLAUDE.md guidelines)
#![allow(missing_docs, clippy::unwrap_used)]

use pierre_mcp_server::database::ChatManager;
use pierre_mcp_server::llm::MessageRole;
use pierre_mcp_server::models::TenantId;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Deterministic tenant ID for tests (fixed bytes representing "tenant-1")
fn test_tenant_id() -> TenantId {
    TenantId::from_uuid(Uuid::from_bytes([
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01,
    ]))
}

/// Second deterministic tenant ID for multi-tenant isolation tests (fixed bytes representing "tenant-2")
fn test_tenant_id_2() -> TenantId {
    TenantId::from_uuid(Uuid::from_bytes([
        0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
        0x02,
    ]))
}

/// Create a test database with chat schema
async fn create_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Create users table first (for foreign key)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            user_status TEXT NOT NULL DEFAULT 'active',
            is_admin INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            last_active TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create test user
    sqlx::query(
        r"
        INSERT INTO users (id, email, password_hash, created_at, last_active)
        VALUES ('user-1', 'test@example.com', 'hash', '2025-01-01', '2025-01-01')
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create chat tables
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS chat_conversations (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            tenant_id TEXT NOT NULL,
            title TEXT NOT NULL,
            model TEXT NOT NULL DEFAULT 'gemini-1.5-flash',
            system_prompt TEXT,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
            role TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
            content TEXT NOT NULL,
            token_count INTEGER,
            finish_reason TEXT,
            created_at TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

// ============================================================================
// Conversation Tests
// ============================================================================

#[tokio::test]
async fn test_create_conversation() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    assert!(!conv.id.is_empty());
    assert_eq!(conv.user_id, "user-1");
    assert_eq!(conv.tenant_id, tenant_id.to_string());
    assert_eq!(conv.title, "Test Chat");
    assert_eq!(conv.model, "gemini-1.5-flash");
    assert!(conv.system_prompt.is_none());
    assert_eq!(conv.total_tokens, 0);
}

#[tokio::test]
async fn test_create_conversation_with_system_prompt() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let system_prompt = "You are a helpful fitness assistant.";
    let conv = manager
        .create_conversation(
            "user-1",
            tenant_id,
            "Fitness Chat",
            "gemini-1.5-pro",
            Some(system_prompt),
        )
        .await
        .unwrap();

    assert_eq!(conv.system_prompt, Some(system_prompt.to_owned()));
    assert_eq!(conv.model, "gemini-1.5-pro");
}

#[tokio::test]
async fn test_get_conversation() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let created = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    let fetched = manager
        .get_conversation(&created.id, "user-1", tenant_id)
        .await
        .unwrap();

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "Test Chat");
}

#[tokio::test]
async fn test_get_conversation_tenant_isolation() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let different_tenant = test_tenant_id_2();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    // Try to access from different tenant - should return None
    let result = manager
        .get_conversation(&conv.id, "user-1", different_tenant)
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_list_conversations() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    // Create multiple conversations
    manager
        .create_conversation("user-1", tenant_id, "Chat 1", "gemini-1.5-flash", None)
        .await
        .unwrap();
    manager
        .create_conversation("user-1", tenant_id, "Chat 2", "gemini-1.5-flash", None)
        .await
        .unwrap();
    manager
        .create_conversation("user-1", tenant_id, "Chat 3", "gemini-1.5-flash", None)
        .await
        .unwrap();

    let list = manager
        .list_conversations("user-1", tenant_id, 10, 0)
        .await
        .unwrap();

    assert_eq!(list.len(), 3);
}

#[tokio::test]
async fn test_list_conversations_pagination() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    // Create multiple conversations
    for i in 1..=5 {
        manager
            .create_conversation(
                "user-1",
                tenant_id,
                &format!("Chat {i}"),
                "gemini-1.5-flash",
                None,
            )
            .await
            .unwrap();
    }

    // Get first 2
    let page1 = manager
        .list_conversations("user-1", tenant_id, 2, 0)
        .await
        .unwrap();
    assert_eq!(page1.len(), 2);

    // Get next 2
    let page2 = manager
        .list_conversations("user-1", tenant_id, 2, 2)
        .await
        .unwrap();
    assert_eq!(page2.len(), 2);

    // Get remaining
    let page3 = manager
        .list_conversations("user-1", tenant_id, 2, 4)
        .await
        .unwrap();
    assert_eq!(page3.len(), 1);
}

#[tokio::test]
async fn test_update_conversation_title() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation(
            "user-1",
            tenant_id,
            "Original Title",
            "gemini-1.5-flash",
            None,
        )
        .await
        .unwrap();

    let updated = manager
        .update_conversation_title(&conv.id, "user-1", tenant_id, "New Title")
        .await
        .unwrap();

    assert!(updated);

    let fetched = manager
        .get_conversation(&conv.id, "user-1", tenant_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(fetched.title, "New Title");
}

#[tokio::test]
async fn test_delete_conversation() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "To Delete", "gemini-1.5-flash", None)
        .await
        .unwrap();

    let deleted = manager
        .delete_conversation(&conv.id, "user-1", tenant_id)
        .await
        .unwrap();

    assert!(deleted);

    let fetched = manager
        .get_conversation(&conv.id, "user-1", tenant_id)
        .await
        .unwrap();

    assert!(fetched.is_none());
}

// ============================================================================
// Message Tests
// ============================================================================

#[tokio::test]
async fn test_add_message() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    let msg = manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::User,
            "Hello, world!",
            Some(5),
            None,
        )
        .await
        .unwrap();

    assert!(!msg.id.is_empty());
    assert_eq!(msg.conversation_id, conv.id);
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "Hello, world!");
    assert_eq!(msg.token_count, Some(5));
}

#[tokio::test]
async fn test_add_assistant_message_with_finish_reason() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    let msg = manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::Assistant,
            "I'm here to help!",
            Some(10),
            Some("STOP"),
        )
        .await
        .unwrap();

    assert_eq!(msg.role, "assistant");
    assert_eq!(msg.finish_reason, Some("STOP".to_owned()));
}

#[tokio::test]
async fn test_get_messages() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    // Add messages
    manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::User,
            "Hello",
            Some(2),
            None,
        )
        .await
        .unwrap();
    manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::Assistant,
            "Hi there!",
            Some(3),
            None,
        )
        .await
        .unwrap();
    manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::User,
            "How are you?",
            Some(4),
            None,
        )
        .await
        .unwrap();

    let messages = manager.get_messages(&conv.id, "user-1").await.unwrap();

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "Hi there!");
    assert_eq!(messages[2].content, "How are you?");
}

#[tokio::test]
async fn test_get_recent_messages() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    // Add 5 messages
    for i in 1..=5 {
        manager
            .add_message(
                &conv.id,
                "user-1",
                MessageRole::User,
                &format!("Message {i}"),
                Some(2),
                None,
            )
            .await
            .unwrap();
    }

    // Get last 3
    let recent = manager
        .get_recent_messages(&conv.id, "user-1", 3)
        .await
        .unwrap();

    assert_eq!(recent.len(), 3);
    // Should be in chronological order
    assert_eq!(recent[0].content, "Message 3");
    assert_eq!(recent[1].content, "Message 4");
    assert_eq!(recent[2].content, "Message 5");
}

#[tokio::test]
async fn test_message_updates_conversation_tokens() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    assert_eq!(conv.total_tokens, 0);

    // Add messages with token counts
    manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::User,
            "Hello",
            Some(10),
            None,
        )
        .await
        .unwrap();
    manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::Assistant,
            "Hi!",
            Some(15),
            None,
        )
        .await
        .unwrap();

    // Check total tokens updated
    let updated = manager
        .get_conversation(&conv.id, "user-1", tenant_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.total_tokens, 25);
}

#[tokio::test]
async fn test_get_message_count() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    // Initially 0
    let count = manager.get_message_count(&conv.id, "user-1").await.unwrap();
    assert_eq!(count, 0);

    // Add messages
    manager
        .add_message(&conv.id, "user-1", MessageRole::User, "1", None, None)
        .await
        .unwrap();
    manager
        .add_message(&conv.id, "user-1", MessageRole::Assistant, "2", None, None)
        .await
        .unwrap();

    let count = manager.get_message_count(&conv.id, "user-1").await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_cascade_delete_messages() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    let conv = manager
        .create_conversation("user-1", tenant_id, "Test Chat", "gemini-1.5-flash", None)
        .await
        .unwrap();

    // Add messages
    manager
        .add_message(&conv.id, "user-1", MessageRole::User, "Hello", None, None)
        .await
        .unwrap();
    manager
        .add_message(
            &conv.id,
            "user-1",
            MessageRole::Assistant,
            "Hi!",
            None,
            None,
        )
        .await
        .unwrap();

    // Verify messages exist
    let count = manager.get_message_count(&conv.id, "user-1").await.unwrap();
    assert_eq!(count, 2);

    // Delete conversation (should cascade delete messages)
    manager
        .delete_conversation(&conv.id, "user-1", tenant_id)
        .await
        .unwrap();

    // Messages should be gone (foreign key cascade)
    let messages = manager.get_messages(&conv.id, "user-1").await.unwrap();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_delete_all_user_conversations() {
    let pool = create_test_db().await;
    let manager = ChatManager::new(pool);

    let tenant_id = test_tenant_id();
    // Create multiple conversations
    manager
        .create_conversation("user-1", tenant_id, "Chat 1", "gemini-1.5-flash", None)
        .await
        .unwrap();
    manager
        .create_conversation("user-1", tenant_id, "Chat 2", "gemini-1.5-flash", None)
        .await
        .unwrap();
    manager
        .create_conversation("user-1", tenant_id, "Chat 3", "gemini-1.5-flash", None)
        .await
        .unwrap();

    let deleted = manager
        .delete_all_user_conversations("user-1", tenant_id)
        .await
        .unwrap();

    assert_eq!(deleted, 3);

    let remaining = manager
        .list_conversations("user-1", tenant_id, 10, 0)
        .await
        .unwrap();

    assert!(remaining.is_empty());
}
