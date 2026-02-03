---
name: sanitized-friendly
insight_type: achievement
sport_type: run
expected_verdict: valid
tier_behavior:
  starter: valid
  professional: valid
  enterprise: valid
tags: [sanitized-safe, achievement, metrics-present]
---

## Content
New half marathon PR today! Finished in 1:42:15, shaving 3 minutes off my previous best. Negative split the race - first half in 52:30, second half in 49:45. Heart rate averaged 165bpm, peaked at 178 in the final mile. The training block focused on tempo runs really paid off.

## Reason
Contains multiple metrics (times, heart rate) that would be redacted under Sanitized policy:
- "1:42:15" → "around 1:45 hours"
- "52:30" → "around 55 minutes"
- "49:45" → "around 50 minutes"
- "165bpm" → "tempo effort (Zone 3-4)"
- "178" → "high intensity (Zone 4-5)"

Under DataRich: passes as-is with full metrics
Under Sanitized: passes with metrics converted to ranges
Under GeneralOnly: would be REJECTED (contains metrics)
