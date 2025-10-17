// ABOUTME: Unit tests for cursor-based pagination module
// ABOUTME: Tests cursor encoding, decoding, and pagination parameter handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::Utc;
use pierre_mcp_server::pagination::{Cursor, CursorPage, PaginationDirection, PaginationParams};

#[test]
fn test_cursor_encoding_decoding() {
    let timestamp = Utc::now();
    let id = "12345";

    let cursor = Cursor::new(timestamp, id);
    let (decoded_timestamp, decoded_id) = cursor.decode().unwrap();

    assert_eq!(decoded_timestamp.timestamp(), timestamp.timestamp());
    assert_eq!(decoded_id, id);
}

#[test]
fn test_cursor_invalid_decode() {
    let invalid_cursor = Cursor::from_string("invalid_base64!@#$".to_string());
    assert!(invalid_cursor.decode().is_none());
}

#[test]
fn test_cursor_page_creation() {
    let items = vec![1, 2, 3];
    let next_cursor = Some(Cursor::new(Utc::now(), "next_id"));
    let page = CursorPage::new(items.clone(), next_cursor, None, true);

    assert_eq!(page.items, items);
    assert_eq!(page.count, 3);
    assert!(page.has_more);
    assert!(page.next_cursor.is_some());
    assert!(page.prev_cursor.is_none());
}

#[test]
fn test_empty_page() {
    let page: CursorPage<i32> = CursorPage::empty();

    assert_eq!(page.items.len(), 0);
    assert_eq!(page.count, 0);
    assert!(!page.has_more);
    assert!(page.next_cursor.is_none());
    assert!(page.prev_cursor.is_none());
}

#[test]
fn test_pagination_params_forward() {
    let cursor = Some(Cursor::new(Utc::now(), "test_id"));
    let params = PaginationParams::forward(cursor, 20);

    assert_eq!(params.limit, 20);
    assert_eq!(params.direction, PaginationDirection::Forward);
    assert!(params.cursor.is_some());
}

#[test]
fn test_pagination_params_backward() {
    let params = PaginationParams::backward(None, 10);

    assert_eq!(params.limit, 10);
    assert_eq!(params.direction, PaginationDirection::Backward);
    assert!(params.cursor.is_none());
}
