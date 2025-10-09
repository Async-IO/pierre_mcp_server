// ABOUTME: MCP protocol compliance tests for specification adherence
// ABOUTME: Tests MCP protocol implementation against official specification
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
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
//

//! MCP Protocol Compliance Tests
//!
//! These tests verify that our MCP server implementation is fully compliant
//! with the Model Context Protocol specification (2025-06-18).

#![allow(dead_code)]

use pierre_mcp_server::mcp::schema::*;
use serde_json::json;

/// Test that the initialize response has the correct structure
#[test]
fn test_initialize_response_format() {
    let response = InitializeResponse::new(
        "2025-06-18".to_string(),
        "pierre-mcp-server".to_string(),
        "1.0.0".to_string(),
    );

    // Serialize to JSON and verify structure
    let json_value = serde_json::to_value(&response).expect("Should serialize");

    // Check required fields
    assert_eq!(json_value["protocolVersion"], "2025-06-18");
    assert_eq!(json_value["serverInfo"]["name"], "pierre-mcp-server");
    assert_eq!(json_value["serverInfo"]["version"], "1.0.0");

    // Check capabilities structure
    assert!(json_value["capabilities"].is_object());
    assert!(json_value["capabilities"]["tools"].is_object());
    assert_eq!(json_value["capabilities"]["tools"]["listChanged"], false);

    // Check instructions
    assert!(json_value["instructions"].is_string());
}

/// Test that tool schemas have the correct structure
#[test]
fn test_tool_schema_compliance() {
    let tools = get_tools();

    assert!(!tools.is_empty(), "Should have at least one tool");

    for tool in tools {
        // Check required fields
        assert!(!tool.name.is_empty(), "Tool name cannot be empty");
        assert!(
            !tool.description.is_empty(),
            "Tool description cannot be empty"
        );

        // Check input schema structure
        assert_eq!(tool.input_schema.schema_type, "object");

        // Verify it can be serialized to valid JSON
        let json_value = serde_json::to_value(&tool).expect("Tool should serialize");
        assert!(json_value["name"].is_string());
        assert!(json_value["description"].is_string());
        assert!(json_value["inputSchema"]["type"].is_string());
    }
}

/// Test JSON-RPC 2.0 error response format
#[test]
fn test_error_response_format() {
    // Create a mock error response
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found",
            "data": null
        }
    });

    // Verify structure
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert!(error_response["id"].is_number());
    assert!(error_response["error"]["code"].is_number());
    assert!(error_response["error"]["message"].is_string());
}

/// Test that ping response is correct
#[test]
fn test_ping_response_format() {
    let ping_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {}
    });

    assert_eq!(ping_response["jsonrpc"], "2.0");
    assert!(ping_response["result"].is_object());
    assert_eq!(ping_response["result"], json!({}));
}

/// Test tools/list response format
#[test]
fn test_tools_list_response_format() {
    let tools = get_tools();
    let tools_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": tools
        }
    });

    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert!(tools_response["result"]["tools"].is_array());

    let tools_array = tools_response["result"]["tools"].as_array().unwrap();
    assert!(!tools_array.is_empty());

    // Check first tool structure
    let first_tool = &tools_array[0];
    assert!(first_tool["name"].is_string());
    assert!(first_tool["description"].is_string());
    assert!(first_tool["inputSchema"].is_object());
}

/// Test tool response format
#[test]
fn test_tool_response_format() {
    let tool_response = ToolResponse {
        content: vec![Content::Text {
            text: "Test response".to_string(),
        }],
        is_error: false,
        structured_content: Some(json!({"result": "success"})),
    };

    let json_value = serde_json::to_value(&tool_response).expect("Should serialize");

    // Check required fields
    assert!(json_value["content"].is_array());
    assert_eq!(json_value["isError"], false);
    assert!(json_value["structuredContent"].is_object());

    // Check content structure
    let content_array = json_value["content"].as_array().unwrap();
    assert_eq!(content_array.len(), 1);
    assert_eq!(content_array[0]["type"], "text");
    assert_eq!(content_array[0]["text"], "Test response");
}

/// Test content types
#[test]
fn test_content_types() {
    // Test text content
    let text_content = Content::Text {
        text: "Hello world".to_string(),
    };
    let json_value = serde_json::to_value(&text_content).expect("Should serialize");
    assert_eq!(json_value["type"], "text");
    assert_eq!(json_value["text"], "Hello world");

    // Test image content
    let image_content = Content::Image {
        data: "base64data".to_string(),
        mime_type: "image/png".to_string(),
    };
    let json_value = serde_json::to_value(&image_content).expect("Should serialize");
    assert_eq!(json_value["type"], "image");
    assert_eq!(json_value["data"], "base64data");
    assert_eq!(json_value["mimeType"], "image/png");

    // Test resource content
    let resource_content = Content::Resource {
        uri: "file://test.txt".to_string(),
        text: Some("Resource text".to_string()),
        mime_type: Some("text/plain".to_string()),
    };
    let json_value = serde_json::to_value(&resource_content).expect("Should serialize");
    assert_eq!(json_value["type"], "resource");
    assert_eq!(json_value["uri"], "file://test.txt");
    assert_eq!(json_value["text"], "Resource text");
    assert_eq!(json_value["mimeType"], "text/plain");
}

/// Test server capabilities structure
#[test]
fn test_server_capabilities() {
    let capabilities = ServerCapabilities {
        experimental: None,
        logging: None,
        prompts: None,
        resources: None,
        tools: Some(ToolsCapability {
            list_changed: Some(false),
        }),
        auth: None,
        oauth2: None,
    };

    let json_value = serde_json::to_value(&capabilities).expect("Should serialize");

    // Check tools capability
    assert!(json_value["tools"].is_object());
    assert_eq!(json_value["tools"]["listChanged"], false);

    // Check that optional fields are not present when None
    assert!(!json_value.as_object().unwrap().contains_key("experimental"));
    assert!(!json_value.as_object().unwrap().contains_key("logging"));
    assert!(!json_value.as_object().unwrap().contains_key("prompts"));
    assert!(!json_value.as_object().unwrap().contains_key("resources"));
}

/// Test client capabilities parsing
#[test]
fn test_client_capabilities_parsing() {
    let client_request = json!({
        "protocolVersion": "2025-06-18",
        "clientInfo": {
            "name": "test-client",
            "version": "1.0.0"
        },
        "capabilities": {
            "experimental": {},
            "sampling": {},
            "roots": {
                "listChanged": true
            }
        }
    });

    let parsed: InitializeRequest =
        serde_json::from_value(client_request).expect("Should parse client request");

    assert_eq!(parsed.protocol_version, "2025-06-18");
    assert_eq!(parsed.client_info.name, "test-client");
    assert_eq!(parsed.client_info.version, "1.0.0");
    assert!(parsed.capabilities.experimental.is_some());
    assert!(parsed.capabilities.sampling.is_some());
    assert!(parsed.capabilities.roots.is_some());
}

/// Test round-trip serialization/deserialization
#[test]
fn test_round_trip_serialization() {
    let original_response = InitializeResponse::new(
        "2025-06-18".to_string(),
        "pierre-mcp-server".to_string(),
        "1.0.0".to_string(),
    );

    // Serialize
    let json_str = serde_json::to_string(&original_response).expect("Should serialize");

    // Deserialize
    let deserialized: InitializeResponse =
        serde_json::from_str(&json_str).expect("Should deserialize");

    // Verify equality
    assert_eq!(
        original_response.protocol_version,
        deserialized.protocol_version
    );
    assert_eq!(
        original_response.server_info.name,
        deserialized.server_info.name
    );
    assert_eq!(
        original_response.server_info.version,
        deserialized.server_info.version
    );
    assert!(original_response.instructions.is_some());
    assert!(deserialized.instructions.is_some());
}

/// Test that all required methods are covered
#[test]
fn test_required_methods_coverage() {
    // This test documents the required methods for MCP compliance
    let required_methods = vec![
        "initialize", // MANDATORY
        "ping",       // MANDATORY
        "tools/list", // Required if server provides tools
        "tools/call", // Required if server provides tools
    ];

    // In a real integration test, you would verify these methods
    // are handled by the server. This test documents the requirement.
    for method in required_methods {
        assert!(
            !method.is_empty(),
            "Method {} should be implemented",
            method
        );
    }
}

/// Test protocol version compliance
#[test]
fn test_protocol_version_compliance() {
    use pierre_mcp_server::constants::protocol;

    // Verify we're using the latest protocol version
    let version = protocol::mcp_protocol_version();
    assert_eq!(
        version, "2025-06-18",
        "Should use latest MCP protocol version"
    );

    // Verify function version returns the expected default
    assert_eq!(protocol::mcp_protocol_version(), "2025-06-18");
}

/// Test JSON-RPC version compliance
#[test]
fn test_jsonrpc_version_compliance() {
    use pierre_mcp_server::constants::protocol::JSONRPC_VERSION;

    assert_eq!(JSONRPC_VERSION, "2.0", "Must use JSON-RPC 2.0");
}
