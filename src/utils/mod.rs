// ABOUTME: Utility modules for common functionality across the application
// ABOUTME: Contains shared utilities for OAuth, parsing, authentication, and HTTP clients
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Authentication utilities and JWT handling
pub mod auth;
/// Error handling utilities
pub mod errors;
/// HTML escaping utilities for XSS prevention in templates
pub mod html;
/// HTTP client configuration and helpers
pub mod http_client;
/// JSON response formatting utilities
pub mod json_responses;
/// Route timeout configuration and middleware
pub mod route_timeout;
/// UUID parsing and validation utilities
pub mod uuid;
