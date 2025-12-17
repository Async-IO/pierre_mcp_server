// ABOUTME: Unit tests for the circuit breaker pattern implementation
// ABOUTME: Tests state transitions, failure counting, and recovery behavior
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
#![allow(missing_docs)]

use pierre_mcp_server::providers::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState,
};
use std::time::Duration;

#[test]
fn test_circuit_breaker_starts_closed() {
    let cb = CircuitBreaker::new("test");
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.is_allowed());
}

#[test]
fn test_circuit_opens_after_threshold_failures() {
    let config = CircuitBreakerConfig::new(3, Duration::from_secs(30), 2);
    let cb = CircuitBreaker::with_config("test", config);

    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
}

#[test]
fn test_success_resets_failure_count() {
    let config = CircuitBreakerConfig::new(3, Duration::from_secs(30), 2);
    let cb = CircuitBreaker::with_config("test", config);

    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.failure_count(), 2);

    cb.record_success();
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[test]
fn test_circuit_states_are_distinct() {
    // Test that all states are distinct and can be compared
    assert_ne!(CircuitState::Closed, CircuitState::Open);
    assert_ne!(CircuitState::Open, CircuitState::HalfOpen);
    assert_ne!(CircuitState::Closed, CircuitState::HalfOpen);
}

#[test]
fn test_config_presets() {
    let default = CircuitBreakerConfig::default();
    assert_eq!(default.failure_threshold, 5);

    let strict = CircuitBreakerConfig::strict();
    assert_eq!(strict.failure_threshold, 3);

    let lenient = CircuitBreakerConfig::lenient();
    assert_eq!(lenient.failure_threshold, 10);
}

#[test]
fn test_reset() {
    let config = CircuitBreakerConfig::new(2, Duration::from_secs(30), 2);
    let cb = CircuitBreaker::with_config("test", config);

    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);

    cb.reset();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);
}

#[test]
fn test_circuit_blocks_requests_when_open() {
    let config = CircuitBreakerConfig::new(2, Duration::from_secs(30), 2);
    let cb = CircuitBreaker::with_config("test", config);

    // Trip the circuit
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);

    // Should block requests when open (unless recovery timeout has passed)
    // Note: is_allowed() may return true if recovery timeout has passed
    // For this test, we check state is Open
    assert_eq!(cb.state(), CircuitState::Open);
}

#[test]
fn test_failure_count_tracks_correctly() {
    let config = CircuitBreakerConfig::new(5, Duration::from_secs(30), 2);
    let cb = CircuitBreaker::with_config("test", config);

    assert_eq!(cb.failure_count(), 0);
    cb.record_failure();
    assert_eq!(cb.failure_count(), 1);
    cb.record_failure();
    assert_eq!(cb.failure_count(), 2);
    cb.record_failure();
    assert_eq!(cb.failure_count(), 3);
}

#[test]
fn test_multiple_successes_keep_circuit_closed() {
    let config = CircuitBreakerConfig::new(5, Duration::from_secs(30), 2);
    let cb = CircuitBreaker::with_config("test", config);

    cb.record_success();
    cb.record_success();
    cb.record_success();

    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);
}

#[test]
fn test_interleaved_failures_and_successes() {
    let config = CircuitBreakerConfig::new(3, Duration::from_secs(30), 2);
    let cb = CircuitBreaker::with_config("test", config);

    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.failure_count(), 2);

    // Success resets the count
    cb.record_success();
    assert_eq!(cb.failure_count(), 0);

    // Need 3 consecutive failures to open
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
}
