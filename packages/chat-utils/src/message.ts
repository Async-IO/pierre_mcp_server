// ABOUTME: Utility functions for message content processing
// ABOUTME: Text transformations and formatting for chat messages

/**
 * Regular expression to match internal context prefixes in messages.
 * Format: [Context:...] at the start of a message
 */
const CONTEXT_PREFIX_REGEX = /^\[Context:[^\]]*\]\s*/i;

/**
 * Strip internal context prefixes from message content before displaying to user.
 * Context prefixes are added by the system for internal use but should not be shown in UI.
 *
 * @param text - The message text that may contain a context prefix
 * @returns The text with any context prefix removed
 *
 * @example
 * stripContextPrefix("[Context:coach] Hello there") // "Hello there"
 * stripContextPrefix("Hello there") // "Hello there"
 */
export const stripContextPrefix = (text: string): string => {
  return text.replace(CONTEXT_PREFIX_REGEX, '');
};
