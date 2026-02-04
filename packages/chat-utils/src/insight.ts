// ABOUTME: Utility functions for insight detection and prompt generation
// ABOUTME: Identifies insight prompts and their corresponding assistant responses

import type { Message } from '@pierre/shared-types';
import { stripContextPrefix } from './message';

/**
 * The standard prefix used for insight generation prompts.
 * This is used as the detection marker for insight messages.
 */
export const INSIGHT_PROMPT_PREFIX = 'Create a shareable insight from this analysis';

/**
 * Check if a message content is an insight generation prompt.
 * Insight prompts are hidden from display in the chat UI.
 *
 * @param content - The message content to check
 * @returns true if the content starts with the insight prompt prefix
 */
export const isInsightPrompt = (content: string): boolean => {
  return content.startsWith(INSIGHT_PROMPT_PREFIX);
};

/**
 * Scan message history to find assistant messages that are insight responses.
 * An insight response is an assistant message that immediately follows an insight prompt.
 *
 * @param messages - Array of chat messages to scan
 * @returns Set of message IDs that are insight responses
 */
export const detectInsightMessages = (messages: Message[]): Set<string> => {
  const insightIds = new Set<string>();
  for (let i = 0; i < messages.length - 1; i++) {
    const currentMsg = messages[i];
    const nextMsg = messages[i + 1];
    // If current is an insight prompt (user) and next is assistant, mark it as insight
    if (
      currentMsg.role === 'user' &&
      isInsightPrompt(currentMsg.content) &&
      nextMsg.role === 'assistant'
    ) {
      insightIds.add(nextMsg.id);
    }
  }
  return insightIds;
};

/**
 * Generate a standardized insight generation prompt from message content.
 * This strips any context prefix from the content before creating the prompt.
 *
 * @param content - The message content to create an insight from
 * @returns The formatted insight prompt string
 */
export const createInsightPrompt = (content: string): string => {
  return `${INSIGHT_PROMPT_PREFIX}:\n\n${stripContextPrefix(content)}`;
};
