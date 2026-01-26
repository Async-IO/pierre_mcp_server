// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Chat domain API - conversations and messages
// ABOUTME: Handles CRUD for chat conversations and sending/receiving messages

import type { AxiosInstance } from 'axios';
import type { Conversation, Message } from '@pierre/shared-types';
import { ENDPOINTS } from '../core/endpoints';

// Re-export types for consumers
export type { Conversation, Message };

export interface ConversationsResponse {
  conversations: Conversation[];
  total: number;
  limit: number;
  offset: number;
}

export interface MessagesResponse {
  messages: Message[];
}

export interface SendMessageResponse {
  user_message: Message;
  assistant_message: Message;
  conversation_updated_at: string;
  model: string;
  execution_time_ms: number;
}

export interface CreateConversationOptions {
  title?: string;
  model?: string;
  system_prompt?: string;
  coach_id?: string;
}

/**
 * Creates the chat API methods bound to an axios instance.
 */
export function createChatApi(axios: AxiosInstance, getBaseUrl: () => string) {
  return {
    /**
     * List all conversations for the current user.
     */
    async getConversations(limit = 50, offset = 0): Promise<ConversationsResponse> {
      const response = await axios.get<ConversationsResponse>(
        `${ENDPOINTS.CHAT.CONVERSATIONS}?limit=${limit}&offset=${offset}`
      );
      return response.data;
    },

    /**
     * Create a new conversation.
     */
    async createConversation(options?: CreateConversationOptions): Promise<Conversation> {
      const response = await axios.post<Conversation>(
        ENDPOINTS.CHAT.CONVERSATIONS,
        options ?? {}
      );
      return response.data;
    },

    /**
     * Get a specific conversation by ID.
     */
    async getConversation(conversationId: string): Promise<Conversation> {
      const response = await axios.get<Conversation>(
        ENDPOINTS.CHAT.CONVERSATION(conversationId)
      );
      return response.data;
    },

    /**
     * Update a conversation (e.g., change title).
     */
    async updateConversation(
      conversationId: string,
      updates: { title?: string }
    ): Promise<Conversation> {
      const response = await axios.put<Conversation>(
        ENDPOINTS.CHAT.CONVERSATION(conversationId),
        updates
      );
      return response.data;
    },

    /**
     * Delete a conversation.
     */
    async deleteConversation(conversationId: string): Promise<void> {
      await axios.delete(ENDPOINTS.CHAT.CONVERSATION(conversationId));
    },

    /**
     * Get messages in a conversation.
     */
    async getConversationMessages(conversationId: string): Promise<MessagesResponse> {
      const response = await axios.get<MessagesResponse>(
        ENDPOINTS.CHAT.MESSAGES(conversationId)
      );
      return response.data;
    },

    /**
     * Send a message in a conversation.
     */
    async sendMessage(conversationId: string, content: string): Promise<SendMessageResponse> {
      const response = await axios.post<SendMessageResponse>(
        ENDPOINTS.CHAT.MESSAGES(conversationId),
        { content, stream: false }
      );
      return response.data;
    },

    /**
     * Get the WebSocket URL for real-time chat.
     */
    getWebSocketUrl(conversationId: string, token?: string): string {
      const baseUrl = getBaseUrl();
      const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws';
      const wsBaseUrl = baseUrl.replace(/^https?/, wsProtocol);
      const wsUrl = `${wsBaseUrl}/api/chat/conversations/${conversationId}/ws`;

      if (token) {
        return `${wsUrl}?token=${encodeURIComponent(token)}`;
      }
      return wsUrl;
    },
  };
}

export type ChatApi = ReturnType<typeof createChatApi>;
