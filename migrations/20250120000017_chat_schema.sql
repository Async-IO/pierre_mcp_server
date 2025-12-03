-- ABOUTME: Chat conversations and messages schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates tables for multi-tenant AI chat functionality with conversation history

-- Chat Conversations Table
-- Stores metadata about each conversation
CREATE TABLE IF NOT EXISTS chat_conversations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    title TEXT NOT NULL,
    model TEXT NOT NULL DEFAULT 'gemini-2.0-flash-exp',
    system_prompt TEXT,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Chat Messages Table
-- Stores individual messages in conversations
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content TEXT NOT NULL,
    token_count INTEGER,
    finish_reason TEXT,
    created_at TEXT NOT NULL
);

-- Indexes for Chat Conversations
CREATE INDEX IF NOT EXISTS idx_chat_conversations_user ON chat_conversations(user_id);
CREATE INDEX IF NOT EXISTS idx_chat_conversations_tenant ON chat_conversations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_chat_conversations_user_tenant ON chat_conversations(user_id, tenant_id);
CREATE INDEX IF NOT EXISTS idx_chat_conversations_updated ON chat_conversations(updated_at DESC);

-- Indexes for Chat Messages
CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation ON chat_messages(conversation_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation_created ON chat_messages(conversation_id, created_at ASC);
