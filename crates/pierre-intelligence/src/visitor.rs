// ABOUTME: Visitor pattern for single-pass activity time series analysis
// ABOUTME: Enables efficient data processing without multiple iterations over streams
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Time Series Visitor Pattern
//!
//! Provides a visitor pattern for processing activity time series data in a single pass.
//! This reduces memory allocations and improves performance when multiple analyses
//! need to be performed on the same data.
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::intelligence::visitor::{TimeSeriesVisitor, StatsCollector};
//! use pierre_mcp_server::models::TimeSeriesData;
//!
//! // Create time series data
//! let time_series = TimeSeriesData {
//!     timestamps: vec![0, 1, 2],
//!     heart_rate: Some(vec![120, 130, 140]),
//!     power: None,
//!     cadence: None,
//!     speed: None,
//!     altitude: None,
//!     temperature: None,
//!     gps_coordinates: None,
//! };
//! let mut stats = StatsCollector::default();
//! time_series.accept(&mut stats);
//!
//! if let Some(avg) = stats.heart_rate.average() {
//!     println!("Average HR: {}", avg);
//! }
//! ```

use crate::models::TimeSeriesData;

/// Visitor trait for processing time series data streams in a single pass.
///
/// Implement this trait to create custom analyzers that process activity data
/// efficiently. Default implementations are no-ops, so you only need to override
/// the methods for data streams you care about.
///
/// The visitor methods receive both the value and the timestamp offset from
/// activity start (in seconds), enabling time-aware analysis.
pub trait TimeSeriesVisitor {
    /// Called before iteration begins. Use for initialization.
    fn start(&mut self) {}

    /// Visit a heart rate measurement.
    ///
    /// # Arguments
    /// * `bpm` - Heart rate in beats per minute
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_heart_rate(&mut self, bpm: u32, timestamp: u32) {}

    /// Visit a power measurement.
    ///
    /// # Arguments
    /// * `watts` - Power output in watts
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_power(&mut self, watts: u32, timestamp: u32) {}

    /// Visit a cadence measurement.
    ///
    /// # Arguments
    /// * `rpm` - Cadence in revolutions/steps per minute
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_cadence(&mut self, rpm: u32, timestamp: u32) {}

    /// Visit a speed measurement.
    ///
    /// # Arguments
    /// * `meters_per_sec` - Speed in meters per second
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_speed(&mut self, meters_per_sec: f32, timestamp: u32) {}

    /// Visit an altitude measurement.
    ///
    /// # Arguments
    /// * `meters` - Altitude in meters
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_altitude(&mut self, meters: f32, timestamp: u32) {}

    /// Visit a temperature measurement.
    ///
    /// # Arguments
    /// * `celsius` - Temperature in degrees Celsius
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_temperature(&mut self, celsius: f32, timestamp: u32) {}

    /// Visit a GPS coordinate.
    ///
    /// # Arguments
    /// * `lat` - Latitude in degrees
    /// * `lon` - Longitude in degrees
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_location(&mut self, lat: f64, lon: f64, timestamp: u32) {}

    /// Called after iteration completes. Use for finalization and cleanup.
    fn finish(&mut self) {}
}

/// Extension trait for `TimeSeriesData` to support the visitor pattern.
///
/// Enables single-pass iteration over time series data with custom analyzers.
pub trait TimeSeriesExt {
    /// Accept a visitor and iterate over all time series data in a single pass.
    fn accept<V: TimeSeriesVisitor>(&self, visitor: &mut V);

    /// Accept multiple visitors and iterate over all data in a single pass.
    fn accept_all(&self, visitors: &mut [&mut dyn TimeSeriesVisitor]);
}

impl TimeSeriesExt for TimeSeriesData {
    /// Accept a visitor and iterate over all time series data in a single pass.
    ///
    /// This method iterates through the timestamps once, calling the appropriate
    /// visitor methods for each available data stream at each timestamp.
    ///
    /// # Arguments
    /// * `visitor` - A mutable reference to a type implementing `TimeSeriesVisitor`
    ///
    /// # Example
    ///
    /// See module-level documentation for a complete example.
    fn accept<V: TimeSeriesVisitor>(&self, visitor: &mut V) {
        visitor.start();

        for (idx, &timestamp) in self.timestamps.iter().enumerate() {
            // Visit heart rate if available at this index
            if let Some(hr_data) = &self.heart_rate {
                if let Some(&bpm) = hr_data.get(idx) {
                    visitor.visit_heart_rate(bpm, timestamp);
                }
            }

            // Visit power if available at this index
            if let Some(power_data) = &self.power {
                if let Some(&watts) = power_data.get(idx) {
                    visitor.visit_power(watts, timestamp);
                }
            }

            // Visit cadence if available at this index
            if let Some(cadence_data) = &self.cadence {
                if let Some(&rpm) = cadence_data.get(idx) {
                    visitor.visit_cadence(rpm, timestamp);
                }
            }

            // Visit speed if available at this index
            if let Some(speed_data) = &self.speed {
                if let Some(&speed) = speed_data.get(idx) {
                    visitor.visit_speed(speed, timestamp);
                }
            }

            // Visit altitude if available at this index
            if let Some(altitude_data) = &self.altitude {
                if let Some(&alt) = altitude_data.get(idx) {
                    visitor.visit_altitude(alt, timestamp);
                }
            }

            // Visit temperature if available at this index
            if let Some(temp_data) = &self.temperature {
                if let Some(&temp) = temp_data.get(idx) {
                    visitor.visit_temperature(temp, timestamp);
                }
            }

            // Visit GPS coordinates if available at this index
            if let Some(gps_data) = &self.gps_coordinates {
                if let Some(&(lat, lon)) = gps_data.get(idx) {
                    visitor.visit_location(lat, lon, timestamp);
                }
            }
        }

        visitor.finish();
    }

    /// Accept multiple visitors and iterate over all data in a single pass.
    ///
    /// This is more efficient than calling `accept` multiple times when you
    /// need to run several analyses on the same data.
    ///
    /// # Arguments
    /// * `visitors` - A slice of mutable visitor references
    fn accept_all(&self, visitors: &mut [&mut dyn TimeSeriesVisitor]) {
        for visitor in visitors.iter_mut() {
            visitor.start();
        }

        for (idx, &timestamp) in self.timestamps.iter().enumerate() {
            if let Some(hr_data) = &self.heart_rate {
                if let Some(&bpm) = hr_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_heart_rate(bpm, timestamp);
                    }
                }
            }

            if let Some(power_data) = &self.power {
                if let Some(&watts) = power_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_power(watts, timestamp);
                    }
                }
            }

            if let Some(cadence_data) = &self.cadence {
                if let Some(&rpm) = cadence_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_cadence(rpm, timestamp);
                    }
                }
            }

            if let Some(speed_data) = &self.speed {
                if let Some(&speed) = speed_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_speed(speed, timestamp);
                    }
                }
            }

            if let Some(altitude_data) = &self.altitude {
                if let Some(&alt) = altitude_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_altitude(alt, timestamp);
                    }
                }
            }

            if let Some(temp_data) = &self.temperature {
                if let Some(&temp) = temp_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_temperature(temp, timestamp);
                    }
                }
            }

            if let Some(gps_data) = &self.gps_coordinates {
                if let Some(&(lat, lon)) = gps_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_location(lat, lon, timestamp);
                    }
                }
            }
        }

        for visitor in visitors.iter_mut() {
            visitor.finish();
        }
    }
}

// === Built-in Visitor Implementations ===

/// Statistics for a numeric stream (min, max, sum, count, average).
#[derive(Debug, Clone, Default)]
pub struct StreamStats {
    /// Minimum value observed
    pub min: Option<f64>,
    /// Maximum value observed
    pub max: Option<f64>,
    /// Sum of all values
    pub sum: f64,
    /// Number of data points
    pub count: u64,
}

impl StreamStats {
    /// Calculate the average value.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn average(&self) -> Option<f64> {
        if self.count > 0 {
            // Safe: Activity data never approaches 2^52 data points where precision loss matters
            Some(self.sum / self.count as f64)
        } else {
            None
        }
    }

    /// Update stats with a new value.
    fn update(&mut self, value: f64) {
        self.min = Some(self.min.map_or(value, |m| m.min(value)));
        self.max = Some(self.max.map_or(value, |m| m.max(value)));
        self.sum += value;
        self.count += 1;
    }
}

/// Collects basic statistics for all numeric streams in a single pass.
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::intelligence::visitor::{TimeSeriesVisitor, StatsCollector};
/// use pierre_mcp_server::models::TimeSeriesData;
///
/// let time_series = TimeSeriesData {
///     timestamps: vec![0, 1, 2, 3, 4],
///     heart_rate: Some(vec![120, 130, 140, 135, 125]),
///     power: Some(vec![200, 220, 240, 230, 210]),
///     cadence: None,
///     speed: None,
///     altitude: None,
///     temperature: None,
///     gps_coordinates: None,
/// };
///
/// let mut stats = StatsCollector::default();
/// time_series.accept(&mut stats);
///
/// // Access statistics for each stream
/// if let Some(avg_hr) = stats.heart_rate.average() {
///     println!("Average HR: {:.1} bpm", avg_hr);
/// }
/// if let (Some(min), Some(max)) = (stats.power.min, stats.power.max) {
///     println!("Power range: {:.0}-{:.0} watts", min, max);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct StatsCollector {
    /// Heart rate statistics
    pub heart_rate: StreamStats,
    /// Power statistics
    pub power: StreamStats,
    /// Cadence statistics
    pub cadence: StreamStats,
    /// Speed statistics
    pub speed: StreamStats,
    /// Altitude statistics
    pub altitude: StreamStats,
    /// Temperature statistics
    pub temperature: StreamStats,
}

impl TimeSeriesVisitor for StatsCollector {
    fn visit_heart_rate(&mut self, bpm: u32, _timestamp: u32) {
        self.heart_rate.update(f64::from(bpm));
    }

    fn visit_power(&mut self, watts: u32, _timestamp: u32) {
        self.power.update(f64::from(watts));
    }

    fn visit_cadence(&mut self, rpm: u32, _timestamp: u32) {
        self.cadence.update(f64::from(rpm));
    }

    fn visit_speed(&mut self, meters_per_sec: f32, _timestamp: u32) {
        self.speed.update(f64::from(meters_per_sec));
    }

    fn visit_altitude(&mut self, meters: f32, _timestamp: u32) {
        self.altitude.update(f64::from(meters));
    }

    fn visit_temperature(&mut self, celsius: f32, _timestamp: u32) {
        self.temperature.update(f64::from(celsius));
    }
}

/// Heart rate zone boundaries (percentage of max HR).
#[derive(Debug, Clone)]
pub struct ZoneBoundaries {
    /// Zone 1 upper boundary (Recovery, typically 60%)
    pub zone1_upper: f64,
    /// Zone 2 upper boundary (Endurance, typically 70%)
    pub zone2_upper: f64,
    /// Zone 3 upper boundary (Tempo, typically 80%)
    pub zone3_upper: f64,
    /// Zone 4 upper boundary (Threshold, typically 90%)
    pub zone4_upper: f64,
}

impl Default for ZoneBoundaries {
    fn default() -> Self {
        Self {
            zone1_upper: 0.60,
            zone2_upper: 0.70,
            zone3_upper: 0.80,
            zone4_upper: 0.90,
        }
    }
}

/// Calculates time spent in each heart rate zone.
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::intelligence::visitor::{
///     TimeSeriesVisitor, ZoneTimeCalculator, ZoneBoundaries
/// };
/// use pierre_mcp_server::models::TimeSeriesData;
///
/// // Create calculator with max HR of 185 bpm
/// let boundaries = ZoneBoundaries::default(); // 60%, 70%, 80%, 90% thresholds
/// let mut zones = ZoneTimeCalculator::new(185, boundaries);
///
/// let time_series = TimeSeriesData {
///     timestamps: vec![0, 1, 2, 3, 4, 5],
///     heart_rate: Some(vec![100, 120, 140, 160, 170, 150]),
///     power: None,
///     cadence: None,
///     speed: None,
///     altitude: None,
///     temperature: None,
///     gps_coordinates: None,
/// };
///
/// time_series.accept(&mut zones);
///
/// let distribution = zones.zone_distribution();
/// println!("Zone 1 (Recovery): {:.1}%", distribution.zone1_pct);
/// println!("Zone 2 (Endurance): {:.1}%", distribution.zone2_pct);
/// println!("Zone 3 (Tempo): {:.1}%", distribution.zone3_pct);
/// println!("Zone 4 (Threshold): {:.1}%", distribution.zone4_pct);
/// println!("Zone 5 (VO2max): {:.1}%", distribution.zone5_pct);
/// ```
#[derive(Debug, Clone)]
pub struct ZoneTimeCalculator {
    max_hr: u32,
    boundaries: ZoneBoundaries,
    zone1_seconds: u32,
    zone2_seconds: u32,
    zone3_seconds: u32,
    zone4_seconds: u32,
    zone5_seconds: u32,
    last_timestamp: Option<u32>,
}

impl ZoneTimeCalculator {
    /// Create a new zone time calculator.
    ///
    /// # Arguments
    /// * `max_hr` - Maximum heart rate for zone calculation
    /// * `boundaries` - Zone boundary percentages
    #[must_use]
    pub const fn new(max_hr: u32, boundaries: ZoneBoundaries) -> Self {
        Self {
            max_hr,
            boundaries,
            zone1_seconds: 0,
            zone2_seconds: 0,
            zone3_seconds: 0,
            zone4_seconds: 0,
            zone5_seconds: 0,
            last_timestamp: None,
        }
    }

    /// Get the zone distribution as percentages.
    #[must_use]
    pub fn zone_distribution(&self) -> ZoneDistributionResult {
        let total = self.zone1_seconds
            + self.zone2_seconds
            + self.zone3_seconds
            + self.zone4_seconds
            + self.zone5_seconds;

        if total == 0 {
            return ZoneDistributionResult::default();
        }

        let total_f64 = f64::from(total);
        ZoneDistributionResult {
            zone1_pct: f64::from(self.zone1_seconds) / total_f64 * 100.0,
            zone2_pct: f64::from(self.zone2_seconds) / total_f64 * 100.0,
            zone3_pct: f64::from(self.zone3_seconds) / total_f64 * 100.0,
            zone4_pct: f64::from(self.zone4_seconds) / total_f64 * 100.0,
            zone5_pct: f64::from(self.zone5_seconds) / total_f64 * 100.0,
            total_seconds: total,
        }
    }

    /// Get the zone for a given heart rate.
    fn get_zone(&self, bpm: u32) -> u8 {
        if self.max_hr == 0 {
            return 1;
        }
        let hr_pct = f64::from(bpm) / f64::from(self.max_hr);

        if hr_pct <= self.boundaries.zone1_upper {
            1
        } else if hr_pct <= self.boundaries.zone2_upper {
            2
        } else if hr_pct <= self.boundaries.zone3_upper {
            3
        } else if hr_pct <= self.boundaries.zone4_upper {
            4
        } else {
            5
        }
    }
}

/// Result of zone time calculation.
#[derive(Debug, Clone, Default)]
pub struct ZoneDistributionResult {
    /// Percentage of time in Zone 1 (Recovery)
    pub zone1_pct: f64,
    /// Percentage of time in Zone 2 (Endurance)
    pub zone2_pct: f64,
    /// Percentage of time in Zone 3 (Tempo)
    pub zone3_pct: f64,
    /// Percentage of time in Zone 4 (Threshold)
    pub zone4_pct: f64,
    /// Percentage of time in Zone 5 (VO2 max)
    pub zone5_pct: f64,
    /// Total time in seconds
    pub total_seconds: u32,
}

impl TimeSeriesVisitor for ZoneTimeCalculator {
    fn visit_heart_rate(&mut self, bpm: u32, timestamp: u32) {
        // Calculate time delta since last measurement
        let delta = self.last_timestamp.map_or(1, |last| {
            if timestamp > last {
                timestamp - last
            } else {
                1
            }
        });

        match self.get_zone(bpm) {
            1 => self.zone1_seconds += delta,
            2 => self.zone2_seconds += delta,
            3 => self.zone3_seconds += delta,
            4 => self.zone4_seconds += delta,
            _ => self.zone5_seconds += delta,
        }

        self.last_timestamp = Some(timestamp);
    }
}

/// Calculates normalized power using the 30-second rolling average method.
///
/// Normalized Power (NP) represents the metabolic cost of an activity,
/// accounting for the non-linear physiological response to varying power outputs.
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::intelligence::visitor::{TimeSeriesVisitor, NormalizedPowerCalculator};
/// use pierre_mcp_server::models::TimeSeriesData;
///
/// // Create power data (at least 30 seconds needed for NP calculation)
/// let power_values: Vec<u32> = (0..60).map(|i| 200 + (i % 50)).collect();
/// let timestamps: Vec<u32> = (0..60).collect();
///
/// let time_series = TimeSeriesData {
///     timestamps,
///     heart_rate: None,
///     power: Some(power_values),
///     cadence: None,
///     speed: None,
///     altitude: None,
///     temperature: None,
///     gps_coordinates: None,
/// };
///
/// let mut np_calc = NormalizedPowerCalculator::default();
/// time_series.accept(&mut np_calc);
///
/// if let Some(np) = np_calc.normalized_power() {
///     println!("Normalized Power: {:.0} watts", np);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct NormalizedPowerCalculator {
    /// Rolling window of last 30 power values
    window: Vec<f64>,
    /// Sum of 30-second average power^4 values
    sum_power4: f64,
    /// Count of 30-second averages calculated
    count: u64,
}

impl NormalizedPowerCalculator {
    /// Rolling window size (30 seconds)
    const WINDOW_SIZE: usize = 30;

    /// Get the calculated normalized power.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn normalized_power(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }

        // Safe: Activity data never approaches 2^52 data points where precision loss matters
        let mean_power4 = self.sum_power4 / self.count as f64;
        Some(mean_power4.powf(0.25))
    }
}

impl TimeSeriesVisitor for NormalizedPowerCalculator {
    #[allow(clippy::cast_precision_loss)]
    fn visit_power(&mut self, watts: u32, _timestamp: u32) {
        self.window.push(f64::from(watts));

        if self.window.len() >= Self::WINDOW_SIZE {
            // Safe: WINDOW_SIZE is 30, well below f64 precision limits
            let window_avg: f64 = self.window.iter().sum::<f64>() / Self::WINDOW_SIZE as f64;
            self.sum_power4 += window_avg.powi(4);
            self.count += 1;

            // Remove oldest value to maintain window size
            self.window.remove(0);
        }
    }
}

/// Detects cardiac decoupling (drift in HR:pace ratio).
///
/// Decoupling occurs when heart rate increases relative to pace over time,
/// indicating cardiovascular fatigue. A decoupling >5% suggests the activity
/// was too intense for the athlete's current aerobic fitness.
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::intelligence::visitor::{TimeSeriesVisitor, DecouplingDetector};
/// use pierre_mcp_server::models::TimeSeriesData;
///
/// // Simulate HR drift: same speed but increasing heart rate over time
/// let timestamps: Vec<u32> = (0..40).collect();
/// let heart_rates: Vec<u32> = (0..40).map(|i| 140 + i / 2).collect(); // HR drifts up
/// let speeds: Vec<f32> = vec![3.5; 40]; // Constant pace
///
/// let time_series = TimeSeriesData {
///     timestamps,
///     heart_rate: Some(heart_rates),
///     power: None,
///     cadence: None,
///     speed: Some(speeds),
///     altitude: None,
///     temperature: None,
///     gps_coordinates: None,
/// };
///
/// let mut detector = DecouplingDetector::default();
/// time_series.accept(&mut detector);
///
/// if let Some(decoupling) = detector.decoupling_percentage() {
///     println!("Cardiac decoupling: {:.1}%", decoupling);
///     if decoupling > 5.0 {
///         println!("Warning: Activity may have been too intense");
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct DecouplingDetector {
    /// Current heart rate value waiting for paired speed
    current_hr: Option<f64>,
    /// Current speed value waiting for paired heart rate
    current_speed: Option<f64>,
    /// Accumulated data points with both HR and speed
    data_points: Vec<(f64, f64)>,
}

impl DecouplingDetector {
    /// Minimum data points required for reliable decoupling calculation
    const MIN_DATA_POINTS: usize = 20;

    /// Calculate decoupling percentage.
    ///
    /// Returns the percentage difference in efficiency (HR/speed ratio)
    /// between the first and second halves of the activity.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn decoupling_percentage(&self) -> Option<f64> {
        if self.data_points.len() < Self::MIN_DATA_POINTS {
            return None;
        }

        let midpoint = self.data_points.len() / 2;
        let first_half = &self.data_points[..midpoint];
        let second_half = &self.data_points[midpoint..];

        // Safe: Activity data never approaches 2^52 data points where precision loss matters
        // Calculate average efficiency (HR/speed) for each half
        let first_avg_hr: f64 =
            first_half.iter().map(|(hr, _)| hr).sum::<f64>() / first_half.len() as f64;
        let first_avg_speed: f64 =
            first_half.iter().map(|(_, s)| s).sum::<f64>() / first_half.len() as f64;

        let second_avg_hr: f64 =
            second_half.iter().map(|(hr, _)| hr).sum::<f64>() / second_half.len() as f64;
        let second_avg_speed: f64 =
            second_half.iter().map(|(_, s)| s).sum::<f64>() / second_half.len() as f64;

        // Avoid division by zero
        if first_avg_speed == 0.0 || second_avg_speed == 0.0 {
            return None;
        }

        let first_efficiency = first_avg_hr / first_avg_speed;
        let second_efficiency = second_avg_hr / second_avg_speed;

        if first_efficiency == 0.0 {
            return None;
        }

        // Decoupling is the percentage increase in HR/speed ratio
        Some((second_efficiency - first_efficiency) / first_efficiency * 100.0)
    }
}

impl TimeSeriesVisitor for DecouplingDetector {
    fn visit_heart_rate(&mut self, bpm: u32, _timestamp: u32) {
        self.current_hr = Some(f64::from(bpm));

        // If we have both HR and speed, record the data point
        if let (Some(hr), Some(speed)) = (self.current_hr, self.current_speed) {
            self.data_points.push((hr, speed));
            self.current_hr = None;
            self.current_speed = None;
        }
    }

    fn visit_speed(&mut self, meters_per_sec: f32, _timestamp: u32) {
        self.current_speed = Some(f64::from(meters_per_sec));

        // If we have both HR and speed, record the data point
        if let (Some(hr), Some(speed)) = (self.current_hr, self.current_speed) {
            self.data_points.push((hr, speed));
            self.current_hr = None;
            self.current_speed = None;
        }
    }
}
