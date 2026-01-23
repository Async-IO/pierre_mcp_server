// ABOUTME: Unit tests for StoreCursor and cursor-based pagination for Coach Store
// ABOUTME: Tests encoding, decoding, and sort order validation for cursor pagination
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::DateTime;
use pierre_mcp_server::pagination::{Cursor, StoreCursor, StoreSortOrder};

// ============================================================================
// StoreCursor Encoding/Decoding Tests
// ============================================================================

#[test]
fn test_store_cursor_newest_roundtrip() {
    let ts = DateTime::from_timestamp_millis(1_700_000_000_000).unwrap();
    let cursor = StoreCursor::newest("coach-123".to_owned(), Some(ts));
    let encoded = cursor.encode();

    let decoded = StoreCursor::decode(&encoded, StoreSortOrder::Newest).unwrap();
    assert_eq!(decoded.sort_by, StoreSortOrder::Newest);
    assert_eq!(decoded.id, "coach-123");
    assert_eq!(decoded.published_at, Some(ts));
    assert!(decoded.install_count.is_none());
    assert!(decoded.title.is_none());
}

#[test]
fn test_store_cursor_popular_roundtrip() {
    let ts = DateTime::from_timestamp_millis(1_700_000_000_000).unwrap();
    let cursor = StoreCursor::popular("coach-456".to_owned(), 42, Some(ts));
    let encoded = cursor.encode();

    let decoded = StoreCursor::decode(&encoded, StoreSortOrder::Popular).unwrap();
    assert_eq!(decoded.sort_by, StoreSortOrder::Popular);
    assert_eq!(decoded.id, "coach-456");
    assert_eq!(decoded.published_at, Some(ts));
    assert_eq!(decoded.install_count, Some(42));
    assert!(decoded.title.is_none());
}

#[test]
fn test_store_cursor_title_roundtrip() {
    let cursor = StoreCursor::title("coach-789".to_owned(), "Marathon Coach".to_owned());
    let encoded = cursor.encode();

    let decoded = StoreCursor::decode(&encoded, StoreSortOrder::Title).unwrap();
    assert_eq!(decoded.sort_by, StoreSortOrder::Title);
    assert_eq!(decoded.id, "coach-789");
    assert_eq!(decoded.title, Some("Marathon Coach".to_owned()));
    assert!(decoded.published_at.is_none());
    assert!(decoded.install_count.is_none());
}

#[test]
fn test_store_cursor_wrong_sort_order_returns_none() {
    let cursor = StoreCursor::newest("coach-123".to_owned(), None);
    let encoded = cursor.encode();

    // Trying to decode a Newest cursor as Popular should fail
    assert!(StoreCursor::decode(&encoded, StoreSortOrder::Popular).is_none());
    assert!(StoreCursor::decode(&encoded, StoreSortOrder::Title).is_none());
}

#[test]
fn test_store_cursor_invalid_base64_returns_none() {
    let invalid = Cursor::from_string("not-valid-base64!!!".to_owned());
    assert!(StoreCursor::decode(&invalid, StoreSortOrder::Newest).is_none());
}

// ============================================================================
// StoreSortOrder Tests
// ============================================================================

#[test]
fn test_store_sort_order_parse() {
    assert_eq!(StoreSortOrder::parse("newest"), StoreSortOrder::Newest);
    assert_eq!(StoreSortOrder::parse("NEWEST"), StoreSortOrder::Newest);
    assert_eq!(StoreSortOrder::parse("popular"), StoreSortOrder::Popular);
    assert_eq!(StoreSortOrder::parse("POPULAR"), StoreSortOrder::Popular);
    assert_eq!(StoreSortOrder::parse("title"), StoreSortOrder::Title);
    assert_eq!(StoreSortOrder::parse("TITLE"), StoreSortOrder::Title);
    assert_eq!(StoreSortOrder::parse("unknown"), StoreSortOrder::Newest);
    assert_eq!(StoreSortOrder::parse(""), StoreSortOrder::Newest);
}

#[test]
fn test_store_sort_order_as_str() {
    assert_eq!(StoreSortOrder::Newest.as_str(), "newest");
    assert_eq!(StoreSortOrder::Popular.as_str(), "popular");
    assert_eq!(StoreSortOrder::Title.as_str(), "title");
}

#[test]
fn test_store_cursor_newest_with_none_timestamp() {
    let cursor = StoreCursor::newest("coach-abc".to_owned(), None);
    let encoded = cursor.encode();

    let decoded = StoreCursor::decode(&encoded, StoreSortOrder::Newest).unwrap();
    assert_eq!(decoded.sort_by, StoreSortOrder::Newest);
    assert_eq!(decoded.id, "coach-abc");
    // When timestamp is None, the encoded value is 0, which DateTime::from_timestamp_millis
    // converts to 1970-01-01, not None
    assert!(decoded.published_at.is_some());
}

#[test]
fn test_store_cursor_popular_high_install_count() {
    let ts = DateTime::from_timestamp_millis(1_700_000_000_000).unwrap();
    let cursor = StoreCursor::popular("popular-coach".to_owned(), 999_999, Some(ts));
    let encoded = cursor.encode();

    let decoded = StoreCursor::decode(&encoded, StoreSortOrder::Popular).unwrap();
    assert_eq!(decoded.install_count, Some(999_999));
}

#[test]
fn test_store_cursor_title_with_special_characters() {
    // Test with special characters that might be tricky for base64
    let cursor = StoreCursor::title("coach-special".to_owned(), "Coach: A & B".to_owned());
    let encoded = cursor.encode();

    let decoded = StoreCursor::decode(&encoded, StoreSortOrder::Title).unwrap();
    assert_eq!(decoded.title, Some("Coach: A & B".to_owned()));
}
