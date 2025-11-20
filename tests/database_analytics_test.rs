// ABOUTME: Integration tests for database analytics functionality
// ABOUTME: Tests JWT usage tracking, goals management, insights storage, and system stats
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::{
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
    rate_limiting::JwtUsage,
};
use uuid::Uuid;

mod common;
use common::*;

async fn create_test_user(db: &Database) -> User {
    let uuid = Uuid::new_v4();
    let (_user_id, user) =
        create_test_user_with_email(db, &format!("analytics_{uuid}@example.com"))
            .await
            .expect("Failed to create user");
    user
}

#[tokio::test]
async fn test_jwt_usage_tracking() {
    let db = common::create_test_database()
        .await
        .expect("Failed to create test database");

    let user = create_test_user(&db).await;

    // Record JWT usage
    let usage = JwtUsage {
        id: None,
        user_id: user.id,
        timestamp: Utc::now(),
        endpoint: "/api/v1/profile".into(),
        method: "GET".into(),
        status_code: 200,
        response_time_ms: Some(25),
        request_size_bytes: Some(128),
        response_size_bytes: Some(512),
        ip_address: Some("192.168.1.1".into()),
        user_agent: Some("TestClient/1.0".into()),
    };

    db.record_jwt_usage(&usage)
        .await
        .expect("Failed to record JWT usage");

    // Check current usage (use a more generous time window for tests)
    let current_usage = db
        .get_jwt_current_usage(user.id)
        .await
        .expect("Failed to get current JWT usage");
    assert_eq!(current_usage, 1);
}

#[tokio::test]
async fn test_goals_management() {
    let db = common::create_test_database()
        .await
        .expect("Failed to create test database");

    let user = create_test_user(&db).await;

    // Create a goal
    let goal_data = serde_json::json!({
        "type": "weekly_distance",
        "target": 50.0,
        "unit": "km",
        "current": 0.0
    });

    let goal_id = db
        .create_goal(user.id, goal_data.clone())
        .await
        .expect("Failed to create goal");

    // Get user goals
    let goals = db
        .get_user_goals(user.id)
        .await
        .expect("Failed to get user goals");
    assert_eq!(goals.len(), 1);
    assert_eq!(goals[0]["type"], "weekly_distance");

    // Update goal progress
    db.update_goal_progress(&goal_id, 25.0)
        .await
        .expect("Failed to update goal progress");
}

#[tokio::test]
async fn test_insights_storage() {
    let db = common::create_test_database()
        .await
        .expect("Failed to create test database");

    let user = create_test_user(&db).await;

    // Store an insight
    let insight_data = serde_json::json!({
        "type": "performance_trend",
        "message": "Your pace has improved by 5% over the last month",
        "severity": "positive"
    });

    let insight_id = db
        .store_insight(user.id, insight_data)
        .await
        .expect("Failed to store insight");

    // Verify the insight was stored with a valid ID
    assert!(!insight_id.is_empty());

    // Get user insights
    let insights = db
        .get_user_insights(user.id, None, Some(10))
        .await
        .expect("Failed to get user insights");
    assert_eq!(insights.len(), 1);
    assert_eq!(insights[0]["type"], "performance_trend");
}

#[tokio::test]
async fn test_system_stats() {
    let db = common::create_test_database()
        .await
        .expect("Failed to create test database");

    // Create multiple users
    for i in 0..3 {
        let (_user_id, _user) =
            create_test_user_with_email(&db, &format!("stats_user_{i}@example.com"))
                .await
                .expect("Failed to create user");
    }

    // Get system stats (user_count, api_key_count)
    let (user_count, api_key_count) = db
        .get_system_stats()
        .await
        .expect("Failed to get system stats");

    assert_eq!(user_count, 3);
    assert_eq!(api_key_count, 0); // No API keys created yet
}
