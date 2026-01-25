// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Chat/Conversations API methods - create, list, update, delete conversations and messages
// ABOUTME: Supports the AI chat interface with conversation history

import { axios } from './client';

export const chatApi = {
  async getConversations(limit: number = 50, offset: number = 0): Promise<{
    conversations: Array<{
      id: string;
      title: string;
      model: string;
      system_prompt?: string;
      total_tokens: number;
      message_count: number;
      created_at: string;
      updated_at: string;
    }>;
    total: number;
    limit: number;
    offset: number;
  }> {
    const response = await axios.get(`/api/chat/conversations?limit=${limit}&offset=${offset}`);
    return response.data;
  },

  async createConversation(data: {
    title: string;
    model?: string;
    system_prompt?: string;
  }): Promise<{
    id: string;
    title: string;
    model: string;
    system_prompt?: string;
    total_tokens: number;
    created_at: string;
    updated_at: string;
  }> {
    const response = await axios.post('/api/chat/conversations', data);
    return response.data;
  },

  async getConversation(conversationId: string): Promise<{
    id: string;
    title: string;
    model: string;
    system_prompt?: string;
    total_tokens: number;
    message_count: number;
    created_at: string;
    updated_at: string;
  }> {
    const response = await axios.get(`/api/chat/conversations/${conversationId}`);
    return response.data;
  },

  async updateConversation(conversationId: string, data: {
    title?: string;
  }): Promise<{
    id: string;
    title: string;
    model: string;
    system_prompt?: string;
    total_tokens: number;
    created_at: string;
    updated_at: string;
  }> {
    const response = await axios.put(`/api/chat/conversations/${conversationId}`, data);
    return response.data;
  },

  async deleteConversation(conversationId: string): Promise<void> {
    await axios.delete(`/api/chat/conversations/${conversationId}`);
  },

  async getConversationMessages(conversationId: string): Promise<{
    messages: Array<{
      id: string;
      role: 'user' | 'assistant' | 'system';
      content: string;
      token_count?: number;
      created_at: string;
    }>;
  }> {
    const response = await axios.get(`/api/chat/conversations/${conversationId}/messages`);
    return response.data;
  },
};
