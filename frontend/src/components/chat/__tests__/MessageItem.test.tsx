// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Tests for MessageItem component
// ABOUTME: Verifies action bar behavior, error state, and feedback states

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import MessageItem from '../MessageItem';
import type { Message, MessageMetadata } from '../types';

const mockAssistantMessage: Message = {
  id: 'msg-1',
  role: 'assistant',
  content: 'This is a test response from Pierre.',
  created_at: new Date().toISOString(),
};

const mockUserMessage: Message = {
  id: 'msg-2',
  role: 'user',
  content: 'Hello Pierre!',
  created_at: new Date().toISOString(),
};

const mockMetadata: MessageMetadata = {
  model: 'gemini-1.5-flash',
  executionTimeMs: 2500,
};

describe('MessageItem', () => {
  describe('basic rendering', () => {
    it('should render assistant message with Pierre avatar', () => {
      render(<MessageItem message={mockAssistantMessage} />);

      expect(screen.getByText('Pierre')).toBeInTheDocument();
      expect(screen.getByText('This is a test response from Pierre.')).toBeInTheDocument();
      expect(screen.getByAltText('Pierre')).toBeInTheDocument();
    });

    it('should render user message with user avatar', () => {
      render(<MessageItem message={mockUserMessage} />);

      expect(screen.getByText('You')).toBeInTheDocument();
      expect(screen.getByText('Hello Pierre!')).toBeInTheDocument();
    });

    it('should display metadata when provided', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          metadata={mockMetadata}
          onCopy={vi.fn()}
        />
      );

      expect(screen.getByText(/gemini-1.5-flash/)).toBeInTheDocument();
      expect(screen.getByText(/2\.5s/)).toBeInTheDocument();
    });
  });

  describe('action buttons', () => {
    it('should render action buttons for assistant insight messages', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          hasInsight={true}
          onCopy={vi.fn()}
          onShare={vi.fn()}
          onShareToFeed={vi.fn()}
          onThumbsUp={vi.fn()}
          onThumbsDown={vi.fn()}
          onRetry={vi.fn()}
        />
      );

      expect(screen.getByTitle('Copy message')).toBeInTheDocument();
      // Share buttons only appear for insight messages
      expect(screen.getByTitle('Share')).toBeInTheDocument();
      expect(screen.getByTitle('Share insight')).toBeInTheDocument();
      expect(screen.getByTitle('Good response')).toBeInTheDocument();
      expect(screen.getByTitle('Poor response')).toBeInTheDocument();
      expect(screen.getByTitle('Regenerate response')).toBeInTheDocument();
      // Lightbulb (Create Insight) should NOT appear for insight messages
      expect(screen.queryByTitle('Create shareable insight')).not.toBeInTheDocument();
    });

    it('should render Create Insight button for non-insight assistant messages', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          hasInsight={false}
          onCopy={vi.fn()}
          onCreateInsight={vi.fn()}
          onThumbsUp={vi.fn()}
          onThumbsDown={vi.fn()}
          onRetry={vi.fn()}
        />
      );

      expect(screen.getByTitle('Copy message')).toBeInTheDocument();
      // Create Insight button appears for non-insight messages
      expect(screen.getByTitle('Create shareable insight')).toBeInTheDocument();
      // Share buttons should NOT appear for non-insight messages
      expect(screen.queryByTitle('Share')).not.toBeInTheDocument();
      expect(screen.queryByTitle('Share insight')).not.toBeInTheDocument();
    });

    it('should not render action buttons for user messages', () => {
      render(
        <MessageItem
          message={mockUserMessage}
          onCopy={vi.fn()}
          onShare={vi.fn()}
        />
      );

      expect(screen.queryByTitle('Copy message')).not.toBeInTheDocument();
      expect(screen.queryByTitle('Share')).not.toBeInTheDocument();
    });

    it('should call onCopy when copy button is clicked', async () => {
      const user = userEvent.setup();
      const onCopy = vi.fn();

      render(
        <MessageItem
          message={mockAssistantMessage}
          onCopy={onCopy}
        />
      );

      await user.click(screen.getByTitle('Copy message'));
      expect(onCopy).toHaveBeenCalledTimes(1);
    });

    it('should call onShare when share button is clicked on insight message', async () => {
      const user = userEvent.setup();
      const onShare = vi.fn();

      render(
        <MessageItem
          message={mockAssistantMessage}
          hasInsight={true}
          onShare={onShare}
        />
      );

      await user.click(screen.getByTitle('Share'));
      expect(onShare).toHaveBeenCalledTimes(1);
    });

    it('should call onCreateInsight when lightbulb button is clicked', async () => {
      const user = userEvent.setup();
      const onCreateInsight = vi.fn();

      render(
        <MessageItem
          message={mockAssistantMessage}
          hasInsight={false}
          onCreateInsight={onCreateInsight}
        />
      );

      await user.click(screen.getByTitle('Create shareable insight'));
      expect(onCreateInsight).toHaveBeenCalledTimes(1);
    });

    it('should call onRetry when retry button is clicked', async () => {
      const user = userEvent.setup();
      const onRetry = vi.fn();

      render(
        <MessageItem
          message={mockAssistantMessage}
          onRetry={onRetry}
        />
      );

      await user.click(screen.getByTitle('Regenerate response'));
      expect(onRetry).toHaveBeenCalledTimes(1);
    });
  });

  describe('feedback states', () => {
    it('should show thumbs up as active when feedback is up', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          feedback="up"
          onThumbsUp={vi.fn()}
          onThumbsDown={vi.fn()}
        />
      );

      const thumbsUpButton = screen.getByTitle('Good response');
      expect(thumbsUpButton).toHaveClass('text-pierre-violet');
    });

    it('should show thumbs down as active when feedback is down', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          feedback="down"
          onThumbsUp={vi.fn()}
          onThumbsDown={vi.fn()}
        />
      );

      const thumbsDownButton = screen.getByTitle('Poor response');
      expect(thumbsDownButton).toHaveClass('text-red-500');
    });

    it('should show both buttons as inactive when feedback is null', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          feedback={null}
          onThumbsUp={vi.fn()}
          onThumbsDown={vi.fn()}
        />
      );

      const thumbsUpButton = screen.getByTitle('Good response');
      const thumbsDownButton = screen.getByTitle('Poor response');

      expect(thumbsUpButton).toHaveClass('text-zinc-500');
      expect(thumbsDownButton).toHaveClass('text-zinc-500');
    });
  });

  describe('error state', () => {
    it('should show only Retry button with label when isError is true', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          isError={true}
          onCopy={vi.fn()}
          onShare={vi.fn()}
          onThumbsUp={vi.fn()}
          onThumbsDown={vi.fn()}
          onRetry={vi.fn()}
        />
      );

      // Should show retry button with label
      expect(screen.getByText('Retry')).toBeInTheDocument();

      // Should NOT show other action buttons
      expect(screen.queryByTitle('Copy message')).not.toBeInTheDocument();
      expect(screen.queryByTitle('Share')).not.toBeInTheDocument();
      expect(screen.queryByTitle('Good response')).not.toBeInTheDocument();
      expect(screen.queryByTitle('Poor response')).not.toBeInTheDocument();
    });

    it('should apply error styling to content when isError is true', () => {
      render(
        <MessageItem
          message={mockAssistantMessage}
          isError={true}
          onRetry={vi.fn()}
        />
      );

      const contentDiv = screen.getByText('This is a test response from Pierre.').closest('div');
      expect(contentDiv).toHaveClass('text-red-400');
    });

    it('should call onRetry when error retry button is clicked', async () => {
      const user = userEvent.setup();
      const onRetry = vi.fn();

      render(
        <MessageItem
          message={mockAssistantMessage}
          isError={true}
          onRetry={onRetry}
        />
      );

      await user.click(screen.getByText('Retry'));
      expect(onRetry).toHaveBeenCalledTimes(1);
    });
  });

  describe('context prefix stripping', () => {
    it('should strip context prefix from message content', () => {
      const messageWithContext: Message = {
        id: 'msg-context',
        role: 'assistant',
        content: '[Context: User just connected Strava] Here is your analysis.',
        created_at: new Date().toISOString(),
      };

      render(<MessageItem message={messageWithContext} />);

      expect(screen.getByText('Here is your analysis.')).toBeInTheDocument();
      expect(screen.queryByText(/Context:/)).not.toBeInTheDocument();
    });
  });
});
