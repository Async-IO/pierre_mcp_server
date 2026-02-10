// ABOUTME: Shared HTTP client with connection pooling for provider API calls
// ABOUTME: Singleton pattern with configurable timeouts initialized at server startup
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use reqwest::{Client, ClientBuilder};
use std::sync::OnceLock;
use std::time::Duration;

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default connection timeout in seconds
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Configured timeout values for the shared client
static CLIENT_TIMEOUTS: OnceLock<(u64, u64)> = OnceLock::new();

/// Global shared HTTP client with configured timeouts
static SHARED_CLIENT: OnceLock<Client> = OnceLock::new();

/// Initialize the shared HTTP client timeout configuration
///
/// Must be called once at server startup before any provider creates HTTP clients.
/// If not called, reasonable defaults are used (30s timeout, 10s connect timeout).
pub fn initialize_shared_client(timeout_secs: u64, connect_timeout_secs: u64) {
    let _ = CLIENT_TIMEOUTS.set((timeout_secs, connect_timeout_secs));
}

/// Get the shared HTTP client for provider API calls
///
/// This client uses connection pooling and configured timeouts.
/// Falls back to default timeouts if `initialize_shared_client()` was not called.
pub fn shared_client() -> &'static Client {
    SHARED_CLIENT.get_or_init(|| {
        let (timeout, connect_timeout) = CLIENT_TIMEOUTS
            .get()
            .copied()
            .unwrap_or((DEFAULT_TIMEOUT_SECS, DEFAULT_CONNECT_TIMEOUT_SECS));

        ClientBuilder::new()
            .timeout(Duration::from_secs(timeout))
            .connect_timeout(Duration::from_secs(connect_timeout))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}
