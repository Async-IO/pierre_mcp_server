// ABOUTME: System coaches seeding module documentation
// ABOUTME: Coaches are seeded via the seed-coaches binary, not at runtime
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! System Coaches Seeding
//!
//! The 9 default AI coaching personas for Pierre Fitness are seeded using
//! a dedicated CLI tool rather than at server startup.
//!
//! # Usage
//!
//! ```bash
//! # After creating an admin user, run:
//! cargo run --bin seed-coaches
//!
//! # Or with verbose output:
//! cargo run --bin seed-coaches -- -v
//! ```
//!
//! # Coaches
//!
//! The system coaches include:
//! - **Training**: 5K Speed Coach, Marathon Coach, Half Marathon Coach
//! - **Recovery**: Sleep Optimization Coach, Recovery & Rest Day Coach
//! - **Nutrition**: Pre-Workout, Post-Workout, Race Day Nutrition Coaches
//! - **Analysis**: Activity Analysis Coach
//!
//! See `src/bin/seed_coaches.rs` for the full implementation.
