# Insight Validation Test Samples

This directory contains sample fitness insights used for testing the insight quality validation system before content is shared to the social feed.

## Directory Structure

```
insight_samples/
├── valid/       # Insights that should pass validation as-is
├── invalid/     # Insights that should be rejected (generic content, no data)
└── improvable/  # Insights that have potential but can be enhanced (premium tiers)
```

## File Format

Each insight is defined in a markdown file with YAML frontmatter:

```markdown
---
name: unique-insight-id
insight_type: achievement | milestone | training_tip | recovery | motivation | coaching_insight
sport_type: run | ride | swim | strength | null
expected_verdict: valid | rejected | improved
tier_behavior:
  starter: valid | rejected
  professional: valid | improved | rejected
  enterprise: valid | improved | rejected
tags: [specific, data-driven, achievement]
---

## Content
The actual insight content that would be shared to the social feed.

## Reason
Explanation of why this insight should receive this verdict (for documentation/testing).
```

## Adding New Test Cases

1. Choose the appropriate directory based on expected verdict
2. Create a markdown file with descriptive name (e.g., `10k-pr-with-splits.md`)
3. Fill in the YAML frontmatter with metadata
4. Add the content and reasoning

## Running Validation Tests

```bash
# Run the insight validation seeder to test all samples
cargo run --bin seed-insight-samples -- --validate

# Test against a specific tier
cargo run --bin seed-insight-samples -- --validate --tier professional
```
