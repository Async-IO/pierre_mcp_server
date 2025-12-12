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
//! ```rust,ignore
//! use pierre_mcp_server::formatters::{OutputFormat, format_output};
//!
//! let activities = vec![/* ... */];
//! let format = OutputFormat::Toon;
//! let output = format_output(&activities, format)?;
//! ```

use serde::Serialize;
use std::fmt;

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

impl std::error::Error for FormatError {}

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
        OutputFormat::Toon => {
            // Convert to serde_json::Value first, then to TOON
            let value = serde_json::to_value(data).map_err(|e| FormatError {
                message: format!("Failed to convert to JSON value: {e}"),
                format,
            })?;
            let options = toon_format::EncodeOptions::default();
            toon_format::encode(&value, &options).map_err(|e| FormatError {
                message: e.to_string(),
                format,
            })?
        }
    };

    Ok(FormattedOutput {
        data,
        format,
        content_type: format.content_type(),
    })
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
        OutputFormat::Toon => {
            // TOON is already human-readable, use standard formatting
            let value = serde_json::to_value(data).map_err(|e| FormatError {
                message: format!("Failed to convert to JSON value: {e}"),
                format,
            })?;
            let options = toon_format::EncodeOptions::default();
            toon_format::encode(&value, &options).map_err(|e| FormatError {
                message: e.to_string(),
                format,
            })?
        }
    };

    Ok(FormattedOutput {
        data,
        format,
        content_type: format.content_type(),
    })
}
