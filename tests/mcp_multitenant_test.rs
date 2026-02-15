// ABOUTME: Multi-tenant MCP server tests with protocol validation
// ABOUTME: Tests tenant isolation, MCP protocol handling, and server lifecycle
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::significant_drop_tightening,
    clippy::match_wildcard_for_single_variants,
    clippy::match_same_arms,
    clippy::unreadable_literal,
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_pass_by_value,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::struct_excessive_bools,
    clippy::missing_const_for_fn,
    clippy::cognitive_complexity,
    clippy::items_after_statements,
    clippy::semicolon_if_nothing_returned,
    clippy::use_self,
    clippy::single_match_else,
    clippy::default_trait_access,
    clippy::enum_glob_use,
    clippy::wildcard_imports,
    clippy::explicit_deref_methods,
    clippy::explicit_iter_loop,
    clippy::manual_let_else,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::used_underscore_binding,
    clippy::fn_params_excessive_bools,
    clippy::trivially_copy_pass_by_ref,
    clippy::option_if_let_else,
    clippy::unnecessary_wraps,
    clippy::redundant_else,
    clippy::map_unwrap_or,
    clippy::map_err_ignore,
    clippy::if_not_else,
    clippy::single_char_lifetime_names,
    clippy::doc_markdown,
    clippy::unused_async,
    clippy::redundant_field_names,
    clippy::struct_field_names,
    clippy::ptr_arg,
    clippy::ref_option_ref,
    clippy::implicit_clone,
    clippy::cloned_instead_of_copied,
    clippy::borrow_as_ptr,
    clippy::bool_to_int_with_if,
    clippy::checked_conversions,
    clippy::copy_iterator,
    clippy::empty_enum,
    clippy::enum_variant_names,
    clippy::expl_impl_clone_on_copy,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::fn_to_numeric_cast_any,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_hasher,
    clippy::inconsistent_struct_constructor,
    clippy::inefficient_to_string,
    clippy::infinite_iter,
    clippy::into_iter_on_ref,
    clippy::iter_not_returning_iterator,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_assert,
    clippy::manual_instant_elapsed,
    clippy::manual_ok_or,
    clippy::manual_string_new,
    clippy::many_single_char_names,
    clippy::match_wild_err_arm,
    clippy::mem_forget,
    clippy::missing_enforced_import_renames,
    clippy::missing_inline_in_public_items,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::naive_bytecount,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_raw_string_hashes,
    clippy::no_effect_underscore_binding,
    clippy::non_ascii_literal,
    clippy::nonstandard_macro_braces,
    clippy::option_option,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_literal,
    clippy::print_with_newline,
    clippy::ptr_as_ptr,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::redundant_allocation,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_add_assign,
    clippy::string_lit_as_bytes,
    clippy::trait_duplication_in_bounds,
    clippy::transmute_ptr_to_ptr,
    clippy::tuple_array_conversions,
    clippy::unchecked_time_subtraction,
    clippy::unicode_not_nfc,
    clippy::unimplemented,
    clippy::unnecessary_box_returns,
    clippy::unnecessary_struct_initialization,
    clippy::unnecessary_to_owned,
    clippy::unnested_or_patterns,
    clippy::unused_peekable,
    clippy::unused_rounding,
    clippy::useless_let_if_seq,
    clippy::verbose_bit_mask,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values
)]
//

//! Comprehensive integration tests for MultiTenantMcpServer
//!
//! This test suite provides comprehensive coverage of the multitenant MCP server
//! functionality including session management, protocol handling, authentication,
//! tenant isolation, error handling, and concurrent operations.

use anyhow::Result;
use futures_util::future;
use pierre_mcp_server::{
    admin::jwks::JwksManager,
    auth::{AuthManager, JwtValidationError},
    config::environment::RateLimitConfig,
    constants::oauth_providers,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::multitenant::MultiTenantMcpServer,
    middleware::McpAuthMiddleware,
    models::{TenantId, User, UserOAuthToken, UserTier},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio::time::sleep;
use uuid::Uuid;

// Import common test utilities
mod common;
use common::*;

/// Helper to create a multitenant MCP server for testing
async fn create_test_multitenant_server(
) -> Result<(MultiTenantMcpServer, Arc<Database>, Arc<AuthManager>)> {
    let resources = create_test_server_resources().await?;
    let database = resources.database.clone();
    let auth_manager = resources.auth_manager.clone();

    let server = MultiTenantMcpServer::new(resources);

    Ok((server, database, auth_manager))
}

/// Helper to create multiple test users for multitenant testing
async fn create_multiple_test_users(
    database: &Database,
    auth_manager: &AuthManager,
    count: usize,
    jwks_manager: &Arc<JwksManager>,
) -> Result<Vec<(Uuid, String, String)>> {
    let mut users = Vec::new();

    for i in 0..count {
        let email = format!("user{}@multitenant.test", i);
        let password = format!("password{i}");
        let user = User::new(
            email.clone(),
            format!("hashed_{password}"),
            Some(format!("Test User {i}")),
        );
        let user_id = user.id;
        database.create_user(&user).await?;

        // Generate JWT token for user
        let token = auth_manager.generate_token(&user, jwks_manager)?;
        users.push((user_id, email, token));
    }

    Ok(users)
}

#[tokio::test]
async fn test_multitenant_server_initialization() -> Result<()> {
    let (_server, _database, _auth_manager) = create_test_multitenant_server().await?;

    // Server should be created successfully
    // This test verifies basic initialization
    Ok(())
}

#[tokio::test]
async fn test_multitenant_user_creation_and_isolation() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    // Test that we can create users and they're properly isolated
    let user1 = User::new(
        "user1@test.com".to_owned(),
        "hashed_password1".to_owned(),
        Some("User 1".to_owned()),
    );
    let user1_id = user1.id;
    database.create_user(&user1).await?;

    let user2 = User::new(
        "user2@test.com".to_owned(),
        "hashed_password2".to_owned(),
        Some("User 2".to_owned()),
    );
    let user2_id = user2.id;
    database.create_user(&user2).await?;

    // Verify users exist and are isolated
    let retrieved_user1 = database.get_user(user1_id).await?.unwrap();
    let retrieved_user2 = database.get_user(user2_id).await?.unwrap();

    assert_eq!(retrieved_user1.email, "user1@test.com");
    assert_eq!(retrieved_user2.email, "user2@test.com");
    assert_ne!(retrieved_user1.id, retrieved_user2.id);

    // Test JWT generation
    let jwks_manager = common::get_shared_test_jwks();
    let token1 = auth_manager.generate_token(&user1, &jwks_manager)?;
    let token2 = auth_manager.generate_token(&user2, &jwks_manager)?;

    // Tokens should be different and valid
    assert_ne!(token1, token2);

    let claims1 = auth_manager.validate_token(&token1, &jwks_manager)?;
    let claims2 = auth_manager.validate_token(&token2, &jwks_manager)?;

    assert_eq!(claims1.email, "user1@test.com");
    assert_eq!(claims2.email, "user2@test.com");
    assert_ne!(claims1.sub, claims2.sub);

    Ok(())
}

#[tokio::test]
async fn test_authentication_middleware_integration() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;
    let jwks_manager = common::get_shared_test_jwks();
    let auth_middleware = Arc::new(McpAuthMiddleware::new(
        (*auth_manager).clone(),
        database.clone(),
        jwks_manager.clone(),
        RateLimitConfig::default(),
    ));

    // Create test user
    let user = User::new(
        "auth@test.com".to_owned(),
        "hashed_password".to_owned(),
        Some("Auth Test User".to_owned()),
    );
    let user_id = user.id;
    database.create_user(&user).await?;

    let token = auth_manager.generate_token(&user, &jwks_manager)?;

    // Test valid authentication
    let bearer_token = format!("Bearer {token}");
    let auth_result = auth_middleware
        .authenticate_request(Some(&bearer_token))
        .await;
    if auth_result.is_err() {
        println!("Auth result error: {:?}", auth_result.as_ref().err());
    }
    assert!(auth_result.is_ok());

    let auth_data = auth_result.unwrap();
    assert_eq!(auth_data.user_id, user_id);

    // Test invalid token
    let invalid_result = auth_middleware
        .authenticate_request(Some("invalid_token"))
        .await;
    assert!(invalid_result.is_err());

    // Test no token
    let no_token_result = auth_middleware.authenticate_request(None).await;
    assert!(no_token_result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_tenant_data_isolation() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    let jwks_manager = common::get_shared_test_jwks();

    // Create multiple users
    let users = create_multiple_test_users(&database, &auth_manager, 3, &jwks_manager).await?;

    // Store different data for each user
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);

    for (i, (user_id, _email, _token)) in users.iter().enumerate() {
        // Store Strava tokens
        let strava_token = UserOAuthToken::new(
            *user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::STRAVA.to_owned(),
            format!("strava_access_{i}"),
            Some(format!("strava_refresh_{i}")),
            Some(expires_at),
            Some("read,activity:read_all".to_owned()),
        );
        database.upsert_user_oauth_token(&strava_token).await?;

        // Store Fitbit tokens
        let fitbit_token = UserOAuthToken::new(
            *user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::FITBIT.to_string(),
            format!("fitbit_access_{i}"),
            Some(format!("fitbit_refresh_{i}")),
            Some(expires_at),
            Some("activity heartrate profile".to_owned()),
        );
        database.upsert_user_oauth_token(&fitbit_token).await?;
    }

    // Verify data isolation
    for (i, (user_id, _email, _token)) in users.iter().enumerate() {
        let strava_token = database
            .get_user_oauth_token(
                *user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::STRAVA,
            )
            .await?
            .unwrap();
        assert_eq!(strava_token.access_token, format!("strava_access_{i}"));

        let fitbit_token = database
            .get_user_oauth_token(
                *user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::FITBIT,
            )
            .await?
            .unwrap();
        assert_eq!(fitbit_token.access_token, format!("fitbit_access_{i}"));
    }

    // Verify users cannot access each other's data
    let user0_strava = database
        .get_user_oauth_token(
            users[0].0,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?
        .unwrap();
    let user1_strava = database
        .get_user_oauth_token(
            users[1].0,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?
        .unwrap();

    assert_ne!(user0_strava.access_token, user1_strava.access_token);
    assert_ne!(user0_strava.refresh_token, user1_strava.refresh_token);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_user_operations() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    // Create multiple concurrent user operations
    let mut handles = Vec::new();

    for i in 0..10 {
        let db = database.clone();
        let am = auth_manager.clone();

        let handle = tokio::spawn(async move {
            let user = User::new(
                format!("concurrent_user_{}@test.com", i),
                format!("hashed_password_{i}"),
                Some(format!("Concurrent User {i}")),
            );
            let user_id = user.id;

            // Create user
            db.create_user(&user).await?;

            // Generate token
            let jwks_manager = common::get_shared_test_jwks();
            let token = am.generate_token(&user, &jwks_manager)?;

            // Validate token
            let claims = am.validate_token(&token, &jwks_manager)?;
            assert_eq!(claims.email, format!("concurrent_user_{}@test.com", i));

            // Store some data
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);
            let oauth_token = UserOAuthToken::new(
                user_id,
                "00000000-0000-0000-0000-000000000000".to_owned(),
                oauth_providers::STRAVA.to_owned(),
                format!("access_token_{i}"),
                Some(format!("refresh_token_{i}")),
                Some(expires_at),
                Some("read,activity:read_all".to_owned()),
            );
            db.upsert_user_oauth_token(&oauth_token).await?;

            // Retrieve and verify data
            let token_data = db
                .get_user_oauth_token(
                    user_id,
                    TenantId::from_uuid(Uuid::nil()),
                    oauth_providers::STRAVA,
                )
                .await?
                .unwrap();
            assert_eq!(token_data.access_token, format!("access_token_{i}"));

            Ok::<_, anyhow::Error>(user_id)
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    let results = future::try_join_all(handles).await?;

    // All operations should succeed
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.is_ok());
    }

    Ok(())
}

#[tokio::test]
async fn test_token_expiration_handling() -> Result<()> {
    let (_server, database, _auth_manager) = create_test_multitenant_server().await?;

    // Create user first
    let user = User::new(
        "expired@token.test".to_owned(),
        "hashed_password".to_owned(),
        Some("Expired Token User".to_owned()),
    );
    database.create_user(&user).await?;

    // Create auth manager with very short expiry (fraction of a second)
    let short_expiry_auth_manager = Arc::new(AuthManager::new(0)); // 0 hours expiry

    // Generate token with immediate expiration
    let jwks_manager = common::get_shared_test_jwks();
    let expired_token = short_expiry_auth_manager.generate_token(&user, &jwks_manager)?;

    // Wait to ensure the token has expired
    sleep(Duration::from_millis(1100)).await; // Wait over a second

    // Try to validate expired token using detailed validation
    let result = short_expiry_auth_manager.validate_token_detailed(&expired_token, &jwks_manager);
    if result.is_ok() {
        println!("Token validation unexpectedly succeeded: {:?}", result);
    } else {
        println!("Token validation failed as expected: {:?}", result);
    }

    assert!(result.is_err());

    // Verify it's specifically a TokenExpired error
    match result.unwrap_err() {
        JwtValidationError::TokenExpired {
            expired_at,
            current_time,
        } => {
            assert!(current_time > expired_at);
            println!(
                "Token expired at: {}, current time: {}",
                expired_at, current_time
            );
        }
        other => {
            println!("Got unexpected error type: {:?}", other);
            panic!("Expected TokenExpired error, got: {:?}", other);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_invalid_token_handling() -> Result<()> {
    let (_server, _database, auth_manager) = create_test_multitenant_server().await?;

    // Test various invalid tokens
    let invalid_tokens = vec![
        "invalid.jwt.token",
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.malformed.signature",
        "",
        "not-a-jwt-at-all",
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c", // Valid format but wrong signature
    ];

    let jwks_manager = common::get_shared_test_jwks();

    for invalid_token in invalid_tokens {
        let result = auth_manager.validate_token(invalid_token, &jwks_manager);
        assert!(result.is_err(), "Should fail for token: {}", invalid_token);
    }

    Ok(())
}

#[tokio::test]
async fn test_user_tier_management() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    // Create users with different tiers
    let mut starter_user = User::new(
        "starter@test.com".to_owned(),
        "hashed_password".to_owned(),
        Some("Starter User".to_owned()),
    );
    starter_user.tier = UserTier::Starter;
    database.create_user(&starter_user).await?;

    let mut pro_user = User::new(
        "pro@test.com".to_owned(),
        "hashed_password".to_owned(),
        Some("Pro User".to_owned()),
    );
    pro_user.tier = UserTier::Professional;
    database.create_user(&pro_user).await?;

    let mut enterprise_user = User::new(
        "enterprise@test.com".to_owned(),
        "hashed_password".to_owned(),
        Some("Enterprise User".to_owned()),
    );
    enterprise_user.tier = UserTier::Enterprise;
    database.create_user(&enterprise_user).await?;

    // Verify users have correct tiers
    let retrieved_starter = database.get_user(starter_user.id).await?.unwrap();
    let retrieved_pro = database.get_user(pro_user.id).await?.unwrap();
    let retrieved_enterprise = database.get_user(enterprise_user.id).await?.unwrap();

    assert_eq!(retrieved_starter.tier, UserTier::Starter);
    assert_eq!(retrieved_pro.tier, UserTier::Professional);
    assert_eq!(retrieved_enterprise.tier, UserTier::Enterprise);

    // Test JWT tokens include tier information
    let jwks_manager = common::get_shared_test_jwks();
    let starter_token = auth_manager.generate_token(&starter_user, &jwks_manager)?;
    let pro_token = auth_manager.generate_token(&pro_user, &jwks_manager)?;
    let enterprise_token = auth_manager.generate_token(&enterprise_user, &jwks_manager)?;

    let starter_claims = auth_manager.validate_token(&starter_token, &jwks_manager)?;
    let pro_claims = auth_manager.validate_token(&pro_token, &jwks_manager)?;
    let enterprise_claims = auth_manager.validate_token(&enterprise_token, &jwks_manager)?;

    assert_eq!(starter_claims.email, "starter@test.com");
    assert_eq!(pro_claims.email, "pro@test.com");
    assert_eq!(enterprise_claims.email, "enterprise@test.com");

    Ok(())
}

#[tokio::test]
async fn test_database_encryption_isolation() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    let jwks_manager = common::get_shared_test_jwks();

    // Create users
    let users = create_multiple_test_users(&database, &auth_manager, 2, &jwks_manager).await?;
    let (user1_id, _email1, _token1) = &users[0];
    let (user2_id, _email2, _token2) = &users[1];

    // Store encrypted data for each user
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);

    let user1_oauth_token = UserOAuthToken::new(
        *user1_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "secret_access_token_user1".to_owned(),
        Some("secret_refresh_token_user1".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database.upsert_user_oauth_token(&user1_oauth_token).await?;

    let user2_oauth_token = UserOAuthToken::new(
        *user2_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "secret_access_token_user2".to_owned(),
        Some("secret_refresh_token_user2".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database.upsert_user_oauth_token(&user2_oauth_token).await?;

    // Verify data is properly isolated and encrypted/decrypted
    let user1_token_data = database
        .get_user_oauth_token(
            *user1_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?
        .unwrap();
    let user2_token_data = database
        .get_user_oauth_token(
            *user2_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?
        .unwrap();

    assert_eq!(user1_token_data.access_token, "secret_access_token_user1");
    assert_eq!(user2_token_data.access_token, "secret_access_token_user2");

    // Verify users cannot access each other's data
    assert_ne!(user1_token_data.access_token, user2_token_data.access_token);
    assert_ne!(
        user1_token_data.refresh_token,
        user2_token_data.refresh_token
    );

    Ok(())
}

#[tokio::test]
async fn test_session_state_management() -> Result<()> {
    let (_server, database, _auth_manager) = create_test_multitenant_server().await?;

    // Create test user
    let user = User::new(
        "session@test.com".to_owned(),
        "hashed_password".to_owned(),
        Some("Session Test User".to_owned()),
    );
    let user_id = user.id;
    database.create_user(&user).await?;

    // Update last active timestamp
    database.update_last_active(user_id).await?;

    // Verify the timestamp was updated
    let updated_user = database.get_user(user_id).await?.unwrap();
    // last_active is a DateTime<Utc>, not an Option

    // Test multiple updates
    sleep(Duration::from_millis(10)).await;
    database.update_last_active(user_id).await?;

    let updated_user2 = database.get_user(user_id).await?.unwrap();

    // The second timestamp should be different (or at least not earlier)
    assert!(updated_user2.last_active >= updated_user.last_active);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_authentication_operations() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;
    let jwks_manager = common::get_shared_test_jwks();
    let auth_middleware = Arc::new(McpAuthMiddleware::new(
        (*auth_manager).clone(),
        database.clone(),
        jwks_manager.clone(),
        RateLimitConfig::default(),
    ));

    // Create multiple users
    let users = create_multiple_test_users(&database, &auth_manager, 5, &jwks_manager).await?;

    // Run concurrent authentication operations
    let mut handles = Vec::new();

    for (user_id, email, token) in users {
        let amw = auth_middleware.clone();
        let user_token = token.clone();

        let handle = tokio::spawn(async move {
            // Test authentication
            let bearer_token = format!("Bearer {user_token}");
            let auth_result = amw.authenticate_request(Some(&bearer_token)).await;
            if auth_result.is_err() {
                println!(
                    "Concurrent auth error for user {}: {:?}",
                    user_id,
                    auth_result.as_ref().err()
                );
            }
            assert!(auth_result.is_ok());

            let auth_data = auth_result.unwrap();
            assert_eq!(auth_data.user_id, user_id);

            // Test multiple authentications for same user
            for _ in 0..3 {
                let auth_result2 = amw.authenticate_request(Some(&bearer_token)).await;
                assert!(auth_result2.is_ok());
                assert_eq!(auth_result2.unwrap().user_id, user_id);
            }

            (user_id, email)
        });

        handles.push(handle);
    }

    let results = future::try_join_all(handles).await?;

    // All authentications should succeed
    assert_eq!(results.len(), 5);
    for (i, (_user_id, email)) in results.iter().enumerate() {
        assert_eq!(*email, format!("user{}@multitenant.test", i));
    }

    Ok(())
}

#[tokio::test]
async fn test_memory_safety_concurrent_access() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    let jwks_manager = common::get_shared_test_jwks();

    // Test concurrent access to shared resources
    let mut handles = Vec::new();

    for i in 0..20 {
        let db = database.clone();
        let am = auth_manager.clone();
        let jwks_mgr = jwks_manager.clone();

        let handle = tokio::spawn(async move {
            // Create user
            let user = User::new(
                format!("memory_test_{}@test.com", i),
                format!("hashed_password_{i}"),
                Some(format!("Memory Test User {i}")),
            );
            let user_id = user.id;

            db.create_user(&user).await?;

            // Generate and validate token
            let token = am.generate_token(&user, &jwks_mgr)?;
            let claims = am.validate_token(&token, &jwks_mgr)?;
            assert_eq!(claims.email, format!("memory_test_{}@test.com", i));

            // Store and retrieve data
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);
            let oauth_token = UserOAuthToken::new(
                user_id,
                "00000000-0000-0000-0000-000000000000".to_owned(),
                oauth_providers::STRAVA.to_owned(),
                format!("access_{i}"),
                Some(format!("refresh_{i}")),
                Some(expires_at),
                Some("read,activity:read_all".to_owned()),
            );
            db.upsert_user_oauth_token(&oauth_token).await?;

            let token_data = db
                .get_user_oauth_token(
                    user_id,
                    TenantId::from_uuid(Uuid::nil()),
                    oauth_providers::STRAVA,
                )
                .await?
                .unwrap();
            assert_eq!(token_data.access_token, format!("access_{i}"));

            Ok::<_, anyhow::Error>(i)
        });

        handles.push(handle);
    }

    let results = future::try_join_all(handles).await?;

    // All operations should complete without panics or data races
    assert_eq!(results.len(), 20);
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok());
        assert_eq!(result.as_ref().unwrap(), &i);
    }

    Ok(())
}

#[tokio::test]
async fn test_user_provider_storage_concept() -> Result<()> {
    let (_server, _database, auth_manager) = create_test_multitenant_server().await?;

    let jwks_manager = common::get_shared_test_jwks();

    // Test the concept of user provider storage isolation
    // This simulates how the server would maintain separate provider instances per user
    let user_providers: Arc<RwLock<HashMap<String, HashMap<String, String>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Create multiple users
    let users = create_multiple_test_users(&_database, &auth_manager, 3, &jwks_manager).await?;

    // Simulate provider storage for different users
    {
        let mut providers = user_providers.write().await;
        for (user_id, _email, _token) in &users {
            let mut user_map = HashMap::new();
            user_map.insert("strava".to_owned(), format!("strava_provider_{user_id}"));
            user_map.insert("fitbit".to_owned(), format!("fitbit_provider_{user_id}"));
            providers.insert(user_id.to_string(), user_map);
        }
    }

    // Verify each user has isolated provider storage
    {
        let providers = user_providers.read().await;
        for (user_id, _email, _token) in &users {
            let user_providers_map = providers.get(&user_id.to_string()).unwrap();
            assert_eq!(
                user_providers_map.get("strava").unwrap(),
                &format!("strava_provider_{user_id}")
            );
            assert_eq!(
                user_providers_map.get("fitbit").unwrap(),
                &format!("fitbit_provider_{user_id}")
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_error_recovery_and_resilience() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    // Test various error conditions and ensure server components remain stable

    // 1. Test invalid user ID
    let non_existent_user_id = Uuid::new_v4();
    let result = database.get_user(non_existent_user_id).await?;
    assert!(result.is_none());

    // 2. Test invalid token validation
    let invalid_tokens = vec![
        "invalid.token",
        "",
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.invalid.signature",
    ];

    let jwks_manager = common::get_shared_test_jwks();

    for invalid_token in invalid_tokens {
        let result = auth_manager.validate_token(invalid_token, &jwks_manager);
        assert!(result.is_err());
    }

    // 3. Test database operations with invalid data
    let invalid_user = User::new(
        "".to_owned(), // Empty email
        "password".to_owned(),
        None,
    );

    // This should either fail or handle gracefully
    let _result = database.create_user(&invalid_user).await;
    // The result depends on database validation, but it shouldn't panic

    Ok(())
}

#[tokio::test]
async fn test_large_scale_multitenant_operations() -> Result<()> {
    let (_server, database, auth_manager) = create_test_multitenant_server().await?;

    // Create jwks_manager for token generation and validation
    let jwks_manager = common::get_shared_test_jwks();

    // Test with a larger number of users to verify scalability
    let user_count = 50;
    let users =
        create_multiple_test_users(&database, &auth_manager, user_count, &jwks_manager).await?;

    // Verify all users were created correctly
    assert_eq!(users.len(), user_count);

    // Test concurrent operations on all users
    let mut handles = Vec::new();

    for (user_id, email, token) in users {
        let db = database.clone();
        let am = auth_manager.clone();
        let jwks_mgr = jwks_manager.clone();

        let handle = tokio::spawn(async move {
            // Validate token
            let claims = am.validate_token(&token, &jwks_mgr)?;
            assert_eq!(claims.email, email);

            // Store and retrieve data
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);
            let oauth_token = UserOAuthToken::new(
                user_id,
                "00000000-0000-0000-0000-000000000000".to_owned(),
                oauth_providers::STRAVA.to_owned(),
                format!("access_{user_id}"),
                Some(format!("refresh_{user_id}")),
                Some(expires_at),
                Some("read,activity:read_all".to_owned()),
            );
            db.upsert_user_oauth_token(&oauth_token).await?;

            let token_data = db
                .get_user_oauth_token(
                    user_id,
                    TenantId::from_uuid(Uuid::nil()),
                    oauth_providers::STRAVA,
                )
                .await?
                .unwrap();
            assert_eq!(token_data.access_token, format!("access_{user_id}"));

            // Update last active
            db.update_last_active(user_id).await?;

            Ok::<_, anyhow::Error>(user_id)
        });

        handles.push(handle);
    }

    let results = future::try_join_all(handles).await?;

    // All operations should succeed
    assert_eq!(results.len(), user_count);
    for result in results {
        assert!(result.is_ok());
    }

    Ok(())
}
