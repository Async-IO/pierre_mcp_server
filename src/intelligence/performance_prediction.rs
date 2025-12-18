// ABOUTME: Performance prediction using VDOT and Riegel formulas for race time estimation
// ABOUTME: Implements Jack Daniels' VDOT methodology and Riegel's race time prediction formula
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::AppError;
use crate::intelligence::algorithms::VdotAlgorithm;
use crate::models::Activity;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Standard race distances in meters
const DISTANCE_5K: f64 = 5_000.0;
const DISTANCE_10K: f64 = 10_000.0;
const DISTANCE_15K: f64 = 15_000.0;
const DISTANCE_HALF_MARATHON: f64 = 21_097.5;
const DISTANCE_MARATHON: f64 = 42_195.0;

/// Riegel formula exponent (typical value for running)
const RIEGEL_EXPONENT: f64 = 1.06;

/// Race predictions for standard distances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacePredictions {
    /// VDOT value (VO2 max adjusted for running economy)
    pub vdot: f64,
    /// Predicted race times in seconds for standard distances
    pub predictions: HashMap<String, f64>,
    /// Source activity used for calculation
    pub based_on_distance_meters: f64,
    /// Duration of source activity in seconds
    pub based_on_time_seconds: f64,
}

/// Performance prediction engine
pub struct PerformancePredictor;

impl PerformancePredictor {
    /// Calculate VDOT from race performance
    ///
    /// VDOT is Jack Daniels' VO2 max adjusted for running economy
    /// Delegates to `VdotAlgorithm::Daniels` for the calculation
    ///
    /// # Arguments
    /// * `distance_meters` - Race distance in meters
    /// * `time_seconds` - Race time in seconds
    ///
    /// # Returns
    /// VDOT value (typically 30-85 for recreational to elite runners)
    ///
    /// # Errors
    /// Returns `AppError::InvalidInput` if time or distance is non-positive, or if velocity is outside valid range
    pub fn calculate_vdot(distance_meters: f64, time_seconds: f64) -> Result<f64, AppError> {
        VdotAlgorithm::Daniels.calculate_vdot(distance_meters, time_seconds)
    }

    /// Predict race time using VDOT tables
    ///
    /// Uses Jack Daniels' VDOT training paces
    /// Delegates to `VdotAlgorithm::Daniels` for the calculation
    ///
    /// # Arguments
    /// * `vdot` - VDOT value
    /// * `target_distance_meters` - Target race distance
    ///
    /// # Returns
    /// Predicted race time in seconds
    ///
    /// # Errors
    /// Returns `AppError::InvalidInput` if VDOT is outside typical range (30-85)
    pub fn predict_time_vdot(vdot: f64, target_distance_meters: f64) -> Result<f64, AppError> {
        VdotAlgorithm::Daniels.predict_time(vdot, target_distance_meters)
    }

    /// Predict race time using Riegel formula
    ///
    /// Riegel's formula: Time2 = Time1 x (Distance2 / Distance1)^1.06
    ///
    /// This is a simpler alternative to VDOT that works reasonably well
    /// for predicting times at different distances
    ///
    /// # Arguments
    /// * `known_distance` - Distance of known race in meters
    /// * `known_time` - Time of known race in seconds
    /// * `target_distance` - Target race distance in meters
    ///
    /// # Errors
    /// Returns `AppError::InvalidInput` if any distance or time is non-positive
    pub fn predict_time_riegel(
        known_distance: f64,
        known_time: f64,
        target_distance: f64,
    ) -> Result<f64, AppError> {
        if known_distance <= 0.0 || known_time <= 0.0 || target_distance <= 0.0 {
            return Err(AppError::invalid_input(
                "All distances and times must be positive".to_owned(),
            ));
        }

        let distance_ratio = target_distance / known_distance;
        let predicted_time = known_time * distance_ratio.powf(RIEGEL_EXPONENT);

        Ok(predicted_time)
    }

    /// Generate predictions for standard race distances
    ///
    /// Given a single race performance, predicts times for 5K, 10K, 15K, Half, Marathon
    ///
    /// # Errors
    /// Returns `AppError::InvalidInput` if distance or time values are invalid for VDOT calculation
    pub fn generate_race_predictions(
        distance_meters: f64,
        time_seconds: f64,
    ) -> Result<RacePredictions, AppError> {
        let vdot = Self::calculate_vdot(distance_meters, time_seconds)?;

        let mut predictions = HashMap::new();

        // Predict standard distances
        let distances = vec![
            ("5K", DISTANCE_5K),
            ("10K", DISTANCE_10K),
            ("15K", DISTANCE_15K),
            ("Half Marathon", DISTANCE_HALF_MARATHON),
            ("Marathon", DISTANCE_MARATHON),
        ];

        for (name, distance) in distances {
            if let Ok(predicted_time) = Self::predict_time_vdot(vdot, distance) {
                predictions.insert(name.to_owned(), predicted_time);
            }
        }

        Ok(RacePredictions {
            vdot,
            predictions,
            based_on_distance_meters: distance_meters,
            based_on_time_seconds: time_seconds,
        })
    }

    /// Generate predictions from a best performance activity
    ///
    /// # Errors
    /// Returns `AppError::InvalidInput` if activity lacks distance or duration data
    pub fn generate_predictions_from_activity(
        activity: &Activity,
    ) -> Result<RacePredictions, AppError> {
        let distance = activity
            .distance_meters()
            .ok_or_else(|| AppError::invalid_input("Activity must have distance".to_owned()))?;

        let duration = activity.duration_seconds();

        #[allow(clippy::cast_precision_loss)]
        let duration_f64 = duration as f64;
        Self::generate_race_predictions(distance, duration_f64)
    }

    /// Find best performance from activities for race prediction
    ///
    /// Looks for fastest pace activities that are likely race efforts (>3km, <2 hours)
    #[must_use]
    pub fn find_best_performance(activities: &[Activity]) -> Option<&Activity> {
        activities
            .iter()
            .filter(|a| {
                // Filter for likely race efforts
                a.distance_meters().is_some_and(|distance| {
                    let duration = a.duration_seconds();
                    #[allow(clippy::cast_precision_loss)]
                    let duration_f64 = duration as f64;
                    // At least 3K distance
                    distance >= 3_000.0
                        // Less than 2 hours
                        && duration < 7_200
                        // Reasonable pace (faster than 8 min/km)
                        && (distance / duration_f64) > (1000.0 / 480.0)
                })
            })
            .max_by(|a, b| {
                // Find fastest pace
                #[allow(clippy::cast_precision_loss)]
                let pace_a = a
                    .distance_meters()
                    .map_or(0.0, |d| d / a.duration_seconds() as f64);
                #[allow(clippy::cast_precision_loss)]
                let pace_b = b
                    .distance_meters()
                    .map_or(0.0, |d| d / b.duration_seconds() as f64);
                pace_a.partial_cmp(&pace_b).unwrap_or(Ordering::Equal)
            })
    }

    /// Format time in seconds to human-readable format (HH:MM:SS)
    #[must_use]
    pub fn format_time(seconds: f64) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total_seconds = seconds.round() as u32;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;

        if hours > 0 {
            format!("{hours}:{minutes:02}:{secs:02}")
        } else {
            format!("{minutes}:{secs:02}")
        }
    }

    /// Format pace in min/km
    #[must_use]
    pub fn format_pace_per_km(meters_per_second: f64) -> String {
        if meters_per_second <= 0.0 {
            return "N/A".to_owned();
        }

        let seconds_per_km = 1000.0 / meters_per_second;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let minutes = (seconds_per_km / 60.0).floor() as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let seconds = (seconds_per_km % 60.0).round() as u32;

        format!("{minutes}:{seconds:02}")
    }
}
