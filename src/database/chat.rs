// ABOUTME: Database operations for AI chat conversations and messages
// ABOUTME: Handles CRUD operations with multi-tenant isolation and conversation history
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::{AppError, AppResult};
use crate::llm::MessageRole;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use pierre_core::models::TenantId;

// Re-export DTOs from pierre-core (canonical definitions)
pub use pierre_core::models::{ConversationRecord, ConversationSummary, MessageRecord};

// ============================================================================
// Chat Manager
// ============================================================================

/// Chat database operations manager
pub struct ChatManager {
    pool: SqlitePool,
}

impl ChatManager {
    /// Create a new chat manager
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Conversation Operations
    // ========================================================================

    /// Create a new conversation
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn create_conversation(
        &self,
        user_id: &str,
        tenant_id: TenantId,
        title: &str,
        model: &str,
        system_prompt: Option<&str>,
    ) -> AppResult<ConversationRecord> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO chat_conversations (id, user_id, tenant_id, title, model, system_prompt, total_tokens, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $7)
            ",
        )
        .bind(&id)
        .bind(user_id)
        .bind(tenant_id.to_string())
        .bind(title)
        .bind(model)
        .bind(system_prompt)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create conversation: {e}")))?;

        Ok(ConversationRecord {
            id,
            user_id: user_id.to_owned(),
            tenant_id: tenant_id.to_string(),
            title: title.to_owned(),
            model: model.to_owned(),
            system_prompt: system_prompt.map(ToOwned::to_owned),
            total_tokens: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Get a conversation by ID with tenant isolation
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: TenantId,
    ) -> AppResult<Option<ConversationRecord>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, model, system_prompt, total_tokens, created_at, updated_at
            FROM chat_conversations
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get conversation: {e}")))?;

        Ok(row.map(|r| ConversationRecord {
            id: r.get("id"),
            user_id: r.get("user_id"),
            tenant_id: r.get("tenant_id"),
            title: r.get("title"),
            model: r.get("model"),
            system_prompt: r.get("system_prompt"),
            total_tokens: r.get("total_tokens"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// List conversations for a user with pagination
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_conversations(
        &self,
        user_id: &str,
        tenant_id: TenantId,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<ConversationSummary>> {
        let rows = sqlx::query(
            r"
            SELECT c.id, c.title, c.model, c.total_tokens, c.created_at, c.updated_at,
                   COUNT(m.id) as message_count
            FROM chat_conversations c
            LEFT JOIN chat_messages m ON m.conversation_id = c.id
            WHERE c.user_id = $1 AND c.tenant_id = $2
            GROUP BY c.id
            ORDER BY c.updated_at DESC
            LIMIT $3 OFFSET $4
            ",
        )
        .bind(user_id)
        .bind(tenant_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list conversations: {e}")))?;

        let summaries = rows
            .into_iter()
            .map(|r| ConversationSummary {
                id: r.get("id"),
                title: r.get("title"),
                model: r.get("model"),
                message_count: r.get("message_count"),
                total_tokens: r.get("total_tokens"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        Ok(summaries)
    }

    /// Update conversation title
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_conversation_title(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: TenantId,
        title: &str,
    ) -> AppResult<bool> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r"
            UPDATE chat_conversations
            SET title = $1, updated_at = $2
            WHERE id = $3 AND user_id = $4 AND tenant_id = $5
            ",
        )
        .bind(title)
        .bind(&now)
        .bind(conversation_id)
        .bind(user_id)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update conversation title: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a conversation and all its messages (cascade)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn delete_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: TenantId,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM chat_conversations
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete conversation: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Add a message to a conversation
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn add_message(
        &self,
        conversation_id: &str,
        user_id: &str,
        role: MessageRole,
        content: &str,
        token_count: Option<u32>,
        finish_reason: Option<&str>,
    ) -> AppResult<MessageRecord> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let role_str = role.as_str();

        // Insert message only if the conversation belongs to the user
        let result = sqlx::query(
            r"
            INSERT INTO chat_messages (id, conversation_id, role, content, token_count, finish_reason, created_at)
            SELECT $1, $2, $3, $4, $5, $6, $7
            WHERE EXISTS (
                SELECT 1 FROM chat_conversations WHERE id = $2 AND user_id = $8
            )
            ",
        )
        .bind(&id)
        .bind(conversation_id)
        .bind(role_str)
        .bind(content)
        .bind(token_count.map(i64::from))
        .bind(finish_reason)
        .bind(&now)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to add message: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(
                "Conversation not found or access denied",
            ));
        }

        // Update conversation's updated_at and total_tokens (ownership already verified above)
        if let Some(tokens) = token_count {
            sqlx::query(
                r"
                UPDATE chat_conversations
                SET updated_at = $1, total_tokens = total_tokens + $2
                WHERE id = $3 AND user_id = $4
                ",
            )
            .bind(&now)
            .bind(i64::from(tokens))
            .bind(conversation_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to update conversation tokens: {e}"))
            })?;
        } else {
            sqlx::query(
                r"
                UPDATE chat_conversations
                SET updated_at = $1
                WHERE id = $2 AND user_id = $3
                ",
            )
            .bind(&now)
            .bind(conversation_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to update conversation timestamp: {e}"))
            })?;
        }

        Ok(MessageRecord {
            id,
            conversation_id: conversation_id.to_owned(),
            role: role_str.to_owned(),
            content: content.to_owned(),
            token_count: token_count.map(i64::from),
            finish_reason: finish_reason.map(ToOwned::to_owned),
            created_at: now,
        })
    }

    /// Get all messages for a conversation in chronological order
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<MessageRecord>> {
        let rows = sqlx::query(
            r"
            SELECT m.id, m.conversation_id, m.role, m.content, m.token_count, m.finish_reason, m.created_at
            FROM chat_messages m
            JOIN chat_conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1 AND c.user_id = $2
            ORDER BY m.created_at ASC
            ",
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get messages: {e}")))?;

        let messages = rows
            .into_iter()
            .map(|r| MessageRecord {
                id: r.get("id"),
                conversation_id: r.get("conversation_id"),
                role: r.get("role"),
                content: r.get("content"),
                token_count: r.get("token_count"),
                finish_reason: r.get("finish_reason"),
                created_at: r.get("created_at"),
            })
            .collect();

        Ok(messages)
    }

    /// Get the last N messages for a conversation (for context window)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_recent_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
        limit: i64,
    ) -> AppResult<Vec<MessageRecord>> {
        let rows = sqlx::query(
            r"
            SELECT m.id, m.conversation_id, m.role, m.content, m.token_count, m.finish_reason, m.created_at
            FROM chat_messages m
            JOIN chat_conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1 AND c.user_id = $2
            ORDER BY m.created_at DESC
            LIMIT $3
            ",
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get recent messages: {e}")))?;

        // Reverse to get chronological order
        let mut messages: Vec<MessageRecord> = rows
            .into_iter()
            .map(|r| MessageRecord {
                id: r.get("id"),
                conversation_id: r.get("conversation_id"),
                role: r.get("role"),
                content: r.get("content"),
                token_count: r.get("token_count"),
                finish_reason: r.get("finish_reason"),
                created_at: r.get("created_at"),
            })
            .collect();
        messages.reverse();

        Ok(messages)
    }

    /// Get message count for a conversation
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_message_count(&self, conversation_id: &str, user_id: &str) -> AppResult<i64> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count
            FROM chat_messages m
            JOIN chat_conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1 AND c.user_id = $2
            ",
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get message count: {e}")))?;

        Ok(row.get("count"))
    }

    /// Delete all conversations for a user (for account cleanup)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn delete_all_user_conversations(
        &self,
        user_id: &str,
        tenant_id: TenantId,
    ) -> AppResult<i64> {
        let result = sqlx::query(
            r"
            DELETE FROM chat_conversations
            WHERE user_id = $1 AND tenant_id = $2
            ",
        )
        .bind(user_id)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete user conversations: {e}")))?;

        #[allow(clippy::cast_possible_wrap)]
        Ok(result.rows_affected() as i64)
    }
}
