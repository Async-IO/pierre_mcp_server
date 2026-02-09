// ABOUTME: Tool-specific error types re-exported from pierre-core
// ABOUTME: Provides structured errors that integrate with the main AppError system
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Tool Error Types
//!
//! Re-exports tool error types from `pierre-core`. These errors provide detailed
//! context for tool-related failures while maintaining compatibility with the
//! main `AppError` system.

pub use pierre_core::errors::tool::*;
