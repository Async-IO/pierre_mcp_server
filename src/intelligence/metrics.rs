//! Advanced fitness metrics calculation and analysis

use crate::models::Activity;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Advanced metrics for activity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

impl Default for AdvancedMetrics {
    fn default() -> Self {
        Self {
            trimp: None,
            aerobic_efficiency: None,
            power_to_weight_ratio: None,
            training_stress_score: None,
            intensity_factor: None,
            variability_index: None,
            efficiency_factor: None,
            decoupling_percentage: None,
            custom_metrics: HashMap::new(),
        }
    }
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

impl MetricsCalculator {
    /// Create a new metrics calculator
    pub fn new() -> Self {
        Self {
            ftp: None,
            lthr: None,
            max_hr: None,
            resting_hr: None,
            weight_kg: None,
        }
    }

    /// Set user parameters for calculations
    pub fn with_user_data(
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
    pub fn calculate_metrics(&self, activity: &Activity) -> Result<AdvancedMetrics> {
        let mut metrics = AdvancedMetrics::default();

        // Calculate TRIMP if heart rate data is available
        if let Some(avg_hr) = activity.average_heart_rate {
            let duration = activity.duration_seconds as i32;
            metrics.trimp = self.calculate_trimp(avg_hr as f32, duration);
        }

        // Power data not available in current Activity model
        // Skip power-based metrics calculation

        // Calculate aerobic efficiency if both HR and pace/power data available
        if let (Some(avg_hr), Some(avg_speed)) = (activity.average_heart_rate, activity.average_speed) {
            metrics.aerobic_efficiency = Some(avg_speed / avg_hr as f64);
        }

        // Calculate efficiency factor for running (pace per heart rate beat)
        if let crate::models::SportType::Run = activity.sport_type {
            if let (Some(avg_hr), Some(avg_speed)) = (activity.average_heart_rate, activity.average_speed) {
                metrics.efficiency_factor = Some(avg_speed / avg_hr as f64 * 60.0); // pace per HR
            }
        }

        Ok(metrics)
    }

    /// Calculate Training Impulse (TRIMP)
    fn calculate_trimp(&self, avg_hr: f32, duration_seconds: i32) -> Option<f64> {
        let (max_hr, resting_hr) = match (self.max_hr, self.resting_hr) {
            (Some(max), Some(rest)) => (max, rest),
            _ => return None,
        };

        let hr_reserve = max_hr - resting_hr;
        let hr_ratio = (avg_hr as f64 - resting_hr) / hr_reserve;
        let duration_minutes = duration_seconds as f64 / 60.0;

        // Simplified TRIMP calculation
        Some(duration_minutes * hr_ratio * 0.64 * (2.718_f64.powf(1.92 * hr_ratio)))
    }

    /// Calculate Training Stress Score (TSS)
    fn calculate_tss(&self, avg_power: f32, ftp: f64, duration_hours: f64) -> Option<f64> {
        let intensity_factor = avg_power as f64 / ftp;
        Some((duration_hours * intensity_factor * intensity_factor * 100.0).round())
    }

    /// Calculate power variability index
    pub fn calculate_variability_index(&self, power_data: &[f32]) -> Option<f64> {
        if power_data.is_empty() {
            return None;
        }

        let avg_power: f64 = power_data.iter().map(|&p| p as f64).sum::<f64>() / power_data.len() as f64;
        
        // Calculate normalized power (simplified)
        let sum_of_squares: f64 = power_data.iter()
            .map(|&p| (p as f64).powi(2))
            .sum();
        let normalized_power = (sum_of_squares / power_data.len() as f64).sqrt();

        Some(normalized_power / avg_power)
    }

    /// Calculate pace decoupling for endurance activities
    pub fn calculate_decoupling(&self, hr_data: &[f32], pace_data: &[f32]) -> Option<f64> {
        if hr_data.len() != pace_data.len() || hr_data.len() < 20 {
            return None;
        }

        let half_point = hr_data.len() / 2;
        
        // First half averages
        let first_half_hr: f64 = hr_data[..half_point].iter().map(|&h| h as f64).sum::<f64>() / half_point as f64;
        let first_half_pace: f64 = pace_data[..half_point].iter().map(|&p| p as f64).sum::<f64>() / half_point as f64;
        
        // Second half averages  
        let second_half_hr: f64 = hr_data[half_point..].iter().map(|&h| h as f64).sum::<f64>() / (hr_data.len() - half_point) as f64;
        let second_half_pace: f64 = pace_data[half_point..].iter().map(|&p| p as f64).sum::<f64>() / (pace_data.len() - half_point) as f64;

        // Calculate efficiency ratios
        let first_efficiency = first_half_pace / first_half_hr;
        let second_efficiency = second_half_pace / second_half_hr;

        // Decoupling percentage
        Some(((second_efficiency - first_efficiency) / first_efficiency) * 100.0)
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
    pub fn from_heart_rate_data(hr_data: &[f32], lthr: f64) -> Self {
        let total_points = hr_data.len() as f64;
        
        let zone1 = hr_data.iter().filter(|&&hr| hr as f64 <= lthr * 0.80).count() as f64;
        let zone2 = hr_data.iter().filter(|&&hr| hr as f64 > lthr * 0.80 && hr as f64 <= lthr * 0.90).count() as f64;
        let zone3 = hr_data.iter().filter(|&&hr| hr as f64 > lthr * 0.90 && hr as f64 <= lthr * 1.00).count() as f64;
        let zone4 = hr_data.iter().filter(|&&hr| hr as f64 > lthr * 1.00 && hr as f64 <= lthr * 1.10).count() as f64;
        let zone5 = hr_data.iter().filter(|&&hr| hr as f64 > lthr * 1.10).count() as f64;

        let mut time_in_zones = HashMap::new();
        time_in_zones.insert("recovery".to_string(), (zone1 / total_points) * 100.0);
        time_in_zones.insert("aerobic".to_string(), (zone2 / total_points) * 100.0);
        time_in_zones.insert("tempo".to_string(), (zone3 / total_points) * 100.0);
        time_in_zones.insert("threshold".to_string(), (zone4 / total_points) * 100.0);
        time_in_zones.insert("vo2max".to_string(), (zone5 / total_points) * 100.0);

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
    pub fn from_power_data(power_data: &[f32], ftp: f64) -> Self {
        let total_points = power_data.len() as f64;
        
        let zone1 = power_data.iter().filter(|&&p| p as f64 <= ftp * 0.55).count() as f64;
        let zone2 = power_data.iter().filter(|&&p| p as f64 > ftp * 0.55 && p as f64 <= ftp * 0.75).count() as f64;
        let zone3 = power_data.iter().filter(|&&p| p as f64 > ftp * 0.75 && p as f64 <= ftp * 0.90).count() as f64;
        let zone4 = power_data.iter().filter(|&&p| p as f64 > ftp * 0.90 && p as f64 <= ftp * 1.05).count() as f64;
        let zone5 = power_data.iter().filter(|&&p| p as f64 > ftp * 1.05).count() as f64;

        let mut time_in_zones = HashMap::new();
        time_in_zones.insert("active_recovery".to_string(), (zone1 / total_points) * 100.0);
        time_in_zones.insert("endurance".to_string(), (zone2 / total_points) * 100.0);
        time_in_zones.insert("tempo".to_string(), (zone3 / total_points) * 100.0);
        time_in_zones.insert("threshold".to_string(), (zone4 / total_points) * 100.0);
        time_in_zones.insert("vo2max".to_string(), (zone5 / total_points) * 100.0);

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
        let calculator = MetricsCalculator::new()
            .with_user_data(None, None, Some(190.0), Some(60.0), None);

        let trimp = calculator.calculate_trimp(150.0, 3600); // 150 bpm for 1 hour
        assert!(trimp.is_some());
        assert!(trimp.unwrap() > 0.0);
    }

    #[test]
    fn test_power_to_weight_ratio() {
        let calculator = MetricsCalculator::new()
            .with_user_data(Some(250.0), None, None, None, Some(70.0));

        let mut activity = Activity::default();
        activity.average_watts = Some(200.0);

        let metrics = calculator.calculate_metrics(&activity).unwrap();
        assert_eq!(metrics.power_to_weight_ratio, Some(200.0 / 70.0));
    }

    #[test]
    fn test_zone_analysis() {
        let hr_data = vec![120.0, 130.0, 140.0, 160.0, 180.0]; // Sample HR data
        let lthr = 160.0;

        let analysis = ZoneAnalysis::from_heart_rate_data(&hr_data, lthr);
        
        // Should have distributed the data across zones
        assert!(analysis.zone1_percentage + analysis.zone2_percentage + 
                analysis.zone3_percentage + analysis.zone4_percentage + 
                analysis.zone5_percentage <= 100.1); // Allow for floating point precision
    }
}