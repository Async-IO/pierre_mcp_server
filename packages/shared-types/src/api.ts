// ABOUTME: Shared TypeScript types for common API structures
// ABOUTME: Chat types, prompt suggestions, and common response patterns

// ========== CHAT TYPES ==========

/** A conversation in the chat system */
export interface Conversation {
  id: string;
  title: string | null;
  /** Model used for this conversation */
  model?: string;
  /** System prompt (custom system instructions) */
  system_prompt?: string;
  /** Coach ID if conversation uses a coach */
  coach_id?: string | null;
  /** Total tokens used in conversation */
  total_tokens?: number;
  /** Number of messages in conversation */
  message_count: number;
  /** When conversation was created */
  created_at: string;
  /** When conversation was last updated */
  updated_at: string;
  /** When last message was sent */
  last_message_at?: string | null;
}

/** A message in a conversation */
export interface Message {
  id: string;
  /** Conversation this message belongs to */
  conversation_id?: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  token_count?: number;
  created_at: string;
  /** Model used for assistant messages */
  model?: string;
  /** Execution time in milliseconds */
  execution_time_ms?: number;
  /** Error flag for failed message responses */
  isError?: boolean;
}

// ========== PROMPT SUGGESTIONS ==========

/** Activity pillar for prompt categorization */
export type ActivityPillar = 'activity' | 'nutrition' | 'recovery';

/** A category of prompt suggestions */
export interface PromptCategory {
  category_key: string;
  category_title: string;
  category_icon: string;
  pillar: ActivityPillar;
  prompts: string[];
}

/** Response for prompt suggestions */
export interface PromptSuggestionsResponse {
  categories: PromptCategory[];
  welcome_prompt: string;
  metadata: {
    timestamp: string;
    api_version: string;
  };
}

// ========== COMMON RESPONSE TYPES ==========

/** Standard API metadata */
export interface ApiMetadata {
  timestamp: string;
  api_version: string;
}

/** Standard paginated response structure */
export interface PaginatedResponse<T> {
  items: T[];
  next_cursor: string | null;
  has_more: boolean;
  metadata: ApiMetadata;
}

/** Standard list response structure */
export interface ListResponse<T> {
  items: T[];
  total: number;
  metadata: ApiMetadata;
}
