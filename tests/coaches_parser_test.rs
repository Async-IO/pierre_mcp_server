// ABOUTME: Unit tests for coach markdown parser
// ABOUTME: Tests frontmatter parsing, section extraction, and token counting
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Test modules don't need documentation
#![allow(missing_docs)]
// Allow unwrap in tests - tests should panic on failure
#![allow(clippy::unwrap_used)]
// Allow raw string hashes for readability in test fixtures
#![allow(clippy::needless_raw_string_hashes)]

use pierre_mcp_server::coaches::{parse_frontmatter, parse_sections, RelationType};
use pierre_mcp_server::database::coaches::{CoachCategory, CoachVisibility};

const SAMPLE_COACH: &str = r#"---
name: test-coach
title: Test Coach
category: training
tags: [test, example]
prerequisites:
  providers: [strava]
  min_activities: 5
visibility: tenant
---

## Purpose
This is a test coach for unit testing the parser.

## When to Use
- When writing tests
- When validating parser functionality

## Instructions
You are a test coach. Help users with testing scenarios.

## Example Inputs
- "How do I test this?"
- "What should I verify?"

## Example Outputs
Provide clear test instructions with expected outcomes.

## Success Criteria
- Tests pass
- Parser works correctly

## Related Coaches
- other-coach (related)
- prereq-coach (prerequisite)
"#;

#[test]
fn test_parse_frontmatter_valid() {
    let result = parse_frontmatter(SAMPLE_COACH);
    assert!(result.is_ok());

    let frontmatter = result.unwrap();
    assert_eq!(frontmatter.name, "test-coach");
    assert_eq!(frontmatter.title, "Test Coach");
    assert_eq!(frontmatter.category, CoachCategory::Training);
    assert_eq!(frontmatter.tags, vec!["test", "example"]);
    assert_eq!(frontmatter.prerequisites.providers, vec!["strava"]);
    assert_eq!(frontmatter.prerequisites.min_activities, 5);
    assert_eq!(frontmatter.visibility, CoachVisibility::Tenant);
}

#[test]
fn test_parse_frontmatter_missing_delimiter() {
    let content = "name: test\ntitle: Test";
    let result = parse_frontmatter(content);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .message
        .contains("must start with YAML frontmatter"));
}

#[test]
fn test_parse_sections_valid() {
    let result = parse_sections(SAMPLE_COACH);
    assert!(result.is_ok());

    let sections = result.unwrap();
    assert!(sections.purpose.contains("test coach for unit testing"));
    assert!(sections.instructions.contains("You are a test coach"));
    assert!(sections.when_to_use.is_some());
    assert!(sections.example_inputs.is_some());
    assert!(sections.example_outputs.is_some());
    assert!(sections.success_criteria.is_some());
    assert_eq!(sections.related_coaches.len(), 2);
}

#[test]
fn test_parse_sections_missing_purpose() {
    let content = r#"---
name: test
title: Test
category: training
---

## Instructions
Some instructions.
"#;
    let result = parse_sections(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Purpose"));
}

#[test]
fn test_parse_sections_missing_instructions() {
    let content = r#"---
name: test
title: Test
category: training
---

## Purpose
Some purpose.
"#;
    let result = parse_sections(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Instructions"));
}

#[test]
fn test_parse_related_coaches() {
    let content = r#"---
name: rel-test
title: Related Test
category: training
---

## Purpose
Test related coaches parsing.

## Instructions
Test instructions.

## Related Coaches
- coach-a (related)
- coach-b (prerequisite)
- coach-c (sequel)
- coach-d (alternative)
- invalid-coach
"#;

    let sections = parse_sections(content).unwrap();
    let related = &sections.related_coaches;

    assert_eq!(related.len(), 4);
    assert_eq!(related[0].slug, "coach-a");
    assert_eq!(related[0].relation_type, RelationType::Related);
    assert_eq!(related[1].slug, "coach-b");
    assert_eq!(related[1].relation_type, RelationType::Prerequisite);
    assert_eq!(related[2].slug, "coach-c");
    assert_eq!(related[2].relation_type, RelationType::Sequel);
    assert_eq!(related[3].slug, "coach-d");
    assert_eq!(related[3].relation_type, RelationType::Alternative);
}

#[test]
fn test_token_count_calculation() {
    // Token count is calculated from counted sections only:
    // purpose, instructions, example_inputs, example_outputs, success_criteria
    // NOT counted: when_to_use, related_coaches
    let content = format!(
        r#"---
name: token-test
title: Token Test
category: training
---

## Purpose
{}

## When to Use
{}

## Instructions
{}

## Example Inputs
{}

## Example Outputs
{}

## Success Criteria
{}
"#,
        "A".repeat(100), // 100 chars - counted
        "F".repeat(100), // 100 chars - NOT counted
        "B".repeat(200), // 200 chars - counted
        "C".repeat(50),  // 50 chars - counted
        "D".repeat(50),  // 50 chars - counted
        "E".repeat(40),  // 40 chars - counted
    );

    let sections = parse_sections(&content).unwrap();

    // Verify section lengths
    assert_eq!(sections.purpose.len(), 100);
    assert_eq!(sections.instructions.len(), 200);
    assert_eq!(sections.example_inputs.as_ref().unwrap().len(), 50);
    assert_eq!(sections.example_outputs.as_ref().unwrap().len(), 50);
    assert_eq!(sections.success_criteria.as_ref().unwrap().len(), 40);
    assert_eq!(sections.when_to_use.as_ref().unwrap().len(), 100);

    // Total counted: 100 + 200 + 50 + 50 + 40 = 440 chars
    // Token estimate: 440 / 4 = 110 tokens
    // Note: This is tested via parse_coach_file which uses CoachDefinition::calculate_token_count
}

#[test]
fn test_category_mobility() {
    let content = r#"---
name: test-mobility
title: Test Mobility Coach
category: mobility
---

## Purpose
A mobility coach.

## Instructions
Help with mobility.
"#;
    let result = parse_frontmatter(content);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().category, CoachCategory::Mobility);
}

#[test]
fn test_default_prerequisites() {
    let content = r#"---
name: minimal
title: Minimal Coach
category: training
---

## Purpose
Minimal coach.

## Instructions
Basic instructions.
"#;
    let frontmatter = parse_frontmatter(content).unwrap();
    assert!(frontmatter.prerequisites.providers.is_empty());
    assert_eq!(frontmatter.prerequisites.min_activities, 0);
    assert!(frontmatter.prerequisites.activity_types.is_empty());
}

#[test]
fn test_default_visibility() {
    let content = r#"---
name: no-visibility
title: No Visibility Coach
category: training
---

## Purpose
Coach without visibility set.

## Instructions
Default visibility test.
"#;
    let frontmatter = parse_frontmatter(content).unwrap();
    // Default visibility should be Private per CoachVisibility default
    assert_eq!(frontmatter.visibility, CoachVisibility::Private);
}
