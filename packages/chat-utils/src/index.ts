// ABOUTME: Main entry point for @pierre/chat-utils package
// ABOUTME: Re-exports all chat utility functions for convenient importing

// Insight detection and generation utilities
export {
  INSIGHT_PROMPT_PREFIX,
  isInsightPrompt,
  detectInsightMessages,
  createInsightPrompt,
  extractInsightContent,
} from './insight';

// Message processing utilities
export { stripContextPrefix } from './message';
