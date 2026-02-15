// ABOUTME: Chat orchestration domain service for multi-step chat operations
// ABOUTME: Extracts conversation creation, message dispatch, and model validation from routes
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::LlmProviderType;
use crate::database::{ConversationRecord, MessageRecord};
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::models::TenantId;

/// Result of creating a new conversation, including validated model
pub struct CreateConversationResult {
    /// The created conversation record
    pub conversation: ConversationRecord,
}

/// Result of persisting a user message
pub struct UserMessageResult {
    /// The persisted user message
    pub message: MessageRecord,
    /// The conversation record (for model/`system_prompt` access)
    pub conversation: ConversationRecord,
}

/// Validate the model and create a conversation.
///
/// Business rules:
/// - Uses requested model if provided
/// - Falls back to `PIERRE_LLM_MODEL` environment variable
/// - Fails if no model can be determined
///
/// # Errors
///
/// Returns `AppError::Config` if no model is specified and `PIERRE_LLM_MODEL` is not set.
/// Returns database errors on conversation creation failure.
pub async fn create_conversation(
    database: &Database,
    user_id: &str,
    tenant_id: TenantId,
    title: &str,
    requested_model: Option<&str>,
    system_prompt: Option<&str>,
) -> AppResult<CreateConversationResult> {
    let model = match requested_model {
        Some(m) => m.to_owned(),
        None => LlmProviderType::model_from_env().ok_or_else(|| {
            AppError::config("No model specified and PIERRE_LLM_MODEL environment variable not set")
        })?,
    };

    let conversation = database
        .chat_create_conversation(user_id, tenant_id, title, &model, system_prompt)
        .await?;

    Ok(CreateConversationResult { conversation })
}

/// Verify conversation ownership and persist user message.
///
/// Business rules:
/// - Conversation must exist and belong to the user/tenant
/// - Message is persisted before LLM dispatch (crash-safe)
/// - Returns both message and conversation (for model/prompt access in LLM step)
///
/// # Errors
///
/// Returns `AppError::NotFound` if the conversation does not exist or belongs to another user.
/// Returns database errors on message persistence failure.
pub async fn persist_user_message(
    database: &Database,
    conversation_id: &str,
    user_id: &str,
    tenant_id: TenantId,
    content: &str,
) -> AppResult<UserMessageResult> {
    // Verify ownership and get conversation details
    let conversation = database
        .chat_get_conversation(conversation_id, user_id, tenant_id)
        .await?
        .ok_or_else(|| AppError::not_found("Conversation not found"))?;

    // Persist user message before LLM dispatch
    let message = database
        .chat_add_message(conversation_id, user_id, "user", content, None, None)
        .await?;

    Ok(UserMessageResult {
        message,
        conversation,
    })
}

/// Get conversation history for LLM context building.
///
/// Returns all messages in the conversation for the given user.
///
/// # Errors
///
/// Returns database errors on message retrieval failure.
pub async fn get_conversation_history(
    database: &Database,
    conversation_id: &str,
    user_id: &str,
) -> AppResult<Vec<MessageRecord>> {
    database.chat_get_messages(conversation_id, user_id).await
}

/// Persist the assistant's response message.
///
/// Called after LLM dispatch + tool execution completes.
/// Returns the persisted message record and updated conversation.
///
/// # Errors
///
/// Returns `AppError::Internal` if the conversation cannot be retrieved after saving.
/// Returns database errors on message persistence failure.
pub async fn persist_assistant_response(
    database: &Database,
    conversation_id: &str,
    user_id: &str,
    tenant_id: TenantId,
    content: &str,
    token_count: Option<u32>,
    finish_reason: Option<&str>,
) -> AppResult<(MessageRecord, ConversationRecord)> {
    let message = database
        .chat_add_message(
            conversation_id,
            user_id,
            "assistant",
            content,
            token_count,
            finish_reason,
        )
        .await?;

    let conversation = database
        .chat_get_conversation(conversation_id, user_id, tenant_id)
        .await?
        .ok_or_else(|| AppError::internal("Failed to get updated conversation"))?;

    Ok((message, conversation))
}
