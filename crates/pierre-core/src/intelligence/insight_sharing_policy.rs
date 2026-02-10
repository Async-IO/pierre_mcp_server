// ABOUTME: Insight sharing policy enum for social feed privacy control
// ABOUTME: User-level policy controlling what fitness data can be shared

use serde::{Deserialize, Serialize};

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
