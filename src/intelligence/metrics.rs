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

/// Safe casting helper functions to avoid clippy warnings
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn safe_u32_to_u16(value: u32) -> u16 {
    value.min(u32::from(u16::MAX)) as u16
}

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
    pub fn calculate_metrics(&self, activity: &Activity) -> Result<AdvancedMetrics> {
        let mut metrics = AdvancedMetrics::default();

        // Calculate basic metrics
        self.calculate_basic_metrics(activity, &mut metrics)?;

        // Calculate power-based metrics
        self.calculate_power_metrics(activity, &mut metrics);

        // Calculate running-specific metrics
        Self::calculate_running_metrics(activity, &mut metrics);

        // Calculate time series metrics
        self.calculate_time_series_metrics(activity, &mut metrics);

        // Calculate environmental metrics
        Self::calculate_environmental_metrics(activity, &mut metrics);

        Ok(metrics)
    }

    /// Calculate basic metrics (TRIMP, TSS, intensity factor)
    fn calculate_basic_metrics(
        &self,
        activity: &Activity,
        metrics: &mut AdvancedMetrics,
    ) -> Result<()> {
        // Calculate TRIMP if heart rate data is available
        if let Some(avg_hr) = activity.average_heart_rate {
            let duration = i32::try_from(activity.duration_seconds)
                .map_err(|_| anyhow::anyhow!("Duration too large for i32"))?;
            metrics.trimp = self.calculate_trimp(avg_hr, duration);
        }

        // Use actual TSS if available, otherwise calculate
        metrics.training_stress_score = activity
            .training_stress_score
            .map(f64::from)
            .or_else(|| self.calculate_tss_from_data(activity));

        // Use actual intensity factor if available, otherwise calculate
        metrics.intensity_factor = activity
            .intensity_factor
            .map(f64::from)
            .or_else(|| self.calculate_intensity_factor(activity));

        Ok(())
    }

    /// Calculate TSS from activity data
    fn calculate_tss_from_data(&self, activity: &Activity) -> Option<f64> {
        // Calculate TSS from power data if available
        if let (Some(avg_power), Some(ftp)) = (activity.average_power, self.ftp) {
            // Safe conversion for duration
            let duration_hours = if activity.duration_seconds > u64::from(u32::MAX) {
                f64::from(u32::MAX) / 3600.0
            } else {
                f64::from(u32::try_from(activity.duration_seconds).unwrap_or(u32::MAX)) / 3600.0
            };
            Some(Self::calculate_tss(avg_power, ftp, duration_hours))
        } else if let (Some(avg_hr), Some(ftp)) = (activity.average_heart_rate, self.ftp) {
            // Fallback to HR-based TSS estimation
            let duration_hours = if activity.duration_seconds > u64::from(u32::MAX) {
                f64::from(u32::MAX) / 3600.0
            } else {
                f64::from(u32::try_from(activity.duration_seconds).unwrap_or(u32::MAX)) / 3600.0
            };
            self.max_hr.map(|max_hr| {
                let hr_percentage = f64::from(avg_hr) / max_hr;
                let estimated_power = ftp * hr_percentage;
                Self::calculate_tss_from_f64(estimated_power, ftp, duration_hours)
            })
        } else {
            None
        }
    }

    /// Calculate intensity factor from activity data
    fn calculate_intensity_factor(&self, activity: &Activity) -> Option<f64> {
        if let (Some(avg_power), Some(ftp)) = (activity.average_power, self.ftp) {
            Some(f64::from(avg_power) / ftp)
        } else {
            None
        }
    }

    /// Calculate power-based metrics
    fn calculate_power_metrics(&self, activity: &Activity, metrics: &mut AdvancedMetrics) {
        // Calculate power-to-weight ratio if power and weight available
        if let (Some(avg_power), Some(weight)) = (activity.average_power, self.weight_kg) {
            let power_to_weight = f64::from(avg_power) / weight;
            metrics.power_to_weight_ratio = Some(power_to_weight);
            metrics.avg_power_to_weight = Some(power_to_weight);
        }

        // Use actual normalized power or calculate from time series data
        metrics.normalized_power = activity.normalized_power.map(f64::from).or_else(|| {
            activity
                .time_series_data
                .as_ref()
                .and_then(|ts| ts.power.as_ref())
                .and_then(|power_data| self.calculate_normalized_power(power_data))
        });

        // Calculate work (energy) if power is available
        if let Some(avg_power) = activity.average_power {
            let duration_hours = if activity.duration_seconds > u64::from(u32::MAX) {
                f64::from(u32::MAX) / 3600.0
            } else {
                f64::from(u32::try_from(activity.duration_seconds).unwrap_or(u32::MAX)) / 3600.0
            };
            metrics.work = Some(f64::from(avg_power) * duration_hours / 1000.0);
            // kJ
        }

        // Calculate aerobic efficiency if both HR and pace/power data available
        if let (Some(avg_hr), Some(avg_speed)) =
            (activity.average_heart_rate, activity.average_speed)
        {
            metrics.aerobic_efficiency = Some(avg_speed / f64::from(avg_hr));
        }
    }

    /// Calculate running-specific metrics
    fn calculate_running_metrics(activity: &Activity, metrics: &mut AdvancedMetrics) {
        if !matches!(
            activity.sport_type,
            crate::models::SportType::Run | crate::models::SportType::TrailRunning
        ) {
            return;
        }

        // Running effectiveness (speed per heart rate)
        if let (Some(avg_hr), Some(avg_speed)) =
            (activity.average_heart_rate, activity.average_speed)
        {
            let effectiveness = avg_speed / f64::from(avg_hr) * EFFICIENCY_TIME_MULTIPLIER;
            metrics.running_effectiveness = Some(effectiveness);
            metrics.efficiency_factor = Some(effectiveness);
        }

        // Stride efficiency
        if let (Some(distance), Some(avg_cadence)) =
            (activity.distance_meters, activity.average_cadence)
        {
            let duration_minutes = if activity.duration_seconds > u64::from(u32::MAX) {
                f64::from(u32::MAX) / 60.0
            } else {
                f64::from(u32::try_from(activity.duration_seconds).unwrap_or(u32::MAX)) / 60.0
            };
            let total_steps = f64::from(avg_cadence) * duration_minutes;
            metrics.stride_efficiency = Some(distance / total_steps);
        }

        // Ground contact balance calculation
        if let Some(gct) = activity.ground_contact_time {
            metrics.ground_contact_balance = Some(Self::calculate_ground_contact_balance(gct));
        }
    }

    /// Calculate time series based metrics
    fn calculate_time_series_metrics(&self, activity: &Activity, metrics: &mut AdvancedMetrics) {
        let Some(time_series) = &activity.time_series_data else {
            return;
        };

        // Calculate variability index from time series power data
        if let Some(power_data) = &time_series.power {
            metrics.variability_index = self.calculate_variability_index(power_data);
        }

        // Calculate decoupling from HR and pace data
        if let (Some(hr_data), Some(speed_data)) = (&time_series.heart_rate, &time_series.speed) {
            // Heart rates are small values (30-220), use safe conversion
            let hr_f32: Vec<f32> = hr_data
                .iter()
                .map(|&hr| f32::from(safe_u32_to_u16(hr)))
                .collect();
            metrics.decoupling_percentage = self.calculate_decoupling(&hr_f32, speed_data);
        }
    }

    /// Calculate environmental impact metrics
    fn calculate_environmental_metrics(activity: &Activity, metrics: &mut AdvancedMetrics) {
        // Temperature stress
        metrics.temperature_stress = activity.temperature.map(Self::calculate_temperature_stress);

        // Altitude adjustment
        metrics.altitude_adjustment = activity
            .average_altitude
            .map(Self::calculate_altitude_adjustment);

        // Estimated recovery time based on training load
        metrics.estimated_recovery_time = metrics
            .training_stress_score
            .map(Self::calculate_recovery_time);
    }

    /// Calculate Training Impulse (TRIMP)
    fn calculate_trimp(&self, avg_hr: u32, duration_seconds: i32) -> Option<f64> {
        let max_hr = self.max_hr?;
        let resting_hr = self.resting_hr?;

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
    fn calculate_tss(avg_power: u32, ftp: f64, duration_hours: f64) -> f64 {
        let intensity_factor = f64::from(avg_power) / ftp;
        (duration_hours * intensity_factor * intensity_factor * TSS_BASE_MULTIPLIER).round()
    }

    /// Calculate Training Stress Score (TSS) from f64 power value
    fn calculate_tss_from_f64(avg_power: f64, ftp: f64, duration_hours: f64) -> f64 {
        let intensity_factor = avg_power / ftp;
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
        let mean_power4 = rolling_avg_power4.iter().sum::<f64>()
            / f64::from(u32::try_from(rolling_avg_power4.len()).unwrap_or(u32::MAX));
        Some(mean_power4.powf(0.25))
    }

    /// Calculate power variability index
    #[must_use]
    pub fn calculate_variability_index(&self, power_data: &[u32]) -> Option<f64> {
        if power_data.is_empty() {
            return None;
        }

        let avg_power: f64 = power_data.iter().map(|&p| f64::from(p)).sum::<f64>()
            / f64::from(u32::try_from(power_data.len()).unwrap_or(u32::MAX));

        // Use normalized power if we can calculate it
        self.calculate_normalized_power(power_data)
            .map(|normalized_power| normalized_power / avg_power)
            .or_else(|| {
                // Fallback to simple variability calculation
                let sum_of_squares: f64 = power_data.iter().map(|&p| f64::from(p).powi(2)).sum();
                let rms_power = (sum_of_squares
                    / f64::from(u32::try_from(power_data.len()).unwrap_or(u32::MAX)))
                .sqrt();
                Some(rms_power / avg_power)
            })
    }

    /// Calculate pace decoupling for endurance activities
    #[must_use]
    pub fn calculate_decoupling(&self, hr_data: &[f32], pace_data: &[f32]) -> Option<f64> {
        if hr_data.len() != pace_data.len() || hr_data.len() < MIN_DECOUPLING_DATA_POINTS {
            return None;
        }

        let half_point = hr_data.len() / 2;
        let first_half_size = f64::from(u32::try_from(half_point).ok()?);
        let second_half_size = f64::from(u32::try_from(hr_data.len() - half_point).ok()?);

        // First half averages
        let first_half_hr: f64 = hr_data[..half_point]
            .iter()
            .map(|&h| f64::from(h))
            .sum::<f64>()
            / first_half_size;
        let first_half_pace: f64 = pace_data[..half_point]
            .iter()
            .map(|&p| f64::from(p))
            .sum::<f64>()
            / first_half_size;

        // Second half averages
        let second_half_hr: f64 = hr_data[half_point..]
            .iter()
            .map(|&h| f64::from(h))
            .sum::<f64>()
            / second_half_size;
        let second_half_pace: f64 = pace_data[half_point..]
            .iter()
            .map(|&p| f64::from(p))
            .sum::<f64>()
            / second_half_size;

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
        let duration_hours = if activity.duration_seconds > u64::from(u32::MAX) {
            f64::from(u32::MAX) / 3600.0
        } else {
            f64::from(u32::try_from(activity.duration_seconds).unwrap_or(u32::MAX)) / 3600.0
        };

        // Use intensity factor if available, otherwise estimate from heart rate
        let intensity = activity
            .intensity_factor
            .map(f64::from)
            .or_else(|| {
                if let (Some(avg_hr), Some(lthr)) = (activity.average_heart_rate, self.lthr) {
                    Some((f64::from(avg_hr) / lthr).min(1.5)) // Cap at 150% of threshold
                } else {
                    None
                }
            })
            .unwrap_or(0.7); // Default moderate intensity

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
        match gct_ms {
            gct if (200.0..=300.0).contains(&gct) => {
                // Good contact time range suggests better balance
                ((250.0 - (gct - 250.0).abs()) / 250.0).mul_add(5.0, 50.0)
            }
            gct if gct < 200.0 => {
                // Very short contact time - might indicate imbalance
                (gct / 200.0).mul_add(5.0, 45.0)
            }
            gct => {
                // Long contact time - might indicate fatigue or imbalance
                55.0 - ((gct - 300.0) / 100.0).min(10.0)
            }
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
    #[must_use]
    pub fn from_heart_rate_data(hr_data: &[f32], lthr: f64) -> Self {
        let total_points = f64::from(u32::try_from(hr_data.len()).unwrap_or(u32::MAX));

        let zone1 = f64::from(
            u32::try_from(
                hr_data
                    .iter()
                    .filter(|&&hr| f64::from(hr) <= lthr * HR_ZONE1_UPPER_LIMIT)
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone2 = f64::from(
            u32::try_from(
                hr_data
                    .iter()
                    .filter(|&&hr| {
                        f64::from(hr) > lthr * HR_ZONE1_UPPER_LIMIT
                            && f64::from(hr) <= lthr * HR_ZONE2_UPPER_LIMIT
                    })
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone3 = f64::from(
            u32::try_from(
                hr_data
                    .iter()
                    .filter(|&&hr| {
                        f64::from(hr) > lthr * HR_ZONE2_UPPER_LIMIT
                            && f64::from(hr) <= lthr * HR_ZONE3_UPPER_LIMIT
                    })
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone4 = f64::from(
            u32::try_from(
                hr_data
                    .iter()
                    .filter(|&&hr| {
                        f64::from(hr) > lthr * HR_ZONE3_UPPER_LIMIT
                            && f64::from(hr) <= lthr * HR_ZONE4_UPPER_LIMIT
                    })
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone5 = f64::from(
            u32::try_from(
                hr_data
                    .iter()
                    .filter(|&&hr| f64::from(hr) > lthr * HR_ZONE4_UPPER_LIMIT)
                    .count(),
            )
            .unwrap_or(0),
        );

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
    #[must_use]
    pub fn from_power_data(power_data: &[f32], ftp: f64) -> Self {
        let total_points = f64::from(u32::try_from(power_data.len()).unwrap_or(u32::MAX));

        let zone1 = f64::from(
            u32::try_from(
                power_data
                    .iter()
                    .filter(|&&p| f64::from(p) <= ftp * POWER_ZONE1_UPPER_LIMIT)
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone2 = f64::from(
            u32::try_from(
                power_data
                    .iter()
                    .filter(|&&p| {
                        f64::from(p) > ftp * POWER_ZONE1_UPPER_LIMIT
                            && f64::from(p) <= ftp * POWER_ZONE2_UPPER_LIMIT
                    })
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone3 = f64::from(
            u32::try_from(
                power_data
                    .iter()
                    .filter(|&&p| {
                        f64::from(p) > ftp * POWER_ZONE2_UPPER_LIMIT
                            && f64::from(p) <= ftp * POWER_ZONE3_UPPER_LIMIT
                    })
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone4 = f64::from(
            u32::try_from(
                power_data
                    .iter()
                    .filter(|&&p| {
                        f64::from(p) > ftp * POWER_ZONE3_UPPER_LIMIT
                            && f64::from(p) <= ftp * POWER_ZONE4_UPPER_LIMIT
                    })
                    .count(),
            )
            .unwrap_or(0),
        );
        let zone5 = f64::from(
            u32::try_from(
                power_data
                    .iter()
                    .filter(|&&p| f64::from(p) > ftp * POWER_ZONE4_UPPER_LIMIT)
                    .count(),
            )
            .unwrap_or(0),
        );

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
