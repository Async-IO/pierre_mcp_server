// ABOUTME: Recovery score aggregation algorithms for combining TSB, sleep, and HRV metrics
// ABOUTME: Implements weighted average, geometric mean, harmonic mean, minimum, and Bayesian methods

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Weight configuration for recovery score aggregation
#[derive(Debug, Clone, Copy)]
struct WeightConfig {
    tsb_weight_full: f64,
    sleep_weight_full: f64,
    hrv_weight_full: f64,
    tsb_weight_no_hrv: f64,
    sleep_weight_no_hrv: f64,
}

/// Recovery score aggregation algorithm selection
///
/// Different algorithms for combining TSB (Training Stress Balance), sleep quality,
/// and HRV (Heart Rate Variability) scores into a unified recovery score:
///
/// - `WeightedAverage`: Linear combination with configurable weights
/// - `GeometricMean`: Multiplicative combination (all factors must be good)
/// - `HarmonicMean`: Conservative approach emphasizing weakest component
/// - `Minimum`: Takes worst score (most conservative)
/// - `Bayesian`: Probabilistic combination with confidence threshold
///
/// # Scientific References
///
/// - Buchheit, M. (2014). "Monitoring training status with HR measures." *Sports Medicine*, 44(S1), 73-81.
/// - Plews, D.J., et al. (2013). "Training adaptation and heart rate variability." *Sports Medicine*, 43(9), 773-781.
/// - Halson, S.L. (2014). "Monitoring training load to understand fatigue." *Sports Medicine*, 44(S2), 139-147.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAggregationAlgorithm {
    /// Weighted Average
    ///
    /// Formula: `Recovery = w_tsbxTSB + w_sleepxSleep + w_hrvxHRV` (if HRV available)
    ///          `Recovery = w_tsbxTSB + w_sleepxSleep` (if no HRV)
    ///
    /// Linear combination with normalized weights (sum to 1.0).
    /// Most common approach, allows fine-tuning importance of each metric.
    ///
    /// Pros: Intuitive, flexible, widely used
    /// Cons: Linear assumption may not reflect physiological reality
    WeightedAverage {
        /// TSB weight when all metrics available (default 0.30)
        tsb_weight_full: f64,
        /// Sleep weight when all metrics available (default 0.40)
        sleep_weight_full: f64,
        /// HRV weight when all metrics available (default 0.30)
        hrv_weight_full: f64,
        /// TSB weight when no HRV (default 0.40)
        tsb_weight_no_hrv: f64,
        /// Sleep weight when no HRV (default 0.60)
        sleep_weight_no_hrv: f64,
    },

    /// Geometric Mean
    ///
    /// Formula: `Recovery = (TSB^w x Sleep^w x HRV^w)^(1/n)` where n = number of components
    ///
    /// Multiplicative combination - all factors must be reasonably good for high score.
    /// Single poor metric has larger impact than in weighted average.
    ///
    /// Pros: Penalizes imbalanced recovery, physiologically sound
    /// Cons: More sensitive to outliers, less intuitive
    GeometricMean,

    /// Harmonic Mean
    ///
    /// Formula: `Recovery = n / (1/TSB + 1/Sleep + 1/HRV)` where n = number of components
    ///
    /// Conservative approach - weakest component dominates the score.
    /// Heavily penalizes poor performance in any single metric.
    ///
    /// Pros: Most conservative, emphasizes addressing weakest link
    /// Cons: May be overly pessimistic, less commonly used
    HarmonicMean,

    /// Minimum (Most Conservative)
    ///
    /// Formula: `Recovery = min(TSB, Sleep, HRV)`
    ///
    /// Takes the worst score among all available metrics.
    /// "You're only as recovered as your weakest system."
    ///
    /// Pros: Maximally conservative, simple to understand
    /// Cons: Ignores positive signals from other metrics
    Minimum,

    /// Bayesian Probabilistic Combination
    ///
    /// Formula: `P(Recovered) = P(TSB) x P(Sleep) x P(HRV) / P(Evidence)`
    ///
    /// Treats each metric as evidence for recovery state.
    /// Combines probabilistic assessments with confidence weighting.
    ///
    /// Pros: Principled uncertainty handling, adaptable to data quality
    /// Cons: Complex, requires calibration, computationally expensive
    Bayesian {
        /// Confidence threshold for low-confidence data (default 0.5)
        confidence_threshold: f64,
    },
}

impl Default for RecoveryAggregationAlgorithm {
    fn default() -> Self {
        Self::WeightedAverage {
            tsb_weight_full: 0.30,
            sleep_weight_full: 0.40,
            hrv_weight_full: 0.30,
            tsb_weight_no_hrv: 0.40,
            sleep_weight_no_hrv: 0.60,
        }
    }
}

impl RecoveryAggregationAlgorithm {
    /// Aggregate TSB, sleep, and HRV scores into unified recovery score
    ///
    /// # Arguments
    ///
    /// * `tsb_score` - Training Stress Balance score (0-100)
    /// * `sleep_score` - Sleep quality score (0-100)
    /// * `hrv_score` - Optional HRV score (0-100)
    ///
    /// # Returns
    ///
    /// Aggregated recovery score (0-100)
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - Scores are outside valid range (0-100)
    /// - Weights don't sum to 1.0 (for weighted average)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let algorithm = RecoveryAggregationAlgorithm::WeightedAverage {
    ///     tsb_weight_full: 0.30,
    ///     sleep_weight_full: 0.40,
    ///     hrv_weight_full: 0.30,
    ///     tsb_weight_no_hrv: 0.40,
    ///     sleep_weight_no_hrv: 0.60,
    /// };
    /// let recovery = algorithm.aggregate(75.0, 80.0, Some(70.0))?;
    /// ```
    pub fn aggregate(
        &self,
        tsb_score: f64,
        sleep_score: f64,
        hrv_score: Option<f64>,
    ) -> AppResult<f64> {
        // Validate inputs
        Self::validate_score(tsb_score, "TSB")?;
        Self::validate_score(sleep_score, "Sleep")?;
        if let Some(hrv) = hrv_score {
            Self::validate_score(hrv, "HRV")?;
        }

        match self {
            Self::WeightedAverage {
                tsb_weight_full,
                sleep_weight_full,
                hrv_weight_full,
                tsb_weight_no_hrv,
                sleep_weight_no_hrv,
            } => Self::calculate_weighted_average(
                tsb_score,
                sleep_score,
                hrv_score,
                WeightConfig {
                    tsb_weight_full: *tsb_weight_full,
                    sleep_weight_full: *sleep_weight_full,
                    hrv_weight_full: *hrv_weight_full,
                    tsb_weight_no_hrv: *tsb_weight_no_hrv,
                    sleep_weight_no_hrv: *sleep_weight_no_hrv,
                },
            ),
            Self::GeometricMean => {
                Self::calculate_geometric_mean(tsb_score, sleep_score, hrv_score)
            }
            Self::HarmonicMean => Self::calculate_harmonic_mean(tsb_score, sleep_score, hrv_score),
            Self::Minimum => Ok(Self::calculate_minimum(tsb_score, sleep_score, hrv_score)),
            Self::Bayesian {
                confidence_threshold,
            } => Ok(Self::calculate_bayesian(
                tsb_score,
                sleep_score,
                hrv_score,
                *confidence_threshold,
            )),
        }
    }

    /// Validate that score is in valid range (0-100)
    fn validate_score(score: f64, name: &str) -> AppResult<()> {
        if !(0.0..=100.0).contains(&score) {
            return Err(AppError::invalid_input(format!(
                "{name} score {score:.1} is outside valid range (0-100)"
            )));
        }
        Ok(())
    }

    /// Calculate weighted average
    fn calculate_weighted_average(
        tsb_score: f64,
        sleep_score: f64,
        hrv_score: Option<f64>,
        weights: WeightConfig,
    ) -> AppResult<f64> {
        let recovery_score = hrv_score.map_or_else(
            || {
                // No HRV: use 2-component weights
                let weight_sum = weights.tsb_weight_no_hrv + weights.sleep_weight_no_hrv;
                if (weight_sum - 1.0).abs() > 0.01 {
                    return Err(AppError::invalid_input(format!(
                        "Weights without HRV must sum to 1.0, got {weight_sum:.3}"
                    )));
                }
                Ok(tsb_score.mul_add(
                    weights.tsb_weight_no_hrv,
                    sleep_score * weights.sleep_weight_no_hrv,
                ))
            },
            |hrv| {
                // HRV available: use 3-component weights
                let weight_sum =
                    weights.tsb_weight_full + weights.sleep_weight_full + weights.hrv_weight_full;
                if (weight_sum - 1.0).abs() > 0.01 {
                    return Err(AppError::invalid_input(format!(
                        "Weights with HRV must sum to 1.0, got {weight_sum:.3}"
                    )));
                }
                Ok(hrv.mul_add(
                    weights.hrv_weight_full,
                    tsb_score.mul_add(
                        weights.tsb_weight_full,
                        sleep_score * weights.sleep_weight_full,
                    ),
                ))
            },
        )?;

        Ok(recovery_score.clamp(0.0, 100.0))
    }

    /// Calculate geometric mean
    fn calculate_geometric_mean(
        tsb_score: f64,
        sleep_score: f64,
        hrv_score: Option<f64>,
    ) -> AppResult<f64> {
        // Geometric mean: (x₁ x x₂ x ... x xₙ)^(1/n)
        let (product, count) = hrv_score.map_or_else(
            || (tsb_score * sleep_score, 2.0),
            |hrv| (tsb_score * sleep_score * hrv, 3.0),
        );

        if product <= 0.0 {
            return Err(AppError::invalid_input(
                "Geometric mean requires all scores to be positive".to_owned(),
            ));
        }

        #[allow(clippy::cast_precision_loss)]
        let recovery_score = product.powf(1.0 / count);
        Ok(recovery_score.clamp(0.0, 100.0))
    }

    /// Calculate harmonic mean
    fn calculate_harmonic_mean(
        tsb_score: f64,
        sleep_score: f64,
        hrv_score: Option<f64>,
    ) -> AppResult<f64> {
        // Harmonic mean: n / (1/x₁ + 1/x₂ + ... + 1/xₙ)

        // Guard against division by zero
        if tsb_score <= 0.0 || sleep_score <= 0.0 || hrv_score.is_some_and(|hrv| hrv <= 0.0) {
            return Err(AppError::invalid_input(
                "Harmonic mean requires all scores to be positive".to_owned(),
            ));
        }

        let (reciprocal_sum, count) = hrv_score.map_or_else(
            || (1.0 / tsb_score + 1.0 / sleep_score, 2.0),
            |hrv| (1.0 / tsb_score + 1.0 / sleep_score + 1.0 / hrv, 3.0),
        );

        #[allow(clippy::cast_precision_loss)]
        let recovery_score = count / reciprocal_sum;
        Ok(recovery_score.clamp(0.0, 100.0))
    }

    /// Calculate minimum (most conservative)
    #[must_use]
    fn calculate_minimum(tsb_score: f64, sleep_score: f64, hrv_score: Option<f64>) -> f64 {
        hrv_score.map_or_else(
            || tsb_score.min(sleep_score),
            |hrv| tsb_score.min(sleep_score).min(hrv),
        )
    }

    /// Calculate Bayesian probabilistic combination
    fn calculate_bayesian(
        tsb_score: f64,
        sleep_score: f64,
        hrv_score: Option<f64>,
        confidence_threshold: f64,
    ) -> f64 {
        // Convert scores to probabilities (0-100 → 0-1)
        let p_tsb = tsb_score / 100.0;
        let p_sleep = sleep_score / 100.0;

        // Apply confidence threshold: low scores have less confidence
        let confidence_tsb = if p_tsb < confidence_threshold {
            0.5
        } else {
            1.0
        };
        let confidence_sleep = if p_sleep < confidence_threshold {
            0.5
        } else {
            1.0
        };

        let (combined_probability, total_confidence) = hrv_score.map_or_else(
            || {
                // No HRV: combine TSB and Sleep
                let p_combined = p_tsb * p_sleep;
                let conf_combined = confidence_tsb * confidence_sleep;
                (p_combined, conf_combined)
            },
            |hrv| {
                // HRV available: combine all three
                let p_hrv = hrv / 100.0;
                let confidence_hrv = if p_hrv < confidence_threshold {
                    0.5
                } else {
                    1.0
                };
                let p_combined = p_tsb * p_sleep * p_hrv;
                let conf_combined = confidence_tsb * confidence_sleep * confidence_hrv;
                (p_combined, conf_combined)
            },
        );

        // Weight the probability by confidence
        let recovery_score = (combined_probability * total_confidence) * 100.0;
        recovery_score.clamp(0.0, 100.0)
    }

    /// Get algorithm name
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::WeightedAverage { .. } => "weighted_average",
            Self::GeometricMean => "geometric_mean",
            Self::HarmonicMean => "harmonic_mean",
            Self::Minimum => "minimum",
            Self::Bayesian { .. } => "bayesian",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::WeightedAverage {
                tsb_weight_full,
                sleep_weight_full,
                hrv_weight_full,
                ..
            } => {
                format!(
                    "Weighted Average (TSB={tsb_weight_full:.2}, Sleep={sleep_weight_full:.2}, HRV={hrv_weight_full:.2})"
                )
            }
            Self::GeometricMean => "Geometric Mean (multiplicative combination)".to_owned(),
            Self::HarmonicMean => "Harmonic Mean (emphasizes weakest component)".to_owned(),
            Self::Minimum => "Minimum (most conservative)".to_owned(),
            Self::Bayesian {
                confidence_threshold,
            } => {
                format!("Bayesian (confidence threshold={confidence_threshold:.2})")
            }
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::WeightedAverage { .. } => {
                "Recovery = w_tsbxTSB + w_sleepxSleep + w_hrvxHRV (weights sum to 1.0)"
            }
            Self::GeometricMean => "Recovery = (TSB x Sleep x HRV)^(1/n)",
            Self::HarmonicMean => "Recovery = n / (1/TSB + 1/Sleep + 1/HRV)",
            Self::Minimum => "Recovery = min(TSB, Sleep, HRV)",
            Self::Bayesian { .. } => {
                "P(Recovered) = P(TSB) x P(Sleep) x P(HRV) weighted by confidence"
            }
        }
    }
}

impl FromStr for RecoveryAggregationAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "weighted_average" | "weighted" => Ok(Self::WeightedAverage {
                tsb_weight_full: 0.30,
                sleep_weight_full: 0.40,
                hrv_weight_full: 0.30,
                tsb_weight_no_hrv: 0.40,
                sleep_weight_no_hrv: 0.60,
            }),
            "geometric_mean" | "geometric" => Ok(Self::GeometricMean),
            "harmonic_mean" | "harmonic" => Ok(Self::HarmonicMean),
            "minimum" | "min" => Ok(Self::Minimum),
            "bayesian" => Ok(Self::Bayesian {
                confidence_threshold: 0.5,
            }),
            other => Err(AppError::invalid_input(format!(
                "Unknown recovery aggregation algorithm: '{other}'. Valid options: weighted_average, geometric_mean, harmonic_mean, minimum, bayesian"
            ))),
        }
    }
}
