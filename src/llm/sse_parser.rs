// ABOUTME: Shared SSE (Server-Sent Events) line-buffering parser for LLM streaming responses
// ABOUTME: Handles partial lines across TCP boundaries and multiple events per chunk
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # SSE Stream Parser
//!
//! A shared line-buffering parser for Server-Sent Events (SSE) used by all LLM providers.
//! Solves two critical correctness issues:
//!
//! 1. **Multiple events per TCP chunk**: When network buffers batch several SSE events
//!    into a single `bytes_stream()` chunk, all events are emitted (not just the first).
//!
//! 2. **Partial JSON across TCP boundaries**: When a JSON payload is split across two
//!    TCP chunks, the line buffer accumulates partial data until a complete line arrives.
//!
//! ## Usage
//!
//! Each provider supplies a `parse_data` closure that converts raw JSON strings into
//! `StreamChunk` values. The SSE framing (line buffering, `data:` prefix stripping,
//! `[DONE]` detection) is handled once here.
//!
//! ```text
//! let stream = create_sse_stream(
//!     response.bytes_stream(),
//!     |json_str| { /* parse provider-specific JSON */ },
//!     "Groq",
//! );
//! ```

use std::collections::VecDeque;
use std::mem;
use std::pin::Pin;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures_util::stream::unfold;
use futures_util::{future, Stream, StreamExt};

use super::{ChatStream, StreamChunk};
use crate::errors::AppError;

/// A parsed SSE event from the stream
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEvent {
    /// A `data:` payload with the JSON string (prefix stripped)
    Data(String),
    /// The `[DONE]` termination signal (OpenAI/Groq convention)
    Done,
}

/// Line-buffering SSE parser that handles partial lines across TCP chunk boundaries
///
/// SSE streams are newline-delimited. TCP does not guarantee alignment between
/// network chunks and SSE event boundaries. This parser buffers incomplete lines
/// and emits complete events only when a full line (terminated by `\n`) is available.
#[derive(Debug)]
pub struct SseLineBuffer {
    /// Accumulated bytes not yet terminated by a newline
    buffer: String,
}

impl Default for SseLineBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SseLineBuffer {
    /// Create a new empty line buffer
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed raw bytes from a TCP chunk into the buffer, returning any complete SSE events
    ///
    /// Bytes are appended to the internal buffer. Complete lines (terminated by `\n`)
    /// are extracted, parsed as SSE events, and returned. Any trailing partial line
    /// remains in the buffer for the next `feed()` call.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        let text = String::from_utf8_lossy(bytes);
        self.buffer.push_str(&text);

        let mut events = Vec::new();

        // Process all complete lines (terminated by \n)
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].trim_end_matches('\r').to_owned();
            self.buffer = self.buffer[newline_pos + 1..].to_owned();

            let trimmed = line.trim();

            // Skip empty lines (SSE event separators)
            if trimmed.is_empty() {
                continue;
            }

            // Check for done signal
            if trimmed == "data: [DONE]" {
                events.push(SseEvent::Done);
                continue;
            }

            // Extract data payload
            if let Some(data) = trimmed.strip_prefix("data: ") {
                if !data.trim().is_empty() {
                    events.push(SseEvent::Data(data.to_owned()));
                }
            }
            // Ignore non-data SSE fields (event:, id:, retry:, comments starting with :)
        }

        events
    }

    /// Flush any remaining buffered content as a final event
    ///
    /// Called when the byte stream ends. If there is a partial line in the buffer
    /// (no trailing newline), attempt to parse it as an SSE event.
    pub fn flush(&mut self) -> Vec<SseEvent> {
        let remaining = mem::take(&mut self.buffer);
        let trimmed = remaining.trim();

        if trimmed.is_empty() {
            return Vec::new();
        }

        if trimmed == "data: [DONE]" {
            return vec![SseEvent::Done];
        }

        if let Some(data) = trimmed.strip_prefix("data: ") {
            if !data.trim().is_empty() {
                return vec![SseEvent::Data(data.to_owned())];
            }
        }

        Vec::new()
    }
}

/// Create a properly-buffered SSE stream from a raw byte stream
///
/// Wraps a `reqwest` byte stream with SSE line buffering. The `parse_data` closure
/// converts provider-specific JSON strings into `StreamChunk` values.
///
/// # Arguments
///
/// * `byte_stream` - Raw bytes from `response.bytes_stream()`
/// * `parse_data` - Closure that parses a JSON string into an optional `StreamChunk`
/// * `provider_name` - Provider name for error messages (e.g., "Groq", "Gemini")
///
/// Returns `None` from `parse_data` to skip events that don't produce output
/// (e.g., empty deltas, metadata-only chunks).
pub fn create_sse_stream<S, F>(
    byte_stream: S,
    parse_data: F,
    provider_name: &'static str,
) -> ChatStream
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    F: Fn(&str) -> Option<Result<StreamChunk, AppError>> + Send + 'static,
{
    let state = SseStreamState {
        parser: SseLineBuffer::new(),
        pending: VecDeque::new(),
        stream_ended: false,
    };

    // Use unfold to maintain parser state across async iterations.
    // Each iteration either drains a pending event or reads the next TCP chunk.
    let stream = unfold(
        (
            Box::pin(byte_stream)
                as Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
            state,
            parse_data,
            provider_name,
        ),
        |(mut byte_stream, mut state, parse_data, provider_name)| async move {
            loop {
                // Drain pending events first (multiple SSE events per TCP chunk)
                if let Some(item) = state.pending.pop_front() {
                    return Some((item, (byte_stream, state, parse_data, provider_name)));
                }

                if state.stream_ended {
                    return None;
                }

                // Read next TCP chunk
                match byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        for event in state.parser.feed(&bytes) {
                            match event {
                                SseEvent::Data(json_str) => {
                                    if let Some(result) = parse_data(&json_str) {
                                        state.pending.push_back(result);
                                    }
                                }
                                SseEvent::Done => {
                                    state.pending.push_back(Ok(StreamChunk {
                                        delta: String::new(),
                                        is_final: true,
                                        finish_reason: Some("stop".to_owned()),
                                    }));
                                }
                            }
                        }
                        // Loop to drain pending events
                    }
                    Some(Err(e)) => {
                        state.stream_ended = true;
                        return Some((
                            Err(AppError::external_service(
                                provider_name,
                                format!("Stream read error: {e}"),
                            )),
                            (byte_stream, state, parse_data, provider_name),
                        ));
                    }
                    None => {
                        // Byte stream ended — flush remaining buffer
                        state.stream_ended = true;
                        for event in state.parser.flush() {
                            match event {
                                SseEvent::Data(json_str) => {
                                    if let Some(result) = parse_data(&json_str) {
                                        state.pending.push_back(result);
                                    }
                                }
                                SseEvent::Done => {
                                    state.pending.push_back(Ok(StreamChunk {
                                        delta: String::new(),
                                        is_final: true,
                                        finish_reason: Some("stop".to_owned()),
                                    }));
                                }
                            }
                        }
                        // Check if flush produced events
                        if let Some(item) = state.pending.pop_front() {
                            return Some((item, (byte_stream, state, parse_data, provider_name)));
                        }
                        return None;
                    }
                }
            }
        },
    );

    // Filter out empty deltas (unless it's the final chunk)
    let filtered = stream.filter(|result| {
        future::ready(
            result
                .as_ref()
                .map_or(true, |chunk| !chunk.delta.is_empty() || chunk.is_final),
        )
    });

    Box::pin(filtered)
}

/// Internal state for the SSE stream unfold
struct SseStreamState {
    parser: SseLineBuffer,
    pending: VecDeque<Result<StreamChunk, AppError>>,
    stream_ended: bool,
}

// ============================================================================
// Retry Configuration
// ============================================================================

/// Shared retry configuration for LLM provider streaming requests
///
/// Streaming retries only cover the initial HTTP request. Once bytes start
/// flowing, the stream is not retried (the client may have already consumed
/// partial output).
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries)
    pub max_retries: u32,
    /// Initial delay before first retry (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay cap for exponential backoff (milliseconds)
    pub max_delay_ms: u64,
}

impl RetryConfig {
    /// Default retry config: 3 retries, 500ms initial, 5s max
    #[must_use]
    pub const fn default_config() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 500,
            max_delay_ms: 5000,
        }
    }

    /// Calculate exponential backoff delay with jitter for a given attempt
    ///
    /// `delay = min(initial_ms * 2^attempt, max_ms) + jitter(0..100ms)`
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay_ms.saturating_mul(1_u64 << attempt);
        let capped_delay = base_delay.min(self.max_delay_ms);
        // Small jitter (0-99ms) to avoid thundering herd
        let jitter = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::from(d.subsec_millis()))
            % 100;
        Duration::from_millis(capped_delay + jitter)
    }
}

/// Check if an HTTP error status code is retryable
///
/// Retryable errors are transient conditions that may resolve on retry:
/// - 429 Too Many Requests (rate limiting)
/// - 503 Service Unavailable (temporary overload)
/// - 502 Bad Gateway (upstream issues)
#[must_use]
pub fn is_retryable_status(status: u16) -> bool {
    matches!(status, 429 | 502 | 503)
}

/// Check if a request error is retryable (connection/timeout errors)
#[must_use]
pub fn is_retryable_request_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout()
}
