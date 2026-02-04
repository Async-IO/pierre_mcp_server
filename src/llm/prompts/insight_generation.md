You are transforming a fitness analysis into a shareable social post.

## Task

Convert the provided analysis into an inspiring, concise message suitable for sharing on social media (Strava, Instagram, fitness communities).

## Output Format

Return ONLY valid JSON with this structure:

```json
{
  "content": "The shareable insight text ready to copy and paste"
}
```

**CRITICAL: Return ONLY the JSON object. No text before or after it.**

## Content Guidelines

1. **Keep the key insights and data points** from the original analysis
2. **Make it inspiring and personal** - speak from the athlete's perspective
3. **Include 3-5 relevant hashtags** at the end of the content
4. **Keep it concise** - ideal for social sharing (under 280 characters if possible, max 500)
5. **Maintain accuracy** - don't invent metrics that weren't in the original
6. **NO introduction or preamble** - never start with "Here is..." or "Here's..."

## Example

Input analysis:
> Your training load is balanced. You're in a building base phase with low overtraining risk. Weekly TSS increased from 189 to 2195. Consider a light taper for upcoming events.

Output:
```json
{
  "content": "Building my aerobic base one week at a time! TSS jumped from 189 to 2195 while keeping overtraining risk low. Consistency is paying off - time to consider a light taper before race day. #MarathonTraining #EnduranceAthlete #ConsistencyWins #BaseBuilding"
}
```

## Analysis to Transform

