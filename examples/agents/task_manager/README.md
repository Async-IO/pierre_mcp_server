# Task Lifecycle Management Example

Demonstrates A2A protocol's **task management** capabilities for long-running operations.

## What This Example Demonstrates

### A2A Task Lifecycle:
1. **Task Creation** - Submit long-running tasks via `tasks/create`
2. **Status Monitoring** - Poll task status with `tasks/get`
3. **State Transitions** - Track progression: pending → running → completed/failed
4. **Result Retrieval** - Get task output when completed
5. **Task Listing** - Query all tasks with `tasks/list`
6. **Task Cancellation** - Cancel running tasks (if supported)

## Task State Machine

```
┌─────────┐
│ pending │  Task created, awaiting execution
└────┬────┘
     │
     v
┌─────────┐
│ running │  Task is actively being processed
└────┬────┘
     │
     ├────────┐
     v        v
┌──────────┐ ┌────────┐
│completed │ │ failed │  Final states
└──────────┘ └────────┘
     ^
     │
┌───────────┐
│ cancelled │  User/system cancelled
└───────────┘
```

## Why Task Management Matters

A2A protocol supports both **synchronous** and **asynchronous** task execution:

- **Short Tasks**: Execute immediately, return result in response
- **Long Tasks**: Accept task, return task ID, client polls for status
- **Very Long Tasks**: Support webhooks for push notifications (not yet implemented in Pierre)

## Quick Start

```bash
# 1. Start Pierre server
cd ../../../
cargo run --bin pierre-mcp-server

# 2. Register A2A client (if not already done)
./examples/agents/fitness_analyzer/run.sh --setup-demo

# 3. Run task manager example
cd examples/agents/task_manager
export PIERRE_A2A_CLIENT_ID="your_client_id"
export PIERRE_A2A_CLIENT_SECRET="your_client_secret"
cargo run
```

## Example Output

```
🚀 A2A Task Lifecycle Management Example
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔐 Authenticating with A2A protocol
✅ Authentication successful

🔄 Demonstrating A2A Task Lifecycle

📝 Creating task: fitness_analysis
✅ Task created: 550e8400-e29b-41d4-a716-446655440000

📊 Task Details:
   ID: 550e8400-e29b-41d4-a716-446655440000
   Type: fitness_analysis
   Status: Pending
   Created: 2024-01-15T10:00:00Z

👀 Monitoring task status...
   [1] Task is pending...
   [2] Task is running...
   [3] ✅ Task completed!

📋 Final Task Status:
   ID: 550e8400-e29b-41d4-a716-446655440000
   Status: Completed
   Updated: 2024-01-15T10:05:00Z
   Result: {
     "analysis": {
       "total_distance": 42195,
       "total_duration": 7200,
       "average_pace": "5:30/km"
     }
   }

📚 All Tasks (15):
   1. 550e8400... - Completed - fitness_analysis
   2. 661f9511... - Running - data_export
   3. 772fa622... - Pending - report_generation
   4. 883fb733... - Completed - goal_tracking
   5. 994fc844... - Failed - invalid_analysis
```

## Key Concepts Demonstrated

### 1. Task Creation
```rust
let task = manager.create_task("fitness_analysis", input_data).await?;
```
Submit a long-running task and receive a task ID for tracking.

### 2. Status Polling
```rust
let task = manager.get_task(task_id).await?;
match task.status {
    TaskStatus::Completed => // Handle result
    TaskStatus::Running => // Continue polling
    TaskStatus::Failed => // Handle error
    _ => {}
}
```
Poll task status periodically until completion.

### 3. Task Listing
```rust
let tasks = manager.list_tasks().await?;
```
Query all tasks for a client, with optional status filtering.

## A2A vs Real-Time Execution

| Scenario | Approach | Example |
|----------|----------|---------|
| Quick query (<1s) | Synchronous tool call | `get_activities` |
| Analysis (1-30s) | Synchronous with timeout | `analyze_activity` |
| Heavy processing (>30s) | Asynchronous task | `generate_annual_report` |
| Scheduled work | Asynchronous task | `weekly_summary_email` |

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `PIERRE_SERVER_URL` | `http://localhost:8081` | Pierre server URL |
| `PIERRE_A2A_CLIENT_ID` | `task_manager_client` | A2A client ID |
| `PIERRE_A2A_CLIENT_SECRET` | `demo_secret_123` | A2A client secret |

## Advanced Features (Not Yet Implemented)

### Webhooks for Push Notifications
```json
POST /a2a/execute
{
  "method": "tasks/pushNotificationConfig/set",
  "params": {
    "webhook_url": "https://my-agent.com/webhooks/task-updates",
    "events": ["task.completed", "task.failed"]
  }
}
```
Instead of polling, receive push notifications when tasks complete.

### Task Prioritization
```json
{
  "method": "tasks/create",
  "params": {
    "task_type": "urgent_analysis",
    "priority": "high",  // high, normal, low
    "input_data": {...}
  }
}
```

### Task Dependencies
```json
{
  "method": "tasks/create",
  "params": {
    "task_type": "summary_report",
    "depends_on": ["task_id_1", "task_id_2"]
  }
}
```

## A2A Specification Compliance

This example demonstrates:

- ✅ Task Creation (`tasks/create`)
- ✅ Task Status Query (`tasks/get`)
- ✅ Task Listing (`tasks/list`)
- ✅ Task State Machine (pending/running/completed/failed/cancelled)
- ✅ JSON-RPC 2.0 over HTTP
- ⚠️ Push Notifications (configured but not yet active in Pierre)

## Learn More

- [A2A Protocol Specification](https://github.com/google/A2A)
- [Pierre Task Management](../../../src/a2a/protocol.rs)
- [A2A vs MCP: When to Use Each](../../../docs/tutorial/chapter-18-a2a-protocol.md)
