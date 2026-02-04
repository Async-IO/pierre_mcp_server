// ABOUTME: Unit tests for insight detection and generation utilities
// ABOUTME: Tests isInsightPrompt, detectInsightMessages, createInsightPrompt functions

import { describe, it, expect } from 'vitest';
import {
  INSIGHT_PROMPT_PREFIX,
  isInsightPrompt,
  detectInsightMessages,
  createInsightPrompt,
  extractInsightContent,
} from '../src/insight';
import { stripContextPrefix } from '../src/message';
import type { Message } from '@pierre/shared-types';

describe('isInsightPrompt', () => {
  it('returns true for messages starting with the insight prompt prefix', () => {
    const content = 'Create a shareable insight from this analysis:\n\nYour training data shows...';
    expect(isInsightPrompt(content)).toBe(true);
  });

  it('returns true for exact prefix match', () => {
    const content = INSIGHT_PROMPT_PREFIX;
    expect(isInsightPrompt(content)).toBe(true);
  });

  it('returns true for prefix with colon and content', () => {
    const content = `${INSIGHT_PROMPT_PREFIX}:\n\nSome analysis content`;
    expect(isInsightPrompt(content)).toBe(true);
  });

  it('returns false for regular user messages', () => {
    expect(isInsightPrompt('How was my workout today?')).toBe(false);
    expect(isInsightPrompt('Show me my training stats')).toBe(false);
  });

  it('returns false for messages containing but not starting with the prefix', () => {
    const content = 'Please Create a shareable insight from this analysis';
    expect(isInsightPrompt(content)).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(isInsightPrompt('')).toBe(false);
  });

  it('returns false for similar but different prefix', () => {
    expect(isInsightPrompt('Create a shareable insight')).toBe(false);
    expect(isInsightPrompt('Create a shareable insight for')).toBe(false);
  });
});

describe('detectInsightMessages', () => {
  const createMessage = (
    id: string,
    role: 'user' | 'assistant' | 'system',
    content: string
  ): Message => ({
    id,
    role,
    content,
    created_at: new Date().toISOString(),
  });

  it('detects assistant messages that follow insight prompts', () => {
    const messages: Message[] = [
      createMessage('1', 'user', 'How was my run?'),
      createMessage('2', 'assistant', 'Your run was great!'),
      createMessage('3', 'user', `${INSIGHT_PROMPT_PREFIX}:\n\nYour run was great!`),
      createMessage('4', 'assistant', 'Here is your shareable insight...'),
    ];

    const insightIds = detectInsightMessages(messages);
    expect(insightIds.size).toBe(1);
    expect(insightIds.has('4')).toBe(true);
    expect(insightIds.has('2')).toBe(false);
  });

  it('detects multiple insight messages', () => {
    const messages: Message[] = [
      createMessage('1', 'user', `${INSIGHT_PROMPT_PREFIX}:\n\nFirst analysis`),
      createMessage('2', 'assistant', 'First insight'),
      createMessage('3', 'user', 'Regular question'),
      createMessage('4', 'assistant', 'Regular answer'),
      createMessage('5', 'user', `${INSIGHT_PROMPT_PREFIX}:\n\nSecond analysis`),
      createMessage('6', 'assistant', 'Second insight'),
    ];

    const insightIds = detectInsightMessages(messages);
    expect(insightIds.size).toBe(2);
    expect(insightIds.has('2')).toBe(true);
    expect(insightIds.has('6')).toBe(true);
    expect(insightIds.has('4')).toBe(false);
  });

  it('returns empty set when no insight prompts exist', () => {
    const messages: Message[] = [
      createMessage('1', 'user', 'Hello'),
      createMessage('2', 'assistant', 'Hi there!'),
    ];

    const insightIds = detectInsightMessages(messages);
    expect(insightIds.size).toBe(0);
  });

  it('returns empty set for empty messages array', () => {
    const insightIds = detectInsightMessages([]);
    expect(insightIds.size).toBe(0);
  });

  it('does not detect insight if followed by user message instead of assistant', () => {
    const messages: Message[] = [
      createMessage('1', 'user', `${INSIGHT_PROMPT_PREFIX}:\n\nAnalysis`),
      createMessage('2', 'user', 'Another user message'),
    ];

    const insightIds = detectInsightMessages(messages);
    expect(insightIds.size).toBe(0);
  });

  it('does not detect insight for non-user insight prompts', () => {
    const messages: Message[] = [
      createMessage('1', 'system', `${INSIGHT_PROMPT_PREFIX}:\n\nSystem prompt`),
      createMessage('2', 'assistant', 'Response'),
    ];

    const insightIds = detectInsightMessages(messages);
    expect(insightIds.size).toBe(0);
  });

  it('handles last message being an insight prompt without response', () => {
    const messages: Message[] = [
      createMessage('1', 'user', 'Hello'),
      createMessage('2', 'assistant', 'Hi!'),
      createMessage('3', 'user', `${INSIGHT_PROMPT_PREFIX}:\n\nAnalysis`),
    ];

    const insightIds = detectInsightMessages(messages);
    expect(insightIds.size).toBe(0);
  });
});

describe('createInsightPrompt', () => {
  it('creates a prompt with the standard prefix', () => {
    const content = 'Your training data shows improvement';
    const prompt = createInsightPrompt(content);
    expect(prompt).toBe(`${INSIGHT_PROMPT_PREFIX}:\n\n${content}`);
    expect(isInsightPrompt(prompt)).toBe(true);
  });

  it('strips context prefix before creating prompt', () => {
    const content = '[Context:training] Your workout was intense';
    const prompt = createInsightPrompt(content);
    expect(prompt).toBe(`${INSIGHT_PROMPT_PREFIX}:\n\nYour workout was intense`);
    expect(prompt).not.toContain('[Context:');
  });

  it('handles content without context prefix', () => {
    const content = 'Plain message content';
    const prompt = createInsightPrompt(content);
    expect(prompt).toBe(`${INSIGHT_PROMPT_PREFIX}:\n\nPlain message content`);
  });

  it('creates valid insight prompt that passes detection', () => {
    const prompt = createInsightPrompt('Some analysis');
    expect(isInsightPrompt(prompt)).toBe(true);
  });
});

describe('stripContextPrefix', () => {
  it('removes context prefix from message', () => {
    const text = '[Context:training] Hello there';
    expect(stripContextPrefix(text)).toBe('Hello there');
  });

  it('handles different context types', () => {
    expect(stripContextPrefix('[Context:coach] Message')).toBe('Message');
    expect(stripContextPrefix('[Context:nutrition] Food advice')).toBe('Food advice');
  });

  it('is case insensitive', () => {
    expect(stripContextPrefix('[CONTEXT:test] Message')).toBe('Message');
    expect(stripContextPrefix('[context:test] Message')).toBe('Message');
  });

  it('preserves text without context prefix', () => {
    const text = 'Regular message';
    expect(stripContextPrefix(text)).toBe('Regular message');
  });

  it('only removes prefix at start of string', () => {
    const text = 'Message with [Context:test] in middle';
    expect(stripContextPrefix(text)).toBe('Message with [Context:test] in middle');
  });

  it('handles empty string', () => {
    expect(stripContextPrefix('')).toBe('');
  });

  it('handles whitespace after prefix', () => {
    // Regex strips trailing whitespace after the prefix bracket
    expect(stripContextPrefix('[Context:x]  Double space')).toBe('Double space');
    expect(stripContextPrefix('[Context:x]\nNewline')).toBe('Newline');
  });
});

describe('INSIGHT_PROMPT_PREFIX', () => {
  it('has the expected value', () => {
    expect(INSIGHT_PROMPT_PREFIX).toBe('Create a shareable insight from this analysis');
  });

  it('is used consistently by isInsightPrompt', () => {
    expect(isInsightPrompt(INSIGHT_PROMPT_PREFIX)).toBe(true);
    expect(isInsightPrompt(`${INSIGHT_PROMPT_PREFIX}: content`)).toBe(true);
  });
});

describe('extractInsightContent', () => {
  it('strips standard intro phrase from insight content', () => {
    const content = "Here's a shareable insight from your recent training analysis:\n\n\"Great job!\"";
    expect(extractInsightContent(content)).toBe('"Great job!"');
  });

  it('strips shorter intro phrase', () => {
    const content = "Here's a shareable insight:\n\nYour training is on track.";
    expect(extractInsightContent(content)).toBe('Your training is on track.');
  });

  it('strips "your shareable insight" variant', () => {
    const content = "Here's your shareable insight:\n\nKeep up the great work!";
    expect(extractInsightContent(content)).toBe('Keep up the great work!');
  });

  it('handles different activity types in intro', () => {
    const runContent = "Here's a shareable insight from your recent run analysis:\n\nYou ran well.";
    expect(extractInsightContent(runContent)).toBe('You ran well.');

    const workoutContent = "Here's a shareable insight from your recent workout:\n\nGreat session!";
    expect(extractInsightContent(workoutContent)).toBe('Great session!');
  });

  it('is case insensitive', () => {
    const content = "HERE'S A SHAREABLE INSIGHT FROM YOUR RECENT TRAINING ANALYSIS:\n\nGood job!";
    expect(extractInsightContent(content)).toBe('Good job!');
  });

  it('preserves content without intro text', () => {
    const content = '"Great job maintaining a balanced training load!"';
    expect(extractInsightContent(content)).toBe('"Great job maintaining a balanced training load!"');
  });

  it('handles empty string', () => {
    expect(extractInsightContent('')).toBe('');
  });

  it('handles content that is only whitespace', () => {
    expect(extractInsightContent('   ')).toBe('');
  });

  it('trims whitespace from result', () => {
    const content = "Here's a shareable insight:\n\n   Your insight text.   ";
    expect(extractInsightContent(content)).toBe('Your insight text.');
  });

  it('handles actual insight content format', () => {
    const content = `Here's a shareable insight from your recent training analysis:

"Great job maintaining a balanced training load! You're in a 'Building Base' phase, demonstrating impressive consistency with significant volume increases over the past few weeks, while keeping your overtraining risk low. Keep up the excellent work! To continue progressing, aim to gradually increase your weekly training stress score (TSS) by 3-5 points. Consider a light taper if you have any upcoming events!"`;

    const expected = `"Great job maintaining a balanced training load! You're in a 'Building Base' phase, demonstrating impressive consistency with significant volume increases over the past few weeks, while keeping your overtraining risk low. Keep up the excellent work! To continue progressing, aim to gradually increase your weekly training stress score (TSS) by 3-5 points. Consider a light taper if you have any upcoming events!"`;

    expect(extractInsightContent(content)).toBe(expected);
  });
});
