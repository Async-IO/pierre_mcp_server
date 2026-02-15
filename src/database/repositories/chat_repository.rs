// ABOUTME: Chat repository implementation for conversation and message management
// ABOUTME: Delegates to DatabaseProvider for tenant-scoped chat persistence
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::ChatRepository;
use crate::database::{ConversationRecord, ConversationSummary, DatabaseError, MessageRecord};
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use pierre_core::models::TenantId;

/// SQLite/PostgreSQL implementation of `ChatRepository`
pub struct ChatRepositoryImpl {
    db: Database,
}

impl ChatRepositoryImpl {
    /// Create a new `ChatRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ChatRepository for ChatRepositoryImpl {
    async fn create_conversation(
        &self,
        user_id: &str,
        tenant_id: TenantId,
        title: &str,
        model: &str,
        system_prompt: Option<&str>,
    ) -> Result<ConversationRecord, DatabaseError> {
        self.db
            .chat_create_conversation(user_id, tenant_id, title, model, system_prompt)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: TenantId,
    ) -> Result<Option<ConversationRecord>, DatabaseError> {
        self.db
            .chat_get_conversation(conversation_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_conversations(
        &self,
        user_id: &str,
        tenant_id: TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationSummary>, DatabaseError> {
        self.db
            .chat_list_conversations(user_id, tenant_id, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_conversation_title(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: TenantId,
        title: &str,
    ) -> Result<bool, DatabaseError> {
        self.db
            .chat_update_conversation_title(conversation_id, user_id, tenant_id, title)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: TenantId,
    ) -> Result<bool, DatabaseError> {
        self.db
            .chat_delete_conversation(conversation_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn add_message(
        &self,
        conversation_id: &str,
        user_id: &str,
        role: &str,
        content: &str,
        token_count: Option<u32>,
        finish_reason: Option<&str>,
    ) -> Result<MessageRecord, DatabaseError> {
        self.db
            .chat_add_message(conversation_id, user_id, role, content, token_count, finish_reason)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<Vec<MessageRecord>, DatabaseError> {
        self.db
            .chat_get_messages(conversation_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_recent_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<MessageRecord>, DatabaseError> {
        self.db
            .chat_get_recent_messages(conversation_id, user_id, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_message_count(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<i64, DatabaseError> {
        self.db
            .chat_get_message_count(conversation_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_all_user_conversations(
        &self,
        user_id: &str,
        tenant_id: TenantId,
    ) -> Result<i64, DatabaseError> {
        self.db
            .chat_delete_all_user_conversations(user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
