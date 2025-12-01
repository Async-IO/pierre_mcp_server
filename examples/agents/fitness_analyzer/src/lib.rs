// ABOUTME: Library exports for fitness_analyzer agent
// ABOUTME: Makes modules available for integration tests
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Allow some clippy lints for example code that would be too pedantic
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(dead_code)] // Some fields are intentionally unused in this example

pub mod a2a_client;
pub mod analyzer;
pub mod config;
pub mod scheduler;