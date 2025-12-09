// ABOUTME: Unit tests for database a2a functionality
// ABOUTME: Validates database a2a behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::{
    a2a::{
        auth::A2AClient,
        client::A2ASession,
        protocol::{A2ATask, TaskStatus},
    },
    api_keys::{ApiKey, ApiKeyTier},
    database::{a2a::A2AUsage, Database},
    models::{User, UserTier},
};
use uuid::Uuid;

async fn create_test_client(db: &Database) -> (A2AClient, Uuid) {
    let unique_id = Uuid::new_v4();

    // First create a test user
    let test_user_id = Uuid::new_v4();
    let user = User {
        id: test_user_id,
        email: format!("test_{unique_id}@example.com"),
        display_name: Some(format!("Test User {unique_id}")),
        password_hash: format!("test_hash_{unique_id}"),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        role: pierre_mcp_server::permissions::UserRole::User,
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    db.create_user(&user)
        .await
        .expect("Failed to create test user");

    // Create a test API key for the user
    let api_key = ApiKey {
        id: format!("test_api_key_{unique_id}"),
        user_id: test_user_id,
        name: format!("Test API Key {unique_id}"),
        description: Some("Test API key for A2A client".into()),
        key_prefix: format!("pk_test_{}", &unique_id.to_string()[0..8]),
        key_hash: format!("test_key_hash_{unique_id}"),
        tier: ApiKeyTier::Professional,
        rate_limit_requests: 1000,
        rate_limit_window_seconds: 3600,
        is_active: true,
        created_at: Utc::now(),
        last_used_at: None,
        expires_at: None,
    };
    db.create_api_key(&api_key)
        .await
        .expect("Failed to create test API key");

    let client = A2AClient {
        id: format!("test_client_{unique_id}"),
        name: format!("Test Client {unique_id}"),
        description: format!("Test A2A client {unique_id}"),
        public_key: format!("test_public_key_{unique_id}"),
        user_id: test_user_id,
        capabilities: vec!["fitness-data-analysis".into()],
        redirect_uris: vec!["https://test.example.com".into()],
        permissions: vec!["read_activities".into(), "write_goals".into()],
        rate_limit_requests: 1000,
        rate_limit_window_seconds: 3600,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db.create_a2a_client(&client, "test_secret", &api_key.id)
        .await
        .expect("Failed to create A2A client");
    (client, test_user_id)
}

#[tokio::test]
async fn test_a2a_client_management() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    let (client, user_id) = create_test_client(&db).await;

    // Get client
    let retrieved = db
        .get_a2a_client(&client.id)
        .await
        .expect("Failed to get A2A client")
        .expect("Client not found");

    assert_eq!(retrieved.id, client.id);
    assert_eq!(retrieved.name, client.name);
    assert_eq!(retrieved.permissions, client.permissions);

    // List clients - check that our client is in the list
    let clients = db
        .list_a2a_clients(&user_id)
        .await
        .expect("Failed to list A2A clients");

    // Find our client in the list
    let found_client = clients.iter().find(|c| c.id == client.id);
    assert!(
        found_client.is_some(),
        "Created client should be in the list"
    );
    assert_eq!(found_client.unwrap().id, client.id);
}

#[tokio::test]
async fn test_a2a_session_management() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    let (client, _user_id) = create_test_client(&db).await;

    // Create session (without user_id to avoid foreign key constraint)
    let session = A2ASession {
        id: format!("session_{}", Uuid::new_v4()),
        client_id: client.id.clone(),
        user_id: None, // No user association for this test
        granted_scopes: vec!["read".into(), "write".into()],
        expires_at: Utc::now() + chrono::Duration::hours(1),
        last_activity: Utc::now(),
        created_at: Utc::now(),
        requests_count: 0,
    };

    let session_token = db
        .create_a2a_session(
            &session.client_id,
            session.user_id.as_ref(),
            &session.granted_scopes,
            1,
        )
        .await
        .expect("Failed to create A2A session");

    // Get session
    let retrieved = db
        .get_a2a_session(&session_token)
        .await
        .expect("Failed to get A2A session")
        .expect("Session not found");

    assert_eq!(retrieved.id, session_token);
    assert_eq!(retrieved.client_id, session.client_id);
    assert_eq!(retrieved.granted_scopes, session.granted_scopes);

    // Update session activity
    db.update_a2a_session_activity(&session_token)
        .await
        .expect("Failed to update session activity");

    // Test getting active sessions for client
    let active_sessions = db
        .get_active_a2a_sessions(&client.id)
        .await
        .expect("Failed to get active sessions");

    assert_eq!(active_sessions.len(), 1);
    assert_eq!(active_sessions[0].id, session_token);
    assert_eq!(active_sessions[0].client_id, client.id);
}

#[tokio::test]
async fn test_a2a_task_management() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    let (client, _user_id) = create_test_client(&db).await;

    // Create task
    let task = A2ATask {
        id: format!("task_{}", Uuid::new_v4()),
        client_id: client.id.clone(),
        task_type: "analysis".into(),
        input_data: serde_json::json!({"data": "test"}),
        output_data: None,
        status: TaskStatus::Pending,
        result: None,
        error: None,
        error_message: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
    };

    let task_id = db
        .create_a2a_task(&task.client_id, None, &task.task_type, &task.input_data)
        .await
        .expect("Failed to create A2A task");

    // Get task
    let retrieved = db
        .get_a2a_task(&task_id)
        .await
        .expect("Failed to get A2A task")
        .expect("Task not found");

    assert_eq!(retrieved.id, task_id);
    assert_eq!(retrieved.status, TaskStatus::Pending);

    // Update task status
    let output_data = serde_json::json!({"result": "success"});
    db.update_a2a_task_status(&task_id, &TaskStatus::Completed, Some(&output_data), None)
        .await
        .expect("Failed to update task status");

    // Verify update
    let updated = db
        .get_a2a_task(&task_id)
        .await
        .expect("Failed to get updated task")
        .expect("Task not found");

    assert_eq!(updated.status, TaskStatus::Completed);
    assert_eq!(updated.output_data, Some(output_data));
    assert!(updated.completed_at.is_some());
}

#[tokio::test]
async fn test_a2a_usage_tracking() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    let (client, _user_id) = create_test_client(&db).await;

    // Record usage
    let usage = A2AUsage {
        id: None,
        client_id: client.id.clone(),
        session_token: None, // No session for this test
        timestamp: Utc::now(),
        tool_name: "analyze".into(),
        request_size_bytes: Some(256),
        response_size_bytes: Some(512),
        response_time_ms: Some(100),
        status_code: 200,
        error_message: None,
        ip_address: Some("127.0.0.1".to_owned()),
        user_agent: Some("test-agent".into()),
        protocol_version: "1.0".into(),
        client_capabilities: vec!["analysis".into()],
        granted_scopes: vec!["read".into()],
    };

    db.record_a2a_usage(&usage)
        .await
        .expect("Failed to record A2A usage");

    // Check current usage
    let current_usage = db
        .get_a2a_client_current_usage(&client.id)
        .await
        .expect("Failed to get current usage");
    assert_eq!(current_usage, 1);

    // Get usage stats
    let stats = db
        .get_a2a_usage_stats(
            &client.id,
            Utc::now() - chrono::Duration::hours(1),
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .expect("Failed to get usage stats");

    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.successful_requests, 1);
    assert_eq!(stats.failed_requests, 0);
}

#[tokio::test]
async fn test_a2a_schema_no_duplicate_columns() {
    let encryption_key = vec![0u8; 32];
    let db = Database::new("sqlite::memory:", encryption_key)
        .await
        .expect("Failed to create database");

    db.migrate().await.expect("Failed to run migrations");

    let columns: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('a2a_clients') ORDER BY name")
            .fetch_all(db.pool())
            .await
            .expect("Failed to query table info");

    let column_names: Vec<String> = columns.into_iter().map(|(name,)| name).collect();

    assert!(
        column_names.contains(&"capabilities".to_owned()),
        "capabilities column should exist in a2a_clients table"
    );
    assert!(
        column_names.contains(&"redirect_uris".to_owned()),
        "redirect_uris column should exist in a2a_clients table"
    );

    let capabilities_count = column_names.iter().filter(|n| *n == "capabilities").count();
    let redirect_uris_count = column_names
        .iter()
        .filter(|n| *n == "redirect_uris")
        .count();

    assert_eq!(
        capabilities_count, 1,
        "capabilities column should appear exactly once (found {capabilities_count} occurrences)"
    );
    assert_eq!(
        redirect_uris_count, 1,
        "redirect_uris column should appear exactly once (found {redirect_uris_count} occurrences)"
    );
}
