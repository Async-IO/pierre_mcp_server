// ABOUTME: Chat route handlers for AI conversation management
// ABOUTME: Provides REST endpoints for creating, listing, and messaging in chat conversations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Chat routes for AI conversations
//!
//! This module handles chat conversation management including creating conversations,
//! sending messages, and streaming AI responses. All handlers require JWT authentication.

use crate::models::ConnectionType;
use crate::models::TenantId;
use crate::{
    auth::AuthResult,
    config::LlmProviderType,
    database::{ConversationRecord, MessageRecord},
    database_plugins::DatabaseProvider,
    errors::AppError,
    llm::{
        get_insight_generation_prompt, get_pierre_system_prompt, ChatMessage, ChatProvider,
        ChatRequest, FunctionCall, FunctionDeclaration, FunctionResponse, TokenUsage, Tool,
    },
    mcp::resources::ServerResources,
    protocols::universal::{UniversalExecutor, UniversalRequest, UniversalResponse},
    security::cookies::get_cookie_value,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Write, sync::Arc, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of tool call iterations before forcing a text response
const MAX_TOOL_ITERATIONS: usize = 10;

/// Prefix used to detect insight generation requests from the frontend.
/// Must match the `INSIGHT_PROMPT_PREFIX` constant in `@pierre/chat-utils`.
const INSIGHT_PROMPT_PREFIX: &str = "Create a shareable insight from this analysis";

// ============================================================================
// Helper Functions
// ============================================================================

/// Strip synthetic function call syntax from LLM content
///
/// Some models (like Llama via Groq) output function calls both as proper `tool_calls`
/// AND as text content using syntax like `<function(name)>{...}</function>` or
/// `<function/name>{...}</function>`.
/// This helper removes that synthetic syntax to avoid displaying it to users.
fn strip_synthetic_function_calls(content: &str) -> Cow<'_, str> {
    use regex::Regex;
    use std::sync::OnceLock;

    fn function_pattern() -> Option<&'static Regex> {
        static PATTERN: OnceLock<Option<Regex>> = OnceLock::new();
        PATTERN
            .get_or_init(|| {
                // Match patterns like:
                // - <function(name)>...</function> (parentheses syntax)
                // - <function/name>...</function> (slash syntax)
                Regex::new(r"<function[/\(][^>]+>[\s\S]*?</function>").ok()
            })
            .as_ref()
    }

    let Some(pattern) = function_pattern() else {
        return Cow::Borrowed(content);
    };

    let cleaned = pattern.replace_all(content, "");
    let trimmed = cleaned.trim();

    if trimmed.is_empty() {
        Cow::Borrowed("")
    } else if trimmed.len() == content.len() {
        Cow::Borrowed(content)
    } else {
        Cow::Owned(trimmed.to_owned())
    }
}

/// JSON response structure for insight generation
#[derive(Debug, Deserialize)]
struct InsightGenerationResponse {
    content: String,
}

/// Parse JSON response from insight generation prompt
///
/// The insight generation prompt returns JSON: `{"content": "..."}`
/// This extracts the content field, falling back to raw content if parsing fails.
fn parse_insight_json_response(raw_content: &str) -> String {
    // Try to parse as JSON
    if let Ok(response) = serde_json::from_str::<InsightGenerationResponse>(raw_content) {
        return response.content;
    }

    // Sometimes LLMs wrap JSON in markdown code blocks, try to extract
    let trimmed = raw_content.trim();
    if let Some(json_start) = trimmed.find('{') {
        if let Some(json_end) = trimmed.rfind('}') {
            let json_str = &trimmed[json_start..=json_end];
            if let Ok(response) = serde_json::from_str::<InsightGenerationResponse>(json_str) {
                return response.content;
            }
        }
    }

    // Fallback: return raw content with warning (avoid logging raw content which may contain user data)
    warn!(
        "Failed to parse insight generation JSON response, using raw content ({} bytes)",
        raw_content.len()
    );
    raw_content.to_owned()
}

// ============================================================================
// Internal Types
// ============================================================================

/// Result of running the multi-turn tool execution loop
struct ToolLoopResult {
    /// Final text content from LLM
    content: String,
    /// Token usage statistics if available
    usage: Option<TokenUsage>,
    /// Finish reason if available
    finish_reason: Option<String>,
    /// Activity list from `get_activities` tool (to prepend to response)
    activity_list: Option<String>,
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a new conversation
#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    /// Conversation title
    pub title: String,
    /// LLM model to use (optional, defaults to provider's default model)
    #[serde(default)]
    pub model: Option<String>,
    /// System prompt for the conversation (optional)
    #[serde(default)]
    pub system_prompt: Option<String>,
}

/// Response for conversation creation
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationResponse {
    /// Conversation ID
    pub id: String,
    /// Conversation title
    pub title: String,
    /// Model used
    pub model: String,
    /// System prompt if set
    pub system_prompt: Option<String>,
    /// Total tokens used
    pub total_tokens: i64,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// Response for listing conversations
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationListResponse {
    /// List of conversations
    pub conversations: Vec<ConversationSummaryResponse>,
    /// Total count
    pub total: usize,
}

/// Summary of a conversation for listing
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationSummaryResponse {
    /// Conversation ID
    pub id: String,
    /// Conversation title
    pub title: String,
    /// Model used
    pub model: String,
    /// Message count
    pub message_count: i64,
    /// Total tokens used
    pub total_tokens: i64,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// Request to update a conversation title
#[derive(Debug, Deserialize)]
pub struct UpdateConversationRequest {
    /// New title
    pub title: String,
}

/// Request to send a message
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// Message content
    pub content: String,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
}

/// Response for a message
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Message ID
    pub id: String,
    /// Role (user/assistant/system)
    pub role: String,
    /// Message content
    pub content: String,
    /// Token count
    pub token_count: Option<i64>,
    /// Creation timestamp
    pub created_at: String,
}

/// Response with chat completion (non-streaming)
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// User message
    pub user_message: MessageResponse,
    /// Assistant response
    pub assistant_message: MessageResponse,
    /// Conversation updated timestamp
    pub conversation_updated_at: String,
    /// LLM model used for the response
    pub model: String,
    /// Total execution time in milliseconds (including tool calls)
    pub execution_time_ms: u64,
}

/// Response for messages list
#[derive(Debug, Serialize, Deserialize)]
pub struct MessagesListResponse {
    /// List of messages
    pub messages: Vec<MessageResponse>,
}

/// Query parameters for listing conversations
#[derive(Debug, Deserialize, Default)]
pub struct ListConversationsQuery {
    /// Maximum number of conversations to return
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: i64,
}

const fn default_limit() -> i64 {
    20
}

// ============================================================================
// Chat Routes
// ============================================================================

/// Chat routes handler
pub struct ChatRoutes;

impl ChatRoutes {
    /// Create all chat routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            // Conversation management
            .route("/api/chat/conversations", post(Self::create_conversation))
            .route("/api/chat/conversations", get(Self::list_conversations))
            .route(
                "/api/chat/conversations/:conversation_id",
                get(Self::get_conversation),
            )
            .route(
                "/api/chat/conversations/:conversation_id",
                put(Self::update_conversation),
            )
            .route(
                "/api/chat/conversations/:conversation_id",
                delete(Self::delete_conversation),
            )
            // Messages
            .route(
                "/api/chat/conversations/:conversation_id/messages",
                get(Self::get_messages),
            )
            // POST messages with MCP tool support (non-streaming)
            .route(
                "/api/chat/conversations/:conversation_id/messages",
                post(Self::send_message),
            )
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(headers, "auth_token") {
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Get user's `tenant_id` (defaults to `user_id` if no tenant)
    async fn get_tenant_id(
        user_id: Uuid,
        resources: &Arc<ServerResources>,
    ) -> Result<TenantId, AppError> {
        let tenants = resources.database.list_tenants_for_user(user_id).await?;
        Ok(tenants
            .first()
            .map_or_else(|| TenantId::from(user_id), |t| t.id))
    }

    /// Get the system prompt text for a conversation
    ///
    /// Uses conversation-specific prompt if set, otherwise returns the default Pierre system prompt.
    fn get_system_prompt_text(conversation: &ConversationRecord) -> String {
        conversation
            .system_prompt
            .clone()
            .unwrap_or_else(|| get_pierre_system_prompt().to_owned())
    }

    /// Build provider context string for inclusion in system prompt
    ///
    /// Uses `provider_connections` as the single source of truth for which providers
    /// are connected, so the LLM doesn't ask users to connect already-available providers.
    async fn build_provider_context(resources: &Arc<ServerResources>, user_id: Uuid) -> String {
        // Get all provider connections (cross-tenant view, single source of truth)
        let Ok(connections) = resources
            .database
            .get_user_provider_connections(user_id, None)
            .await
        else {
            return String::new();
        };

        if connections.is_empty() {
            return String::new();
        }

        let mut context = String::from("\n\n## Connected Fitness Data Providers\n\n");
        context.push_str("The user has the following data sources available:\n");
        for conn in &connections {
            let label = if conn.connection_type == ConnectionType::Synthetic {
                Cow::Owned(format!("{} (test data)", conn.provider))
            } else {
                Cow::Borrowed(conn.provider.as_str())
            };
            // Write trait used to avoid format_push_string lint
            let _ = writeln!(context, "- ✓ {label}");
        }
        context.push_str("\nUse the connected providers to fetch activity data. ");
        context
            .push_str("Do NOT ask the user to connect providers that are already connected above.");

        context
    }

    /// Get augmented system prompt with provider context
    async fn get_augmented_system_prompt(
        conversation: &ConversationRecord,
        resources: &Arc<ServerResources>,
        user_id: Uuid,
    ) -> String {
        let base_prompt = Self::get_system_prompt_text(conversation);
        let provider_context = Self::build_provider_context(resources, user_id).await;

        if provider_context.is_empty() {
            base_prompt
        } else {
            format!("{base_prompt}{provider_context}")
        }
    }

    /// Get startup query for a coach conversation if applicable
    ///
    /// The `system_prompt` is stored in conversations when a coach is selected.
    /// This function looks up the coach by `system_prompt` and returns its startup query.
    ///
    /// Returns `Some(query)` only if:
    /// - This is the first message in the conversation (`history_len == 1`)
    /// - The conversation has a custom `system_prompt` (indicates a coach)
    /// - The coach has a `startup_query` configured
    ///
    /// The `startup_query` if found, None otherwise.
    async fn get_startup_query_if_applicable(
        resources: &Arc<ServerResources>,
        history_len: usize,
        system_prompt: Option<&String>,
        tenant_id: TenantId,
    ) -> Option<String> {
        use crate::database::coaches::CoachesManager;

        // Only inject on first message
        if history_len != 1 {
            return None;
        }

        // Must have a system prompt (indicates coach conversation)
        let prompt = system_prompt?;

        // Only SQLite is supported for coaches - PostgreSQL databases skip startup query
        let pool = resources.database.sqlite_pool()?;

        let coaches_manager = CoachesManager::new(pool.clone());

        match coaches_manager
            .get_startup_query_by_system_prompt(prompt, tenant_id)
            .await
        {
            Ok(Some(query)) => {
                info!(
                    "Found startup query for coach conversation: {}",
                    &query[..query.len().min(50)]
                );
                Some(query)
            }
            Ok(None) => None,
            Err(e) => {
                warn!("Failed to get startup query: {e}");
                None
            }
        }
    }

    /// Get LLM provider based on `PIERRE_LLM_PROVIDER` environment variable
    async fn get_llm_provider() -> Result<ChatProvider, AppError> {
        ChatProvider::from_env().await
    }

    /// Build LLM messages from conversation history and optional system prompt
    fn build_llm_messages(
        system_prompt: Option<&str>,
        history: &[MessageRecord],
    ) -> Vec<ChatMessage> {
        let mut messages = Vec::with_capacity(history.len() + 1);

        if let Some(prompt) = system_prompt {
            messages.push(ChatMessage::system(prompt));
        }

        for msg in history {
            let chat_msg = match msg.role.as_str() {
                "user" => ChatMessage::user(&msg.content),
                "assistant" => ChatMessage::assistant(&msg.content),
                "system" => ChatMessage::system(&msg.content),
                _ => continue,
            };
            messages.push(chat_msg);
        }

        messages
    }

    /// Build connection-related tool definitions
    fn build_connection_tools() -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "get_connection_status".to_owned(),
                description: "Check which fitness providers are connected".to_owned(),
                parameters: Some(serde_json::json!({"type": "object", "properties": {}})),
            },
            FunctionDeclaration {
                name: "connect_provider".to_owned(),
                description: "Connect to a fitness provider via OAuth".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"provider": {"type": "string"}},
                    "required": ["provider"]
                })),
            },
            FunctionDeclaration {
                name: "disconnect_provider".to_owned(),
                description: "Disconnect a fitness provider".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"provider": {"type": "string"}},
                    "required": ["provider"]
                })),
            },
        ]
    }

    /// Build activity data tool definitions
    fn build_activity_tools() -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "get_activities".to_owned(),
                description: "Get user's recent fitness activities".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "limit": {"type": "integer"},
                        "offset": {"type": "integer"}
                    },
                    "required": ["provider"]
                })),
            },
            FunctionDeclaration {
                name: "get_athlete".to_owned(),
                description: "Get user's athlete profile information".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"provider": {"type": "string"}},
                    "required": ["provider"]
                })),
            },
            FunctionDeclaration {
                name: "get_stats".to_owned(),
                description: "Get user's overall fitness statistics".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"provider": {"type": "string"}},
                    "required": ["provider"]
                })),
            },
        ]
    }

    /// Build analysis tool definitions
    fn build_analysis_tools() -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "analyze_activity".to_owned(),
                description: "Deep analysis of a specific activity".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "activity_id": {"type": "string"}
                    },
                    "required": ["provider", "activity_id"]
                })),
            },
            FunctionDeclaration {
                name: "get_activity_intelligence".to_owned(),
                description: "AI-powered insights including location and weather".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "activity_id": {"type": "string"},
                        "include_location": {"type": "boolean"},
                        "include_weather": {"type": "boolean"}
                    },
                    "required": ["provider", "activity_id"]
                })),
            },
            FunctionDeclaration {
                name: "analyze_performance_trends".to_owned(),
                description: "Analyze performance trends over time".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "timeframe": {"type": "string"},
                        "metric": {"type": "string"},
                        "sport_type": {"type": "string"}
                    },
                    "required": ["provider", "timeframe", "metric"]
                })),
            },
            FunctionDeclaration {
                name: "compare_activities".to_owned(),
                description: "Compare activity against similar or personal bests".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "activity_id": {"type": "string"},
                        "comparison_type": {"type": "string"}
                    },
                    "required": ["provider", "activity_id", "comparison_type"]
                })),
            },
            FunctionDeclaration {
                name: "calculate_fitness_score".to_owned(),
                description: "Calculate comprehensive fitness score".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "timeframe": {"type": "string"},
                        "sleep_provider": {"type": "string"}
                    },
                    "required": ["provider"]
                })),
            },
            FunctionDeclaration {
                name: "analyze_training_load".to_owned(),
                description: "Analyze training load and recovery needs".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "timeframe": {"type": "string"},
                        "sleep_provider": {"type": "string"}
                    },
                    "required": ["provider"]
                })),
            },
        ]
    }

    /// Build recovery and recommendation tool definitions
    fn build_recovery_tools() -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "suggest_rest_day".to_owned(),
                description: "AI recommendation for rest day".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "activity_provider": {"type": "string"},
                        "sleep_provider": {"type": "string"}
                    }
                })),
            },
            FunctionDeclaration {
                name: "generate_recommendations".to_owned(),
                description: "Get personalized training recommendations".to_owned(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "recommendation_type": {"type": "string"},
                        "activity_id": {"type": "string"}
                    },
                    "required": ["provider"]
                })),
            },
        ]
    }

    /// Build Gemini tool definitions from MCP tool registry
    fn build_mcp_tools() -> Tool {
        let mut declarations = Vec::with_capacity(14);
        declarations.extend(Self::build_connection_tools());
        declarations.extend(Self::build_activity_tools());
        declarations.extend(Self::build_analysis_tools());
        declarations.extend(Self::build_recovery_tools());
        Tool {
            function_declarations: declarations,
        }
    }

    /// Run the multi-turn tool execution loop with the LLM provider
    ///
    /// # Errors
    ///
    /// Returns error if LLM call fails or tool execution fails.
    async fn run_tool_loop(
        provider: &ChatProvider,
        executor: &UniversalExecutor,
        llm_messages: &mut Vec<ChatMessage>,
        tools: &Tool,
        model: &str,
        user_id: &str,
        tenant_id: TenantId,
    ) -> Result<ToolLoopResult, AppError> {
        // Track activity list across iterations (to prepend to final response)
        let mut captured_activity_list: Option<String> = None;

        for iteration in 0..MAX_TOOL_ITERATIONS {
            let llm_request = ChatRequest::new(llm_messages.clone()).with_model(model);
            let response = provider
                .complete_with_tools(&llm_request, Some(vec![tools.clone()]))
                .await?;

            // Check for function calls
            if let Some(ref function_calls) = response.function_calls {
                if !function_calls.is_empty() {
                    info!(
                        "Iteration {}: Executing {} tool calls",
                        iteration,
                        function_calls.len()
                    );

                    let function_responses =
                        Self::execute_function_calls(executor, function_calls, user_id, tenant_id)
                            .await?;

                    // Add assistant's text to messages if present (strip synthetic function syntax)
                    if let Some(ref text) = response.content {
                        let cleaned = strip_synthetic_function_calls(text);
                        if !cleaned.is_empty() {
                            llm_messages.push(ChatMessage::assistant(&*cleaned));
                        }
                    }

                    // Add function responses as user messages, capturing activity list if present
                    if let Some(list) =
                        Self::add_function_responses_to_messages(llm_messages, &function_responses)
                    {
                        captured_activity_list = Some(list);
                    }
                    continue;
                }
            }

            // No function calls - we have a text response (strip any synthetic function syntax)
            let content = response
                .content
                .map(|c| strip_synthetic_function_calls(&c).into_owned())
                .unwrap_or_default();
            return Ok(ToolLoopResult {
                content,
                usage: response.usage,
                finish_reason: response.finish_reason,
                activity_list: captured_activity_list,
            });
        }

        // Max iterations reached - return empty response
        Ok(ToolLoopResult {
            content: String::new(),
            usage: None,
            finish_reason: Some("max_iterations".to_owned()),
            activity_list: captured_activity_list,
        })
    }

    /// Execute a batch of function calls and return responses
    async fn execute_function_calls(
        executor: &UniversalExecutor,
        function_calls: &[FunctionCall],
        user_id: &str,
        tenant_id: TenantId,
    ) -> Result<Vec<FunctionResponse>, AppError> {
        let mut responses = Vec::with_capacity(function_calls.len());
        for function_call in function_calls {
            info!("Executing tool: {}", function_call.name);
            let tool_response =
                Self::execute_mcp_tool(executor, function_call, user_id, tenant_id).await;
            responses.push(Self::build_function_response(function_call, &tool_response));
        }
        Ok(responses)
    }

    /// Add function responses as user messages for next LLM iteration
    /// Returns the activity list if found (to prepend to final response)
    fn add_function_responses_to_messages(
        llm_messages: &mut Vec<ChatMessage>,
        function_responses: &[FunctionResponse],
    ) -> Option<String> {
        // Track activity list to return for prepending to final response
        let mut activity_list_content: Option<String> = None;

        for func_response in function_responses {
            let response_text =
                serde_json::to_string(&func_response.response).unwrap_or_else(|_| "{}".to_owned());

            // For get_activities, extract the activity_list to prepend to final response
            if func_response.name == "get_activities" {
                if let Some(activity_list) = func_response
                    .response
                    .get("activity_list")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    let list_len = activity_list.len();
                    activity_list_content = Some(activity_list.to_owned());
                    info!("Extracted activity list ({list_len} chars) to prepend to response");
                }
            }

            // All tool results use the same format
            let name = &func_response.name;
            let message = format!("[Tool Result for {name}]: {response_text}");
            llm_messages.push(ChatMessage::user(message));
        }

        // Return activity list for prepending to final response (guarantees user sees data)
        activity_list_content
    }

    /// Execute an MCP tool call and return the result
    /// Tool execution errors are converted to failed responses so the LLM can handle them gracefully
    async fn execute_mcp_tool(
        executor: &UniversalExecutor,
        function_call: &FunctionCall,
        user_id: &str,
        tenant_id: TenantId,
    ) -> UniversalResponse {
        let request = UniversalRequest {
            tool_name: function_call.name.clone(), // Ownership transfer for tool execution
            parameters: function_call.args.clone(), // Ownership transfer for parameters
            user_id: user_id.to_owned(),
            protocol: "chat".to_owned(),
            tenant_id: Some(tenant_id.to_string()),
            progress_token: None,
            cancellation_token: None,
            progress_reporter: None,
        };

        match executor.execute_tool(request).await {
            Ok(response) => response,
            Err(e) => {
                // Convert tool execution errors to failed responses
                // This allows the LLM to provide a helpful alternative response
                UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Tool execution failed: {e}")),
                    metadata: None,
                }
            }
        }
    }

    /// Build function response for Gemini from MCP tool response
    fn build_function_response(
        function_call: &FunctionCall,
        response: &UniversalResponse,
    ) -> FunctionResponse {
        let result_value = if response.success {
            response
                .result
                .clone() // Clone needed: returning owned data from reference
                .unwrap_or_else(|| serde_json::json!({"status": "success"}))
        } else {
            serde_json::json!({
                "error": response.error.as_deref().unwrap_or("Unknown error")
            })
        };

        FunctionResponse {
            name: function_call.name.clone(), // Clone needed: creating new struct from reference
            response: result_value,
        }
    }

    // ========================================================================
    // Conversation Handlers
    // ========================================================================

    /// Create a new conversation
    async fn create_conversation(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<CreateConversationRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        // Use model from request, or fall back to PIERRE_LLM_MODEL env var
        let model = match request.model.clone() {
            Some(m) => m,
            None => LlmProviderType::model_from_env().ok_or_else(|| {
                AppError::config(
                    "No model specified and PIERRE_LLM_MODEL environment variable not set",
                )
            })?,
        };

        let conv = resources
            .database
            .chat_create_conversation(
                &auth.user_id.to_string(),
                tenant_id,
                &request.title,
                &model,
                request.system_prompt.as_deref(),
            )
            .await?;

        let response = ConversationResponse {
            id: conv.id,
            title: conv.title,
            model: conv.model,
            system_prompt: conv.system_prompt,
            total_tokens: conv.total_tokens,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
        };

        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// List user's conversations
    async fn list_conversations(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<ListConversationsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let conversations = resources
            .database
            .chat_list_conversations(
                &auth.user_id.to_string(),
                tenant_id,
                query.limit,
                query.offset,
            )
            .await?;

        let total = conversations.len();
        let response = ConversationListResponse {
            conversations: conversations
                .into_iter()
                .map(|c| ConversationSummaryResponse {
                    id: c.id,
                    title: c.title,
                    model: c.model,
                    message_count: c.message_count,
                    total_tokens: c.total_tokens,
                    created_at: c.created_at,
                    updated_at: c.updated_at,
                })
                .collect(),
            total,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Get a specific conversation
    async fn get_conversation(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(conversation_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let conv = resources
            .database
            .chat_get_conversation(&conversation_id, &auth.user_id.to_string(), tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Conversation not found"))?;

        let response = ConversationResponse {
            id: conv.id,
            title: conv.title,
            model: conv.model,
            system_prompt: conv.system_prompt,
            total_tokens: conv.total_tokens,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Update a conversation title
    async fn update_conversation(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(conversation_id): Path<String>,
        Json(request): Json<UpdateConversationRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let updated = resources
            .database
            .chat_update_conversation_title(
                &conversation_id,
                &auth.user_id.to_string(),
                tenant_id,
                &request.title,
            )
            .await?;

        if !updated {
            return Err(AppError::not_found("Conversation not found"));
        }

        // Fetch and return the updated conversation (proper REST response)
        let conv = resources
            .database
            .chat_get_conversation(&conversation_id, &auth.user_id.to_string(), tenant_id)
            .await?
            .ok_or_else(|| AppError::internal("Conversation not found after update"))?;

        let response = ConversationResponse {
            id: conv.id,
            title: conv.title,
            model: conv.model,
            system_prompt: conv.system_prompt,
            total_tokens: conv.total_tokens,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Delete a conversation
    async fn delete_conversation(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(conversation_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let deleted = resources
            .database
            .chat_delete_conversation(&conversation_id, &auth.user_id.to_string(), tenant_id)
            .await?;

        if !deleted {
            return Err(AppError::not_found("Conversation not found"));
        }

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    // ========================================================================
    // Message Handlers
    // ========================================================================

    /// Get messages for a conversation
    async fn get_messages(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(conversation_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        // Verify user owns this conversation
        resources
            .database
            .chat_get_conversation(&conversation_id, &auth.user_id.to_string(), tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Conversation not found"))?;

        let messages = resources
            .database
            .chat_get_messages(&conversation_id, &auth.user_id.to_string())
            .await?;

        let messages_list: Vec<MessageResponse> = messages
            .into_iter()
            .map(|m| MessageResponse {
                id: m.id,
                role: m.role,
                content: m.content,
                token_count: m.token_count,
                created_at: m.created_at,
            })
            .collect();

        let response = MessagesListResponse {
            messages: messages_list,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Send a message and get a response (non-streaming) with MCP tool execution
    async fn send_message(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(conversation_id): Path<String>,
        Json(request): Json<SendMessageRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        // Get conversation to verify ownership and get model/system prompt
        let conv = resources
            .database
            .chat_get_conversation(&conversation_id, &auth.user_id.to_string(), tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Conversation not found"))?;

        // Save user message
        let user_msg = resources
            .database
            .chat_add_message(
                &conversation_id,
                &auth.user_id.to_string(),
                "user",
                &request.content,
                None,
                None,
            )
            .await?;

        // Get conversation history and build LLM messages with system prompt
        let history = resources
            .database
            .chat_get_messages(&conversation_id, &auth.user_id.to_string())
            .await?;

        // Check if this is an insight generation request
        // These use a dedicated prompt optimized for clean, shareable output
        let is_insight_request = request.content.starts_with(INSIGHT_PROMPT_PREFIX);

        let mut llm_messages = if is_insight_request {
            // For insight generation: use dedicated prompt and extract just the analysis
            let insight_prompt = get_insight_generation_prompt();

            // Extract the analysis content (everything after the prefix and colon/newlines)
            let analysis_content = request
                .content
                .strip_prefix(INSIGHT_PROMPT_PREFIX)
                .unwrap_or(&request.content)
                .trim_start_matches(':')
                .trim();

            // Build messages without history - insight generation is a single-turn task
            vec![
                ChatMessage::system(insight_prompt),
                ChatMessage::user(analysis_content),
            ]
        } else {
            // Normal conversation: use augmented system prompt with full history
            let system_prompt_text =
                Self::get_augmented_system_prompt(&conv, &resources, auth.user_id).await;
            Self::build_llm_messages(Some(system_prompt_text.as_str()), &history)
        };

        // Inject startup query if this is the first message in a coach conversation
        // (only for non-insight requests)
        // The startup query runs before the user's message to fetch relevant context
        if !is_insight_request {
            if let Some(startup_query) = Self::get_startup_query_if_applicable(
                &resources,
                history.len(),
                conv.system_prompt.as_ref(),
                tenant_id,
            )
            .await
            {
                // Insert startup query as user message right after system prompt
                // Position 1 is after system message (position 0) and before user's actual message
                llm_messages.insert(1, ChatMessage::user(&startup_query));
            }
        }

        // Build MCP tools for function calling
        let tools = Self::build_mcp_tools();

        // Get LLM provider
        let provider = Self::get_llm_provider().await?;

        // Create MCP executor for tool calls
        let executor = UniversalExecutor::new(resources.clone()); // Arc clone for executor creation

        // Track execution time for the entire LLM + tool loop
        let start_time = Instant::now();

        // Run multi-turn tool execution loop
        let result = Self::run_tool_loop(
            &provider,
            &executor,
            &mut llm_messages,
            &tools,
            &conv.model,
            &auth.user_id.to_string(),
            tenant_id,
        )
        .await?;

        // Safe cast: execution time will never exceed u64::MAX milliseconds (~584 million years)
        #[allow(clippy::cast_possible_truncation)]
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Calculate token count from usage
        let token_count = result.usage.map(|u| u.completion_tokens);

        // For insight requests, parse the JSON response to extract clean content
        let processed_content = if is_insight_request {
            parse_insight_json_response(&result.content)
        } else {
            result.content.clone()
        };

        // Prepend activity list to content if present (guarantees user sees formatted data)
        let final_content = if let Some(ref list) = result.activity_list {
            info!(
                "Prepending activity list ({} chars) to LLM response",
                list.len()
            );
            format!("{list}\n\n---\n\n**Analysis:**\n\n{processed_content}")
        } else {
            processed_content
        };

        // Save assistant response
        let assistant_msg = resources
            .database
            .chat_add_message(
                &conversation_id,
                &auth.user_id.to_string(),
                "assistant",
                &final_content,
                token_count,
                result.finish_reason.as_deref(),
            )
            .await?;

        // Get updated conversation for timestamp
        let updated_conv = resources
            .database
            .chat_get_conversation(&conversation_id, &auth.user_id.to_string(), tenant_id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to get updated conversation"))?;

        let response = ChatCompletionResponse {
            user_message: MessageResponse {
                id: user_msg.id,
                role: user_msg.role,
                content: user_msg.content,
                token_count: user_msg.token_count,
                created_at: user_msg.created_at,
            },
            assistant_message: MessageResponse {
                id: assistant_msg.id,
                role: assistant_msg.role,
                content: assistant_msg.content,
                token_count: assistant_msg.token_count,
                created_at: assistant_msg.created_at,
            },
            conversation_updated_at: updated_conv.updated_at,
            model: conv.model.clone(),
            execution_time_ms,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }
}
