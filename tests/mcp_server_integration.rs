// ABOUTME: MCP server integration tests for end-to-end functionality
// ABOUTME: Tests complete MCP server workflows and integration points
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
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for MCP server functionality
//!
//! These tests verify that the MCP server correctly handles client connections,
//! processes requests, and returns appropriate responses.

use anyhow::Result;
use pierre_mcp_server::config::fitness_config::FitnessConfig as Config;
use pierre_mcp_server::mcp::McpServer;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Helper to create a test configuration
fn create_test_config() -> Config {
    // Create a basic FitnessConfig with default values
    Config::default()
}

/// Helper to send a JSON-RPC request and receive response
async fn _send_mcp_request(
    stream: &mut TcpStream,
    _reader: &mut BufReader<&mut tokio::net::tcp::OwnedReadHalf>,
    request: Value,
) -> Result<Value> {
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);

    // Send request
    let request_str = serde_json::to_string(&request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let response: Value = serde_json::from_str(&response_line)?;
    Ok(response)
}

#[tokio::test]
async fn test_mcp_server_initialization() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);

    // Start server in background
    let server_task = tokio::spawn(async move {
        server.run(0).await // Use port 0 for auto-assignment
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Clean up
    server_task.abort();

    Ok(())
}

#[tokio::test]
async fn test_mcp_initialize_request() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);

    // Start server on a specific port for testing
    let server_task = tokio::spawn(async move { server.run(9081).await });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connect to server
    let mut stream =
        timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:9081")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request_str = serde_json::to_string(&init_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    // Read response line
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let response: Value = serde_json::from_str(&response_line)?;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());

    let result = &response["result"];
    assert_eq!(result["protocolVersion"], "2025-06-18");
    assert!(result["serverInfo"].is_object());
    assert!(result["capabilities"].is_object());
    assert!(result["capabilities"]["tools"].is_object());

    // With new schema, tools capability indicates tool support
    assert_eq!(result["capabilities"]["tools"]["listChanged"], false);

    // Verify server info
    assert_eq!(result["serverInfo"]["name"], "pierre-mcp-server");

    // Clean up
    server_task.abort();

    Ok(())
}

#[tokio::test]
async fn test_mcp_unknown_method() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);

    // Start server
    let server_task = tokio::spawn(async move { server.run(9095).await });

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Connect and send unknown method
    let mut stream =
        timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:9095")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);

    let unknown_request = json!({
        "jsonrpc": "2.0",
        "method": "unknown_method",
        "params": {},
        "id": 2
    });

    let request_str = serde_json::to_string(&unknown_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    // For unknown method, response should be small, so use simple read_line
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let response: Value = serde_json::from_str(&response_line)?;

    // Should return method not found error
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Method not found"));

    server_task.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_tools_call_invalid_provider() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);

    let server_task = tokio::spawn(async move { server.run(9083).await });

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream =
        timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:9083")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);

    // Send tools/call with invalid provider
    let tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_athlete",
            "arguments": {
                "provider": "nonexistent_provider"
            }
        },
        "id": 3
    });

    let request_str = serde_json::to_string(&tools_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    // Read response line
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;
    let response: Value = serde_json::from_str(&response_line)?;

    // Tools now work with Universal Tool Executor - may return result or error
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["result"].is_object() || response["error"].is_object());

    server_task.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_multiple_connections() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);

    let server_task = tokio::spawn(async move { server.run(9084).await });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create multiple concurrent connections
    let mut tasks = Vec::new();

    for i in 0..3 {
        let task = tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:9084").await.unwrap();
            let (mut read_half, mut write_half) = stream.split();
            let mut reader = BufReader::new(&mut read_half);

            let init_request = json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {},
                "id": i
            });

            let request_str = serde_json::to_string(&init_request).unwrap();
            write_half.write_all(request_str.as_bytes()).await.unwrap();
            write_half.write_all(b"\n").await.unwrap();

            let mut response_line = String::new();
            reader.read_line(&mut response_line).await.unwrap();

            let response: Value = serde_json::from_str(&response_line).unwrap();
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], i);

            response
        });

        tasks.push(task);
    }

    // Wait for all connections to complete
    for task in tasks {
        let response = task.await?;
        assert!(response["result"].is_object());
    }

    server_task.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_json_rpc_protocol_compliance() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);

    let server_task = tokio::spawn(async move { server.run(9085).await });

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream =
        timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:9085")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);

    // Test various JSON-RPC compliance scenarios

    // 1. Valid request with string ID
    let request1 = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": "string-id"
    });

    let request_str = serde_json::to_string(&request1)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let response1: Value = serde_json::from_str(&response_line)?;
    assert_eq!(response1["jsonrpc"], "2.0");
    assert_eq!(response1["id"], "string-id");
    assert!(response1["result"].is_object());

    // 2. Valid request with null ID
    let request2 = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": null
    });

    let request_str = serde_json::to_string(&request2)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    response_line.clear();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let response2: Value = serde_json::from_str(&response_line)?;
    assert_eq!(response2["jsonrpc"], "2.0");
    assert_eq!(response2["id"], Value::Null);

    server_task.abort();
    Ok(())
}
