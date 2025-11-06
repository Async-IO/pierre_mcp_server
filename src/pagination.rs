// ABOUTME: Cursor-based pagination module for efficient data traversal
// ABOUTME: Provides opaque cursor encoding for secure and consistent pagination
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Opaque pagination cursor containing encoded position information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cursor(String);

impl Cursor {
    /// Create a new cursor from timestamp and ID
    ///
    /// # Arguments
    /// * `timestamp` - The timestamp of the item (for consistent ordering)
    /// * `id` - The unique identifier of the item
    #[must_use]
    pub fn new(timestamp: DateTime<Utc>, id: &str) -> Self {
        let cursor_data = format!("{}:{}", timestamp.timestamp_millis(), id);
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            cursor_data.as_bytes(),
        );
        Self(encoded)
    }

    /// Decode cursor into timestamp and ID components
    ///
    /// Returns `None` if cursor is invalid or malformed
    #[must_use]
    pub fn decode(&self) -> Option<(DateTime<Utc>, String)> {
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &self.0)
                .ok()?;
        let decoded_str = String::from_utf8(decoded).ok()?;
        let parts: Vec<&str> = decoded_str.split(':').collect();

        if parts.len() != 2 {
            return None;
        }

        let timestamp_millis = parts[0].parse::<i64>().ok()?;
        let id = parts[1].to_owned();
        let datetime = DateTime::from_timestamp_millis(timestamp_millis)?;

        Some((datetime, id))
    }

    /// Get the raw cursor string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create cursor from raw string (for deserialization)
    #[must_use]
    pub const fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Paginated response containing items and pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPage<T> {
    /// The items in this page
    pub items: Vec<T>,

    /// Cursor pointing to the next page (if available)
    pub next_cursor: Option<Cursor>,

    /// Cursor pointing to the previous page (if available)
    pub prev_cursor: Option<Cursor>,

    /// Whether there are more items after this page
    pub has_more: bool,

    /// Total number of items in this page
    pub count: usize,
}

impl<T> CursorPage<T> {
    /// Create a new cursor page
    #[must_use]
    pub const fn new(
        items: Vec<T>,
        next_cursor: Option<Cursor>,
        prev_cursor: Option<Cursor>,
        has_more: bool,
    ) -> Self {
        let count = items.len();
        Self {
            items,
            next_cursor,
            prev_cursor,
            has_more,
            count,
        }
    }

    /// Create an empty page
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            items: Vec::new(),
            next_cursor: None,
            prev_cursor: None,
            has_more: false,
            count: 0,
        }
    }
}

/// Pagination parameters for cursor-based queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Cursor to start from (exclusive)
    pub cursor: Option<Cursor>,

    /// Maximum number of items to return
    pub limit: usize,

    /// Direction to paginate (forward or backward)
    pub direction: PaginationDirection,
}

impl PaginationParams {
    /// Create new forward pagination parameters
    #[must_use]
    pub const fn forward(cursor: Option<Cursor>, limit: usize) -> Self {
        Self {
            cursor,
            limit,
            direction: PaginationDirection::Forward,
        }
    }

    /// Create new backward pagination parameters
    #[must_use]
    pub const fn backward(cursor: Option<Cursor>, limit: usize) -> Self {
        Self {
            cursor,
            limit,
            direction: PaginationDirection::Backward,
        }
    }
}

/// Direction for pagination
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaginationDirection {
    /// Paginate forward (older to newer)
    Forward,
    /// Paginate backward (newer to older)
    Backward,
}

impl Default for PaginationDirection {
    fn default() -> Self {
        Self::Forward
    }
}
