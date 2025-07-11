// ABOUTME: Integration tests for multi-tenant architecture and functionality
// ABOUTME: Tests tenant isolation, data separation, and multi-tenant workflows
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
    clippy::manual_let_else,
    clippy::manual_ok_or,
    clippy::manual_string_new,
    clippy::many_single_char_names,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
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
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_add_assign,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::trait_duplication_in_bounds,
    clippy::transmute_ptr_to_ptr,
    clippy::trivially_copy_pass_by_ref,
    clippy::tuple_array_conversions,
    clippy::unchecked_duration_subtraction,
    clippy::unicode_not_nfc,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_box_returns,
    clippy::unnecessary_struct_initialization,
    clippy::unnecessary_to_owned,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unused_peekable,
    clippy::unused_rounding,
    clippy::useless_let_if_seq,
    clippy::verbose_bit_mask,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values
)]

use anyhow::Result;
use pierre_mcp_server::{
    auth::{generate_jwt_secret, AuthManager},
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    routes::{AuthRoutes, LoginRequest, RegisterRequest},
};
use tempfile::TempDir;
use uuid::Uuid;

/// Test full multi-tenant authentication flow
#[tokio::test]
async fn test_multitenant_auth_flow() -> Result<()> {
    // Setup
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();
    let jwt_secret = generate_jwt_secret().to_vec();

    let database = Database::new(&database_url, encryption_key).await?;
    let auth_manager = AuthManager::new(jwt_secret, 24);
    let auth_routes = AuthRoutes::new(database.clone(), auth_manager.clone());

    // Test user registration
    let register_request = RegisterRequest {
        email: "test@multitenant.com".to_string(),
        password: "securepassword123".to_string(),
        display_name: Some("Multi-Tenant User".to_string()),
    };

    let register_response = auth_routes.register(register_request).await?;
    assert!(!register_response.user_id.is_empty());
    assert_eq!(register_response.message, "User registered successfully");

    // Parse user ID
    let user_id = Uuid::parse_str(&register_response.user_id)?;

    // Verify user exists in database
    let user = database.get_user(user_id).await?.unwrap();
    assert_eq!(user.email, "test@multitenant.com");
    assert_eq!(user.display_name, Some("Multi-Tenant User".to_string()));
    assert!(user.is_active);

    // Test user login
    let login_request = LoginRequest {
        email: "test@multitenant.com".to_string(),
        password: "securepassword123".to_string(),
    };

    let login_response = auth_routes.login(login_request).await?;
    assert!(!login_response.jwt_token.is_empty());
    assert_eq!(login_response.user.email, "test@multitenant.com");
    assert_eq!(login_response.user.user_id, register_response.user_id);

    // Test JWT token validation
    let claims = auth_manager.validate_token(&login_response.jwt_token)?;
    assert_eq!(claims.email, "test@multitenant.com");
    assert_eq!(claims.sub, register_response.user_id);

    // Test duplicate registration fails
    let duplicate_request = RegisterRequest {
        email: "test@multitenant.com".to_string(),
        password: "differentpassword".to_string(),
        display_name: None,
    };

    let duplicate_result = auth_routes.register(duplicate_request).await;
    assert!(duplicate_result.is_err());
    assert!(duplicate_result
        .unwrap_err()
        .to_string()
        .contains("already exists"));

    // Test login with wrong password fails
    let wrong_password_request = LoginRequest {
        email: "test@multitenant.com".to_string(),
        password: "wrongpassword".to_string(),
    };

    let wrong_password_result = auth_routes.login(wrong_password_request).await;
    assert!(wrong_password_result.is_err());
    assert!(wrong_password_result
        .unwrap_err()
        .to_string()
        .contains("Invalid email or password"));

    Ok(())
}

/// Test database encryption functionality
#[tokio::test]
async fn test_database_encryption() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("encryption_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();

    let database = Database::new(&database_url, encryption_key).await?;

    // Create user
    let user = pierre_mcp_server::models::User::new(
        "encryption@test.com".to_string(),
        "bcrypt_hashed_password".to_string(),
        Some("Encryption Test".to_string()),
    );
    let user_id = database.create_user(&user).await?;

    // Store encrypted Strava token
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);
    database
        .update_strava_token(
            user_id,
            "secret_access_token_123",
            "secret_refresh_token_456",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await?;

    // Retrieve and decrypt token
    let decrypted_token = database.get_strava_token(user_id).await?.unwrap();
    assert_eq!(decrypted_token.access_token, "secret_access_token_123");
    assert_eq!(decrypted_token.refresh_token, "secret_refresh_token_456");
    assert_eq!(decrypted_token.scope, "read,activity:read_all");

    Ok(())
}

/// Test JWT authentication edge cases
#[tokio::test]
async fn test_jwt_edge_cases() -> Result<()> {
    let auth_manager = AuthManager::new(generate_jwt_secret().to_vec(), 1); // 1 hour expiry

    let user = pierre_mcp_server::models::User::new(
        "jwt@test.com".to_string(),
        "hashed_password".to_string(),
        Some("JWT Test".to_string()),
    );

    // Test token generation and validation
    let token = auth_manager.generate_token(&user)?;
    let claims = auth_manager.validate_token(&token)?;
    assert_eq!(claims.email, "jwt@test.com");
    assert_eq!(claims.sub, user.id.to_string());

    // Test token refresh
    let refreshed_token = auth_manager.refresh_token(&token, &user)?;
    let refreshed_claims = auth_manager.validate_token(&refreshed_token)?;
    assert_eq!(refreshed_claims.email, claims.email);
    assert_eq!(refreshed_claims.sub, claims.sub);

    // Test invalid token
    let invalid_token = "invalid.token.here";
    let invalid_result = auth_manager.validate_token(invalid_token);
    assert!(invalid_result.is_err());

    // Test malformed token
    let malformed_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.malformed.signature";
    let malformed_result = auth_manager.validate_token(malformed_token);
    assert!(malformed_result.is_err());

    Ok(())
}

/// Test user isolation in multi-tenant database
#[tokio::test]
async fn test_user_isolation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("isolation_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();

    let database = Database::new(&database_url, encryption_key).await?;

    // Create two users
    let user1 = pierre_mcp_server::models::User::new(
        "user1@isolation.test".to_string(),
        "password1".to_string(),
        Some("User One".to_string()),
    );
    let user1_id = database.create_user(&user1).await?;

    let user2 = pierre_mcp_server::models::User::new(
        "user2@isolation.test".to_string(),
        "password2".to_string(),
        Some("User Two".to_string()),
    );
    let user2_id = database.create_user(&user2).await?;

    // Store tokens for each user
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);

    database
        .update_strava_token(
            user1_id,
            "user1_access_token",
            "user1_refresh_token",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await?;

    database
        .update_strava_token(
            user2_id,
            "user2_access_token",
            "user2_refresh_token",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await?;

    // Verify user isolation - each user can only access their own tokens
    let user1_token = database.get_strava_token(user1_id).await?.unwrap();
    assert_eq!(user1_token.access_token, "user1_access_token");

    let user2_token = database.get_strava_token(user2_id).await?.unwrap();
    assert_eq!(user2_token.access_token, "user2_access_token");

    // Verify users cannot access each other's data
    assert_ne!(user1_token.access_token, user2_token.access_token);
    assert_ne!(user1_token.refresh_token, user2_token.refresh_token);

    Ok(())
}

/// Test input validation
#[tokio::test]
async fn test_input_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("validation_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();
    let jwt_secret = generate_jwt_secret().to_vec();

    let database = Database::new(&database_url, encryption_key).await?;
    let auth_manager = AuthManager::new(jwt_secret, 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);

    // Test invalid email formats
    let invalid_emails = vec!["not-an-email", "@domain.com", "user@", "user", "a@b", ""];

    for invalid_email in invalid_emails {
        let request = RegisterRequest {
            email: invalid_email.to_string(),
            password: "validpassword123".to_string(),
            display_name: None,
        };

        let result = auth_routes.register(request).await;
        assert!(
            result.is_err(),
            "Should reject invalid email: {}",
            invalid_email
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid email format"));
    }

    // Test short passwords
    let short_passwords = vec!["1234567", "short", "", "a"];

    for short_password in short_passwords {
        let request = RegisterRequest {
            email: "test@valid.com".to_string(),
            password: short_password.to_string(),
            display_name: None,
        };

        let result = auth_routes.register(request).await;
        assert!(
            result.is_err(),
            "Should reject short password: {}",
            short_password
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least 8 characters"));
    }

    // Test valid inputs
    let valid_request = RegisterRequest {
        email: "valid@email.com".to_string(),
        password: "validpassword123".to_string(),
        display_name: Some("Valid User".to_string()),
    };

    let result = auth_routes.register(valid_request).await;
    assert!(result.is_ok(), "Should accept valid inputs");

    Ok(())
}
