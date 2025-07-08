use pierre_mcp_server::database::Database;
use pierre_mcp_server::models::{DecryptedToken, User, UserTier};
use uuid::Uuid;

#[tokio::test]
async fn test_strava_token_storage() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    // Create a test user
    let user = User {
        id: Uuid::new_v4(),
        email: format!("strava_{}@example.com", Uuid::new_v4()),
        display_name: None,
        password_hash: "hashed".into(),
        tier: UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };

    db.create_user(&user).await.expect("Failed to create user");

    // Create test token with timestamp precision truncated to seconds
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(3600);
    let expires_at_truncated =
        chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).expect("Valid timestamp");
    let token = DecryptedToken {
        access_token: "test_access_token".into(),
        refresh_token: "test_refresh_token".into(),
        expires_at: expires_at_truncated,
        scope: "read,activity:read_all".into(),
    };

    // Store token
    db.update_strava_token(user.id, &token)
        .await
        .expect("Failed to update Strava token");

    // Retrieve token
    let retrieved = db
        .get_strava_token(user.id)
        .await
        .expect("Failed to get Strava token")
        .expect("Token not found");

    assert_eq!(retrieved.access_token, token.access_token);
    assert_eq!(retrieved.refresh_token, token.refresh_token);
    assert_eq!(retrieved.expires_at, token.expires_at);
    assert_eq!(retrieved.scope, token.scope);

    // Clear token
    db.clear_strava_token(user.id)
        .await
        .expect("Failed to clear Strava token");

    // Verify cleared
    let cleared = db
        .get_strava_token(user.id)
        .await
        .expect("Failed to get Strava token");
    assert!(cleared.is_none());
}

#[tokio::test]
async fn test_fitbit_token_storage() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    // Create a test user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: format!("fitbit_{user_id}@example.com"),
        display_name: None,
        password_hash: "hashed".into(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };

    db.create_user(&user).await.expect("Failed to create user");

    // Create test token with timestamp precision truncated to seconds
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(7200);
    let expires_at_truncated =
        chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).expect("Valid timestamp");
    let token = DecryptedToken {
        access_token: "fitbit_access_token".into(),
        refresh_token: "fitbit_refresh_token".into(),
        expires_at: expires_at_truncated,
        scope: "activity heartrate location".into(),
    };

    // Store token
    db.update_fitbit_token(user.id, &token)
        .await
        .expect("Failed to update Fitbit token");

    // Retrieve token
    let retrieved = db
        .get_fitbit_token(user.id)
        .await
        .expect("Failed to get Fitbit token")
        .expect("Token not found");

    assert_eq!(retrieved.access_token, token.access_token);
    assert_eq!(retrieved.refresh_token, token.refresh_token);
    assert_eq!(retrieved.expires_at, token.expires_at);
    assert_eq!(retrieved.scope, token.scope);
}
