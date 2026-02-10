// ABOUTME: Streaming activity iterator for memory-efficient paginated fetching
// ABOUTME: Implements futures::Stream for async iteration over large activity histories
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Streaming Activity Iterator
//!
//! This module provides memory-efficient streaming over paginated activity data.
//! Instead of loading all activities into a `Vec<Activity>`, the iterator lazily
//! fetches pages on demand, keeping memory usage bounded.
//!
//! ## Design Pattern
//!
//! Follows the pattern of `std::io::Lines` - a buffered iterator that fetches
//! data lazily. Since provider methods are async, we implement `futures::Stream`
//! rather than `std::iter::Iterator`.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use futures_util::StreamExt;
//! use pierre_mcp_server::providers::activity_iterator::ActivityStreamExt;
//!
//! async fn process_activities(provider: &dyn pierre_mcp_server::providers::CoreFitnessProvider) {
//!     let mut stream = provider.activities_stream(50);
//!
//!     while let Some(result) = stream.next().await {
//!         match result {
//!             Ok(activity) => println!("Activity: {}", activity.name()),
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//! }
//! ```
//!
//! ## Memory Efficiency
//!
//! For a user with 1000 activities fetched in pages of 50:
//! - **Vec approach**: Allocates memory for all 1000 activities at once
//! - **Stream approach**: Holds at most 50 activities in buffer at any time

use std::collections::VecDeque;
use std::pin::Pin;

use async_stream::try_stream;
use futures_util::Stream;

use crate::core::FitnessProvider;
use crate::errors::provider::ProviderError;
use crate::models::Activity;
use crate::pagination::{Cursor, PaginationParams};

/// Default page size for activity streaming
pub const DEFAULT_PAGE_SIZE: usize = 50;

/// Minimum page size to prevent excessive API calls
pub const MIN_PAGE_SIZE: usize = 10;

/// Maximum page size to prevent memory issues
pub const MAX_PAGE_SIZE: usize = 200;

/// Configuration for activity streaming behavior
#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    /// Number of activities to fetch per page
    pub page_size: usize,
    /// Maximum total activities to fetch (None for unlimited)
    pub max_activities: Option<usize>,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            page_size: DEFAULT_PAGE_SIZE,
            max_activities: None,
        }
    }
}

impl StreamConfig {
    /// Create configuration with specified page size
    #[must_use]
    pub fn with_page_size(page_size: usize) -> Self {
        Self {
            page_size: page_size.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE),
            max_activities: None,
        }
    }

    /// Set maximum number of activities to fetch
    #[must_use]
    pub const fn with_max_activities(mut self, max: usize) -> Self {
        self.max_activities = Some(max);
        self
    }
}

/// Type alias for the activity stream returned by `activities_stream`
pub type ActivityStream<'a> =
    Pin<Box<dyn Stream<Item = Result<Activity, ProviderError>> + Send + 'a>>;

/// Create a streaming iterator over activities from a provider
///
/// This function creates a `Stream` that lazily fetches activities page by page,
/// yielding each activity individually. The stream maintains an internal buffer
/// and automatically fetches the next page when the buffer is exhausted.
///
/// # Arguments
///
/// * `provider` - The fitness provider to fetch activities from
/// * `config` - Configuration controlling page size and limits
///
/// # Returns
///
/// A pinned boxed Stream that yields `Result<Activity, ProviderError>`
///
/// # Example
///
/// ```rust,no_run
/// use futures_util::StreamExt;
/// use pierre_mcp_server::providers::activity_iterator::{create_activity_stream, StreamConfig};
///
/// async fn example(provider: &dyn pierre_mcp_server::providers::CoreFitnessProvider) {
///     let config = StreamConfig::with_page_size(25).with_max_activities(100);
///     let mut stream = create_activity_stream(provider, config);
///
///     while let Some(result) = stream.next().await {
///         match result {
///             Ok(activity) => println!("Activity: {}", activity.name()),
///             Err(e) => eprintln!("Error: {}", e),
///         }
///     }
/// }
/// ```
pub fn create_activity_stream(
    provider: &dyn FitnessProvider,
    config: StreamConfig,
) -> ActivityStream<'_> {
    let page_size = config.page_size.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE);
    let max_activities = config.max_activities;

    Box::pin(try_stream! {
        let mut buffer: VecDeque<Activity> = VecDeque::new();
        let mut next_cursor: Option<Cursor> = None;
        let mut yielded_count: usize = 0;
        let mut exhausted = false;

        loop {
            // Check if we've hit the max activities limit
            if let Some(max) = max_activities {
                if yielded_count >= max {
                    break;
                }
            }

            // Try to yield from buffer first
            if let Some(activity) = buffer.pop_front() {
                yielded_count += 1;
                yield activity;
                continue;
            }

            // Buffer is empty - check if we've exhausted all pages
            if exhausted {
                break;
            }

            // Fetch next page
            let params = PaginationParams::forward(next_cursor.take(), page_size);

            let page = provider.get_activities_cursor(&params).await.map_err(|e| {
                // Convert AppError to ProviderError
                ProviderError::ApiError {
                    provider: provider.name().to_owned(),
                    status_code: 500,
                    message: e.to_string(),
                    retryable: false,
                }
            })?;

            // Add activities to buffer
            buffer.extend(page.items);

            // Update cursor and exhaustion state
            if page.has_more {
                next_cursor = page.next_cursor;
            } else {
                exhausted = true;
            }
        }
    })
}

/// Extension trait for creating activity streams from providers
pub trait ActivityStreamExt {
    /// Create a streaming iterator over all activities
    ///
    /// # Arguments
    ///
    /// * `page_size` - Number of activities to fetch per API call
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use futures_util::StreamExt;
    /// use pierre_mcp_server::providers::activity_iterator::ActivityStreamExt;
    ///
    /// async fn example(provider: &dyn pierre_mcp_server::providers::CoreFitnessProvider) {
    ///     let mut stream = provider.activities_stream(50);
    ///     while let Some(result) = stream.next().await {
    ///         println!("Got activity: {:?}", result);
    ///     }
    /// }
    /// ```
    fn activities_stream(&self, page_size: usize) -> ActivityStream<'_>;

    /// Create a streaming iterator with custom configuration
    fn activities_stream_with_config(&self, config: StreamConfig) -> ActivityStream<'_>;

    /// Create a streaming iterator with a maximum number of activities
    ///
    /// This is a convenience method for limiting the total number of activities
    /// fetched, useful for getting "recent N activities" efficiently.
    fn activities_stream_limited(
        &self,
        page_size: usize,
        max_activities: usize,
    ) -> ActivityStream<'_>;
}

impl<T: FitnessProvider> ActivityStreamExt for T {
    fn activities_stream(&self, page_size: usize) -> ActivityStream<'_> {
        create_activity_stream(self, StreamConfig::with_page_size(page_size))
    }

    fn activities_stream_with_config(&self, config: StreamConfig) -> ActivityStream<'_> {
        create_activity_stream(self, config)
    }

    fn activities_stream_limited(
        &self,
        page_size: usize,
        max_activities: usize,
    ) -> ActivityStream<'_> {
        let config = StreamConfig::with_page_size(page_size).with_max_activities(max_activities);
        create_activity_stream(self, config)
    }
}

impl ActivityStreamExt for dyn FitnessProvider + '_ {
    fn activities_stream(&self, page_size: usize) -> ActivityStream<'_> {
        create_activity_stream(self, StreamConfig::with_page_size(page_size))
    }

    fn activities_stream_with_config(&self, config: StreamConfig) -> ActivityStream<'_> {
        create_activity_stream(self, config)
    }

    fn activities_stream_limited(
        &self,
        page_size: usize,
        max_activities: usize,
    ) -> ActivityStream<'_> {
        let config = StreamConfig::with_page_size(page_size).with_max_activities(max_activities);
        create_activity_stream(self, config)
    }
}

impl ActivityStreamExt for dyn FitnessProvider + Send + '_ {
    fn activities_stream(&self, page_size: usize) -> ActivityStream<'_> {
        create_activity_stream(self, StreamConfig::with_page_size(page_size))
    }

    fn activities_stream_with_config(&self, config: StreamConfig) -> ActivityStream<'_> {
        create_activity_stream(self, config)
    }

    fn activities_stream_limited(
        &self,
        page_size: usize,
        max_activities: usize,
    ) -> ActivityStream<'_> {
        let config = StreamConfig::with_page_size(page_size).with_max_activities(max_activities);
        create_activity_stream(self, config)
    }
}

impl ActivityStreamExt for dyn FitnessProvider + Send + Sync + '_ {
    fn activities_stream(&self, page_size: usize) -> ActivityStream<'_> {
        create_activity_stream(self, StreamConfig::with_page_size(page_size))
    }

    fn activities_stream_with_config(&self, config: StreamConfig) -> ActivityStream<'_> {
        create_activity_stream(self, config)
    }

    fn activities_stream_limited(
        &self,
        page_size: usize,
        max_activities: usize,
    ) -> ActivityStream<'_> {
        let config = StreamConfig::with_page_size(page_size).with_max_activities(max_activities);
        create_activity_stream(self, config)
    }
}
