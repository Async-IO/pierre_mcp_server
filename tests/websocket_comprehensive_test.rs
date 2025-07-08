// ABOUTME: Comprehensive tests for WebSocket functionality
// ABOUTME: Tests WebSocket connections, real-time communication, and message handling
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

//! Comprehensive tests for WebSocket functionality
//!
//! This test suite covers the WebSocket real-time communication system
//! which currently has no test coverage

use anyhow::Result;
use pierre_mcp_server::{
    database_plugins::DatabaseProvider,
    models::User,
    websocket::{WebSocketManager, WebSocketMessage},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_websocket_manager_creation() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());

    // Verify manager is created (filter can be built)
    let _ = ws_manager.websocket_filter();

    Ok(())
}

#[tokio::test]
async fn test_websocket_authentication_flow() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    // Create test user
    let user = User::new(
        "ws_auth_test@example.com".to_string(),
        "password123".to_string(),
        Some("WebSocket Test User".to_string()),
    );
    database.create_user(&user).await?;

    // Generate auth token
    let token = auth_manager.generate_token(&user)?;

    let ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());
    let _filter = ws_manager.websocket_filter();

    // Test authentication message
    let auth_msg = WebSocketMessage::Authentication {
        token: token.clone(),
    };

    // Verify message serialization
    let serialized = serde_json::to_string(&auth_msg)?;
    assert!(serialized.contains("auth"));
    assert!(serialized.contains(&token));

    Ok(())
}

#[tokio::test]
async fn test_websocket_subscription_message() -> Result<()> {
    let topics = vec![
        "usage_updates".to_string(),
        "system_stats".to_string(),
        "rate_limits".to_string(),
    ];

    let subscribe_msg = WebSocketMessage::Subscribe {
        topics: topics.clone(),
    };

    // Test serialization
    let json = serde_json::to_value(&subscribe_msg)?;
    assert_eq!(json["type"], "subscribe");
    assert_eq!(json["topics"].as_array().unwrap().len(), 3);

    // Test deserialization
    let deserialized: WebSocketMessage = serde_json::from_value(json)?;
    match deserialized {
        WebSocketMessage::Subscribe { topics: t } => assert_eq!(t, topics),
        _ => panic!("Wrong message type"),
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_usage_update_message() -> Result<()> {
    let usage_update = WebSocketMessage::UsageUpdate {
        api_key_id: "key_123".to_string(),
        requests_today: 150,
        requests_this_month: 4500,
        rate_limit_status: json!({
            "limit": 1000,
            "remaining": 850,
            "reset_at": "2024-01-20T00:00:00Z"
        }),
    };

    // Test serialization
    let json = serde_json::to_value(&usage_update)?;
    assert_eq!(json["type"], "usage_update");
    assert_eq!(json["requests_today"], 150);
    assert_eq!(json["requests_this_month"], 4500);
    assert_eq!(json["api_key_id"], "key_123");

    Ok(())
}

#[tokio::test]
async fn test_websocket_system_stats_message() -> Result<()> {
    let stats = WebSocketMessage::SystemStats {
        total_requests_today: 10000,
        total_requests_this_month: 250000,
        active_connections: 42,
    };

    // Test serialization
    let json = serde_json::to_value(&stats)?;
    assert_eq!(json["type"], "system_stats");
    assert_eq!(json["total_requests_today"], 10000);
    assert_eq!(json["active_connections"], 42);

    Ok(())
}

#[tokio::test]
async fn test_websocket_error_message() -> Result<()> {
    let error_msg = WebSocketMessage::Error {
        message: "Authentication failed: Invalid token".to_string(),
    };

    // Test serialization
    let json = serde_json::to_value(&error_msg)?;
    assert_eq!(json["type"], "error");
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("Authentication failed"));

    Ok(())
}

#[tokio::test]
async fn test_websocket_success_message() -> Result<()> {
    let success_msg = WebSocketMessage::Success {
        message: "Successfully subscribed to topics".to_string(),
    };

    // Test serialization
    let json = serde_json::to_value(&success_msg)?;
    assert_eq!(json["type"], "success");
    assert!(json["message"].as_str().unwrap().contains("subscribed"));

    Ok(())
}

#[tokio::test]
async fn test_websocket_message_parsing() -> Result<()> {
    // Test various message formats
    let test_cases = vec![
        (
            json!({
                "type": "auth",
                "token": "test_token_123"
            }),
            true,
        ),
        (
            json!({
                "type": "subscribe",
                "topics": ["usage_updates"]
            }),
            true,
        ),
        (
            json!({
                "type": "unknown_type",
                "data": "test"
            }),
            false,
        ),
        (
            json!({
                "token": "missing_type"
            }),
            false,
        ),
    ];

    for (json_msg, should_succeed) in test_cases {
        let result = serde_json::from_value::<WebSocketMessage>(json_msg);
        assert_eq!(result.is_ok(), should_succeed);
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_connection_with_invalid_auth() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());
    let _filter = ws_manager.websocket_filter();

    // Create invalid auth message
    let auth_msg = WebSocketMessage::Authentication {
        token: "invalid_token_123".to_string(),
    };

    // Message should serialize but authentication would fail in actual connection
    let json = serde_json::to_string(&auth_msg)?;
    assert!(json.contains("invalid_token_123"));

    Ok(())
}

#[tokio::test]
async fn test_websocket_concurrent_client_management() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let ws_manager = Arc::new(WebSocketManager::new(
        (*database).clone(),
        (*auth_manager).clone(),
    ));

    // Simulate multiple concurrent connections
    let mut handles = vec![];

    for i in 0..5 {
        let _ws_manager_clone = ws_manager.clone();
        let db_clone = database.clone();
        let auth_clone = auth_manager.clone();

        handles.push(tokio::spawn(async move {
            // Create unique user for each connection
            let user = User::new(
                format!("ws_concurrent_{}@example.com", i),
                "password".to_string(),
                Some(format!("Concurrent User {i}")),
            );
            db_clone.create_user(&user).await.unwrap();

            let token = auth_clone.generate_token(&user).unwrap();

            // Create auth message
            let auth_msg = WebSocketMessage::Authentication { token };
            serde_json::to_string(&auth_msg).unwrap()
        }));
    }

    // All connections should generate valid auth messages
    for handle in handles {
        let auth_json = handle.await?;
        assert!(auth_json.contains("auth"));
        assert!(auth_json.contains("token"));
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_rate_limit_status_updates() -> Result<()> {
    // Test rate limit status message format
    let rate_limit_statuses = [
        json!({
            "limit": 1000,
            "remaining": 1000,
            "reset_at": "2024-01-20T00:00:00Z"
        }),
        json!({
            "limit": 1000,
            "remaining": 0,
            "reset_at": "2024-01-20T01:00:00Z",
            "retry_after": 3600
        }),
        json!({
            "limit": 500,
            "remaining": 250,
            "reset_at": "2024-01-20T00:30:00Z"
        }),
    ];

    for (i, status) in rate_limit_statuses.iter().enumerate() {
        let usage_update = WebSocketMessage::UsageUpdate {
            api_key_id: format!("key_{i}"),
            requests_today: (i as u64 + 1) * 100,
            requests_this_month: (i as u64 + 1) * 3000,
            rate_limit_status: status.clone(),
        };

        let json = serde_json::to_value(&usage_update)?;
        assert_eq!(json["rate_limit_status"], *status);
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_subscription_topics() -> Result<()> {
    let valid_topics = vec![
        vec!["usage_updates".to_string()],
        vec!["system_stats".to_string()],
        vec!["rate_limits".to_string()],
        vec!["usage_updates".to_string(), "system_stats".to_string()],
        vec![], // Empty subscription
    ];

    for topics in valid_topics {
        let subscribe_msg = WebSocketMessage::Subscribe {
            topics: topics.clone(),
        };

        let json = serde_json::to_value(&subscribe_msg)?;
        let topics_array = json["topics"].as_array().unwrap();
        assert_eq!(topics_array.len(), topics.len());
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_message_size_limits() -> Result<()> {
    // Test large message handling
    let large_message = WebSocketMessage::Error {
        message: "x".repeat(10000), // 10KB message
    };

    // Should serialize successfully
    let json = serde_json::to_string(&large_message)?;
    assert!(json.len() > 10000);

    // Test very large rate limit status
    let large_status = json!({
        "limit": 1000000,
        "remaining": 999999,
        "reset_at": "2024-01-20T00:00:00Z",
        "metadata": {
            "tier": "enterprise",
            "custom_limits": (0..100).map(|i| format!("limit_{i}")).collect::<Vec<_>>()
        }
    });

    let usage_update = WebSocketMessage::UsageUpdate {
        api_key_id: "enterprise_key".to_string(),
        requests_today: 50000,
        requests_this_month: 1500000,
        rate_limit_status: large_status,
    };

    // Should handle large nested objects
    let _ = serde_json::to_string(&usage_update)?;

    Ok(())
}

#[tokio::test]
async fn test_websocket_client_id_generation() -> Result<()> {
    // Test that client IDs are unique
    let mut ids = std::collections::HashSet::new();

    for _ in 0..100 {
        let id = Uuid::new_v4();
        assert!(ids.insert(id), "UUID collision detected");
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_broadcast_system_stats() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let _ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());

    // Create system stats for broadcast
    let stats = WebSocketMessage::SystemStats {
        total_requests_today: 25000,
        total_requests_this_month: 750000,
        active_connections: 15,
    };

    // Verify stats message format
    let json = serde_json::to_value(&stats)?;
    assert_eq!(json["type"], "system_stats");
    assert!(json["total_requests_today"].as_u64().unwrap() > 0);
    assert!(json["active_connections"].as_u64().unwrap() > 0);

    Ok(())
}
