// ABOUTME: Chat route handlers for AI conversation management
// ABOUTME: Provides REST endpoints for creating, listing, and messaging in chat conversations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Chat routes for AI conversations
//!
//! This module handles chat conversation management including creating conversations,
//! sending messages, and streaming AI responses. All handlers require JWT authentication.

use crate::{
    database::ChatManager,
    database_plugins::DatabaseProvider,
    errors::AppError,
    llm::{
        get_pierre_system_prompt, ChatMessage, ChatRequest, FunctionCall, FunctionDeclaration,
        FunctionResponse, GeminiProvider, LlmProvider, Tool,
    },
    mcp::resources::ServerResources,
    protocols::universal::{UniversalExecutor, UniversalRequest, UniversalResponse},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, post, put},
    Json, Router,
};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, convert::Infallible, sync::Arc};
use tokio_stream::StreamExt;
use tracing::info;

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of tool call iterations before forcing a text response
const MAX_TOOL_ITERATIONS: usize = 10;

// ============================================================================
// Internal Types
// ============================================================================

/// Result of running the multi-turn tool execution loop
struct ToolLoopResult {
    /// Final text content from LLM
    content: String,
    /// Token usage statistics if available
    usage: Option<crate::llm::TokenUsage>,
    /// Finish reason if available
    finish_reason: Option<String>,
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a new conversation
#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    /// Conversation title
    pub title: String,
    /// LLM model to use (optional, defaults to gemini-2.5-flash)
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
            .route(
                "/api/chat/conversations/:conversation_id/messages",
                post(Self::send_message),
            )
            // Streaming endpoint
            .route(
                "/api/chat/conversations/:conversation_id/stream",
                post(Self::send_message_stream),
            )
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &axum::http::HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<crate::auth::AuthResult, AppError> {
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) =
                crate::security::cookies::get_cookie_value(headers, "auth_token")
            {
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
        user_id: uuid::Uuid,
        resources: &Arc<ServerResources>,
    ) -> Result<String, AppError> {
        let user = resources.database.get_user(user_id).await?;
        Ok(user
            .and_then(|u| u.tenant_id)
            .unwrap_or_else(|| user_id.to_string()))
    }

    /// Create a `ChatManager` from server resources
    fn create_chat_manager(resources: &ServerResources) -> Result<ChatManager, AppError> {
        let pool = resources
            .database
            .sqlite_pool()
            .ok_or_else(|| AppError::internal("Chat feature requires SQLite database"))?
            .clone();
        Ok(ChatManager::new(pool))
    }

    /// Get LLM provider (currently only Gemini)
    fn get_llm_provider() -> Result<GeminiProvider, AppError> {
        GeminiProvider::from_env()
    }

    /// Build LLM messages from conversation history and optional system prompt
    fn build_llm_messages(
        system_prompt: Option<&str>,
        history: &[crate::database::MessageRecord],
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

    /// Run the multi-turn tool execution loop with Gemini
    ///
    /// # Errors
    ///
    /// Returns error if LLM call fails or tool execution fails.
    async fn run_tool_loop(
        provider: &GeminiProvider,
        executor: &UniversalExecutor,
        llm_messages: &mut Vec<ChatMessage>,
        tools: &Tool,
        model: &str,
        user_id: &str,
        tenant_id: &str,
    ) -> Result<ToolLoopResult, AppError> {
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

                    // Add assistant's text to messages if present
                    if let Some(ref text) = response.content {
                        if !text.is_empty() {
                            llm_messages.push(ChatMessage::assistant(text));
                        }
                    }

                    // Add function responses as user messages
                    Self::add_function_responses_to_messages(llm_messages, &function_responses);
                    continue;
                }
            }

            // No function calls - we have a text response
            return Ok(ToolLoopResult {
                content: response.content.unwrap_or_default(),
                usage: response.usage,
                finish_reason: response.finish_reason,
            });
        }

        // Max iterations reached - return empty response
        Ok(ToolLoopResult {
            content: String::new(),
            usage: None,
            finish_reason: Some("max_iterations".to_owned()),
        })
    }

    /// Execute a batch of function calls and return responses
    async fn execute_function_calls(
        executor: &UniversalExecutor,
        function_calls: &[FunctionCall],
        user_id: &str,
        tenant_id: &str,
    ) -> Result<Vec<FunctionResponse>, AppError> {
        let mut responses = Vec::with_capacity(function_calls.len());
        for function_call in function_calls {
            info!("Executing tool: {}", function_call.name);
            let tool_response =
                Self::execute_mcp_tool(executor, function_call, user_id, tenant_id).await?;
            responses.push(Self::build_function_response(function_call, &tool_response));
        }
        Ok(responses)
    }

    /// Add function responses as user messages for next LLM iteration
    fn add_function_responses_to_messages(
        llm_messages: &mut Vec<ChatMessage>,
        function_responses: &[FunctionResponse],
    ) {
        for func_response in function_responses {
            let response_text =
                serde_json::to_string(&func_response.response).unwrap_or_else(|_| "{}".to_owned());
            llm_messages.push(ChatMessage::user(format!(
                "[Tool Result for {}]: {}",
                func_response.name, response_text
            )));
        }
    }

    /// Execute an MCP tool call and return the result
    async fn execute_mcp_tool(
        executor: &UniversalExecutor,
        function_call: &FunctionCall,
        user_id: &str,
        tenant_id: &str,
    ) -> Result<UniversalResponse, AppError> {
        let request = UniversalRequest {
            tool_name: function_call.name.clone(), // Ownership transfer for tool execution
            parameters: function_call.args.clone(), // Ownership transfer for parameters
            user_id: user_id.to_owned(),
            protocol: "chat".to_owned(),
            tenant_id: Some(tenant_id.to_owned()),
            progress_token: None,
            cancellation_token: None,
            progress_reporter: None,
        };

        executor
            .execute_tool(request)
            .await
            .map_err(|e| AppError::internal(format!("Tool execution failed: {e}")))
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
        headers: axum::http::HeaderMap,
        Json(request): Json<CreateConversationRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let model = request.model.as_deref().unwrap_or("gemini-2.5-flash");
        let chat_manager = Self::create_chat_manager(&resources)?;

        let conv = chat_manager
            .create_conversation(
                &auth.user_id.to_string(),
                &tenant_id,
                &request.title,
                model,
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
        headers: axum::http::HeaderMap,
        Query(query): Query<ListConversationsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let chat_manager = Self::create_chat_manager(&resources)?;

        let conversations = chat_manager
            .list_conversations(
                &auth.user_id.to_string(),
                &tenant_id,
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
        headers: axum::http::HeaderMap,
        Path(conversation_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let chat_manager = Self::create_chat_manager(&resources)?;

        let conv = chat_manager
            .get_conversation(&conversation_id, &auth.user_id.to_string(), &tenant_id)
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
        headers: axum::http::HeaderMap,
        Path(conversation_id): Path<String>,
        Json(request): Json<UpdateConversationRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let chat_manager = Self::create_chat_manager(&resources)?;

        let updated = chat_manager
            .update_conversation_title(
                &conversation_id,
                &auth.user_id.to_string(),
                &tenant_id,
                &request.title,
            )
            .await?;

        if !updated {
            return Err(AppError::not_found("Conversation not found"));
        }

        Ok((StatusCode::OK, Json(serde_json::json!({"success": true}))).into_response())
    }

    /// Delete a conversation
    async fn delete_conversation(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(conversation_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let chat_manager = Self::create_chat_manager(&resources)?;

        let deleted = chat_manager
            .delete_conversation(&conversation_id, &auth.user_id.to_string(), &tenant_id)
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
        headers: axum::http::HeaderMap,
        Path(conversation_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let chat_manager = Self::create_chat_manager(&resources)?;

        // Verify user owns this conversation
        chat_manager
            .get_conversation(&conversation_id, &auth.user_id.to_string(), &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Conversation not found"))?;

        let messages = chat_manager.get_messages(&conversation_id).await?;

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
        headers: axum::http::HeaderMap,
        Path(conversation_id): Path<String>,
        Json(request): Json<SendMessageRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let chat_manager = Self::create_chat_manager(&resources)?;

        // Get conversation to verify ownership and get model/system prompt
        let conv = chat_manager
            .get_conversation(&conversation_id, &auth.user_id.to_string(), &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Conversation not found"))?;

        // Save user message
        let user_msg = chat_manager
            .add_message(
                &conversation_id,
                crate::llm::MessageRole::User,
                &request.content,
                None,
                None,
            )
            .await?;

        // Get conversation history and build LLM messages with Pierre system prompt
        let history = chat_manager.get_messages(&conversation_id).await?;
        let system_prompt: Cow<'_, str> = conv.system_prompt.as_ref().map_or_else(
            || Cow::Borrowed(get_pierre_system_prompt()),
            |s| Cow::Borrowed(s.as_str()),
        );
        let mut llm_messages = Self::build_llm_messages(Some(&system_prompt), &history);

        // Build MCP tools for function calling
        let tools = Self::build_mcp_tools();

        // Get LLM provider
        let provider = Self::get_llm_provider()?;

        // Create MCP executor for tool calls
        let executor = UniversalExecutor::new(resources.clone()); // Arc clone for executor creation

        // Run multi-turn tool execution loop
        let result = Self::run_tool_loop(
            &provider,
            &executor,
            &mut llm_messages,
            &tools,
            &conv.model,
            &auth.user_id.to_string(),
            &tenant_id,
        )
        .await?;

        // Calculate token count from usage
        let token_count = result.usage.map(|u| u.completion_tokens);

        // Save assistant response
        let assistant_msg = chat_manager
            .add_message(
                &conversation_id,
                crate::llm::MessageRole::Assistant,
                &result.content,
                token_count,
                result.finish_reason.as_deref(),
            )
            .await?;

        // Get updated conversation for timestamp
        let updated_conv = chat_manager
            .get_conversation(&conversation_id, &auth.user_id.to_string(), &tenant_id)
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
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Send a message and stream the response via SSE
    async fn send_message_stream(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(conversation_id): Path<String>,
        Json(request): Json<SendMessageRequest>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let tenant_id = Self::get_tenant_id(auth.user_id, &resources).await?;

        let chat_manager = Self::create_chat_manager(&resources)?;

        // Get conversation to verify ownership and get model/system prompt
        let conv = chat_manager
            .get_conversation(&conversation_id, &auth.user_id.to_string(), &tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Conversation not found"))?;

        // Save user message
        let user_msg = chat_manager
            .add_message(
                &conversation_id,
                crate::llm::MessageRole::User,
                &request.content,
                None,
                None,
            )
            .await?;

        // Get conversation history and build LLM messages
        let history = chat_manager.get_messages(&conversation_id).await?;
        let llm_messages = Self::build_llm_messages(conv.system_prompt.as_deref(), &history);

        // Get LLM streaming response
        let provider = Self::get_llm_provider()?;
        let llm_request = ChatRequest::new(llm_messages)
            .with_model(&conv.model)
            .with_streaming();

        let mut llm_stream = provider.complete_stream(&llm_request).await?;

        // Create stream for SSE
        // Clone values needed for the async block
        let conv_id = conversation_id.clone();
        let pool = resources
            .database
            .sqlite_pool()
            .ok_or_else(|| AppError::internal("Chat feature requires SQLite database"))?
            .clone();

        let stream = async_stream::stream! {
            let mut full_content = String::new();
            let mut finish_reason = None;

            // Send user message event first
            let user_event = serde_json::json!({
                "type": "user_message",
                "message": {
                    "id": user_msg.id,
                    "role": "user",
                    "content": user_msg.content,
                    "created_at": user_msg.created_at
                }
            });
            yield Ok(Event::default().data(user_event.to_string()));

            // Stream chunks
            while let Some(chunk_result) = llm_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        full_content.push_str(&chunk.delta);

                        let chunk_event = serde_json::json!({
                            "type": "chunk",
                            "delta": chunk.delta,
                            "is_final": chunk.is_final
                        });
                        yield Ok(Event::default().data(chunk_event.to_string()));

                        if chunk.is_final {
                            finish_reason = chunk.finish_reason;
                        }
                    }
                    Err(e) => {
                        let error_event = serde_json::json!({
                            "type": "error",
                            "message": e.to_string()
                        });
                        yield Ok(Event::default().data(error_event.to_string()));
                        return;
                    }
                }
            }

            // Save complete assistant message
            let chat_mgr = ChatManager::new(pool);
            match chat_mgr.add_message(
                &conv_id,
                crate::llm::MessageRole::Assistant,
                &full_content,
                None, // We don't have token count from streaming
                finish_reason.as_deref(),
            ).await {
                Ok(assistant_msg) => {
                    let done_event = serde_json::json!({
                        "type": "done",
                        "message": {
                            "id": assistant_msg.id,
                            "role": "assistant",
                            "content": full_content,
                            "created_at": assistant_msg.created_at
                        }
                    });
                    yield Ok(Event::default().data(done_event.to_string()));
                }
                Err(e) => {
                    let error_event = serde_json::json!({
                        "type": "error",
                        "message": format!("Failed to save message: {e}")
                    });
                    yield Ok(Event::default().data(error_event.to_string()));
                }
            }
        };

        Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
    }
}
