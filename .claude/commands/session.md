---
description: Session Management Skill
---

# Session Management

Manage Linear session tracking for Claude Code sessions. Sessions provide persistent memory across conversations.

## Usage

This command is invoked with `/session` and optional arguments:
- `/session` or `/session status` - Show current session status and recent activity
- `/session update <notes>` - Add work log entry to session issue
- `/session decision <decision>` - Document a key decision made
- `/session link <issue-id>` - Link a related issue to this session
- `/session end` - Add end-of-session summary with notes for next session

## Context

The SessionStart hook automatically creates or resumes a Linear session issue. The hook output shows:
```
📋 Linear Session RESUMED: ASY-XXX (branch) - https://linear.app/...
```

This command is for **updating** that session with meaningful content during work.

## Instructions

### On `/session` or `/session status`

1. Get the current session issue ID from the most recent hook output (shown at session start)
2. Use `mcp__linear-server__get_issue` to fetch the session issue details
3. Use `mcp__linear-server__list_comments` to get recent activity
4. Display a summary:
   - Session issue ID and URL
   - Current git branch and status
   - Recent comments/activity
   - In-progress issues assigned to me

### On `/session update <notes>`

1. Get the session issue ID from hook output
2. Format a comment with:
   ```
   ## Work Update - [timestamp]

   <notes provided by user or summary of recent work>

   **Files changed:** [list key files if applicable]
   **Related commits:** [recent commit hashes if any]
   ```
3. Use `mcp__linear-server__create_comment` to add the comment
4. Confirm the update was added

### On `/session decision <decision>`

1. Get the session issue ID
2. Format a comment:
   ```
   ## Decision Logged - [timestamp]

   **Decision:** <decision text>
   **Context:** <brief context if apparent from conversation>
   ```
3. Add the comment to the session issue

### On `/session link <issue-id>`

1. Get the session issue ID
2. Use `mcp__linear-server__get_issue` to fetch both issues
3. Use `mcp__linear-server__update_issue` with `relatedTo` to link them
4. Add a comment noting the linkage

### On `/session end`

1. Get the session issue ID
2. Gather session summary:
   - Review recent commits on current branch
   - List files changed during session
   - Summarize key accomplishments from conversation
3. Format end-of-session comment:
   ```
   ## Session Ended - [timestamp]

   ### Accomplished
   - [bullet list of completed work]

   ### In Progress
   - [any incomplete work]

   ### Notes for Next Session
   - [important context for resuming]
   - [any blockers or pending decisions]

   ### Files Modified
   - [key files changed]
   ```
4. Add the comment to the session issue

## Best Practices

- **Update frequently**: Run `/session update` after completing significant tasks
- **Document decisions**: Use `/session decision` for architectural or implementation choices
- **Link issues**: Use `/session link` to connect work items to the session
- **End cleanly**: Run `/session end` before ending a session with incomplete work

## Error Handling

- If session issue ID is not found in recent output, ask user to check the session hook output
- If Linear API fails, show the error and suggest checking LINEAR_API_KEY
- If issue doesn't exist, the session may have been from a previous day - suggest starting a new session
