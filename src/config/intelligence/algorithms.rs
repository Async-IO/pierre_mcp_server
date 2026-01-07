// ABOUTME: Algorithm selection configuration for fitness calculations
// ABOUTME: Configures TSS, MaxHR, FTP, LTHR, and VO2max algorithm implementations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Algorithm Selection Configuration
//!
//! Configures which algorithm implementation to use for various fitness calculations.
//! Each algorithm type uses enum dispatch for type-safe selection with minimal runtime overhead.
//!
//! # Algorithm Types
//!
//! - **TSS**: Training Stress Score calculation (`avg_power`, `normalized_power`, `hybrid`)
//! - **`MaxHR`**: Maximum heart rate estimation (`fox`, `tanaka`, `nes`, `gulati`)
//! - **FTP**: Functional Threshold Power estimation
//! - **LTHR**: Lactate Threshold Heart Rate estimation
//! - **`VO2max`**: Maximum oxygen uptake estimation
//!
//! # Configuration Methods
//!
//! 1. Environment variables (highest priority):
//!    ```bash
//!    export PIERRE_TSS_ALGORITHM=normalized_power
//!    export PIERRE_MAXHR_ALGORITHM=tanaka
//!    ```
//!
//! 2. Default values (if env vars not set)

use serde::{Deserialize, Serialize};

/// Algorithm Selection Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// TSS calculation algorithm: `avg_power`, `normalized_power`, or `hybrid`
    #[serde(default = "default_tss_algorithm")]
    pub tss: String,

    /// Max HR estimation algorithm: `fox`, `tanaka`, `nes`, or `gulati`
    #[serde(default = "default_maxhr_algorithm")]
    pub maxhr: String,

    /// FTP estimation algorithm: `20min_test`, `from_vo2max`, `ramp_test`, etc.
    #[serde(default = "default_ftp_algorithm")]
    pub ftp: String,

    /// LTHR estimation algorithm: `from_maxhr`, `from_30min`, etc.
    #[serde(default = "default_lthr_algorithm")]
    pub lthr: String,

    /// `VO2max` estimation algorithm: `from_vdot`, `cooper_test`, etc.
    #[serde(default = "default_vo2max_algorithm")]
    pub vo2max: String,
}

/// Default TSS algorithm (`avg_power` for backwards compatibility)
fn default_tss_algorithm() -> String {
    "avg_power".to_owned()
}

/// Default Max HR algorithm (tanaka as most accurate)
fn default_maxhr_algorithm() -> String {
    "tanaka".to_owned()
}

/// Default FTP algorithm (`from_vo2max` as most accessible)
fn default_ftp_algorithm() -> String {
    "from_vo2max".to_owned()
}

/// Default LTHR algorithm (`from_maxhr` as most common)
fn default_lthr_algorithm() -> String {
    "from_maxhr".to_owned()
}

/// Default `VO2max` algorithm (`from_vdot` as most validated)
fn default_vo2max_algorithm() -> String {
    "from_vdot".to_owned()
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            tss: default_tss_algorithm(),
            maxhr: default_maxhr_algorithm(),
            ftp: default_ftp_algorithm(),
            lthr: default_lthr_algorithm(),
            vo2max: default_vo2max_algorithm(),
        }
    }
}
