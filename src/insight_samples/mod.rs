// ABOUTME: Insight sample parsing from markdown files for validation testing
// ABOUTME: Converts markdown with YAML frontmatter to structured insight test definitions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Insight Sample Parser
//!
//! This module provides parsing functionality for insight sample markdown files.
//! Insights are defined in markdown format with YAML frontmatter and structured sections,
//! following the same format as coach definitions.
//!
//! ## File Format
//!
//! ```markdown
//! ---
//! name: 10k-pr-with-splits
//! insight_type: achievement
//! sport_type: run
//! expected_verdict: valid
//! tier_behavior:
//!   starter: valid
//!   professional: valid
//!   enterprise: valid
//! tags: [specific, data-driven]
//! ---
//!
//! ## Content
//! The actual insight content that would be shared.
//!
//! ## Reason
//! Why this insight should receive this verdict.
//! ```

/// Insight sample markdown file parser with frontmatter and section extraction
pub mod parser;

pub use parser::{
    parse_insight_sample_content, parse_insight_sample_file, InsightSampleDefinition,
    InsightSampleFrontmatter, InsightSampleSections, TierBehavior,
};
