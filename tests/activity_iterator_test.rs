// ABOUTME: Tests for activity streaming iterator for memory-efficient paginated fetching
// ABOUTME: Validates StreamConfig, page size clamping, and ActivityStream creation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::providers::activity_iterator::{
    StreamConfig, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MIN_PAGE_SIZE,
};

#[test]
fn test_stream_config_default() {
    let config = StreamConfig::default();
    assert_eq!(config.page_size, DEFAULT_PAGE_SIZE);
    assert!(config.max_activities.is_none());
}

#[test]
fn test_stream_config_page_size_clamping_too_small() {
    let config = StreamConfig::with_page_size(5);
    assert_eq!(config.page_size, MIN_PAGE_SIZE);
}

#[test]
fn test_stream_config_page_size_clamping_too_large() {
    let config = StreamConfig::with_page_size(500);
    assert_eq!(config.page_size, MAX_PAGE_SIZE);
}

#[test]
fn test_stream_config_page_size_valid() {
    let config = StreamConfig::with_page_size(75);
    assert_eq!(config.page_size, 75);
}

#[test]
fn test_stream_config_with_max_activities() {
    let config = StreamConfig::default().with_max_activities(100);
    assert_eq!(config.max_activities, Some(100));
}

#[test]
fn test_stream_config_builder_chain() {
    let config = StreamConfig::with_page_size(30).with_max_activities(500);
    assert_eq!(config.page_size, 30);
    assert_eq!(config.max_activities, Some(500));
}
