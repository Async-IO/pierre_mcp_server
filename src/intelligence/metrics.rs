// ABOUTME: Advanced fitness metrics calculation and performance analysis algorithms
// ABOUTME: Computes training load, power metrics, heart rate zones, and physiological indicators
//! Advanced fitness metrics calculation and analysis

// Future: use crate::config::intelligence_config::{IntelligenceConfig};
use crate::intelligence::physiological_constants::{
    metrics_constants::{
        EFFICIENCY_TIME_MULTIPLIER, MIN_DECOUPLING_DATA_POINTS, TRIMP_BASE_MULTIPLIER,
        TRIMP_EXPONENTIAL_FACTOR, TSS_BASE_MULTIPLIER,
    },
    zone_percentages::{
        HR_ZONE1_UPPER_LIMIT, HR_ZONE2_UPPER_LIMIT, HR_ZONE3_UPPER_LIMIT, HR_ZONE4_UPPER_LIMIT,
        POWER_ZONE1_UPPER_LIMIT, POWER_ZONE2_UPPER_LIMIT, POWER_ZONE3_UPPER_LIMIT,
        POWER_ZONE4_UPPER_LIMIT,
    },
};
use crate::models::Activity;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Advanced metrics for activity analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdvancedMetrics {
    /// Training impulse (TRIMP) score
    pub trimp: Option<f64>,
    /// Aerobic efficiency ratio
    pub aerobic_efficiency: Option<f64>,
    /// Power-to-weight ratio (W/kg)
    pub power_to_weight_ratio: Option<f64>,
    /// Training stress score (TSS)
    pub training_stress_score: Option<f64>,
    /// Intensity factor
    pub intensity_factor: Option<f64>,
    /// Variability index
    pub variability_index: Option<f64>,
    /// Efficiency factor
    pub efficiency_factor: Option<f64>,
    /// Decoupling percentage
    pub decoupling_percentage: Option<f64>,

    // Enhanced power metrics
    /// Normalized power (4th root of 30-second rolling average of power^4)
    pub normalized_power: Option<f64>,
    /// Work (kilojoules)
    pub work: Option<f64>,
    /// Average power-to-weight ratio
    pub avg_power_to_weight: Option<f64>,

    // Running-specific metrics
    /// Running effectiveness (speed per heart rate)
    pub running_effectiveness: Option<f64>,
    /// Stride efficiency (distance per stride)
    pub stride_efficiency: Option<f64>,
    /// Ground contact balance
    pub ground_contact_balance: Option<f64>,

    // Recovery and physiological metrics
    /// Estimated recovery time in hours
    pub estimated_recovery_time: Option<f64>,
    /// Training load (combination of duration and intensity)
    pub training_load: Option<f64>,
    /// Aerobic/anaerobic contribution percentage
    pub aerobic_contribution: Option<f64>,

    // Environmental impact metrics
    /// Temperature stress factor
    pub temperature_stress: Option<f64>,
    /// Altitude adjustment factor
    pub altitude_adjustment: Option<f64>,

    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Metrics calculator for activities
pub struct MetricsCalculator {
    /// User's functional threshold power (FTP)
    pub ftp: Option<f64>,
    /// User's lactate threshold heart rate (LTHR)
    pub lthr: Option<f64>,
    /// User's maximum heart rate
    pub max_hr: Option<f64>,
    /// User's resting heart rate
    pub resting_hr: Option<f64>,
    /// User's weight in kg
    pub weight_kg: Option<f64>,
}

impl Default for MetricsCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCalculator {
    /// Create a new metrics calculator
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ftp: None,
            lthr: None,
            max_hr: None,
            resting_hr: None,
            weight_kg: None,
        }
    }

    /// Set user parameters for calculations
    #[must_use]
    pub const fn with_user_data(
        mut self,
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Self {
        self.ftp = ftp;
        self.lthr = lthr;
        self.max_hr = max_hr;
        self.resting_hr = resting_hr;
        self.weight_kg = weight_kg;
        self
    }

    /// Calculate all available metrics for an activity
    ///
    /// # Errors
    /// Returns an error if metrics calculation fails
    #[allow(clippy::too_many_lines)]
    pub fn calculate_metrics(&self, activity: &Activity) -> Result<AdvancedMetrics> {
        let mut metrics = AdvancedMetrics::default();

        // Calculate TRIMP if heart rate data is available
        if let Some(avg_hr) = activity.average_heart_rate {
            #[allow(clippy::cast_possible_truncation)]
            let duration = activity.duration_seconds as i32;
            #[allow(clippy::cast_precision_loss)]
            {
                metrics.trimp = self.calculate_trimp(avg_hr as f32, duration);
            }
        }

        // Use actual TSS if available, otherwise calculate
        metrics.training_stress_score =
            activity.training_stress_score.map(f64::from).or_else(|| {
                // Calculate TSS from power data if available
                if let (Some(avg_power), Some(ftp)) = (activity.average_power, self.ftp) {
                    #[allow(clippy::cast_precision_loss)]
                    let duration_hours = activity.duration_seconds as f64 / 3600.0;
                    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
                    Some(Self::calculate_tss(avg_power as f32, ftp, duration_hours))
                } else if let (Some(avg_hr), Some(ftp)) = (activity.average_heart_rate, self.ftp) {
                    // Fallback to HR-based TSS estimation
                    #[allow(clippy::cast_precision_loss)]
                    let duration_hours = activity.duration_seconds as f64 / 3600.0;
                    #[allow(clippy::option_if_let_else)]
                    if let Some(max_hr) = self.max_hr {
                        let hr_percentage = f64::from(avg_hr) / max_hr;
                        let estimated_power = ftp * hr_percentage;
                        #[allow(clippy::cast_possible_truncation)]
                        Some(Self::calculate_tss(estimated_power as f32, ftp, duration_hours))
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        // Use actual intensity factor if available, otherwise calculate
        metrics.intensity_factor = activity.intensity_factor.map(f64::from).or_else(|| {
            if let (Some(avg_power), Some(ftp)) = (activity.average_power, self.ftp) {
                Some(f64::from(avg_power) / ftp)
            } else {
                None
            }
        });

        // Calculate power-to-weight ratio if power and weight available
        if let (Some(avg_power), Some(weight)) = (activity.average_power, self.weight_kg) {
            metrics.power_to_weight_ratio = Some(f64::from(avg_power) / weight);
            metrics.avg_power_to_weight = Some(f64::from(avg_power) / weight);
        }

        // Use actual normalized power or calculate from time series data
        metrics.normalized_power = activity.normalized_power.map(f64::from).or_else(|| {
            #[allow(clippy::option_if_let_else)]
            if let Some(time_series) = &activity.time_series_data {
                #[allow(clippy::option_if_let_else)]
                if let Some(power_data) = &time_series.power {
                    self.calculate_normalized_power(power_data)
                } else {
                    None
                }
            } else {
                None
            }
        });

        // Calculate work (energy) if power is available
        if let Some(avg_power) = activity.average_power {
            #[allow(clippy::cast_precision_loss)]
            let duration_hours = activity.duration_seconds as f64 / 3600.0;
            metrics.work = Some(f64::from(avg_power) * duration_hours / 1000.0);
            // kJ
        }

        // Calculate aerobic efficiency if both HR and pace/power data available
        if let (Some(avg_hr), Some(avg_speed)) =
            (activity.average_heart_rate, activity.average_speed)
        {
            metrics.aerobic_efficiency = Some(avg_speed / f64::from(avg_hr));
        }

        // Running-specific metrics
        if matches!(
            activity.sport_type,
            crate::models::SportType::Run | crate::models::SportType::TrailRunning
        ) {
            // Running effectiveness (speed per heart rate)
            if let (Some(avg_hr), Some(avg_speed)) =
                (activity.average_heart_rate, activity.average_speed)
            {
                metrics.running_effectiveness =
                    Some(avg_speed / f64::from(avg_hr) * EFFICIENCY_TIME_MULTIPLIER);
                metrics.efficiency_factor =
                    Some(avg_speed / f64::from(avg_hr) * EFFICIENCY_TIME_MULTIPLIER);
            }

            // Stride efficiency
            if let (Some(distance), Some(avg_cadence), Some(duration)) = (
                activity.distance_meters,
                activity.average_cadence,
                Some(activity.duration_seconds),
            ) {
                #[allow(clippy::cast_precision_loss)]
                let total_steps = (f64::from(avg_cadence) * duration as f64) / 60.0; // Convert cadence from steps/min
                metrics.stride_efficiency = Some(distance / total_steps);
            }

            // Ground contact balance calculation
            if let Some(gct) = activity.ground_contact_time {
                // Calculate balance based on ground contact time analysis
                // A balanced runner typically has 45-55% left/right balance
                let balance_score = Self::calculate_ground_contact_balance(gct);
                metrics.ground_contact_balance = Some(balance_score);
            }
        }

        // Calculate variability index from time series power data
        if let Some(time_series) = &activity.time_series_data {
            if let Some(power_data) = &time_series.power {
                metrics.variability_index = self.calculate_variability_index(power_data);
            }

            // Calculate decoupling from HR and pace data
            if let (Some(hr_data), Some(speed_data)) = (&time_series.heart_rate, &time_series.speed)
            {
                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_precision_loss)]
                let hr_f32: Vec<f32> = hr_data.iter().map(|&hr| hr as f32).collect();
                metrics.decoupling_percentage = self.calculate_decoupling(&hr_f32, speed_data);
            }
        }

        // Environmental impact calculations
        #[allow(clippy::option_if_let_else)]
        if let Some(temp) = activity.temperature {
            metrics.temperature_stress = Some(Self::calculate_temperature_stress(temp));
        }

        #[allow(clippy::option_if_let_else)]
        if let Some(altitude) = activity.average_altitude {
            metrics.altitude_adjustment = Some(Self::calculate_altitude_adjustment(altitude));
        }

        // Estimated recovery time based on training load
        #[allow(clippy::option_if_let_else)]
        if let Some(tss) = metrics.training_stress_score {
            metrics.estimated_recovery_time = Some(Self::calculate_recovery_time(tss));
        }

        Ok(metrics)
    }

    /// Calculate Training Impulse (TRIMP)
    fn calculate_trimp(&self, avg_hr: f32, duration_seconds: i32) -> Option<f64> {
        #[allow(clippy::question_mark)]
        let Some(max_hr) = self.max_hr else {
            return None;
        };
        #[allow(clippy::question_mark)]
        let Some(resting_hr) = self.resting_hr else {
            return None;
        };

        let hr_reserve = max_hr - resting_hr;
        let hr_ratio = (f64::from(avg_hr) - resting_hr) / hr_reserve;
        let duration_minutes = f64::from(duration_seconds) / 60.0;

        // Simplified TRIMP calculation using established constants
        Some(
            duration_minutes
                * hr_ratio
                * TRIMP_BASE_MULTIPLIER
                * (TRIMP_EXPONENTIAL_FACTOR * hr_ratio).exp(),
        )
    }

    /// Calculate Training Stress Score (TSS)
    fn calculate_tss(avg_power: f32, ftp: f64, duration_hours: f64) -> f64 {
        let intensity_factor = f64::from(avg_power) / ftp;
        (duration_hours * intensity_factor * intensity_factor * TSS_BASE_MULTIPLIER).round()
    }

    /// Calculate normalized power (4th root of 30-second rolling average of power^4)
    #[must_use]
    pub fn calculate_normalized_power(&self, power_data: &[u32]) -> Option<f64> {
        if power_data.len() < 30 {
            return None; // Need at least 30 seconds of data
        }

        // Convert to f64 for calculations
        let power_f64: Vec<f64> = power_data.iter().map(|&p| f64::from(p)).collect();

        // Calculate 30-second rolling averages of power^4
        let mut rolling_avg_power4 = Vec::new();
        for i in 29..power_f64.len() {
            let window = &power_f64[(i - 29)..=i];
            let avg_power4: f64 = window.iter().map(|&p| p.powi(4)).sum::<f64>() / 30.0;
            rolling_avg_power4.push(avg_power4);
        }

        if rolling_avg_power4.is_empty() {
            return None;
        }

        // Take the average of all 30-second power^4 values, then take 4th root
        #[allow(clippy::cast_precision_loss)]
        let mean_power4 = rolling_avg_power4.iter().sum::<f64>() / rolling_avg_power4.len() as f64;
        Some(mean_power4.powf(0.25))
    }

    /// Calculate power variability index
    #[must_use]
    pub fn calculate_variability_index(&self, power_data: &[u32]) -> Option<f64> {
        if power_data.is_empty() {
            return None;
        }

        #[allow(clippy::cast_precision_loss)]
        let avg_power: f64 =
            power_data.iter().map(|&p| f64::from(p)).sum::<f64>() / power_data.len() as f64;

        // Use normalized power if we can calculate it
        #[allow(clippy::option_if_let_else)]
        if let Some(normalized_power) = self.calculate_normalized_power(power_data) {
            Some(normalized_power / avg_power)
        } else {
            // Fallback to simple variability calculation
            let sum_of_squares: f64 = power_data.iter().map(|&p| f64::from(p).powi(2)).sum();
            #[allow(clippy::cast_precision_loss)]
            let rms_power = (sum_of_squares / power_data.len() as f64).sqrt();
            Some(rms_power / avg_power)
        }
    }

    /// Calculate pace decoupling for endurance activities
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn calculate_decoupling(&self, hr_data: &[f32], pace_data: &[f32]) -> Option<f64> {
        if hr_data.len() != pace_data.len() || hr_data.len() < MIN_DECOUPLING_DATA_POINTS {
            return None;
        }

        let half_point = hr_data.len() / 2;

        // First half averages
        let first_half_hr: f64 = hr_data[..half_point]
            .iter()
            .map(|&h| f64::from(h))
            .sum::<f64>()
            / half_point as f64;
        let first_half_pace: f64 = pace_data[..half_point]
            .iter()
            .map(|&p| f64::from(p))
            .sum::<f64>()
            / half_point as f64;

        // Second half averages
        let second_half_hr: f64 = hr_data[half_point..]
            .iter()
            .map(|&h| f64::from(h))
            .sum::<f64>()
            / (hr_data.len() - half_point) as f64;
        let second_half_pace: f64 = pace_data[half_point..]
            .iter()
            .map(|&p| f64::from(p))
            .sum::<f64>()
            / (pace_data.len() - half_point) as f64;

        // Calculate efficiency ratios
        let first_efficiency = first_half_pace / first_half_hr;
        let second_efficiency = second_half_pace / second_half_hr;

        // Decoupling percentage
        Some(((second_efficiency - first_efficiency) / first_efficiency) * 100.0)
    }

    /// Calculate temperature stress factor
    fn calculate_temperature_stress(temperature: f32) -> f64 {
        // Temperature stress increases outside the optimal range of 10-20°C
        let optimal_min = 10.0;
        let optimal_max = 20.0;

        if temperature >= optimal_min && temperature <= optimal_max {
            1.0 // No stress
        } else if temperature < optimal_min {
            // Cold stress increases as temperature drops
            1.0 + f64::from(((optimal_min - temperature) / 10.0).clamp(0.0, 2.0))
        } else {
            // Heat stress increases as temperature rises
            1.0 + f64::from(((temperature - optimal_max) / 10.0).clamp(0.0, 3.0))
        }
    }

    /// Calculate altitude adjustment factor
    fn calculate_altitude_adjustment(altitude: f32) -> f64 {
        // Performance decreases with altitude due to reduced oxygen
        // Approximately 1% performance loss per 100m above 1500m
        if altitude <= 1500.0 {
            1.0 // No adjustment needed
        } else {
            let altitude_effect = (altitude - 1500.0) / 10000.0; // 1% per 100m
            1.0 + f64::from(altitude_effect.min(0.20)) // Cap at 20% adjustment
        }
    }

    /// Calculate estimated recovery time based on training stress
    const fn calculate_recovery_time(tss: f64) -> f64 {
        // Simple recovery time estimation based on TSS
        // Formula: Recovery hours = TSS / 10 (simplified)
        (tss / 10.0).clamp(2.0, 72.0) // Minimum 2 hours, maximum 72 hours
    }

    /// Calculate training load combining duration and intensity
    ///
    /// # Errors
    /// This function does not return a Result but returns None if calculation cannot be performed
    #[must_use]
    pub fn calculate_training_load(&self, activity: &Activity) -> Option<f64> {
        #[allow(clippy::cast_precision_loss)]
        let duration_hours = activity.duration_seconds as f64 / 3600.0;

        // Use intensity factor if available, otherwise estimate from heart rate
        #[allow(clippy::option_if_let_else)]
        let intensity = if let Some(if_val) = activity.intensity_factor {
            f64::from(if_val)
        } else if let (Some(avg_hr), Some(lthr)) = (activity.average_heart_rate, self.lthr) {
            (f64::from(avg_hr) / lthr).min(1.5) // Cap at 150% of threshold
        } else {
            0.7 // Default moderate intensity
        };

        Some(duration_hours * intensity * 100.0) // Arbitrary scaling factor
    }

    /// Calculate aerobic vs anaerobic contribution
    ///
    /// # Errors
    /// This function does not return a Result but returns None if calculation cannot be performed
    #[must_use]
    pub fn calculate_aerobic_contribution(&self, activity: &Activity) -> Option<f64> {
        // Estimate based on heart rate zones or intensity
        if let (Some(avg_hr), Some(lthr)) = (activity.average_heart_rate, self.lthr) {
            let hr_ratio = f64::from(avg_hr) / lthr;

            if hr_ratio <= 0.85 {
                Some(95.0) // Mostly aerobic
            } else if hr_ratio <= 1.0 {
                Some(80.0) // Mixed aerobic/anaerobic
            } else if hr_ratio <= 1.15 {
                Some(60.0) // More anaerobic
            } else {
                Some(40.0) // Heavily anaerobic
            }
        } else {
            None
        }
    }

    /// Calculate ground contact balance from ground contact time
    fn calculate_ground_contact_balance(ground_contact_time: u32) -> f64 {
        // Ground contact time in milliseconds - analyze for balance estimation
        // Typical ground contact times: 200-300ms for recreational runners
        // Balanced runners have consistent contact times

        let gct_ms = f64::from(ground_contact_time);

        // Estimate balance based on contact time patterns
        // This is a simplified calculation that would ideally use left/right foot data
        #[allow(clippy::option_if_let_else)]
        if (200.0..=300.0).contains(&gct_ms) {
            // Good contact time range suggests better balance
            ((250.0 - (gct_ms - 250.0).abs()) / 250.0).mul_add(5.0, 50.0)
        } else if gct_ms < 200.0 {
            // Very short contact time - might indicate imbalance
            (gct_ms / 200.0).mul_add(5.0, 45.0)
        } else {
            // Long contact time - might indicate fatigue or imbalance
            55.0 - ((gct_ms - 300.0) / 100.0).min(10.0)
        }
    }
}

/// Zone-based analysis for heart rate or power
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneAnalysis {
    pub zone1_percentage: f64,
    pub zone2_percentage: f64,
    pub zone3_percentage: f64,
    pub zone4_percentage: f64,
    pub zone5_percentage: f64,
    pub time_in_zones: HashMap<String, f64>,
}

impl ZoneAnalysis {
    /// Calculate time in zones based on heart rate data
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn from_heart_rate_data(hr_data: &[f32], lthr: f64) -> Self {
        let total_points = hr_data.len() as f64;

        let zone1 = hr_data
            .iter()
            .filter(|&&hr| f64::from(hr) <= lthr * HR_ZONE1_UPPER_LIMIT)
            .count() as f64;
        let zone2 = hr_data
            .iter()
            .filter(|&&hr| {
                f64::from(hr) > lthr * HR_ZONE1_UPPER_LIMIT
                    && f64::from(hr) <= lthr * HR_ZONE2_UPPER_LIMIT
            })
            .count() as f64;
        let zone3 = hr_data
            .iter()
            .filter(|&&hr| {
                f64::from(hr) > lthr * HR_ZONE2_UPPER_LIMIT
                    && f64::from(hr) <= lthr * HR_ZONE3_UPPER_LIMIT
            })
            .count() as f64;
        let zone4 = hr_data
            .iter()
            .filter(|&&hr| {
                f64::from(hr) > lthr * HR_ZONE3_UPPER_LIMIT
                    && f64::from(hr) <= lthr * HR_ZONE4_UPPER_LIMIT
            })
            .count() as f64;
        let zone5 = hr_data
            .iter()
            .filter(|&&hr| f64::from(hr) > lthr * HR_ZONE4_UPPER_LIMIT)
            .count() as f64;

        let mut time_in_zones = HashMap::new();
        time_in_zones.insert("recovery".into(), (zone1 / total_points) * 100.0);
        time_in_zones.insert("aerobic".into(), (zone2 / total_points) * 100.0);
        time_in_zones.insert("tempo".into(), (zone3 / total_points) * 100.0);
        time_in_zones.insert("threshold".into(), (zone4 / total_points) * 100.0);
        time_in_zones.insert("vo2max".into(), (zone5 / total_points) * 100.0);

        Self {
            zone1_percentage: (zone1 / total_points) * 100.0,
            zone2_percentage: (zone2 / total_points) * 100.0,
            zone3_percentage: (zone3 / total_points) * 100.0,
            zone4_percentage: (zone4 / total_points) * 100.0,
            zone5_percentage: (zone5 / total_points) * 100.0,
            time_in_zones,
        }
    }

    /// Calculate time in zones based on power data
    #[allow(clippy::cast_precision_loss)]
    pub fn from_power_data(power_data: &[f32], ftp: f64) -> Self {
        let total_points = power_data.len() as f64;

        let zone1 = power_data
            .iter()
            .filter(|&&p| f64::from(p) <= ftp * POWER_ZONE1_UPPER_LIMIT)
            .count() as f64;
        let zone2 = power_data
            .iter()
            .filter(|&&p| {
                f64::from(p) > ftp * POWER_ZONE1_UPPER_LIMIT
                    && f64::from(p) <= ftp * POWER_ZONE2_UPPER_LIMIT
            })
            .count() as f64;
        let zone3 = power_data
            .iter()
            .filter(|&&p| {
                f64::from(p) > ftp * POWER_ZONE2_UPPER_LIMIT
                    && f64::from(p) <= ftp * POWER_ZONE3_UPPER_LIMIT
            })
            .count() as f64;
        let zone4 = power_data
            .iter()
            .filter(|&&p| {
                f64::from(p) > ftp * POWER_ZONE3_UPPER_LIMIT
                    && f64::from(p) <= ftp * POWER_ZONE4_UPPER_LIMIT
            })
            .count() as f64;
        let zone5 = power_data
            .iter()
            .filter(|&&p| f64::from(p) > ftp * POWER_ZONE4_UPPER_LIMIT)
            .count() as f64;

        let mut time_in_zones = HashMap::new();
        time_in_zones.insert("active_recovery".into(), (zone1 / total_points) * 100.0);
        time_in_zones.insert("endurance".into(), (zone2 / total_points) * 100.0);
        time_in_zones.insert("tempo".into(), (zone3 / total_points) * 100.0);
        time_in_zones.insert("threshold".into(), (zone4 / total_points) * 100.0);
        time_in_zones.insert("vo2max".into(), (zone5 / total_points) * 100.0);

        Self {
            zone1_percentage: (zone1 / total_points) * 100.0,
            zone2_percentage: (zone2 / total_points) * 100.0,
            zone3_percentage: (zone3 / total_points) * 100.0,
            zone4_percentage: (zone4 / total_points) * 100.0,
            zone5_percentage: (zone5 / total_points) * 100.0,
            time_in_zones,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trimp_calculation() {
        let calculator =
            MetricsCalculator::new().with_user_data(None, None, Some(190.0), Some(60.0), None);

        let trimp = calculator.calculate_trimp(150.0, 3600); // 150 bpm for 1 hour
        assert!(trimp.is_some());
        assert!(trimp.unwrap() > 0.0);
    }

    #[test]
    fn test_power_to_weight_ratio() {
        let calculator =
            MetricsCalculator::new().with_user_data(Some(250.0), None, None, None, Some(70.0));

        let activity = Activity {
            average_speed: Some(10.0),
            average_power: Some(250), // Now we have power data
            ..Activity::default()
        };

        let metrics = calculator.calculate_metrics(&activity).unwrap();
        // Power-to-weight ratio should be 250W / 70kg ≈ 3.57 W/kg
        assert!(metrics.power_to_weight_ratio.is_some());
        let ratio = metrics.power_to_weight_ratio.unwrap();
        assert!((ratio - 3.57).abs() < 0.1);
    }

    #[test]
    fn test_zone_analysis() {
        let hr_data = vec![120.0, 130.0, 140.0, 160.0, 180.0]; // Sample HR data
        let lthr = 160.0;

        let analysis = ZoneAnalysis::from_heart_rate_data(&hr_data, lthr);

        // Should have distributed the data across zones
        assert!(
            analysis.zone1_percentage
                + analysis.zone2_percentage
                + analysis.zone3_percentage
                + analysis.zone4_percentage
                + analysis.zone5_percentage
                <= 100.1
        ); // Allow for floating point precision
    }

    #[test]
    fn test_normalized_power_calculation() {
        let calculator = MetricsCalculator::new();
        let power_data = vec![200, 250, 300, 280, 220, 240, 260, 290, 310, 270]; // 10 seconds of data

        // Should return None for insufficient data
        assert!(calculator.calculate_normalized_power(&power_data).is_none());

        // Test with sufficient data (30+ points)
        let mut long_power_data = vec![250; 60]; // 60 seconds at 250W
        long_power_data.extend(vec![300; 30]); // 30 seconds at 300W

        let np = calculator
            .calculate_normalized_power(&long_power_data)
            .unwrap();
        assert!(np > 250.0 && np < 300.0); // Should be between average and max
    }

    #[test]
    fn test_enhanced_metrics_calculation() {
        let calculator = MetricsCalculator::new().with_user_data(
            Some(280.0),
            Some(165.0),
            Some(190.0),
            Some(60.0),
            Some(75.0),
        );

        let activity = Activity {
            average_power: Some(250),
            normalized_power: Some(265),
            intensity_factor: Some(0.89),
            training_stress_score: Some(85.0),
            temperature: Some(25.0),
            average_altitude: Some(2000.0),
            ..Activity::default()
        };

        let metrics = calculator.calculate_metrics(&activity).unwrap();

        // Verify enhanced metrics are calculated
        assert_eq!(metrics.power_to_weight_ratio, Some(250.0 / 75.0));
        assert_eq!(metrics.normalized_power, Some(265.0));
        // Check intensity factor with tolerance for f32->f64 conversion precision
        if let Some(if_val) = metrics.intensity_factor {
            assert!((if_val - 0.89).abs() < 0.01, "Intensity factor: {}", if_val);
        } else {
            panic!("Intensity factor should be calculated");
        }
        assert_eq!(metrics.training_stress_score, Some(85.0));
        assert!(metrics.temperature_stress.is_some());
        assert!(metrics.altitude_adjustment.is_some());
        assert!(metrics.estimated_recovery_time.is_some());
    }
}
