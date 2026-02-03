// ABOUTME: Parser for insight sample markdown files with YAML frontmatter
// ABOUTME: Extracts structured test data for insight validation testing
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult, ErrorCode};
use crate::intelligence::insight_validation::ValidationVerdict;
use crate::models::{InsightType, UserTier};

/// Expected behavior per user tier
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierBehavior {
    /// Expected verdict for Starter tier users
    #[serde(default)]
    pub starter: String,

    /// Expected verdict for Professional tier users
    #[serde(default)]
    pub professional: String,

    /// Expected verdict for Enterprise tier users
    #[serde(default)]
    pub enterprise: String,
}

impl TierBehavior {
    /// Get the expected verdict for a given user tier
    #[must_use]
    pub fn verdict_for_tier(&self, tier: &UserTier) -> &str {
        match tier {
            UserTier::Starter => &self.starter,
            UserTier::Professional => &self.professional,
            UserTier::Enterprise => &self.enterprise,
        }
    }

    /// Check if actual verdict matches expected for a tier
    #[must_use]
    pub fn matches_expected(&self, tier: &UserTier, actual: &ValidationVerdict) -> bool {
        let expected = self.verdict_for_tier(tier);
        match actual {
            ValidationVerdict::Valid => expected == "valid",
            ValidationVerdict::Improved { .. } => expected == "improved",
            ValidationVerdict::Rejected { .. } => expected == "rejected",
        }
    }
}

/// YAML frontmatter parsed from insight sample markdown file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightSampleFrontmatter {
    /// Unique identifier for this sample (kebab-case, matches filename)
    pub name: String,

    /// Type of insight being tested
    pub insight_type: InsightType,

    /// Sport type context (optional)
    #[serde(default)]
    pub sport_type: Option<String>,

    /// Primary expected verdict (valid, rejected, improved)
    pub expected_verdict: String,

    /// Expected behavior per user tier
    #[serde(default)]
    pub tier_behavior: TierBehavior,

    /// Searchable tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Markdown sections parsed from insight sample file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsightSampleSections {
    /// The actual insight content to be validated (Required)
    pub content: String,

    /// Explanation of why this insight should receive this verdict (Optional)
    pub reason: Option<String>,
}

/// Complete insight sample definition combining frontmatter and sections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightSampleDefinition {
    /// Parsed YAML frontmatter
    pub frontmatter: InsightSampleFrontmatter,

    /// Parsed markdown sections
    pub sections: InsightSampleSections,

    /// Source file path (relative to `insight_samples` directory)
    pub source_file: String,

    /// Content hash for change detection
    pub content_hash: String,
}

impl InsightSampleDefinition {
    /// Calculate SHA-256 hash of the file content
    fn calculate_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Get the content to validate
    #[must_use]
    pub fn content(&self) -> &str {
        &self.sections.content
    }

    /// Get the insight type
    #[must_use]
    pub fn insight_type(&self) -> InsightType {
        self.frontmatter.insight_type
    }

    /// Check if actual verdict matches expected for a tier
    #[must_use]
    pub fn matches_expected(&self, tier: &UserTier, actual: &ValidationVerdict) -> bool {
        self.frontmatter
            .tier_behavior
            .matches_expected(tier, actual)
    }
}

/// Parse YAML frontmatter from markdown content
///
/// # Errors
/// Returns error if frontmatter delimiters are missing or YAML is invalid
pub fn parse_frontmatter(content: &str) -> AppResult<InsightSampleFrontmatter> {
    let content = content.trim();

    if !content.starts_with("---") {
        return Err(AppError::new(
            ErrorCode::InvalidFormat,
            "Insight sample file must start with YAML frontmatter (---)",
        ));
    }

    let after_first = &content[3..];
    let end_pos = after_first.find("\n---").ok_or_else(|| {
        AppError::new(
            ErrorCode::InvalidFormat,
            "Insight sample file missing closing frontmatter delimiter (---)",
        )
    })?;

    let yaml_content = &after_first[..end_pos].trim();

    serde_yaml::from_str(yaml_content).map_err(|e| {
        AppError::new(
            ErrorCode::InvalidFormat,
            format!("Invalid YAML frontmatter: {e}"),
        )
    })
}

/// Parse markdown sections from content (after frontmatter)
///
/// # Errors
/// Returns error if required sections (Content) are missing
pub fn parse_sections(content: &str) -> AppResult<InsightSampleSections> {
    let content = content.trim();

    // Skip frontmatter to get to sections
    let body = content.strip_prefix("---").map_or(content, |after_first| {
        after_first
            .find("\n---")
            .map_or(content, |end_pos| after_first[end_pos + 4..].trim())
    });

    let mut sections = InsightSampleSections::default();

    // Parse sections by looking for ## headers
    let section_pattern = "## ";
    let mut current_section: Option<&str> = None;
    let mut current_content = String::new();

    for line in body.lines() {
        if let Some(header) = line.strip_prefix(section_pattern) {
            // Save previous section
            if let Some(section_name) = current_section {
                save_section(&mut sections, section_name, &current_content);
            }

            // Start new section
            current_section = Some(header.trim());
            current_content.clear();
        } else if current_section.is_some() {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    // Save final section
    if let Some(section_name) = current_section {
        save_section(&mut sections, section_name, &current_content);
    }

    // Validate required sections
    if sections.content.is_empty() {
        return Err(AppError::new(
            ErrorCode::MissingRequiredField,
            "Insight sample file missing required section: ## Content",
        ));
    }

    Ok(sections)
}

/// Save parsed content to the appropriate section field
fn save_section(sections: &mut InsightSampleSections, name: &str, content: &str) {
    let trimmed = content.trim();

    match name {
        "Content" => trimmed.clone_into(&mut sections.content),
        "Reason" => sections.reason = Some(trimmed.to_owned()),
        _ => {
            // Unknown section - ignore silently for forward compatibility
        }
    }
}

/// Parse a complete insight sample markdown file
///
/// # Arguments
/// * `path` - Path to the insight sample markdown file
///
/// # Errors
/// Returns error if file cannot be read, or content is invalid
pub fn parse_insight_sample_file(path: &Path) -> AppResult<InsightSampleDefinition> {
    let content = fs::read_to_string(path).map_err(|e| {
        AppError::new(
            ErrorCode::StorageError,
            format!("Failed to read insight sample file {}: {e}", path.display()),
        )
    })?;

    let frontmatter = parse_frontmatter(&content)?;
    let sections = parse_sections(&content)?;

    // Validate that name matches filename
    let filename = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        AppError::new(ErrorCode::InvalidFormat, "Invalid insight sample filename")
    })?;

    if frontmatter.name != filename {
        return Err(AppError::new(
            ErrorCode::InvalidFormat,
            format!(
                "Insight sample name '{}' does not match filename '{}'",
                frontmatter.name, filename
            ),
        ));
    }

    let content_hash = InsightSampleDefinition::calculate_hash(&content);

    // Get relative source path (category/filename.md)
    let source_file = path
        .iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    Ok(InsightSampleDefinition {
        frontmatter,
        sections,
        source_file,
        content_hash,
    })
}

/// Parse insight sample definition from markdown string content
///
/// # Arguments
/// * `content` - Markdown content with YAML frontmatter
/// * `source_name` - Optional source identifier for the content
///
/// # Errors
/// Returns error if content is invalid
pub fn parse_insight_sample_content(
    content: &str,
    source_name: Option<&str>,
) -> AppResult<InsightSampleDefinition> {
    let frontmatter = parse_frontmatter(content)?;
    let sections = parse_sections(content)?;

    let content_hash = InsightSampleDefinition::calculate_hash(content);

    let source_file = source_name.map_or_else(
        || format!("imported/{}.md", frontmatter.name),
        ToString::to_string,
    );

    Ok(InsightSampleDefinition {
        frontmatter,
        sections,
        source_file,
        content_hash,
    })
}
