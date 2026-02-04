// ABOUTME: Integration tests for the shared SSE parser used by all LLM providers
// ABOUTME: Validates correct handling of multi-event chunks, partial lines, and stream termination
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(missing_docs)]

use bytes::Bytes;
use futures_util::{stream, StreamExt};
use pierre_mcp_server::errors::AppError;
use pierre_mcp_server::llm::sse_parser::{
    create_sse_stream, is_retryable_status, RetryConfig, SseEvent, SseLineBuffer,
};
use pierre_mcp_server::llm::StreamChunk;

/// Helper: create an SSE stream from raw byte chunks and collect all `StreamChunks`
async fn collect_stream_chunks(
    chunks: Vec<Vec<u8>>,
    parse_fn: fn(&str) -> Option<Result<StreamChunk, AppError>>,
) -> Vec<StreamChunk> {
    let byte_stream = stream::iter(
        chunks
            .into_iter()
            .map(|b| Ok::<Bytes, reqwest::Error>(Bytes::from(b))),
    );

    let sse_stream = create_sse_stream(byte_stream, parse_fn, "Test");

    futures_util::pin_mut!(sse_stream);

    let mut results = Vec::new();
    while let Some(item) = sse_stream.next().await {
        assert!(item.is_ok(), "SSE stream produced an unexpected error");
        results.extend(item.ok());
    }
    results
}

/// Simple JSON parser for tests: extracts "content" field from `{"content":"..."}`
fn test_parse_data(json_str: &str) -> Option<Result<StreamChunk, AppError>> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let content = value.get("content")?.as_str()?;
    let is_final = value
        .get("done")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    Some(Ok(StreamChunk {
        delta: content.to_owned(),
        is_final,
        finish_reason: if is_final {
            Some("stop".to_owned())
        } else {
            None
        },
    }))
}

#[tokio::test]
async fn test_single_event_per_chunk_stream() {
    let chunks = vec![
        b"data: {\"content\":\"Hello\"}\n\n".to_vec(),
        b"data: {\"content\":\" world\"}\n\n".to_vec(),
        b"data: [DONE]\n\n".to_vec(),
    ];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].delta, "Hello");
    assert!(!results[0].is_final);
    assert_eq!(results[1].delta, " world");
    assert!(!results[1].is_final);
    // Last chunk is the [DONE] signal
    assert!(results[2].is_final);
}

#[tokio::test]
async fn test_multiple_events_per_chunk_stream() {
    // Simulate TCP batching: three SSE events in one chunk
    let chunks = vec![
        b"data: {\"content\":\"a\"}\n\ndata: {\"content\":\"b\"}\n\ndata: {\"content\":\"c\"}\n\n"
            .to_vec(),
        b"data: [DONE]\n\n".to_vec(),
    ];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(
        results.len(),
        4,
        "Should emit all 3 events + DONE, got: {results:?}"
    );
    assert_eq!(results[0].delta, "a");
    assert_eq!(results[1].delta, "b");
    assert_eq!(results[2].delta, "c");
    assert!(results[3].is_final);
}

#[tokio::test]
async fn test_partial_json_across_chunks_stream() {
    // JSON split across two TCP chunks
    let chunks = vec![
        b"data: {\"content\":\"hel".to_vec(),
        b"lo\"}\n\ndata: {\"content\":\"world\"}\n\n".to_vec(),
        b"data: [DONE]\n\n".to_vec(),
    ];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(
        results.len(),
        3,
        "Should reconstruct split JSON, got: {results:?}"
    );
    assert_eq!(results[0].delta, "hello");
    assert_eq!(results[1].delta, "world");
    assert!(results[2].is_final);
}

#[tokio::test]
async fn test_stream_ends_without_done_signal() {
    // Gemini pattern: no [DONE], stream just ends
    let chunks = vec![
        b"data: {\"content\":\"first\"}\n\n".to_vec(),
        b"data: {\"content\":\"last\",\"done\":true}\n\n".to_vec(),
    ];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].delta, "first");
    assert_eq!(results[1].delta, "last");
    assert!(results[1].is_final);
}

#[tokio::test]
async fn test_empty_chunks_between_events() {
    let chunks = vec![
        b"\n\n\n".to_vec(),
        b"data: {\"content\":\"hello\"}\n\n".to_vec(),
        b"\n\n".to_vec(),
        b"data: [DONE]\n\n".to_vec(),
    ];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].delta, "hello");
    assert!(results[1].is_final);
}

#[tokio::test]
async fn test_many_small_byte_chunks() {
    // Extreme fragmentation: each byte is its own chunk
    let full_event = b"data: {\"content\":\"ok\"}\n\n";
    let chunks: Vec<Vec<u8>> = full_event.iter().map(|b| vec![*b]).collect();

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].delta, "ok");
}

#[tokio::test]
async fn test_flush_on_stream_end() {
    // Partial line at end of stream (no trailing newline)
    let chunks = vec![b"data: {\"content\":\"final\"}".to_vec()];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(results.len(), 1, "Flush should emit the partial line");
    assert_eq!(results[0].delta, "final");
}

#[tokio::test]
async fn test_unparseable_json_skipped() {
    let chunks = vec![
        b"data: {\"content\":\"good\"}\n\n".to_vec(),
        b"data: not-valid-json\n\n".to_vec(),
        b"data: {\"content\":\"also good\"}\n\n".to_vec(),
        b"data: [DONE]\n\n".to_vec(),
    ];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(
        results.len(),
        3,
        "Bad JSON should be skipped, got: {results:?}"
    );
    assert_eq!(results[0].delta, "good");
    assert_eq!(results[1].delta, "also good");
    assert!(results[2].is_final);
}

#[tokio::test]
async fn test_crlf_line_endings() {
    // Windows-style line endings
    let chunks = vec![b"data: {\"content\":\"hi\"}\r\n\r\ndata: [DONE]\r\n\r\n".to_vec()];

    let results = collect_stream_chunks(chunks, test_parse_data).await;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].delta, "hi");
    assert!(results[1].is_final);
}

// ============================================================================
// SseLineBuffer unit tests
// ============================================================================

#[test]
fn test_single_event_per_chunk() {
    let mut parser = SseLineBuffer::new();
    let events = parser.feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        SseEvent::Data("{\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}".to_owned())
    );
}

#[test]
fn test_multiple_events_per_chunk() {
    let mut parser = SseLineBuffer::new();
    let chunk = b"data: {\"content\":\"hello\"}\n\ndata: {\"content\":\"world\"}\n\ndata: {\"content\":\"!\"}\n\n";
    let events = parser.feed(chunk);
    assert_eq!(events.len(), 3);
    assert_eq!(
        events[0],
        SseEvent::Data("{\"content\":\"hello\"}".to_owned())
    );
    assert_eq!(
        events[1],
        SseEvent::Data("{\"content\":\"world\"}".to_owned())
    );
    assert_eq!(events[2], SseEvent::Data("{\"content\":\"!\"}".to_owned()));
}

#[test]
fn test_partial_json_across_chunks() {
    let mut parser = SseLineBuffer::new();

    // First chunk: incomplete JSON line (no newline)
    let events1 = parser.feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"hel");
    assert!(events1.is_empty(), "Partial line should not emit events");

    // Second chunk: completes the JSON line
    let events2 = parser.feed(b"lo\"}}]}\n\n");
    assert_eq!(events2.len(), 1);
    assert_eq!(
        events2[0],
        SseEvent::Data("{\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}".to_owned())
    );
}

#[test]
fn test_done_signal() {
    let mut parser = SseLineBuffer::new();
    let events = parser.feed(b"data: {\"content\":\"hi\"}\n\ndata: [DONE]\n\n");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], SseEvent::Data("{\"content\":\"hi\"}".to_owned()));
    assert_eq!(events[1], SseEvent::Done);
}

#[test]
fn test_empty_lines_skipped() {
    let mut parser = SseLineBuffer::new();
    let events = parser.feed(b"\n\n\ndata: {\"content\":\"hi\"}\n\n\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], SseEvent::Data("{\"content\":\"hi\"}".to_owned()));
}

#[test]
fn test_flush_partial_line() {
    let mut parser = SseLineBuffer::new();
    let events = parser.feed(b"data: {\"final\":true}");
    assert!(events.is_empty(), "No newline = no events yet");

    let flushed = parser.flush();
    assert_eq!(flushed.len(), 1);
    assert_eq!(flushed[0], SseEvent::Data("{\"final\":true}".to_owned()));
}

#[test]
fn test_flush_empty_buffer() {
    let mut parser = SseLineBuffer::new();
    let flushed = parser.flush();
    assert!(flushed.is_empty());
}

#[test]
fn test_carriage_return_handling_unit() {
    let mut parser = SseLineBuffer::new();
    let events = parser.feed(b"data: {\"content\":\"hi\"}\r\n\r\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], SseEvent::Data("{\"content\":\"hi\"}".to_owned()));
}

#[test]
fn test_non_data_fields_ignored() {
    let mut parser = SseLineBuffer::new();
    let events =
        parser.feed(b"event: message\nid: 123\nretry: 5000\ndata: {\"content\":\"hi\"}\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], SseEvent::Data("{\"content\":\"hi\"}".to_owned()));
}

#[test]
fn test_multiple_partial_chunks() {
    let mut parser = SseLineBuffer::new();

    // Three separate chunks that together form two events
    let e1 = parser.feed(b"data: {\"a\":");
    assert!(e1.is_empty());

    let e2 = parser.feed(b"1}\n\ndata: {\"b\":");
    assert_eq!(e2.len(), 1);
    assert_eq!(e2[0], SseEvent::Data("{\"a\":1}".to_owned()));

    let e3 = parser.feed(b"2}\n\n");
    assert_eq!(e3.len(), 1);
    assert_eq!(e3[0], SseEvent::Data("{\"b\":2}".to_owned()));
}

#[test]
fn test_retry_config_delay() {
    let config = RetryConfig::default_config();
    // Attempt 0: ~500ms + jitter
    let delay0 = config.delay_for_attempt(0);
    assert!(delay0.as_millis() >= 500);
    assert!(delay0.as_millis() < 700);

    // Attempt 1: ~1000ms + jitter
    let delay1 = config.delay_for_attempt(1);
    assert!(delay1.as_millis() >= 1000);
    assert!(delay1.as_millis() < 1200);

    // Attempt 3: capped at 5000ms + jitter
    let delay3 = config.delay_for_attempt(3);
    assert!(delay3.as_millis() >= 4000);
    assert!(delay3.as_millis() <= 5100);
}

#[test]
fn test_retryable_status_codes() {
    assert!(is_retryable_status(429));
    assert!(is_retryable_status(502));
    assert!(is_retryable_status(503));
    assert!(!is_retryable_status(200));
    assert!(!is_retryable_status(400));
    assert!(!is_retryable_status(401));
    assert!(!is_retryable_status(404));
    assert!(!is_retryable_status(500));
}
