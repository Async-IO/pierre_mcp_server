// ABOUTME: Performance prediction using VDOT and Riegel formulas for race time estimation
// ABOUTME: Implements Jack Daniels' VDOT methodology and Riegel's race time prediction formula
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::errors::AppError;
use crate::models::Activity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard race distances in meters
const DISTANCE_5K: f64 = 5_000.0;
const DISTANCE_10K: f64 = 10_000.0;
const DISTANCE_15K: f64 = 15_000.0;
const DISTANCE_HALF_MARATHON: f64 = 21_097.5;
const DISTANCE_MARATHON: f64 = 42_195.0;

/// Riegel formula exponent (typical value for running)
const RIEGEL_EXPONENT: f64 = 1.06;

/// Minimum velocity for VDOT calculation (m/min)
const MIN_VELOCITY: f64 = 100.0;

/// Maximum velocity for VDOT calculation (m/min)
const MAX_VELOCITY: f64 = 500.0;

/// Race predictions for standard distances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacePredictions {
    /// VDOT value (VO2 max adjusted for running economy)
    pub vdot: f64,
    /// Predicted race times in seconds for standard distances
    pub predictions: HashMap<String, f64>,
    /// Source activity used for calculation
    pub based_on_distance_meters: f64,
    pub based_on_time_seconds: f64,
}

/// Performance prediction engine
pub struct PerformancePredictor;

impl PerformancePredictor {
    /// Calculate VDOT from race performance
    ///
    /// VDOT is Jack Daniels' VO2 max adjusted for running economy
    /// Formula: VO2 = -4.60 + 0.182258 × velocity + 0.000104 × velocity²
    /// where velocity is in meters per minute
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
        if time_seconds <= 0.0 {
            return Err(AppError::invalid_input("Time must be positive".to_string()));
        }

        if distance_meters <= 0.0 {
            return Err(AppError::invalid_input(
                "Distance must be positive".to_string(),
            ));
        }

        // Convert to velocity in meters per minute
        let velocity = (distance_meters / time_seconds) * 60.0;

        if !(MIN_VELOCITY..=MAX_VELOCITY).contains(&velocity) {
            return Err(AppError::invalid_input(format!(
                "Velocity {velocity:.1} m/min is outside valid range ({MIN_VELOCITY}-{MAX_VELOCITY})"
            )));
        }

        // Jack Daniels' VO2 formula: VO2 = -4.60 + 0.182258×v + 0.000104×v²
        let vo2 = (0.000_104 * velocity).mul_add(velocity, 0.182_258f64.mul_add(velocity, -4.60));

        // VDOT = VO2max. We divide by the percent-max adjustment because the adjustment
        // represents what fraction of VO2max was used during the race.
        // To get true VO2max (VDOT), we need: VDOT = VO2_during_race / percent_used
        let percent_used = Self::calculate_percent_max_adjustment(time_seconds);
        let vdot = vo2 / percent_used;

        Ok(vdot)
    }

    /// Calculate adjustment factor based on race duration
    ///
    /// Shorter races use less of VO2 max due to oxygen deficit
    /// Longer races use less due to accumulated fatigue
    fn calculate_percent_max_adjustment(time_seconds: f64) -> f64 {
        let time_minutes = time_seconds / 60.0;

        // Adjustment factors based on race duration
        if time_minutes < 5.0 {
            0.97 // Very short race - oxygen deficit
        } else if time_minutes < 15.0 {
            0.99 // 5K range
        } else if time_minutes < 30.0 {
            1.00 // 10K-15K range - optimal
        } else if time_minutes < 90.0 {
            0.98 // Half marathon range
        } else {
            0.95 // Marathon+ range - fatigue accumulation
        }
    }

    /// Predict race time using VDOT tables
    ///
    /// Uses Jack Daniels' VDOT training paces
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
        if !(30.0..=85.0).contains(&vdot) {
            return Err(AppError::invalid_input(format!(
                "VDOT {vdot:.1} is outside typical range (30-85)"
            )));
        }

        // Calculate velocity at VO2 max (reverse of VDOT formula)
        // vo2 = -4.60 + 0.182258 × v + 0.000104 × v²
        // Solve quadratic: 0.000104v² + 0.182258v - (vo2 + 4.60) = 0

        let a: f64 = 0.000_104;
        let b: f64 = 0.182_258;
        let c: f64 = -(vdot + 4.60);

        let discriminant = b.mul_add(b, -(4.0 * a * c));
        if discriminant < 0.0 {
            return Err(AppError::internal("Invalid VDOT calculation".to_string()));
        }

        let velocity_max = (-b + discriminant.sqrt()) / (2.0 * a);

        // Calculate race-specific velocity based on distance
        let race_velocity = Self::calculate_race_velocity(velocity_max, target_distance_meters);

        // Calculate time from velocity
        let time_seconds = (target_distance_meters / race_velocity) * 60.0;

        Ok(time_seconds)
    }

    /// Calculate sustainable race velocity based on distance
    ///
    /// Longer races require lower percentage of VO2 max velocity
    fn calculate_race_velocity(velocity_max: f64, distance_meters: f64) -> f64 {
        let percent_max = if distance_meters <= DISTANCE_5K {
            0.98 // 5K: 98% of VO2 max velocity
        } else if distance_meters <= DISTANCE_10K {
            0.94 // 10K: 94% of VO2 max velocity
        } else if distance_meters <= DISTANCE_15K {
            0.91 // 15K: 91% of VO2 max velocity
        } else if distance_meters <= DISTANCE_HALF_MARATHON {
            0.88 // Half marathon: 88% of VO2 max velocity
        } else if distance_meters <= DISTANCE_MARATHON {
            0.84 // Marathon: 84% of VO2 max velocity
        } else {
            // Ultra distances: progressively lower percentages
            let marathon_ratio = distance_meters / DISTANCE_MARATHON;
            (marathon_ratio - 1.0).mul_add(-0.02, 0.84).max(0.70)
        };

        velocity_max * percent_max
    }

    /// Predict race time using Riegel formula
    ///
    /// Riegel's formula: Time2 = Time1 × (Distance2 / Distance1)^1.06
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
                "All distances and times must be positive".to_string(),
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
                predictions.insert(name.to_string(), predicted_time);
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
            .distance_meters
            .ok_or_else(|| AppError::invalid_input("Activity must have distance".to_string()))?;

        let duration = activity.duration_seconds;

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
                a.distance_meters.is_some_and(|distance| {
                    let duration = a.duration_seconds;
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
                    .distance_meters
                    .map_or(0.0, |d| d / a.duration_seconds as f64);
                #[allow(clippy::cast_precision_loss)]
                let pace_b = b
                    .distance_meters
                    .map_or(0.0, |d| d / b.duration_seconds as f64);
                pace_a
                    .partial_cmp(&pace_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
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
            return "N/A".to_string();
        }

        let seconds_per_km = 1000.0 / meters_per_second;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let minutes = (seconds_per_km / 60.0).floor() as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let seconds = (seconds_per_km % 60.0).round() as u32;

        format!("{minutes}:{seconds:02}")
    }
}
