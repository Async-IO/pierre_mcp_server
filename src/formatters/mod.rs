// ABOUTME: Output format abstraction for serializing data to multiple formats
// ABOUTME: Supports JSON (default) and TOON (token-efficient for LLMs)
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Output Format Abstraction Layer
//!
//! This module provides pluggable serialization formats for API responses.
//! The primary motivation is supporting TOON (Token-Oriented Object Notation)
//! which achieves ~40% token reduction compared to JSON, making it ideal for
//! LLM consumption of large datasets like a year's worth of fitness activities.
//!
//! ## Supported Formats
//!
//! - **JSON**: Default format, universal compatibility
//! - **TOON**: Token-efficient format optimized for LLM input
//!
//! ## Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::formatters::{OutputFormat, format_output};
//!
//! let activities = vec!["morning_run", "evening_ride"];
//! let format = OutputFormat::Toon;
//! if let Ok(output) = format_output(&activities, format) {
//!     println!("Formatted: {}", output.data);
//! }
//! ```

use serde::Serialize;
use std::{error::Error, fmt};
#[cfg(feature = "toon")]
use toon_format::EncodeOptions;
use tracing::debug;

/// Output serialization format selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// JSON format (default) - universal compatibility
    #[default]
    Json,
    /// TOON format - Token-Oriented Object Notation for LLM efficiency
    /// Achieves ~40% token reduction compared to JSON
    Toon,
}

impl OutputFormat {
    /// Parse format from string parameter (case-insensitive)
    /// Returns `Json` for unrecognized values (backwards compatible)
    #[must_use]
    pub fn from_str_param(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "toon" => Self::Toon,
            _ => Self::Json,
        }
    }

    /// Get the MIME content type for this format
    #[must_use]
    pub const fn content_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            // TOON doesn't have an official MIME type yet, use vendor prefix
            Self::Toon => "application/vnd.toon",
        }
    }

    /// Get the format name as a string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Toon => "toon",
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metrics for tracking token efficiency of output formatting
///
/// These metrics help understand the token savings achieved by using
/// different output formats, particularly TOON vs JSON.
#[derive(Debug, Clone, Serialize)]
pub struct TokenEfficiencyMetrics {
    /// The format used for the output
    pub format_used: String,
    /// Estimated tokens in the output (chars / 4 approximation)
    pub estimated_tokens: usize,
    /// Byte size of the output
    pub byte_size: usize,
    /// JSON equivalent byte size (for comparison)
    pub json_equivalent_size: usize,
    /// Token savings percentage compared to JSON (0-100)
    pub token_savings_percent: f64,
    /// Compression ratio (JSON size / actual size)
    pub compression_ratio: f64,
}

impl TokenEfficiencyMetrics {
    /// Calculate token efficiency metrics from formatted output
    ///
    /// Uses the common approximation of 4 characters per token for LLM tokenization.
    /// Calculates savings by comparing against JSON equivalent if TOON was used.
    #[must_use]
    pub fn calculate<T: Serialize>(data: &T, output: &FormattedOutput) -> Self {
        let byte_size = output.data.len();
        let estimated_tokens = Self::estimate_tokens(&output.data);

        // Calculate JSON equivalent size for comparison
        let json_equivalent_size = serde_json::to_string(data)
            .map(|s| s.len())
            .unwrap_or(byte_size);

        let (token_savings_percent, compression_ratio) =
            if json_equivalent_size > 0 && byte_size > 0 {
                let ratio = json_equivalent_size as f64 / byte_size as f64;
                let savings = ((json_equivalent_size as f64 - byte_size as f64)
                    / json_equivalent_size as f64)
                    * 100.0;
                (savings.max(0.0), ratio)
            } else {
                (0.0, 1.0)
            };

        Self {
            format_used: output.format.as_str().to_owned(),
            estimated_tokens,
            byte_size,
            json_equivalent_size,
            token_savings_percent,
            compression_ratio,
        }
    }

    /// Estimate token count from string using 4 chars per token approximation
    ///
    /// This is a simplified estimation. Actual tokenization varies by model
    /// but 4 chars/token is a reasonable average for English text and code.
    #[must_use]
    pub fn estimate_tokens(text: &str) -> usize {
        // Use character count / 4 as approximation, rounding up
        text.chars().count().div_ceil(4)
    }

    /// Log the efficiency metrics for telemetry
    pub fn log(&self, operation: &str) {
        debug!(
            operation = %operation,
            format_used = %self.format_used,
            estimated_tokens = %self.estimated_tokens,
            byte_size = %self.byte_size,
            json_equivalent_size = %self.json_equivalent_size,
            token_savings_percent = %format!("{:.1}", self.token_savings_percent),
            compression_ratio = %format!("{:.2}", self.compression_ratio),
            event_type = "token_efficiency",
            "Output format efficiency metrics"
        );
    }
}

/// Formatted output containing the serialized data and metadata
#[derive(Debug, Clone)]
pub struct FormattedOutput {
    /// The serialized data as a string
    pub data: String,
    /// The format used for serialization
    pub format: OutputFormat,
    /// The MIME content type
    pub content_type: &'static str,
}

impl FormattedOutput {
    /// Calculate token efficiency metrics for this output
    ///
    /// Requires the original data for JSON comparison calculation.
    #[must_use]
    pub fn calculate_efficiency<T: Serialize>(&self, original_data: &T) -> TokenEfficiencyMetrics {
        TokenEfficiencyMetrics::calculate(original_data, self)
    }

    /// Get the estimated token count for this output
    #[must_use]
    pub fn estimated_tokens(&self) -> usize {
        TokenEfficiencyMetrics::estimate_tokens(&self.data)
    }
}

/// Error type for formatting operations
#[derive(Debug, Clone)]
pub struct FormatError {
    /// Error message describing what went wrong
    pub message: String,
    /// The format that was being used when the error occurred
    pub format: OutputFormat,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Format error ({}): {}", self.format, self.message)
    }
}

impl Error for FormatError {}

/// Format serializable data to the specified output format
///
/// # Arguments
/// * `data` - Any serializable data structure
/// * `format` - The desired output format
///
/// # Returns
/// * `Ok(FormattedOutput)` - Successfully formatted data with metadata
/// * `Err(FormatError)` - Serialization failed
///
/// # Errors
/// Returns `FormatError` if:
/// - JSON serialization fails (for JSON format)
/// - Converting to JSON value fails (for TOON format)
/// - TOON encoding fails (for TOON format)
///
/// # Example
/// ```rust,no_run
/// use pierre_mcp_server::formatters::{format_output, OutputFormat};
///
/// let activities = vec!["activity1", "activity2"];
/// if let Ok(output) = format_output(&activities, OutputFormat::Toon) {
///     println!("Formatted as {}: {}", output.format, output.data);
/// }
/// ```
pub fn format_output<T: Serialize>(
    data: &T,
    format: OutputFormat,
) -> Result<FormattedOutput, FormatError> {
    let data = match format {
        OutputFormat::Json => serde_json::to_string(data).map_err(|e| FormatError {
            message: e.to_string(),
            format,
        })?,
        OutputFormat::Toon => encode_toon(data, format)?,
    };

    Ok(FormattedOutput {
        data,
        format,
        content_type: format.content_type(),
    })
}

/// Format serializable data with token efficiency telemetry
///
/// This function formats the data and logs token efficiency metrics for monitoring.
/// Use this variant when you want to track format efficiency for telemetry purposes.
///
/// # Arguments
/// * `data` - Any serializable data structure
/// * `format` - The desired output format
/// * `operation` - Name of the operation for telemetry labeling
///
/// # Returns
/// * `Ok((FormattedOutput, TokenEfficiencyMetrics))` - Formatted data with efficiency metrics
/// * `Err(FormatError)` - Serialization failed
///
/// # Errors
/// Returns `FormatError` if serialization fails for either format.
///
/// # Example
/// ```rust,no_run
/// use pierre_mcp_server::formatters::{format_output_with_telemetry, OutputFormat};
///
/// let activities = vec!["activity1", "activity2"];
/// if let Ok((output, metrics)) = format_output_with_telemetry(&activities, OutputFormat::Toon, "get_activities") {
///     println!("Saved {}% tokens vs JSON", metrics.token_savings_percent as u32);
/// }
/// ```
pub fn format_output_with_telemetry<T: Serialize>(
    data: &T,
    format: OutputFormat,
    operation: &str,
) -> Result<(FormattedOutput, TokenEfficiencyMetrics), FormatError> {
    let output = format_output(data, format)?;
    let metrics = output.calculate_efficiency(data);

    // Log telemetry
    metrics.log(operation);

    Ok((output, metrics))
}

/// Format serializable data to pretty-printed output (for debugging/display)
///
/// # Arguments
/// * `data` - Any serializable data structure
/// * `format` - The desired output format
///
/// # Returns
/// * `Ok(FormattedOutput)` - Successfully formatted data with metadata
/// * `Err(FormatError)` - Serialization failed
///
/// # Errors
/// Returns `FormatError` if:
/// - JSON serialization fails (for JSON format)
/// - Converting to JSON value fails (for TOON format)
/// - TOON encoding fails (for TOON format)
pub fn format_output_pretty<T: Serialize>(
    data: &T,
    format: OutputFormat,
) -> Result<FormattedOutput, FormatError> {
    let data = match format {
        OutputFormat::Json => serde_json::to_string_pretty(data).map_err(|e| FormatError {
            message: e.to_string(),
            format,
        })?,
        OutputFormat::Toon => encode_toon(data, format)?,
    };

    Ok(FormattedOutput {
        data,
        format,
        content_type: format.content_type(),
    })
}

/// Encode data to TOON format when the `toon` feature is enabled,
/// or fall back to JSON when disabled.
#[cfg(feature = "toon")]
fn encode_toon<T: Serialize>(data: &T, format: OutputFormat) -> Result<String, FormatError> {
    let value = serde_json::to_value(data).map_err(|e| FormatError {
        message: format!("Failed to convert to JSON value: {e}"),
        format,
    })?;
    let options = EncodeOptions::default();
    toon_format::encode(&value, &options).map_err(|e| FormatError {
        message: e.to_string(),
        format,
    })
}

/// Fallback: TOON feature disabled, serialize as JSON instead
#[cfg(not(feature = "toon"))]
fn encode_toon<T: Serialize>(data: &T, format: OutputFormat) -> Result<String, FormatError> {
    debug!("TOON format requested but toon feature is disabled, falling back to JSON");
    serde_json::to_string(data).map_err(|e| FormatError {
        message: e.to_string(),
        format,
    })
}
