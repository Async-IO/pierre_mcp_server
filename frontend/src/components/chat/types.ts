// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Shared types for chat components
// ABOUTME: Centralizes type definitions used across chat-related components

import type { Conversation, Coach } from '@pierre/shared-types';

export interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  token_count?: number;
  created_at: string;
}

export interface ConversationListResponse {
  conversations: Conversation[];
  total: number;
}

export interface MessageMetadata {
  model: string;
  executionTimeMs: number;
}

export type MessageFeedback = 'up' | 'down' | null;

export interface OAuthNotification {
  provider: string;
  timestamp: number;
}

export interface DeleteConfirmation {
  id: string;
  title: string | null;
}

export interface CoachDeleteConfirmation {
  id: string;
  title: string;
}

export interface PendingCoachAction {
  prompt: string;
  systemPrompt?: string;
}

export interface CoachFormData {
  title: string;
  description: string;
  system_prompt: string;
  category: string;
}

export const DEFAULT_COACH_FORM_DATA: CoachFormData = {
  title: '',
  description: '',
  system_prompt: '',
  category: 'Training',
};

// Re-export for convenience
export type { Conversation, Coach };
