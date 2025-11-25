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
//! use pierre_mcp_server::intelligence::algorithms::tss::TssAlgorithm;
//! use pierre_mcp_server::models::Activity;
//! use pierre_mcp_server::errors::AppResult;
//!
//! # fn example(activity: &Activity, ftp: f64, duration_hours: f64) -> AppResult<()> {
//! let algorithm = TssAlgorithm::NormalizedPower { window_seconds: 30 };
//! let tss = algorithm.calculate(activity, ftp, duration_hours)?;
//! # Ok(())
//! # }
//! ```

/// Functional Threshold Power (FTP) calculation algorithms
pub mod ftp;
/// Lactate Threshold Heart Rate (LTHR) estimation algorithms
pub mod lthr;
/// Maximum Heart Rate (`MaxHR`) calculation methods
pub mod maxhr;
/// Recovery score aggregation algorithms
pub mod recovery_aggregation;
/// Training load calculation methods (TSS, TRIMP, etc.)
pub mod training_load;
/// Training Impulse (TRIMP) calculation algorithms
pub mod trimp;
/// Training Stress Score (TSS) calculation algorithms
pub mod tss;
/// VDOT running performance calculation
pub mod vdot;
/// `VO2max` estimation algorithms
pub mod vo2max;

// Re-export algorithm types

/// FTP (Functional Threshold Power) calculation algorithm
pub use ftp::FtpAlgorithm;
/// LTHR (Lactate Threshold Heart Rate) estimation algorithm
pub use lthr::LthrAlgorithm;
/// Maximum heart rate calculation algorithm
pub use maxhr::MaxHrAlgorithm;
/// Recovery score aggregation algorithm
pub use recovery_aggregation::RecoveryAggregationAlgorithm;
/// Training load calculation algorithm
pub use training_load::TrainingLoadAlgorithm;
/// TRIMP (Training Impulse) calculation algorithm
pub use trimp::TrimpAlgorithm;
/// TSS (Training Stress Score) calculation algorithm
pub use tss::TssAlgorithm;
/// VDOT running performance algorithm
pub use vdot::VdotAlgorithm;
/// `VO2max` estimation algorithm
pub use vo2max::Vo2maxAlgorithm;
