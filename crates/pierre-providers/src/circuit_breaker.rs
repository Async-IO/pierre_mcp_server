// ABOUTME: Circuit breaker pattern implementation for external provider API calls
// ABOUTME: Prevents cascading failures by failing fast when providers are unavailable
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::future::Future;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{info, warn};

use super::errors::provider::ProviderError;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Circuit is open - requests fail immediately
    Open,
    /// Testing recovery - allowing one request through
    HalfOpen,
}

impl CircuitState {
    /// Convert from atomic u8 representation
    const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Closed,
            1 => Self::Open,
            _ => Self::HalfOpen,
        }
    }

    /// Convert to atomic u8 representation
    const fn to_u8(self) -> u8 {
        match self {
            Self::Closed => 0,
            Self::Open => 1,
            Self::HalfOpen => 2,
        }
    }
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Duration to wait before attempting recovery (half-open state)
    pub recovery_timeout: Duration,
    /// Duration after which success count resets in closed state
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 2,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new circuit breaker configuration
    #[must_use]
    pub const fn new(
        failure_threshold: u32,
        recovery_timeout: Duration,
        success_threshold: u32,
    ) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            success_threshold,
        }
    }

    /// Create a stricter configuration for unreliable providers
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 3,
        }
    }

    /// Create a lenient configuration for generally reliable providers
    #[must_use]
    pub const fn lenient() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout: Duration::from_secs(15),
            success_threshold: 1,
        }
    }
}

/// Thread-safe circuit breaker for external API calls
///
/// Implements the circuit breaker pattern to prevent cascading failures
/// when external provider APIs become unavailable or start failing.
///
/// # States
///
/// - **Closed**: Normal operation, requests pass through. Failures are counted.
/// - **Open**: Circuit is tripped after threshold failures. All requests fail immediately.
/// - **Half-Open**: After recovery timeout, one request is allowed through to test recovery.
///
/// # Thread Safety
///
/// All state is managed with atomic operations, making this safe for concurrent access
/// without requiring mutex locks.
pub struct CircuitBreaker {
    /// Provider name for logging and error messages
    provider_name: String,
    /// Current state (0=Closed, 1=Open, 2=HalfOpen)
    state: AtomicU32,
    /// Count of consecutive failures
    failure_count: AtomicU32,
    /// Count of consecutive successes in half-open state
    success_count: AtomicU32,
    /// Timestamp (epoch millis) when circuit was opened
    last_failure_time: AtomicU64,
    /// Configuration for thresholds and timeouts
    config: CircuitBreakerConfig,
    /// Start time for calculating elapsed durations
    start_instant: Instant,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    #[must_use]
    pub fn new(provider_name: &str) -> Self {
        Self::with_config(provider_name, CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom configuration
    #[must_use]
    pub fn with_config(provider_name: &str, config: CircuitBreakerConfig) -> Self {
        Self {
            provider_name: provider_name.to_owned(),
            state: AtomicU32::new(CircuitState::Closed.to_u8().into()),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            config,
            start_instant: Instant::now(),
        }
    }

    /// Get current circuit state
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn state(&self) -> CircuitState {
        CircuitState::from_u8(self.state.load(Ordering::SeqCst) as u8)
    }

    /// Get current failure count
    #[must_use]
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }

    /// Check if circuit allows requests
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        match self.state() {
            CircuitState::Closed => true,
            CircuitState::Open => self.should_attempt_recovery(),
            CircuitState::HalfOpen => false, // Only one request at a time in half-open
        }
    }

    /// Check if we should attempt recovery from open state
    fn should_attempt_recovery(&self) -> bool {
        let last_failure = self.last_failure_time.load(Ordering::SeqCst);
        let elapsed_ms = self.elapsed_millis();
        // Recovery timeout is typically 30-60 seconds, well within u64 range
        #[allow(clippy::cast_possible_truncation)]
        let recovery_ms = self.config.recovery_timeout.as_millis() as u64;

        if elapsed_ms.saturating_sub(last_failure) >= recovery_ms {
            // Attempt transition to half-open
            let expected = CircuitState::Open.to_u8().into();
            let new_state = CircuitState::HalfOpen.to_u8().into();
            if self
                .state
                .compare_exchange(expected, new_state, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                info!(
                    provider = %self.provider_name,
                    "Circuit breaker transitioning to half-open state for recovery test"
                );
                return true;
            }
        }
        false
    }

    /// Get elapsed time in milliseconds since circuit breaker creation
    fn elapsed_millis(&self) -> u64 {
        // Circuit breakers typically live for minutes/hours, well within u64 millisecond range
        #[allow(clippy::cast_possible_truncation)]
        {
            self.start_instant.elapsed().as_millis() as u64
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        match self.state() {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.success_threshold {
                    // Recovery confirmed, close circuit
                    self.state
                        .store(CircuitState::Closed.to_u8().into(), Ordering::SeqCst);
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    info!(
                        provider = %self.provider_name,
                        "Circuit breaker closed - provider recovered"
                    );
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&self) {
        match self.state() {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.failure_threshold {
                    // Trip the circuit
                    self.state
                        .store(CircuitState::Open.to_u8().into(), Ordering::SeqCst);
                    self.last_failure_time
                        .store(self.elapsed_millis(), Ordering::SeqCst);
                    warn!(
                        provider = %self.provider_name,
                        failures = count,
                        threshold = self.config.failure_threshold,
                        recovery_timeout_secs = self.config.recovery_timeout.as_secs(),
                        "Circuit breaker opened - provider failing"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Recovery test failed, re-open circuit
                self.state
                    .store(CircuitState::Open.to_u8().into(), Ordering::SeqCst);
                self.last_failure_time
                    .store(self.elapsed_millis(), Ordering::SeqCst);
                self.success_count.store(0, Ordering::SeqCst);
                warn!(
                    provider = %self.provider_name,
                    "Circuit breaker re-opened - recovery test failed"
                );
            }
            CircuitState::Open => {
                // Update last failure time
                self.last_failure_time
                    .store(self.elapsed_millis(), Ordering::SeqCst);
            }
        }
    }

    /// Execute an async operation with circuit breaker protection
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::CircuitBreakerOpen` if the circuit is open and recovery
    /// timeout hasn't elapsed. Otherwise returns the result of the wrapped operation.
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, ProviderError>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<ProviderError>,
    {
        if !self.is_allowed() {
            return Err(ProviderError::CircuitBreakerOpen {
                provider: self.provider_name.clone(),
                retry_after_secs: self.time_until_recovery(),
            });
        }

        match operation.await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                let provider_error = e.into();
                // Only count retryable errors as failures for circuit breaker
                if provider_error.is_retryable() {
                    self.record_failure();
                }
                Err(provider_error)
            }
        }
    }

    /// Calculate time remaining until recovery can be attempted
    fn time_until_recovery(&self) -> u64 {
        let last_failure = self.last_failure_time.load(Ordering::SeqCst);
        let elapsed = self.elapsed_millis();
        // Recovery timeout is typically 30-60 seconds, well within u64 range
        #[allow(clippy::cast_possible_truncation)]
        let recovery_ms = self.config.recovery_timeout.as_millis() as u64;

        let time_since_failure = elapsed.saturating_sub(last_failure);
        recovery_ms
            .saturating_sub(time_since_failure)
            .saturating_add(999)
            / 1000 // Convert to seconds, rounding up
    }

    /// Force reset the circuit breaker to closed state
    ///
    /// Use sparingly - this should typically only be called during testing
    /// or after manual verification that a provider has recovered.
    pub fn reset(&self) {
        self.state
            .store(CircuitState::Closed.to_u8().into(), Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        info!(
            provider = %self.provider_name,
            "Circuit breaker manually reset to closed state"
        );
    }
}
