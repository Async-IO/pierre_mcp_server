# Pierre Coaches

This directory contains coach definitions in markdown format. Each coach file defines an AI coaching persona for the Pierre Fitness platform.

## Directory Structure

```
coaches/
├── README.md                    # This file
├── training/                    # Training and race preparation coaches
├── nutrition/                   # Nutrition and fueling coaches
├── recovery/                    # Sleep and recovery coaches
└── mobility/                    # Mobility, stretching, and flexibility coaches
```

## Categories

| Category | Description |
|----------|-------------|
| training | Race preparation, speed work, endurance building |
| nutrition | Pre/post workout fueling, race day nutrition |
| recovery | Sleep optimization, rest days, overtraining prevention |
| mobility | Stretching, flexibility, warm-ups, injury prevention |
| recipes | Meal planning and recipe generation |
| custom | User-defined coaches |

## Coach File Format

Each coach is defined in a markdown file with YAML frontmatter followed by structured sections.

### Frontmatter

```yaml
---
name: marathon-coach
title: Marathon Training Coach
category: training
tags: [running, marathon, endurance, long-runs, race-strategy]
prerequisites:
  providers: [strava]
  min_activities: 10
  activity_types: [Run]
visibility: tenant
---
```

**Required fields:**
- `name`: Unique slug identifier (kebab-case, matches filename without .md)
- `title`: Display name for the coach
- `category`: One of: training, nutrition, recovery, mobility, recipes, custom

**Optional fields:**
- `tags`: Array of searchable tags
- `prerequisites`: Requirements to use this coach
  - `providers`: Required OAuth providers (e.g., strava, garmin)
  - `min_activities`: Minimum number of activities required
  - `activity_types`: Required activity types (e.g., Run, Ride, Swim)
- `visibility`: Access level (tenant, public, private)

### Sections

Each coach file contains these sections:

#### Purpose (Required)
One paragraph describing what this coach does and its expertise area. This becomes the coach's description in the UI.

#### When to Use (Optional)
Bullet list of scenarios when a user should consult this coach. Not counted in token budget.

#### Instructions (Required)
The core AI system prompt. This is what makes the coach unique. Write in second person ("You are a..."). Include:
- Expertise areas
- Specific knowledge domains
- How to interact with users
- What questions to ask
- What advice to give

#### Example Inputs (Optional)
Sample questions or prompts users might ask this coach.

#### Example Outputs (Optional)
Description of the response style and format.

#### Success Criteria (Optional)
What defines a successful coaching interaction.

#### Related Coaches (Optional)
Links to other coaches with relationship type:
- `related`: General relationship (bidirectional)
- `alternative`: Alternative coach for similar needs (bidirectional)
- `prerequisite`: Must consult before this coach (directional)
- `sequel`: Consult after this coach (directional)

Example:
```markdown
## Related Coaches
- half-marathon-coach (related)
- 5k-speed-coach (prerequisite)
```

## Token Counting

The following sections are counted toward the token budget:
- Purpose
- Instructions
- Example Inputs
- Example Outputs
- Success Criteria

The following sections are NOT counted:
- When to Use
- Prerequisites (frontmatter)
- Related Coaches

## Creating a New Coach

1. Choose the appropriate category directory
2. Create a file named `{slug}-coach.md`
3. Add frontmatter with required fields
4. Write the Purpose section
5. Write the Instructions section (most important)
6. Add optional sections as needed
7. Run `cargo run --bin seed-coaches` to load into database

## Example Coach File

```markdown
---
name: example-coach
title: Example Coach
category: training
tags: [example]
visibility: tenant
---

## Purpose
Brief description of what this coach specializes in.

## When to Use
- Scenario 1
- Scenario 2

## Instructions
You are an expert coach specializing in [area]. Your expertise includes:
- Skill 1
- Skill 2

When giving advice, always ask about [relevant context].

## Example Inputs
- "How do I improve my [X]?"
- "What's the best way to [Y]?"

## Example Outputs
Provide specific, actionable advice with concrete numbers and timelines.

## Success Criteria
- User receives personalized advice
- Recommendations are based on their data

## Related Coaches
- related-coach (related)
```
