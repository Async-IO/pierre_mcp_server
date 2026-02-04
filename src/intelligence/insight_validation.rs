// ABOUTME: LLM-powered insight quality validation for social feed sharing
// ABOUTME: Validates, redacts sensitive data, and improves content based on user policy and tier
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Insight Quality Validation
//!
//! This module provides LLM-powered validation for fitness content before
//! sharing to the social feed. It ensures only genuine, data-driven insights
//! are shared while offering content improvement for premium tiers.
//!
//! ## User Sharing Policy
//!
//! Each user can configure their sharing policy:
//!
//! - **DataRich**: Allow all metrics (times, paces, HR, power)
//! - **Sanitized**: Auto-redact specific numbers to ranges
//! - **GeneralOnly**: Reject content containing specific metrics
//! - **Disabled**: No sharing allowed
//!
//! ## Tier-Based Behavior
//!
//! - **Starter**: Validation only - rejects generic content, passes valid content unchanged
//! - **Professional/Enterprise**: Validation + improvement - can enhance weak content
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::intelligence::insight_validation::{
//!     validate_insight_with_policy, ValidationVerdict, InsightSharingPolicy,
//! };
//! use pierre_mcp_server::llm::{ChatProvider, LlmProvider};
//! use pierre_mcp_server::models::{InsightType, UserTier};
//!
//! async fn example(provider: &dyn LlmProvider) {
//!     let result = validate_insight_with_policy(
//!         provider,
//!         "Just completed a 10K in 45:32 - new PR!",
//!         InsightType::Achievement,
//!         &UserTier::Professional,
//!         &InsightSharingPolicy::Sanitized,
//!     ).await;
//! }
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use tracing::{debug, warn};

use crate::errors::AppError;
use crate::llm::{get_insight_validation_prompt, ChatMessage, ChatRequest, LlmProvider};
use crate::models::{InsightType, UserTier};

// ============================================================================
// Sharing Policy
// ============================================================================

/// User-level policy controlling what data can be shared in insights
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InsightSharingPolicy {
    /// Allow insights with all metrics (times, paces, HR, power)
    #[default]
    DataRich,
    /// Auto-redact specific numbers to ranges (45:32 → "sub-46 minutes")
    Sanitized,
    /// Only allow insights without specific metrics
    GeneralOnly,
    /// No sharing allowed
    Disabled,
}

impl InsightSharingPolicy {
    /// Database/API string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DataRich => "data_rich",
            Self::Sanitized => "sanitized",
            Self::GeneralOnly => "general_only",
            Self::Disabled => "disabled",
        }
    }

    /// Parse from database string representation
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "data_rich" | "datarich" => Some(Self::DataRich),
            "sanitized" => Some(Self::Sanitized),
            "general_only" | "generalonly" => Some(Self::GeneralOnly),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    /// Human-readable description
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::DataRich => "Share insights with all metrics visible",
            Self::Sanitized => "Share insights with metrics converted to ranges",
            Self::GeneralOnly => "Share only general insights without specific data",
            Self::Disabled => "Insight sharing is disabled",
        }
    }
}

// ============================================================================
// Validation Result Types
// ============================================================================

/// Result of insight quality validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "lowercase")]
pub enum ValidationVerdict {
    /// Content is acceptable as-is
    Valid,
    /// Content can be improved (premium tier only)
    Improved {
        /// The improved version of the content
        improved_content: String,
        /// Explanation of improvements made
        reason: String,
    },
    /// Content is rejected (not suitable for sharing)
    Rejected {
        /// Reason for rejection
        reason: String,
    },
}

impl ValidationVerdict {
    /// Check if the verdict allows sharing
    #[must_use]
    pub const fn allows_sharing(&self) -> bool {
        matches!(self, Self::Valid | Self::Improved { .. })
    }

    /// Get the content to use for sharing (original or improved)
    #[must_use]
    pub fn content_for_sharing<'a>(&'a self, original: &'a str) -> Option<&'a str> {
        match self {
            Self::Valid => Some(original),
            Self::Improved {
                improved_content, ..
            } => Some(improved_content.as_str()),
            Self::Rejected { .. } => None,
        }
    }

    /// Get the rejection reason if rejected
    #[must_use]
    pub fn rejection_reason(&self) -> Option<&str> {
        match self {
            Self::Rejected { reason } => Some(reason),
            _ => None,
        }
    }
}

/// Full validation result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightValidationResult {
    /// The validation verdict
    pub verdict: ValidationVerdict,
    /// Original content that was validated
    pub original_content: String,
    /// Final content after any redaction/improvement (for UI display)
    pub final_content: String,
    /// User tier that influenced the validation behavior
    pub tier: UserTier,
    /// Sharing policy that was applied
    pub policy: InsightSharingPolicy,
    /// Whether improvement was applied (for premium tiers)
    pub was_improved: bool,
    /// Whether redaction was applied (for sanitized policy)
    pub was_redacted: bool,
    /// List of redactions applied (for UI transparency)
    pub redactions: Vec<RedactionInfo>,
}

impl InsightValidationResult {
    /// Get the final content to use for sharing
    #[must_use]
    pub fn content_for_sharing(&self) -> Option<&str> {
        if self.verdict.allows_sharing() {
            Some(&self.final_content)
        } else {
            None
        }
    }

    /// Check if sharing is allowed
    #[must_use]
    pub const fn can_share(&self) -> bool {
        self.verdict.allows_sharing()
    }

    /// Check if content was modified (improved or redacted)
    #[must_use]
    pub const fn was_modified(&self) -> bool {
        self.was_improved || self.was_redacted
    }
}

/// Information about a single redaction for UI transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionInfo {
    /// Type of data that was redacted
    pub data_type: InsightMetricType,
    /// Original value (for preview/undo in UI)
    pub original: String,
    /// Redacted value
    pub redacted: String,
}

/// Types of metrics that can be detected and redacted in insights
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightMetricType {
    /// Time duration (e.g., 45:32, 3:42:15)
    Time,
    /// Pace (e.g., 4:30/km, 7:15/mi)
    Pace,
    /// Heart rate (e.g., 168bpm, HR 145)
    HeartRate,
    /// Power (e.g., 285W, 3.8w/kg)
    Power,
    /// Distance (e.g., 10K, 42.2km, 26.2mi)
    Distance,
    /// Speed (e.g., 25km/h, 15.5mph)
    Speed,
    /// Cadence (e.g., 180spm, 90rpm)
    Cadence,
    /// Training metrics (e.g., CTL 47, TSS 85, FTP 270)
    TrainingMetric,
}

// ============================================================================
// Data Detection and Redaction
// ============================================================================

/// Regex patterns for detecting fitness metrics in content
/// Stored as Option to handle compilation failures gracefully (should never fail for static patterns)
static TIME_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    // Matches: 45:32, 3:42:15, 1:23:45.6
    Regex::new(r"\b(\d{1,2}):(\d{2})(?::(\d{2}))?(?:\.(\d+))?\b").ok()
});

static PACE_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    // Matches: 4:30/km, 7:15/mi, 5:00 per km
    Regex::new(r"\b(\d{1,2}):(\d{2})\s*(?:/|per\s*)(km|mi|mile|kilometer)\b").ok()
});

static HEART_RATE_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    // Matches: 168bpm, 145 bpm, HR 168, heart rate 145
    Regex::new(r"(?i)\b(?:HR|heart\s*rate)?\s*(\d{2,3})\s*(?:bpm|beats)?\b").ok()
});

static POWER_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    // Matches: 285W, 280 watts, 3.8w/kg, 3.5 watts/kg
    Regex::new(r"(?i)\b(\d{2,4})\s*(?:w|watts?)(?:/kg)?\b|\b(\d+\.?\d*)\s*w/kg\b").ok()
});

static DISTANCE_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    // Matches: 10K, 42.2km, 26.2mi, 100 miles, 5 kilometers
    Regex::new(r"(?i)\b(\d+\.?\d*)\s*(k|km|mi|mile|miles|kilometers?|meters?|m)\b").ok()
});

static TRAINING_METRIC_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    // Matches: CTL 47, TSS 85, FTP 270, ATL 52, VDOT 48
    Regex::new(r"(?i)\b(CTL|TSS|FTP|ATL|VDOT|IF|NP)\s*(?:of\s*)?(\d+\.?\d*)\b").ok()
});

/// Detected metric in content
#[derive(Debug, Clone)]
pub struct DetectedMetric {
    /// Type of metric
    pub metric_type: InsightMetricType,
    /// Original matched text
    pub original: String,
    /// Start position in content
    pub start: usize,
    /// End position in content
    pub end: usize,
}

/// Detect all fitness metrics in content
#[must_use]
pub fn detect_metrics(content: &str) -> Vec<DetectedMetric> {
    let mut metrics = Vec::new();

    // Detect times (but not pace - pace has /km or /mi)
    if let Some(pattern) = TIME_PATTERN.as_ref() {
        for cap in pattern.captures_iter(content) {
            if let Some(matched) = cap.get(0) {
                // Skip if this looks like pace (followed by /km or /mi)
                let after = &content[matched.end()..];
                if !after.trim_start().starts_with('/') && !after.to_lowercase().starts_with("per")
                {
                    metrics.push(DetectedMetric {
                        metric_type: InsightMetricType::Time,
                        original: matched.as_str().to_owned(),
                        start: matched.start(),
                        end: matched.end(),
                    });
                }
            }
        }
    }

    // Detect paces
    if let Some(pattern) = PACE_PATTERN.as_ref() {
        for cap in pattern.captures_iter(content) {
            if let Some(matched) = cap.get(0) {
                metrics.push(DetectedMetric {
                    metric_type: InsightMetricType::Pace,
                    original: matched.as_str().to_owned(),
                    start: matched.start(),
                    end: matched.end(),
                });
            }
        }
    }

    // Detect heart rates (be careful not to match years or other numbers)
    if let Some(pattern) = HEART_RATE_PATTERN.as_ref() {
        for cap in pattern.captures_iter(content) {
            if let Some(hr_match) = cap.get(1) {
                let hr: u32 = hr_match.as_str().parse().unwrap_or(0);
                // Only consider reasonable HR values (40-220)
                if (40..=220).contains(&hr) {
                    if let Some(matched) = cap.get(0) {
                        metrics.push(DetectedMetric {
                            metric_type: InsightMetricType::HeartRate,
                            original: matched.as_str().to_owned(),
                            start: matched.start(),
                            end: matched.end(),
                        });
                    }
                }
            }
        }
    }

    // Detect power
    if let Some(pattern) = POWER_PATTERN.as_ref() {
        for cap in pattern.captures_iter(content) {
            if let Some(matched) = cap.get(0) {
                metrics.push(DetectedMetric {
                    metric_type: InsightMetricType::Power,
                    original: matched.as_str().to_owned(),
                    start: matched.start(),
                    end: matched.end(),
                });
            }
        }
    }

    // Detect distances
    if let Some(pattern) = DISTANCE_PATTERN.as_ref() {
        for cap in pattern.captures_iter(content) {
            if let Some(matched) = cap.get(0) {
                metrics.push(DetectedMetric {
                    metric_type: InsightMetricType::Distance,
                    original: matched.as_str().to_owned(),
                    start: matched.start(),
                    end: matched.end(),
                });
            }
        }
    }

    // Detect training metrics (CTL, TSS, FTP, etc.)
    if let Some(pattern) = TRAINING_METRIC_PATTERN.as_ref() {
        for cap in pattern.captures_iter(content) {
            if let Some(matched) = cap.get(0) {
                metrics.push(DetectedMetric {
                    metric_type: InsightMetricType::TrainingMetric,
                    original: matched.as_str().to_owned(),
                    start: matched.start(),
                    end: matched.end(),
                });
            }
        }
    }

    // Sort by position for consistent replacement
    metrics.sort_by_key(|m| m.start);
    metrics
}

/// Check if content contains any fitness metrics
#[must_use]
pub fn contains_metrics(content: &str) -> bool {
    !detect_metrics(content).is_empty()
}

/// Redact a single metric to a range/description
fn redact_metric(metric: &DetectedMetric) -> String {
    match metric.metric_type {
        InsightMetricType::Time => redact_time(&metric.original),
        InsightMetricType::Pace => redact_pace(&metric.original),
        InsightMetricType::HeartRate => redact_heart_rate(&metric.original),
        InsightMetricType::Power => redact_power(&metric.original),
        InsightMetricType::Distance => redact_distance(&metric.original),
        InsightMetricType::Speed => "moderate speed".to_owned(),
        InsightMetricType::Cadence => "good cadence".to_owned(),
        InsightMetricType::TrainingMetric => redact_training_metric(&metric.original),
    }
}

/// Redact time to a range (45:32 → "sub-46 minutes")
fn redact_time(time_str: &str) -> String {
    // Parse MM:SS or HH:MM:SS
    let parts: Vec<&str> = time_str.split(':').collect();
    match parts.len() {
        2 => {
            // MM:SS format
            let minutes: u32 = parts[0].parse().unwrap_or(0);
            let seconds: u32 = parts[1]
                .split('.')
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            let total_secs = minutes * 60 + seconds;

            if total_secs < 300 {
                // Under 5 minutes - probably a short interval
                let rounded_mins = (minutes + 1).min(5);
                format!("around {rounded_mins} minutes")
            } else if total_secs < 3600 {
                // Under 1 hour
                let rounded = ((minutes + 2) / 5) * 5; // Round to nearest 5
                format!("around {rounded} minutes")
            } else {
                format!("over {minutes} minutes")
            }
        }
        3 => {
            // HH:MM:SS format - round minutes to nearest 30
            let hours: u32 = parts[0].parse().unwrap_or(0);
            let minutes: u32 = parts[1].parse().unwrap_or(0);
            let rounded_minutes = ((minutes + 15) / 30) * 30;
            format!("around {hours}:{rounded_minutes:02} hours")
        }
        _ => "a good time".to_owned(),
    }
}

/// Redact pace to a range (4:30/km → "around 4:30/km pace")
fn redact_pace(pace_str: &str) -> String {
    // Keep the unit but generalize
    if pace_str.contains("km") {
        "a solid per-km pace".to_owned()
    } else {
        "a solid per-mile pace".to_owned()
    }
}

/// Redact heart rate to zone description
fn redact_heart_rate(hr_str: &str) -> String {
    // Extract the number
    let hr: u32 = hr_str
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0);

    // Convert to approximate zone (assuming max HR ~190)
    if hr < 120 {
        "easy effort (Zone 1-2)".to_owned()
    } else if hr < 150 {
        "moderate effort (Zone 2-3)".to_owned()
    } else if hr < 170 {
        "tempo effort (Zone 3-4)".to_owned()
    } else {
        "high intensity (Zone 4-5)".to_owned()
    }
}

/// Redact power to a range
fn redact_power(power_str: &str) -> String {
    if power_str.to_lowercase().contains("w/kg") {
        "good watts per kilo".to_owned()
    } else {
        // Extract watts
        let watts: u32 = power_str
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap_or(0);

        if watts < 150 {
            "endurance power".to_owned()
        } else if watts < 250 {
            "tempo power".to_owned()
        } else if watts < 350 {
            "threshold power".to_owned()
        } else {
            "high power output".to_owned()
        }
    }
}

/// Redact distance to approximate
fn redact_distance(distance_str: &str) -> String {
    let lower = distance_str.to_lowercase();

    // Common race distances
    if lower.contains("5k") || lower.contains("5 k") {
        return "a 5K".to_owned();
    }
    if lower.contains("10k") || lower.contains("10 k") {
        return "a 10K".to_owned();
    }
    if lower.contains("half") || lower.contains("21k") || lower.contains("13.1") {
        return "a half marathon".to_owned();
    }
    if lower.contains("marathon") || lower.contains("42k") || lower.contains("26.2") {
        return "a marathon".to_owned();
    }

    // Generic distance
    if lower.contains("km") || lower.contains("kilometer") {
        "a good distance".to_owned()
    } else if lower.contains("mi") {
        "a solid mileage".to_owned()
    } else {
        "a respectable distance".to_owned()
    }
}

/// Redact training metric to description
fn redact_training_metric(metric_str: &str) -> String {
    let upper = metric_str.to_uppercase();

    if upper.contains("CTL") {
        "improving fitness (CTL)".to_owned()
    } else if upper.contains("TSS") {
        "appropriate training stress".to_owned()
    } else if upper.contains("FTP") {
        "solid FTP".to_owned()
    } else if upper.contains("ATL") {
        "manageable fatigue".to_owned()
    } else if upper.contains("VDOT") {
        "good VDOT score".to_owned()
    } else {
        "good training metric".to_owned()
    }
}

/// Redact all metrics in content and return redaction info
#[must_use]
pub fn redact_content(content: &str) -> (String, Vec<RedactionInfo>) {
    let metrics = detect_metrics(content);

    if metrics.is_empty() {
        return (content.to_owned(), Vec::new());
    }

    let mut result = String::with_capacity(content.len());
    let mut redactions = Vec::new();
    let mut last_end = 0;

    for metric in metrics {
        // Add text before this metric
        result.push_str(&content[last_end..metric.start]);

        // Redact the metric
        let redacted = redact_metric(&metric);
        result.push_str(&redacted);

        redactions.push(RedactionInfo {
            data_type: metric.metric_type,
            original: metric.original.clone(),
            redacted: redacted.clone(),
        });

        last_end = metric.end;
    }

    // Add remaining text
    result.push_str(&content[last_end..]);

    (result, redactions)
}

// ============================================================================
// LLM Response Parsing
// ============================================================================

/// Raw LLM response structure for parsing
#[derive(Debug, Deserialize)]
struct LlmValidationResponse {
    verdict: String,
    reason: String,
    #[serde(default)]
    improved_content: Option<String>,
}

impl LlmValidationResponse {
    /// Convert to `ValidationVerdict` based on user tier
    fn into_verdict(self, tier: &UserTier) -> ValidationVerdict {
        match self.verdict.to_lowercase().as_str() {
            "valid" => ValidationVerdict::Valid,
            "improved" => {
                // Only premium tiers get improvements
                if matches!(tier, UserTier::Professional | UserTier::Enterprise) {
                    if let Some(improved) = self.improved_content {
                        ValidationVerdict::Improved {
                            improved_content: improved,
                            reason: self.reason,
                        }
                    } else {
                        // No improved content provided, treat as valid
                        ValidationVerdict::Valid
                    }
                } else {
                    // Starter tier: treat improvable content as valid (pass through)
                    ValidationVerdict::Valid
                }
            }
            "rejected" => ValidationVerdict::Rejected {
                reason: self.reason,
            },
            other => {
                warn!("Unknown validation verdict from LLM: {other}, treating as valid");
                ValidationVerdict::Valid
            }
        }
    }
}

// ============================================================================
// Validation Service
// ============================================================================

/// Internal LLM validation result (simpler structure for internal use)
struct InternalValidationResult {
    verdict: ValidationVerdict,
    was_improved: bool,
}

/// Validate insight content quality using LLM (internal function)
///
/// Uses LLM to analyze the content and determine if it's suitable for
/// sharing on the social feed. Behavior varies by user tier:
///
/// - **Starter**: Returns `Valid` or `Rejected` only
/// - **Professional/Enterprise**: Returns `Valid`, `Improved`, or `Rejected`
///
/// # Arguments
///
/// * `provider` - LLM provider for validation (implements `LlmProvider` trait)
/// * `content` - The insight content to validate
/// * `insight_type` - Type of insight being shared
/// * `user_tier` - User's subscription tier
///
/// # Errors
///
/// Returns an error if the LLM call fails or response parsing fails.
async fn validate_insight(
    provider: &dyn LlmProvider,
    content: &str,
    insight_type: InsightType,
    user_tier: &UserTier,
) -> Result<InternalValidationResult, AppError> {
    debug!(
        "Validating insight content for {} tier, type: {:?}",
        user_tier, insight_type
    );

    // Build the validation prompt
    let system_prompt = get_insight_validation_prompt();
    let user_message = format!(
        "Please evaluate this fitness content for social sharing:\n\n\
        Content Type: {}\n\
        Content:\n{}\n\n\
        Return your evaluation as JSON.",
        insight_type.description(),
        content
    );

    let messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_message),
    ];

    let request = ChatRequest::new(messages).with_temperature(0.3); // Low temperature for consistent evaluation

    // Call LLM
    let response = provider.complete(&request).await?;

    // Parse response
    let verdict = parse_llm_response(&response.content, user_tier)?;
    let was_improved = matches!(verdict, ValidationVerdict::Improved { .. });

    Ok(InternalValidationResult {
        verdict,
        was_improved,
    })
}

/// Parse LLM response into a validation verdict
fn parse_llm_response(response: &str, user_tier: &UserTier) -> Result<ValidationVerdict, AppError> {
    // Try to extract JSON from the response (LLM might include extra text)
    let json_str = extract_json(response)?;

    let llm_response: LlmValidationResponse = serde_json::from_str(&json_str).map_err(|e| {
        warn!("Failed to parse LLM validation response: {e}");
        AppError::internal(format!("Failed to parse validation response: {e}"))
    })?;

    Ok(llm_response.into_verdict(user_tier))
}

/// Extract JSON from LLM response that might contain extra text
fn extract_json(response: &str) -> Result<String, AppError> {
    // First try: parse the whole response as JSON
    if serde_json::from_str::<serde_json::Value>(response).is_ok() {
        return Ok(response.to_owned());
    }

    // Second try: find JSON object in the response
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            let json_candidate = &response[start..=end];
            if serde_json::from_str::<serde_json::Value>(json_candidate).is_ok() {
                return Ok(json_candidate.to_owned());
            }
        }
    }

    // Third try: look for JSON in code blocks
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start..].find("```\n") {
            let json_block = &response[start + 7..start + end];
            return extract_json(json_block.trim());
        }
    }

    Err(AppError::internal(
        "Could not extract valid JSON from LLM response",
    ))
}

// ============================================================================
// Quick Validation (Skip LLM for obvious cases)
// ============================================================================

/// Patterns that indicate generic/placeholder content
const REJECTION_PATTERNS: &[&str] = &[
    "how can i assist",
    "how can i help",
    "what would you like",
    "i'm here to help",
    "let me know if",
    "feel free to ask",
    "is there anything",
    "what can i do for you",
];

/// Check if content should be rejected without LLM call
///
/// Returns `Some(reason)` if content matches known rejection patterns,
/// `None` if LLM validation is needed.
#[must_use]
pub fn quick_reject_check(content: &str) -> Option<String> {
    let lower = content.to_lowercase();

    // Check for generic patterns
    for pattern in REJECTION_PATTERNS {
        if lower.contains(pattern) {
            return Some(
                "Content appears to be a generic assistant response rather than a fitness insight"
                    .to_owned(),
            );
        }
    }

    // Check for very short content
    if content.trim().len() < 20 {
        return Some("Content is too short to be a meaningful insight".to_owned());
    }

    None
}

/// Validate insight with quick rejection check before LLM call
///
/// This is the recommended entry point - it first checks for obvious
/// rejection cases to save LLM calls, then falls back to full validation.
///
/// # Arguments
///
/// * `provider` - LLM provider for validation (implements `LlmProvider` trait)
/// * `content` - The insight content to validate
/// * `insight_type` - Type of insight being shared
/// * `user_tier` - User's subscription tier
///
/// # Errors
///
/// Returns an error if the LLM validation call fails.
pub async fn validate_insight_with_quick_check(
    provider: &dyn LlmProvider,
    content: &str,
    insight_type: InsightType,
    user_tier: &UserTier,
) -> Result<InsightValidationResult, AppError> {
    // Use default DataRich policy for backwards compatibility
    validate_insight_with_policy(
        provider,
        content,
        insight_type,
        user_tier,
        &InsightSharingPolicy::DataRich,
    )
    .await
}

/// Build a rejection result with the given reason
fn build_rejection_result(
    reason: String,
    content: &str,
    tier: &UserTier,
    policy: InsightSharingPolicy,
) -> InsightValidationResult {
    InsightValidationResult {
        verdict: ValidationVerdict::Rejected { reason },
        original_content: content.to_owned(),
        final_content: content.to_owned(),
        tier: tier.clone(),
        policy,
        was_improved: false,
        was_redacted: false,
        redactions: Vec::new(),
    }
}

/// Redaction result containing processed content and info
struct RedactionResult {
    content: String,
    redactions: Vec<RedactionInfo>,
    was_redacted: bool,
}

/// Apply redaction based on policy and whether content has metrics
fn apply_policy_redaction(
    content: &str,
    policy: InsightSharingPolicy,
    has_metrics: bool,
) -> RedactionResult {
    if matches!(policy, InsightSharingPolicy::Sanitized) && has_metrics {
        let (redacted, redaction_info) = redact_content(content);
        debug!("Applied {} redactions", redaction_info.len());
        RedactionResult {
            content: redacted,
            redactions: redaction_info,
            was_redacted: true,
        }
    } else {
        RedactionResult {
            content: content.to_owned(),
            redactions: Vec::new(),
            was_redacted: false,
        }
    }
}

/// Validate insight with user's sharing policy
///
/// This is the full-featured entry point that handles:
/// 1. Policy check (disabled, `general_only` checks)
/// 2. Quick rejection for obvious bad content
/// 3. Redaction for sanitized policy
/// 4. LLM quality validation
/// 5. Content improvement for premium tiers
///
/// # Arguments
///
/// * `provider` - LLM provider for validation (implements `LlmProvider` trait)
/// * `content` - The insight content to validate
/// * `insight_type` - Type of insight being shared
/// * `user_tier` - User's subscription tier
/// * `policy` - User's sharing policy
///
/// # Errors
///
/// Returns an error if the LLM call fails or response parsing fails.
pub async fn validate_insight_with_policy(
    provider: &dyn LlmProvider,
    content: &str,
    insight_type: InsightType,
    user_tier: &UserTier,
    policy: &InsightSharingPolicy,
) -> Result<InsightValidationResult, AppError> {
    debug!(
        "Validating insight with policy {:?} for {} tier, type: {:?}",
        policy, user_tier, insight_type
    );

    // Step 1: Check if sharing is disabled
    if matches!(policy, InsightSharingPolicy::Disabled) {
        return Ok(build_rejection_result(
            "Insight sharing is disabled for your account".to_owned(),
            content,
            user_tier,
            *policy,
        ));
    }

    // Step 2: Quick rejection check for generic content
    if let Some(reason) = quick_reject_check(content) {
        debug!("Quick rejection: {reason}");
        return Ok(build_rejection_result(reason, content, user_tier, *policy));
    }

    // Step 3: Check for metrics based on policy
    let has_metrics = contains_metrics(content);

    if matches!(policy, InsightSharingPolicy::GeneralOnly) && has_metrics {
        return Ok(build_rejection_result(
            "Your sharing policy only allows general insights without specific metrics. \
            Please rephrase without specific times, paces, or measurements."
                .to_owned(),
            content,
            user_tier,
            *policy,
        ));
    }

    // Step 4: Apply redaction if sanitized policy
    let redaction_result = apply_policy_redaction(content, *policy, has_metrics);

    // Step 5: LLM quality validation
    let llm_result =
        validate_insight(provider, &redaction_result.content, insight_type, user_tier).await?;

    // Step 6: Build final result
    // Improved verdict uses the enhanced content; Valid/Rejected keep original
    let final_content = match &llm_result.verdict {
        ValidationVerdict::Improved {
            improved_content, ..
        } => improved_content.clone(),
        ValidationVerdict::Valid | ValidationVerdict::Rejected { .. } => {
            redaction_result.content.clone()
        }
    };

    Ok(InsightValidationResult {
        verdict: llm_result.verdict,
        original_content: content.to_owned(),
        final_content,
        tier: user_tier.clone(),
        policy: *policy,
        was_improved: llm_result.was_improved,
        was_redacted: redaction_result.was_redacted,
        redactions: redaction_result.redactions,
    })
}
