// ABOUTME: Error-related constants including codes and messages
// ABOUTME: Organizes error handling constants by protocol and domain
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Error constants module

/// Error codes and messages
pub mod codes;

// Re-export all error constants

/// Re-export all error code constants
pub use codes::*;
