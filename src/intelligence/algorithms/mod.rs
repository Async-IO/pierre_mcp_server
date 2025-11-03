// ABOUTME: Algorithm abstraction layer enabling pluggable calculation methods for fitness intelligence
// ABOUTME: Provides enum-based dispatch for TSS, TRIMP, MaxHR, and statistical analysis algorithms

//! Algorithm Selection Module
//!
//! This module provides a type-safe, enum-based system for selecting between different
//! algorithm implementations for fitness calculations. Each algorithm type uses enum
//! dispatch for built-in implementations with an extension point for custom algorithms.
//!
//! # Design Philosophy
//!
//! - **Type Safety**: Algorithms are enums, not strings or booleans
//! - **Performance**: Enum dispatch is fast (no vtable overhead for built-in algorithms)
//! - **Extensibility**: Custom variant allows for future plugin support
//! - **Idiomatic**: Matches Rust patterns like `std::io::ErrorKind`
//!
//! # Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::intelligence::algorithms::TssAlgorithm;
//!
//! let algorithm = TssAlgorithm::NormalizedPower { window_seconds: 30 };
//! let tss = algorithm.calculate(&activity, ftp, duration_hours)?;
//! ```

pub mod ftp;
pub mod lthr;
pub mod maxhr;
pub mod recovery_aggregation;
pub mod training_load;
pub mod trimp;
pub mod tss;
pub mod vdot;
pub mod vo2max;

// Re-export algorithm types
pub use ftp::FtpAlgorithm;
pub use lthr::LthrAlgorithm;
pub use maxhr::MaxHrAlgorithm;
pub use recovery_aggregation::RecoveryAggregationAlgorithm;
pub use training_load::TrainingLoadAlgorithm;
pub use trimp::TrimpAlgorithm;
pub use tss::TssAlgorithm;
pub use vdot::VdotAlgorithm;
pub use vo2max::Vo2maxAlgorithm;
