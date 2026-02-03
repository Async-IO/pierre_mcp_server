// ABOUTME: Parser for coach markdown files with YAML frontmatter
// ABOUTME: Extracts structured data from Claude Skills-style coach definitions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::database::coaches::{CoachCategory, CoachVisibility};
use crate::errors::{AppError, AppResult, ErrorCode};

/// Token estimation constant: average characters per token for system prompts
const CHARS_PER_TOKEN: usize = 4;

/// Prerequisites required to use a coach
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoachPrerequisites {
    /// Required OAuth providers (e.g., strava, garmin)
    #[serde(default)]
    pub providers: Vec<String>,

    /// Minimum number of activities required
    #[serde(default)]
    pub min_activities: u32,

    /// Required activity types (e.g., Run, Ride, Swim)
    #[serde(default)]
    pub activity_types: Vec<String>,
}

/// Type of relationship between coaches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// General relationship (bidirectional)
    Related,
    /// Alternative coach for similar needs (bidirectional)
    Alternative,
    /// Must consult before this coach (directional)
    Prerequisite,
    /// Consult after this coach (directional)
    Sequel,
}

impl RelationType {
    /// Parse relation type from string
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "related" => Some(Self::Related),
            "alternative" => Some(Self::Alternative),
            "prerequisite" => Some(Self::Prerequisite),
            "sequel" => Some(Self::Sequel),
            _ => None,
        }
    }
}

/// A related coach reference parsed from markdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedCoach {
    /// Slug of the related coach
    pub slug: String,

    /// Type of relationship
    pub relation_type: RelationType,
}

/// YAML frontmatter parsed from coach markdown file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachFrontmatter {
    /// Unique slug identifier (kebab-case, matches filename)
    pub name: String,

    /// Display name for the coach
    pub title: String,

    /// Category for organization
    pub category: CoachCategory,

    /// Searchable tags
    #[serde(default)]
    pub tags: Vec<String>,

    /// Prerequisites to use this coach
    #[serde(default)]
    pub prerequisites: CoachPrerequisites,

    /// Access level (defaults to tenant)
    #[serde(default)]
    pub visibility: CoachVisibility,
}

/// Markdown sections parsed from coach file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoachSections {
    /// One paragraph describing the coach's purpose (Required)
    pub purpose: String,

    /// When to use this coach (Optional, not counted in tokens)
    pub when_to_use: Option<String>,

    /// Core AI system prompt instructions (Required)
    pub instructions: String,

    /// Example questions users might ask (Optional)
    pub example_inputs: Option<String>,

    /// Description of response style (Optional)
    pub example_outputs: Option<String>,

    /// What defines success (Optional)
    pub success_criteria: Option<String>,

    /// Related coaches with relationship types (Optional, not counted in tokens)
    pub related_coaches: Vec<RelatedCoach>,
}

/// Complete coach definition combining frontmatter and sections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachDefinition {
    /// Parsed YAML frontmatter
    pub frontmatter: CoachFrontmatter,

    /// Parsed markdown sections
    pub sections: CoachSections,

    /// Source file path (relative to coaches directory)
    pub source_file: String,

    /// Content hash for change detection
    pub content_hash: String,

    /// Estimated token count for sections that count toward budget
    pub token_count: u32,
}

impl CoachDefinition {
    /// Calculate token count from sections that count toward budget
    ///
    /// Counted: `purpose`, `instructions`, `example_inputs`, `example_outputs`, `success_criteria`
    /// Not counted: `when_to_use`, `prerequisites`, `related_coaches`
    fn calculate_token_count(sections: &CoachSections) -> u32 {
        let total_chars = sections.purpose.len()
            + sections.instructions.len()
            + sections.example_inputs.as_ref().map_or(0, String::len)
            + sections.example_outputs.as_ref().map_or(0, String::len)
            + sections.success_criteria.as_ref().map_or(0, String::len);

        #[allow(clippy::cast_possible_truncation)]
        let token_count = (total_chars / CHARS_PER_TOKEN) as u32;
        token_count
    }

    /// Calculate SHA-256 hash of the file content
    fn calculate_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

/// Parse YAML frontmatter from markdown content
///
/// # Errors
/// Returns error if frontmatter delimiters are missing or YAML is invalid
pub fn parse_frontmatter(content: &str) -> AppResult<CoachFrontmatter> {
    let content = content.trim();

    if !content.starts_with("---") {
        return Err(AppError::new(
            ErrorCode::InvalidFormat,
            "Coach file must start with YAML frontmatter (---)",
        ));
    }

    let after_first = &content[3..];
    let end_pos = after_first.find("\n---").ok_or_else(|| {
        AppError::new(
            ErrorCode::InvalidFormat,
            "Coach file missing closing frontmatter delimiter (---)",
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
/// Returns error if required sections (Purpose, Instructions) are missing
pub fn parse_sections(content: &str) -> AppResult<CoachSections> {
    let content = content.trim();

    // Skip frontmatter to get to sections
    let body = content.strip_prefix("---").map_or(content, |after_first| {
        after_first
            .find("\n---")
            .map_or(content, |end_pos| after_first[end_pos + 4..].trim())
    });

    let mut sections = CoachSections::default();

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
    if sections.purpose.is_empty() {
        return Err(AppError::new(
            ErrorCode::MissingRequiredField,
            "Coach file missing required section: ## Purpose",
        ));
    }
    if sections.instructions.is_empty() {
        return Err(AppError::new(
            ErrorCode::MissingRequiredField,
            "Coach file missing required section: ## Instructions",
        ));
    }

    Ok(sections)
}

/// Save parsed content to the appropriate section field
fn save_section(sections: &mut CoachSections, name: &str, content: &str) {
    let trimmed = content.trim();

    match name {
        "Purpose" => trimmed.clone_into(&mut sections.purpose),
        "When to Use" => sections.when_to_use = Some(trimmed.to_owned()),
        "Instructions" => trimmed.clone_into(&mut sections.instructions),
        "Example Inputs" => sections.example_inputs = Some(trimmed.to_owned()),
        "Example Outputs" => sections.example_outputs = Some(trimmed.to_owned()),
        "Success Criteria" => sections.success_criteria = Some(trimmed.to_owned()),
        "Related Coaches" => {
            sections.related_coaches = parse_related_coaches(trimmed);
        }
        _ => {
            // Unknown section - ignore silently for forward compatibility
        }
    }
}

/// Parse related coaches from markdown list
/// Format: `- coach-slug (relation_type)`
fn parse_related_coaches(content: &str) -> Vec<RelatedCoach> {
    let mut related = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with('-') {
            continue;
        }

        let item = line[1..].trim();

        // Parse "coach-slug (relation_type)" format
        if let Some(paren_start) = item.find('(') {
            if let Some(paren_end) = item.find(')') {
                let slug = item[..paren_start].trim().to_owned();
                let relation_str = item[paren_start + 1..paren_end].trim();

                if let Some(relation_type) = RelationType::parse(relation_str) {
                    related.push(RelatedCoach {
                        slug,
                        relation_type,
                    });
                }
            }
        }
    }

    related
}

/// Parse a complete coach markdown file
///
/// # Arguments
/// * `path` - Path to the coach markdown file
///
/// # Errors
/// Returns error if file cannot be read, or content is invalid
pub fn parse_coach_file(path: &Path) -> AppResult<CoachDefinition> {
    let content = fs::read_to_string(path).map_err(|e| {
        AppError::new(
            ErrorCode::StorageError,
            format!("Failed to read coach file {}: {e}", path.display()),
        )
    })?;

    let frontmatter = parse_frontmatter(&content)?;
    let sections = parse_sections(&content)?;

    // Validate that name matches filename
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::new(ErrorCode::InvalidFormat, "Invalid coach filename"))?;

    if frontmatter.name != filename {
        return Err(AppError::new(
            ErrorCode::InvalidFormat,
            format!(
                "Coach name '{}' does not match filename '{}'",
                frontmatter.name, filename
            ),
        ));
    }

    let token_count = CoachDefinition::calculate_token_count(&sections);
    let content_hash = CoachDefinition::calculate_hash(&content);

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

    Ok(CoachDefinition {
        frontmatter,
        sections,
        source_file,
        content_hash,
        token_count,
    })
}

/// Parse coach definition from markdown string content
///
/// # Arguments
/// * `content` - Markdown content with YAML frontmatter
/// * `source_name` - Optional source identifier for the content
///
/// # Errors
/// Returns error if content is invalid
pub fn parse_coach_content(content: &str, source_name: Option<&str>) -> AppResult<CoachDefinition> {
    let frontmatter = parse_frontmatter(content)?;
    let sections = parse_sections(content)?;

    let token_count = CoachDefinition::calculate_token_count(&sections);
    let content_hash = CoachDefinition::calculate_hash(content);

    let source_file = source_name.map_or_else(
        || format!("imported/{}.md", frontmatter.name),
        ToString::to_string,
    );

    Ok(CoachDefinition {
        frontmatter,
        sections,
        source_file,
        content_hash,
        token_count,
    })
}

/// Convert coach definition to markdown format
///
/// Generates markdown with YAML frontmatter and structured sections
#[must_use]
pub fn to_markdown(definition: &CoachDefinition) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    // YAML frontmatter
    output.push_str("---\n");
    let _ = writeln!(output, "name: {}", definition.frontmatter.name);
    let _ = writeln!(output, "title: {}", definition.frontmatter.title);
    let _ = writeln!(
        output,
        "category: {}",
        definition.frontmatter.category.as_str()
    );

    // Tags as YAML array
    if !definition.frontmatter.tags.is_empty() {
        output.push_str("tags: [");
        output.push_str(&definition.frontmatter.tags.join(", "));
        output.push_str("]\n");
    }

    // Prerequisites
    let prereqs = &definition.frontmatter.prerequisites;
    if !prereqs.providers.is_empty()
        || prereqs.min_activities > 0
        || !prereqs.activity_types.is_empty()
    {
        output.push_str("prerequisites:\n");
        if !prereqs.providers.is_empty() {
            output.push_str("  providers: [");
            output.push_str(&prereqs.providers.join(", "));
            output.push_str("]\n");
        }
        if prereqs.min_activities > 0 {
            let _ = writeln!(output, "  min_activities: {}", prereqs.min_activities);
        }
        if !prereqs.activity_types.is_empty() {
            output.push_str("  activity_types: [");
            output.push_str(&prereqs.activity_types.join(", "));
            output.push_str("]\n");
        }
    }

    // Visibility (only if not default)
    if definition.frontmatter.visibility != CoachVisibility::Tenant {
        let _ = writeln!(
            output,
            "visibility: {}",
            definition.frontmatter.visibility.as_str()
        );
    }

    output.push_str("---\n\n");

    // Sections
    output.push_str("## Purpose\n\n");
    output.push_str(&definition.sections.purpose);
    output.push_str("\n\n");

    if let Some(when_to_use) = &definition.sections.when_to_use {
        output.push_str("## When to Use\n\n");
        output.push_str(when_to_use);
        output.push_str("\n\n");
    }

    output.push_str("## Instructions\n\n");
    output.push_str(&definition.sections.instructions);
    output.push_str("\n\n");

    if let Some(example_inputs) = &definition.sections.example_inputs {
        output.push_str("## Example Inputs\n\n");
        output.push_str(example_inputs);
        output.push_str("\n\n");
    }

    if let Some(example_outputs) = &definition.sections.example_outputs {
        output.push_str("## Example Outputs\n\n");
        output.push_str(example_outputs);
        output.push_str("\n\n");
    }

    if let Some(success_criteria) = &definition.sections.success_criteria {
        output.push_str("## Success Criteria\n\n");
        output.push_str(success_criteria);
        output.push_str("\n\n");
    }

    if !definition.sections.related_coaches.is_empty() {
        output.push_str("## Related Coaches\n\n");
        for related in &definition.sections.related_coaches {
            let _ = writeln!(output, "- {} ({:?})", related.slug, related.relation_type);
        }
        output.push('\n');
    }

    output
}
