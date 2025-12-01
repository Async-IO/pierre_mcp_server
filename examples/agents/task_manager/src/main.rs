// ABOUTME: Demonstrates A2A task lifecycle management with status tracking
// ABOUTME: Shows how to create, monitor, and manage long-running A2A tasks
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Task Lifecycle Management Example
//!
//! This example demonstrates A2A protocol's task management capabilities:
//! 1. Creating long-running tasks
//! 2. Monitoring task status with polling
//! 3. Handling task state transitions (pending → running → completed/failed)
//! 4. Retrieving task results
//! 5. Listing and filtering tasks
//! 6. Cancelling tasks
//!
//! ## A2A Task Lifecycle
//! ```text
//! ┌─────────┐
//! │ pending │  Task created, awaiting execution
//! └────┬────┘
//!      │
//!      v
//! ┌─────────┐
//! │ running │  Task is actively being processed
//! └────┬────┘
//!      │
//!      ├────────┐
//!      v        v
//! ┌──────────┐ ┌────────┐
//! │completed │ │ failed │  Final states
//! └──────────┘ └────────┘
//! ```

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

/// Task status enumeration
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A2A Task representation
#[derive(Debug, Deserialize, Serialize)]
struct A2ATask {
    id: String,
    status: TaskStatus,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    client_id: String,
    task_type: String,
    input_data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
    updated_at: String,
}

/// Task Manager Client
struct TaskManager {
    http_client: Client,
    server_url: String,
    access_token: Option<String>,
    client_id: String,
}

impl TaskManager {
    /// Create a new task manager
    fn new(server_url: String, client_id: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("TaskManagerExample/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            server_url,
            access_token: None,
            client_id,
        }
    }

    /// Authenticate with A2A protocol
    async fn authenticate(&mut self, client_secret: &str) -> Result<()> {
        info!("🔐 Authenticating with A2A protocol");

        let auth_payload = json!({
            "client_id": self.client_id,
            "client_secret": client_secret,
            "grant_type": "client_credentials",
            "scopes": ["read", "write", "tasks"]
        });

        let response = self
            .http_client
            .post(format!("{}/a2a/auth", self.server_url))
            .header("Content-Type", "application/json")
            .json(&auth_payload)
            .send()
            .await
            .context("Failed to authenticate")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Authentication failed: {error_text}");
        }

        let auth_response: Value = response.json().await?;
        self.access_token = Some(
            auth_response
                .get("session_token")
                .or_else(|| auth_response.get("access_token"))
                .and_then(|t| t.as_str())
                .context("No access token in response")?
                .to_string(),
        );

        info!("✅ Authentication successful");
        Ok(())
    }

    /// Create a new task
    async fn create_task(&self, task_type: &str, input_data: Value) -> Result<A2ATask> {
        info!("📝 Creating task: {}", task_type);

        let request_id = Uuid::new_v4().to_string();
        let request = json!({
            "jsonrpc": "2.0",
            "method": "a2a/tasks/create",
            "params": {
                "client_id": self.client_id,
                "task_type": task_type,
                "input_data": input_data
            },
            "id": request_id
        });

        let response = self
            .http_client
            .post(format!("{}/a2a/execute", self.server_url))
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.access_token.as_ref().context("Not authenticated")?),
            )
            .json(&request)
            .send()
            .await
            .context("Failed to create task")?;

        let json_response: Value = response.json().await?;

        if let Some(error) = json_response.get("error") {
            anyhow::bail!("Task creation failed: {error}");
        }

        let task: A2ATask = serde_json::from_value(
            json_response
                .get("result")
                .context("No result in response")?
                .clone(),
        )?;

        info!("✅ Task created: {}", task.id);
        Ok(task)
    }

    /// Get task status
    async fn get_task(&self, task_id: &str) -> Result<A2ATask> {
        let request_id = Uuid::new_v4().to_string();
        let request = json!({
            "jsonrpc": "2.0",
            "method": "a2a/tasks/get",
            "params": {
                "task_id": task_id
            },
            "id": request_id
        });

        let response = self
            .http_client
            .post(format!("{}/a2a/execute", self.server_url))
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.access_token.as_ref().context("Not authenticated")?),
            )
            .json(&request)
            .send()
            .await
            .context("Failed to get task")?;

        let json_response: Value = response.json().await?;

        if let Some(error) = json_response.get("error") {
            anyhow::bail!("Get task failed: {error}");
        }

        let task: A2ATask = serde_json::from_value(
            json_response
                .get("result")
                .context("No result in response")?
                .clone(),
        )?;

        Ok(task)
    }

    /// List all tasks
    async fn list_tasks(&self) -> Result<Vec<A2ATask>> {
        info!("📋 Listing all tasks");

        let request_id = Uuid::new_v4().to_string();
        let request = json!({
            "jsonrpc": "2.0",
            "method": "a2a/tasks/list",
            "params": {
                "client_id": self.client_id,
                "limit": 50
            },
            "id": request_id
        });

        let response = self
            .http_client
            .post(format!("{}/a2a/execute", self.server_url))
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.access_token.as_ref().context("Not authenticated")?),
            )
            .json(&request)
            .send()
            .await
            .context("Failed to list tasks")?;

        let json_response: Value = response.json().await?;

        if let Some(error) = json_response.get("error") {
            anyhow::bail!("List tasks failed: {error}");
        }

        let result = json_response
            .get("result")
            .context("No result in response")?;

        let tasks: Vec<A2ATask> = serde_json::from_value(
            result
                .get("tasks")
                .context("No tasks in result")?
                .clone(),
        )?;

        info!("✅ Found {} tasks", tasks.len());
        Ok(tasks)
    }

    /// Monitor task until completion
    async fn monitor_task(&self, task_id: &str, max_attempts: usize) -> Result<A2ATask> {
        info!("👀 Monitoring task: {}", task_id);

        for attempt in 1..=max_attempts {
            let task = self.get_task(task_id).await?;

            match task.status {
                TaskStatus::Pending => {
                    info!("   [{}] Task is pending...", attempt);
                }
                TaskStatus::Running => {
                    info!("   [{}] Task is running...", attempt);
                }
                TaskStatus::Completed => {
                    info!("   [{}] ✅ Task completed!", attempt);
                    return Ok(task);
                }
                TaskStatus::Failed => {
                    warn!("   [{}] ❌ Task failed: {:?}", attempt, task.error_message);
                    return Ok(task);
                }
                TaskStatus::Cancelled => {
                    warn!("   [{}] 🚫 Task was cancelled", attempt);
                    return Ok(task);
                }
            }

            if attempt < max_attempts {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        anyhow::bail!("Task monitoring timeout after {max_attempts} attempts")
    }

    /// Demonstrate full task lifecycle
    async fn demonstrate_task_lifecycle(&self) -> Result<()> {
        info!("\n🔄 Demonstrating A2A Task Lifecycle\n");

        // Create a fitness analysis task
        let input_data = json!({
            "analysis_type": "weekly_summary",
            "date_range": {
                "start": "2024-01-01",
                "end": "2024-01-07"
            },
            "metrics": ["distance", "duration", "elevation"]
        });

        // Step 1: Create task
        let task = self.create_task("fitness_analysis", input_data).await?;
        info!("\n📊 Task Details:");
        info!("   ID: {}", task.id);
        info!("   Type: {}", task.task_type);
        info!("   Status: {:?}", task.status);
        info!("   Created: {}", task.created_at);

        // Step 2: Monitor task (simulated - in reality, Pierre doesn't have async task execution yet)
        info!("\n👀 Monitoring task status...");
        let final_task = self.monitor_task(&task.id, 5).await?;

        info!("\n📋 Final Task Status:");
        info!("   ID: {}", final_task.id);
        info!("   Status: {:?}", final_task.status);
        info!("   Updated: {}", final_task.updated_at);

        if let Some(result) = final_task.output_data {
            info!("   Result: {}", serde_json::to_string_pretty(&result)?);
        }

        // Step 3: List all tasks
        let tasks = self.list_tasks().await?;
        info!("\n📚 All Tasks ({}):", tasks.len());
        for (idx, t) in tasks.iter().take(5).enumerate() {
            info!("   {}. {} - {:?} - {}", idx + 1, t.id, t.status, t.task_type);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("task_manager_example=info")
        .init();

    info!("🚀 A2A Task Lifecycle Management Example");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load configuration
    let server_url = std::env::var("PIERRE_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string());

    let client_id = std::env::var("PIERRE_A2A_CLIENT_ID")
        .unwrap_or_else(|_| "task_manager_client".to_string());

    let client_secret = std::env::var("PIERRE_A2A_CLIENT_SECRET")
        .unwrap_or_else(|_| "demo_secret_123".to_string());

    // Create task manager
    let mut manager = TaskManager::new(server_url, client_id);

    // Authenticate
    if let Err(e) = manager.authenticate(&client_secret).await {
        tracing::error!("❌ Authentication failed: {}", e);
        tracing::error!("   Make sure to register an A2A client first:");
        tracing::error!("   See examples/agents/fitness_analyzer/README.md for setup instructions");
        return Err(e);
    }

    // Run demonstration
    match manager.demonstrate_task_lifecycle().await {
        Ok(()) => {
            info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            info!("✅ Task Lifecycle Demo Completed Successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            tracing::error!("❌ Task Lifecycle Demo Failed: {}", e);
            Err(e)
        }
    }
}
