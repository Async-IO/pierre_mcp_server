// ABOUTME: Protocol handlers module providing MCP, A2A, and REST API interfaces
// ABOUTME: Unified entry point for all communication protocols supported by Pierre server
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Universal Protocol Support
//!
//! This module provides a universal interface for executing tools
//! across different protocols (MCP, A2A) supported by Pierre.

/// Protocol conversion utilities
pub mod converter;
/// Universal protocol abstractions
pub mod universal;

/// Protocol converter for translating between protocols
pub use converter::ProtocolConverter;
/// Universal request structure
pub use universal::UniversalRequest;
/// Universal response structure
pub use universal::UniversalResponse;
/// Universal tool definition
pub use universal::UniversalTool;
/// Universal tool executor
pub use universal::UniversalToolExecutor;

// Re-export protocol error types from pierre-core
pub use pierre_core::errors::protocol::{ProtocolError, ProtocolType};
