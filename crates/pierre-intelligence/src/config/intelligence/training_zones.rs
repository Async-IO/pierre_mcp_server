// ABOUTME: Training zone percentages configuration types
// ABOUTME: Handles VDOT and FTP zone percentage settings for training intensity
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::physiological_constants::training_zone_percentages::{ftp, vdot};
use serde::{Deserialize, Serialize};
use std::env;

/// Training zone percentages configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingZonesConfig {
    /// VDOT easy pace zone percentage
    pub vdot_easy_zone_percent: f64,
    /// VDOT tempo pace zone percentage
    pub vdot_tempo_zone_percent: f64,
    /// VDOT threshold pace zone percentage
    pub vdot_threshold_zone_percent: f64,
    /// VDOT interval pace zone percentage
    pub vdot_interval_zone_percent: f64,
    /// VDOT repetition pace zone percentage
    pub vdot_repetition_zone_percent: f64,
    /// FTP Zone 1 percentage (Active Recovery)
    pub ftp_zone1_percent: u32,
    /// FTP Zone 2 percentage (Endurance)
    pub ftp_zone2_percent: u32,
    /// FTP Zone 3 percentage (Tempo)
    pub ftp_zone3_percent: u32,
    /// FTP Zone 4 percentage (Lactate Threshold)
    pub ftp_zone4_percent: u32,
    /// FTP Zone 5 percentage (VO2 Max)
    pub ftp_zone5_percent: u32,
}

impl Default for TrainingZonesConfig {
    fn default() -> Self {
        Self {
            vdot_easy_zone_percent: vdot::EASY_ZONE_PERCENT,
            vdot_tempo_zone_percent: vdot::TEMPO_ZONE_PERCENT,
            vdot_threshold_zone_percent: vdot::THRESHOLD_ZONE_PERCENT,
            vdot_interval_zone_percent: vdot::INTERVAL_ZONE_PERCENT,
            vdot_repetition_zone_percent: vdot::REPETITION_ZONE_PERCENT,
            ftp_zone1_percent: ftp::ZONE1_PERCENT,
            ftp_zone2_percent: ftp::ZONE2_PERCENT,
            ftp_zone3_percent: ftp::ZONE3_PERCENT,
            ftp_zone4_percent: ftp::ZONE4_PERCENT,
            ftp_zone5_percent: ftp::ZONE5_PERCENT,
        }
    }
}

impl TrainingZonesConfig {
    /// Load training zones configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            vdot_easy_zone_percent: env::var("TRAINING_ZONES_VDOT_EASY_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(vdot::EASY_ZONE_PERCENT),
            vdot_tempo_zone_percent: env::var("TRAINING_ZONES_VDOT_TEMPO_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(vdot::TEMPO_ZONE_PERCENT),
            vdot_threshold_zone_percent: env::var("TRAINING_ZONES_VDOT_THRESHOLD_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(vdot::THRESHOLD_ZONE_PERCENT),
            vdot_interval_zone_percent: env::var("TRAINING_ZONES_VDOT_INTERVAL_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(vdot::INTERVAL_ZONE_PERCENT),
            vdot_repetition_zone_percent: env::var("TRAINING_ZONES_VDOT_REPETITION_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(vdot::REPETITION_ZONE_PERCENT),
            ftp_zone1_percent: env::var("TRAINING_ZONES_FTP_ZONE1_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(ftp::ZONE1_PERCENT),
            ftp_zone2_percent: env::var("TRAINING_ZONES_FTP_ZONE2_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(ftp::ZONE2_PERCENT),
            ftp_zone3_percent: env::var("TRAINING_ZONES_FTP_ZONE3_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(ftp::ZONE3_PERCENT),
            ftp_zone4_percent: env::var("TRAINING_ZONES_FTP_ZONE4_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(ftp::ZONE4_PERCENT),
            ftp_zone5_percent: env::var("TRAINING_ZONES_FTP_ZONE5_PERCENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(ftp::ZONE5_PERCENT),
        }
    }
}
