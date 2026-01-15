#!/bin/bash
# ABOUTME: Automatically creates or resumes Linear session issues for Claude Code sessions.
# ABOUTME: Uses LINEAR_API_KEY to manage sessions via the Linear GraphQL API.

set -euo pipefail

# Source .envrc if it exists and LINEAR_API_KEY is not set
if [[ -z "${LINEAR_API_KEY:-}" ]] && [[ -f ".envrc" ]]; then
    # shellcheck source=/dev/null
    source .envrc 2>/dev/null || true
fi

# Configuration
TEAM_NAME="Async-io"
CLAUDE_SESSION_LABEL_NAME="claude-session"
LINEAR_API_URL="https://api.linear.app/graphql"

# Generate session identifier: YYYY-MM-DD-<project_name>-<short_hash>
generate_session_id() {
    local date_part
    local project_name
    local path_hash

    date_part=$(date +%Y-%m-%d)
    project_name=$(basename "$(pwd)" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]-_')
    path_hash=$(echo -n "$(pwd)" | md5sum 2>/dev/null || md5 -q 2>/dev/null || echo "$(pwd)" | shasum | cut -c1-4)
    path_hash=$(echo "$path_hash" | cut -c1-4)

    echo "Session: ${date_part}-${project_name}-${path_hash}"
}

# Get git branch if available
get_git_branch() {
    git branch --show-current 2>/dev/null || echo "none"
}

# Extract Linear issue ID from branch name (e.g., jfarcand/asy-16-some-title -> ASY-16)
extract_issue_from_branch() {
    local branch="$1"
    echo "$branch" | grep -oiE 'asy-[0-9]+' | head -1 | tr '[:lower:]' '[:upper:]' || echo ""
}

# Check if we have LINEAR_API_KEY
has_api_key() {
    [[ -n "${LINEAR_API_KEY:-}" ]]
}

# Execute GraphQL query against Linear API
linear_query() {
    local query="$1"
    curl -s -X POST "$LINEAR_API_URL" \
        -H "Content-Type: application/json" \
        -H "Authorization: $LINEAR_API_KEY" \
        -d "$query"
}

# Look up team ID by name
get_team_id() {
    local team_name="$1"
    local query
    query=$(cat <<EOF
{"query": "query { teams(filter: { name: { eq: \\"${team_name}\\" } }) { nodes { id name } } }"}
EOF
)
    local result
    result=$(linear_query "$query")
    echo "$result" | jq -r '.data.teams.nodes[0].id // empty' 2>/dev/null
}

# Look up label ID by name (workspace-level labels)
get_label_id() {
    local label_name="$1"
    local query
    query=$(cat <<EOF
{"query": "query { issueLabels(filter: { name: { eq: \\"${label_name}\\" } }) { nodes { id name } } }"}
EOF
)
    local result
    result=$(linear_query "$query")
    echo "$result" | jq -r '.data.issueLabels.nodes[0].id // empty' 2>/dev/null
}

# Search for existing session issue
search_session() {
    local session_id="$1"
    local team_id="$2"
    local query
    query=$(cat <<EOF
{"query": "query { issues(filter: { team: { id: { eq: \\"${team_id}\\" } }, title: { contains: \\"${session_id}\\" } }, first: 1) { nodes { id identifier title url } } }"}
EOF
)
    linear_query "$query"
}

# Create a new session issue
create_session() {
    local session_id="$1"
    local git_branch="$2"
    local team_id="$3"
    local label_id="$4"
    local description

    description="## Claude Code Session\\n\\n**Started:** $(date '+%Y-%m-%d %H:%M')\\n**Project:** $(basename "$(pwd)")\\n**Branch:** ${git_branch}\\n\\n### Work Done\\n- (Updated during session)\\n\\n### Decisions Made\\n- (Document key decisions here)\\n\\n### Related Issues\\n- (Links added automatically)"

    local query
    query=$(cat <<EOF
{"query": "mutation { issueCreate(input: { teamId: \\"${team_id}\\", title: \\"${session_id}\\", description: \\"${description}\\", labelIds: [\\"${label_id}\\"] }) { success issue { id identifier title url } } }"}
EOF
)
    linear_query "$query"
}

# Add a comment to an existing issue
add_comment() {
    local issue_id="$1"
    local body="$2"
    local query
    query=$(cat <<EOF
{"query": "mutation { commentCreate(input: { issueId: \\"${issue_id}\\", body: \\"${body}\\" }) { success } }"}
EOF
)
    linear_query "$query"
}

# Check if session was resumed recently (within last 5 minutes)
was_resumed_recently() {
    local issue_id="$1"
    local query
    query=$(cat <<EOF
{"query": "query { issue(id: \\"${issue_id}\\") { comments(first: 1, orderBy: createdAt) { nodes { body createdAt } } } }"}
EOF
)
    local result
    result=$(linear_query "$query")
    local last_comment
    last_comment=$(echo "$result" | jq -r '.data.issue.comments.nodes[0] // empty' 2>/dev/null)

    if [[ -z "$last_comment" ]] || [[ "$last_comment" == "null" ]]; then
        echo "false"
        return
    fi

    local body
    body=$(echo "$last_comment" | jq -r '.body')

    # Only check if it's a "Session resumed" comment
    if [[ "$body" != Session\ resumed* ]]; then
        echo "false"
        return
    fi

    local created_at
    created_at=$(echo "$last_comment" | jq -r '.createdAt')
    local comment_epoch
    comment_epoch=$(date -j -f "%Y-%m-%dT%H:%M:%S" "${created_at%%.*}" "+%s" 2>/dev/null || date -d "${created_at}" "+%s" 2>/dev/null || echo "0")
    local now_epoch
    now_epoch=$(date "+%s")
    local diff=$((now_epoch - comment_epoch))

    # If less than 5 minutes (300 seconds) ago, consider it recent
    if [[ $diff -lt 300 ]]; then
        echo "true"
    else
        echo "false"
    fi
}

# Fetch in-progress issues assigned to me
fetch_in_progress() {
    local query
    query='{"query": "query { issues(filter: { assignee: { isMe: { eq: true } }, state: { type: { eq: \"started\" } } }, first: 5) { nodes { identifier title state { name } } } }"}'
    linear_query "$query"
}

# Output results for Claude (compact format for hooks)
output_session_status() {
    local session_id="$1"
    local git_branch="$2"
    local branch_issue="$3"
    local action="$4"
    local issue_id="$5"
    local issue_url="$6"
    local in_progress="$7"

    # Compact single-line output for hook visibility
    echo "📋 Linear Session ${action}: ${issue_id} (${git_branch}) - ${issue_url}"

    # Show in-progress issues if any
    if [[ "$in_progress" != "[]" ]] && [[ -n "$in_progress" ]]; then
        local issues
        issues=$(echo "$in_progress" | jq -r '.[].identifier' 2>/dev/null | tr '\n' ' ')
        if [[ -n "$issues" ]]; then
            echo "📊 In-Progress: ${issues}"
        fi
    fi
}

# Output instructions for Claude when API key is missing
output_missing_api_key_prompt() {
    local session_id="$1"
    local git_branch="$2"

    cat << EOF
🔐 LINEAR SESSION INIT: API key required

CLAUDE ACTION REQUIRED: Use AskUserQuestion NOW to ask:
"I need your Linear API key to track this session. Get it from https://linear.app/settings/api"

After getting key, run: curl -s -X POST https://api.linear.app/graphql -H "Authorization: <KEY>" -H "Content-Type: application/json" -d '{"query": "mutation { issueCreate(input: { teamId: \"e3ec7c28-3981-4e28-92c6-02c218b2cce2\", title: \"${session_id}\", labelIds: [\"9c42e36e-0752-4166-8ee7-dfaa51f7eb62\"] }) { success issue { identifier url } } }"}'

SESSION: ${session_id} | BRANCH: ${git_branch}
EOF
}

# Main execution
main() {
    local session_id
    local git_branch
    local branch_issue
    local team_id
    local label_id
    local search_result
    local existing_issue
    local action
    local issue_id
    local issue_url
    local in_progress_result
    local in_progress

    session_id=$(generate_session_id)
    git_branch=$(get_git_branch)
    branch_issue=$(extract_issue_from_branch "$git_branch")

    # Check for API key - if missing, prompt Claude to ask user
    if ! has_api_key; then
        output_missing_api_key_prompt "$session_id" "$git_branch"
        exit 0
    fi

    # Look up team and label IDs dynamically
    team_id=$(get_team_id "$TEAM_NAME")
    if [[ -z "$team_id" ]]; then
        echo "ERROR: Team '${TEAM_NAME}' not found" >&2
        exit 1
    fi

    label_id=$(get_label_id "$CLAUDE_SESSION_LABEL_NAME")
    if [[ -z "$label_id" ]]; then
        echo "ERROR: Label '${CLAUDE_SESSION_LABEL_NAME}' not found" >&2
        exit 1
    fi

    # Search for existing session
    search_result=$(search_session "$session_id" "$team_id")
    existing_issue=$(echo "$search_result" | jq -r '.data.issues.nodes[0] // empty' 2>/dev/null)

    if [[ -n "$existing_issue" ]] && [[ "$existing_issue" != "null" ]]; then
        # Resume existing session
        issue_id=$(echo "$existing_issue" | jq -r '.identifier')
        issue_url=$(echo "$existing_issue" | jq -r '.url')
        local issue_uuid
        issue_uuid=$(echo "$existing_issue" | jq -r '.id')

        # Only add resume comment if not resumed recently (prevents spam)
        if [[ "$(was_resumed_recently "$issue_uuid")" == "false" ]]; then
            add_comment "$issue_uuid" "Session resumed at $(date '+%H:%M')" > /dev/null 2>&1
        fi
        action="RESUMED"
    else
        # Create new session
        local create_result
        create_result=$(create_session "$session_id" "$git_branch" "$team_id" "$label_id")

        local success
        success=$(echo "$create_result" | jq -r '.data.issueCreate.success' 2>/dev/null)

        if [[ "$success" == "true" ]]; then
            issue_id=$(echo "$create_result" | jq -r '.data.issueCreate.issue.identifier')
            issue_url=$(echo "$create_result" | jq -r '.data.issueCreate.issue.url')
            action="CREATED"
        else
            echo "ERROR: Failed to create session issue"
            echo "$create_result" | jq '.' 2>/dev/null || echo "$create_result"
            exit 1
        fi
    fi

    # Fetch in-progress issues
    in_progress_result=$(fetch_in_progress)
    in_progress=$(echo "$in_progress_result" | jq -r '.data.issues.nodes' 2>/dev/null)

    # Output status
    output_session_status "$session_id" "$git_branch" "$branch_issue" "$action" "$issue_id" "$issue_url" "$in_progress"
}

main "$@"
