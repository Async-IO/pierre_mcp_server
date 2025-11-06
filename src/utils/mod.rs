// ABOUTME: Utility modules for common functionality across the application
// ABOUTME: Contains shared utilities for OAuth, parsing, authentication, and HTTP clients
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Authentication utilities and JWT handling
pub mod auth;
/// Error handling utilities
pub mod errors;
/// HTTP client configuration and helpers
pub mod http_client;
/// JSON response formatting utilities
pub mod json_responses;
/// Route timeout configuration and middleware
pub mod route_timeout;
/// UUID parsing and validation utilities
pub mod uuid;
