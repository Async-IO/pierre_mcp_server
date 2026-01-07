// ABOUTME: Logging and PII redaction configuration types
// ABOUTME: Handles log redaction features with custom serialization for bitflags
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::middleware::redaction::RedactionFeatures;
use serde::{Deserialize, Serialize};
use std::env;

/// Logging and PII redaction configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Enable PII redaction in logs (default: true in production, false in dev)
    pub redact_pii: bool,
    /// Which redaction features to enable (headers, body fields, emails)
    pub redaction_features: RedactionFeatures,
    /// Placeholder for redacted sensitive data
    pub redaction_placeholder: String,
    /// Enable sampling for debug-level logs (reduces log volume)
    pub debug_sampling_enabled: bool,
    /// Debug log sampling rate (1.0 = all logs, 0.1 = 10% of logs)
    pub debug_sampling_rate: f64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            redact_pii: true, // Enabled by default for safety
            redaction_features: RedactionFeatures::ALL,
            redaction_placeholder: "[REDACTED]".to_owned(),
            debug_sampling_enabled: false,
            debug_sampling_rate: 1.0,
        }
    }
}

impl LoggingConfig {
    /// Load logging and PII redaction configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        let redact_pii = env_var_or("PIERRE_LOG_REDACT", "true")
            .parse()
            .unwrap_or(true);

        // Build redaction features bitflags from environment variables
        let mut features = RedactionFeatures::empty();
        if env_var_or("PIERRE_LOG_REDACT_HEADERS", "true")
            .parse()
            .unwrap_or(true)
        {
            features |= RedactionFeatures::HEADERS;
        }
        if env_var_or("PIERRE_LOG_REDACT_BODY", "true")
            .parse()
            .unwrap_or(true)
        {
            features |= RedactionFeatures::BODY_FIELDS;
        }
        if env_var_or("PIERRE_LOG_MASK_EMAILS", "true")
            .parse()
            .unwrap_or(true)
        {
            features |= RedactionFeatures::EMAILS;
        }

        Self {
            redact_pii,
            redaction_features: features,
            redaction_placeholder: env_var_or("PIERRE_REDACTION_PLACEHOLDER", "[REDACTED]"),
            debug_sampling_enabled: env_var_or("PIERRE_LOG_SAMPLE_ENABLED", "false")
                .parse()
                .unwrap_or(false),
            debug_sampling_rate: env_var_or("PIERRE_LOG_SAMPLE_RATE_DEBUG", "1.0")
                .parse()
                .unwrap_or(1.0),
        }
    }
}

// Custom Serialize/Deserialize to handle bitflags as individual bool fields
impl Serialize for LoggingConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("LoggingConfig", 7)?;
        state.serialize_field("redact_pii", &self.redact_pii)?;
        state.serialize_field(
            "redact_headers",
            &self.redaction_features.contains(RedactionFeatures::HEADERS),
        )?;
        state.serialize_field(
            "redact_body_fields",
            &self
                .redaction_features
                .contains(RedactionFeatures::BODY_FIELDS),
        )?;
        state.serialize_field(
            "mask_emails",
            &self.redaction_features.contains(RedactionFeatures::EMAILS),
        )?;
        state.serialize_field("redaction_placeholder", &self.redaction_placeholder)?;
        state.serialize_field("debug_sampling_enabled", &self.debug_sampling_enabled)?;
        state.serialize_field("debug_sampling_rate", &self.debug_sampling_rate)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for LoggingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, IgnoredAny, MapAccess, Visitor};
        use std::fmt;

        struct LoggingConfigVisitor;

        impl<'de> Visitor<'de> for LoggingConfigVisitor {
            type Value = LoggingConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct LoggingConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<LoggingConfig, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut redact_pii = None;
                let mut redact_headers = None;
                let mut redact_body_fields = None;
                let mut mask_emails = None;
                let mut redaction_placeholder = None;
                let mut debug_sampling_enabled = None;
                let mut debug_sampling_rate = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "redact_pii" => {
                            redact_pii = Some(map.next_value()?);
                        }
                        "redact_headers" => {
                            redact_headers = Some(map.next_value()?);
                        }
                        "redact_body_fields" => {
                            redact_body_fields = Some(map.next_value()?);
                        }
                        "mask_emails" => {
                            mask_emails = Some(map.next_value()?);
                        }
                        "redaction_placeholder" => {
                            redaction_placeholder = Some(map.next_value()?);
                        }
                        "debug_sampling_enabled" => {
                            debug_sampling_enabled = Some(map.next_value()?);
                        }
                        "debug_sampling_rate" => {
                            debug_sampling_rate = Some(map.next_value()?);
                        }
                        _ => {
                            let _: IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let redact_pii =
                    redact_pii.ok_or_else(|| de::Error::missing_field("redact_pii"))?;
                let redact_headers = redact_headers.unwrap_or(true);
                let redact_body_fields = redact_body_fields.unwrap_or(true);
                let mask_emails = mask_emails.unwrap_or(true);
                let redaction_placeholder =
                    redaction_placeholder.unwrap_or_else(|| "[REDACTED]".to_owned());
                let debug_sampling_enabled = debug_sampling_enabled.unwrap_or(false);
                let debug_sampling_rate = debug_sampling_rate.unwrap_or(1.0);

                let mut features = RedactionFeatures::empty();
                if redact_headers {
                    features |= RedactionFeatures::HEADERS;
                }
                if redact_body_fields {
                    features |= RedactionFeatures::BODY_FIELDS;
                }
                if mask_emails {
                    features |= RedactionFeatures::EMAILS;
                }

                Ok(LoggingConfig {
                    redact_pii,
                    redaction_features: features,
                    redaction_placeholder,
                    debug_sampling_enabled,
                    debug_sampling_rate,
                })
            }
        }

        const FIELDS: &[&str] = &[
            "redact_pii",
            "redact_headers",
            "redact_body_fields",
            "mask_emails",
            "redaction_placeholder",
            "debug_sampling_enabled",
            "debug_sampling_rate",
        ];
        deserializer.deserialize_struct("LoggingConfig", FIELDS, LoggingConfigVisitor)
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
