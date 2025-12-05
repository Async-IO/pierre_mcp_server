// ABOUTME: Common benchmark utilities and test fixtures for performance testing
// ABOUTME: Provides reusable data generators and setup functions for Criterion benchmarks
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Common benchmark utilities and test fixtures.
//!
//! Provides reusable data generators and setup functions for Criterion benchmarks.

pub mod fixtures;

// Re-export commonly used fixtures for benchmark modules
// Each benchmark imports only what it needs to avoid unused import warnings
