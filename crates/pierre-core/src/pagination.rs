// ABOUTME: Cursor-based pagination module for efficient data traversal
// ABOUTME: Provides opaque cursor encoding for secure and consistent pagination
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fmt::{self, Display, Formatter};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
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
        let encoded = base64::Engine::encode(&URL_SAFE_NO_PAD, cursor_data.as_bytes());
        Self(encoded)
    }

    /// Decode cursor into timestamp and ID components
    ///
    /// Returns `None` if cursor is invalid or malformed
    #[must_use]
    pub fn decode(&self) -> Option<(DateTime<Utc>, String)> {
        let decoded = base64::Engine::decode(&URL_SAFE_NO_PAD, &self.0).ok()?;
        let decoded_str = String::from_utf8(decoded).ok()?;

        // Split on first ':' only so IDs containing ':' are preserved
        let (timestamp_str, id) = decoded_str.split_once(':')?;

        let timestamp_millis = timestamp_str.parse::<i64>().ok()?;
        let datetime = DateTime::from_timestamp_millis(timestamp_millis)?;

        Some((datetime, id.to_owned()))
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

impl Display for Cursor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PaginationDirection {
    /// Paginate forward (older to newer)
    #[default]
    Forward,
    /// Paginate backward (newer to older)
    Backward,
}

/// Sort order for Coach Store pagination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StoreSortOrder {
    /// Sort by newest (`published_at` DESC, id DESC)
    /// Cursor contains: (`published_at_millis`, id)
    #[default]
    Newest,
    /// Sort by popularity (`install_count` DESC, `published_at` DESC, id DESC)
    /// Cursor contains: (`install_count`, `published_at_millis`, id)
    Popular,
    /// Sort by title alphabetically (title ASC, id ASC)
    /// Cursor contains: (title, id)
    Title,
}

impl StoreSortOrder {
    /// Parse sort order from string (case-insensitive)
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "popular" => Self::Popular,
            "title" => Self::Title,
            _ => Self::Newest,
        }
    }

    /// Get string representation for API responses
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Newest => "newest",
            Self::Popular => "popular",
            Self::Title => "title",
        }
    }
}

/// Sort-aware cursor for Coach Store pagination
///
/// This cursor encodes the sort order along with the cursor position values,
/// ensuring that pagination works correctly even when sort order changes.
#[derive(Debug, Clone)]
pub struct StoreCursor {
    /// The sort order this cursor was created for
    pub sort_by: StoreSortOrder,
    /// The ID of the last item
    pub id: String,
    /// The timestamp (for Newest and Popular sorts)
    pub published_at: Option<DateTime<Utc>>,
    /// The install count (for Popular sort)
    pub install_count: Option<u32>,
    /// The title (for Title sort)
    pub title: Option<String>,
}

impl StoreCursor {
    /// Encode the cursor to a base64 string for API transmission
    ///
    /// Format: Base64(`sort_type|value1|value2|...|id`)
    /// - Newest: `newest|published_at_millis|id`
    /// - Popular: `popular|install_count|published_at_millis|id`
    /// - Title: `title|title_value|id`
    #[must_use]
    pub fn encode(&self) -> Cursor {
        let data = match self.sort_by {
            StoreSortOrder::Newest => {
                let ts = self.published_at.map_or(0, |dt| dt.timestamp_millis());
                format!("newest|{}|{}", ts, self.id)
            }
            StoreSortOrder::Popular => {
                let count = self.install_count.unwrap_or(0);
                let ts = self.published_at.map_or(0, |dt| dt.timestamp_millis());
                format!("popular|{}|{}|{}", count, ts, self.id)
            }
            StoreSortOrder::Title => {
                let title = self.title.as_deref().unwrap_or("");
                format!("title|{}|{}", title, self.id)
            }
        };
        let encoded = base64::Engine::encode(&URL_SAFE_NO_PAD, data.as_bytes());
        Cursor::from_string(encoded)
    }

    /// Decode a cursor string, validating it matches the expected sort order
    ///
    /// Returns `None` if:
    /// - The cursor is malformed
    /// - The sort order in the cursor doesn't match `expected_sort`
    #[must_use]
    pub fn decode(cursor: &Cursor, expected_sort: StoreSortOrder) -> Option<Self> {
        let decoded_bytes = base64::Engine::decode(&URL_SAFE_NO_PAD, cursor.as_str()).ok()?;
        let decoded_str = String::from_utf8(decoded_bytes).ok()?;

        // Split on first '|' to get the sort type prefix
        let (sort_type, rest) = decoded_str.split_once('|')?;

        match sort_type {
            "newest" if expected_sort == StoreSortOrder::Newest => {
                // Format: newest|ts_millis|id
                // Split on first '|' so IDs containing '|' are preserved
                let (ts_str, id) = rest.split_once('|')?;
                let ts_millis = ts_str.parse::<i64>().ok()?;
                let published_at = DateTime::from_timestamp_millis(ts_millis);
                Some(Self {
                    sort_by: StoreSortOrder::Newest,
                    id: id.to_owned(),
                    published_at,
                    install_count: None,
                    title: None,
                })
            }
            "popular" if expected_sort == StoreSortOrder::Popular => {
                // Format: popular|count|ts_millis|id
                // count and ts are numeric (no '|'), split sequentially
                let (count_str, after_count) = rest.split_once('|')?;
                let (ts_str, id) = after_count.split_once('|')?;
                let install_count = count_str.parse::<u32>().ok()?;
                let ts_millis = ts_str.parse::<i64>().ok()?;
                let published_at = DateTime::from_timestamp_millis(ts_millis);
                Some(Self {
                    sort_by: StoreSortOrder::Popular,
                    id: id.to_owned(),
                    published_at,
                    install_count: Some(install_count),
                    title: None,
                })
            }
            "title" if expected_sort == StoreSortOrder::Title => {
                // Format: title|title_value|id
                // Title may contain '|', so split from the right to extract the ID
                // (IDs are UUIDs or numeric and do not contain '|')
                let (title, id) = rest.rsplit_once('|')?;
                Some(Self {
                    sort_by: StoreSortOrder::Title,
                    id: id.to_owned(),
                    published_at: None,
                    install_count: None,
                    title: Some(title.to_owned()),
                })
            }
            _ => None,
        }
    }

    /// Create a cursor for Newest sort order
    #[must_use]
    pub const fn newest(id: String, published_at: Option<DateTime<Utc>>) -> Self {
        Self {
            sort_by: StoreSortOrder::Newest,
            id,
            published_at,
            install_count: None,
            title: None,
        }
    }

    /// Create a cursor for Popular sort order
    #[must_use]
    pub const fn popular(
        id: String,
        install_count: u32,
        published_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            sort_by: StoreSortOrder::Popular,
            id,
            published_at,
            install_count: Some(install_count),
            title: None,
        }
    }

    /// Create a cursor for Title sort order
    #[must_use]
    pub const fn title(id: String, title: String) -> Self {
        Self {
            sort_by: StoreSortOrder::Title,
            id,
            published_at: None,
            install_count: None,
            title: Some(title),
        }
    }
}
