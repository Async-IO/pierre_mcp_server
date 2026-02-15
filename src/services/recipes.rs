// ABOUTME: Recipe and coach export/import business logic extracted from route handlers
// ABOUTME: Handles markdown conversion, filename generation, and diff computation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::coaches::{
    CoachDefinition, CoachFrontmatter, CoachPrerequisites, CoachSections, CoachStartup,
};
use crate::database::coaches::Coach;

/// Convert a Coach database model to `CoachDefinition` for export
///
/// Transforms the stored coach data into the markdown-exportable format
/// used for coach file interchange.
#[must_use]
pub fn coach_to_definition(coach: &Coach) -> CoachDefinition {
    let name = coach
        .title
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    CoachDefinition {
        frontmatter: CoachFrontmatter {
            name,
            title: coach.title.clone(),
            category: coach.category,
            tags: coach.tags.clone(),
            prerequisites: CoachPrerequisites::default(),
            visibility: coach.visibility,
            startup: CoachStartup::default(),
        },
        sections: CoachSections {
            purpose: coach.description.clone().unwrap_or_default(),
            when_to_use: None,
            instructions: coach.system_prompt.clone(),
            example_inputs: if coach.sample_prompts.is_empty() {
                None
            } else {
                Some(
                    coach
                        .sample_prompts
                        .iter()
                        .map(|p| format!("- {p}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            },
            example_outputs: None,
            success_criteria: None,
            related_coaches: Vec::new(),
        },
        source_file: format!("exported/{}.md", coach.id),
        content_hash: String::new(),
        token_count: coach.token_count,
    }
}

/// Generate a safe filename from coach title for markdown export
///
/// Converts to lowercase, replaces spaces with hyphens, and strips
/// non-alphanumeric characters (except hyphens).
#[must_use]
pub fn generate_coach_filename(title: &str) -> String {
    let safe_name: String = title
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();

    format!("{safe_name}.md")
}

/// A field-level change between two coach version snapshots
#[derive(Debug)]
pub struct FieldChange {
    /// Name of the changed field
    pub field: String,
    /// Previous value (None if field was added)
    pub old_value: Option<serde_json::Value>,
    /// New value (None if field was removed)
    pub new_value: Option<serde_json::Value>,
}

/// Compute field-level differences between two JSON coach version snapshots
///
/// Compares specific coach fields (title, description, `system_prompt`, category,
/// tags, `sample_prompts`, visibility) and returns a list of changes.
#[must_use]
pub fn compute_version_diff(from: &serde_json::Value, to: &serde_json::Value) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    let fields = [
        "title",
        "description",
        "system_prompt",
        "category",
        "tags",
        "sample_prompts",
        "visibility",
    ];

    for field in fields {
        let old_val = from.get(field);
        let new_val = to.get(field);

        match (old_val, new_val) {
            (Some(old), Some(new)) if old != new => {
                changes.push(FieldChange {
                    field: field.to_owned(),
                    old_value: Some(old.clone()),
                    new_value: Some(new.clone()),
                });
            }
            (None, Some(new)) => {
                changes.push(FieldChange {
                    field: field.to_owned(),
                    old_value: None,
                    new_value: Some(new.clone()),
                });
            }
            (Some(old), None) => {
                changes.push(FieldChange {
                    field: field.to_owned(),
                    old_value: Some(old.clone()),
                    new_value: None,
                });
            }
            _ => {}
        }
    }

    changes
}
