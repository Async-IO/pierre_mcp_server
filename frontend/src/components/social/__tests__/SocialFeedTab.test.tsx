// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unit tests for SocialFeedTab component
// ABOUTME: Tests feed display, reactions, share modal, and adapt functionality

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import SocialFeedTab from '../SocialFeedTab';
import { socialApi } from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  socialApi: {
    getFeed: vi.fn(),
    getInsightSuggestions: vi.fn(),
    addReaction: vi.fn(),
    removeReaction: vi.fn(),
    shareInsight: vi.fn(),
    adaptInsight: vi.fn(),
  },
}));

const mockFeedItems = {
  items: [
    {
      insight: {
        id: 'insight-1',
        user_id: 'user-2',
        visibility: 'friends_only',
        insight_type: 'achievement',
        sport_type: 'Running',
        content: 'Just completed my first marathon training block! Feeling strong.',
        title: 'Marathon Ready',
        training_phase: 'build',
        reaction_count: 5,
        adapt_count: 2,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        expires_at: null,
      },
      author: {
        user_id: 'user-2',
        display_name: 'Jane Doe',
        email: 'jane@example.com',
      },
      reactions: {
        like: 3,
        celebrate: 2,
        inspire: 0,
        support: 0,
        total: 5,
      },
      user_reaction: null,
      user_has_adapted: false,
    },
  ],
  next_cursor: null,
  has_more: false,
  metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
};

describe('SocialFeedTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(socialApi.getFeed).mockResolvedValue(mockFeedItems);
    vi.mocked(socialApi.addReaction).mockResolvedValue({
      reaction: {
        id: 'reaction-1',
        insight_id: 'insight-1',
        user_id: 'user-1',
        reaction_type: 'like',
        created_at: '2024-01-01T00:00:00Z',
      },
      updated_counts: {
        like: 4,
        celebrate: 2,
        inspire: 0,
        support: 0,
        total: 6,
      },
      metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
    });
    vi.mocked(socialApi.removeReaction).mockResolvedValue(undefined);
  });

  it('should render the Social Feed tab with title', async () => {
    render(<SocialFeedTab />);

    expect(screen.getByText('Social Feed')).toBeInTheDocument();
    expect(screen.getByText('Coach insights from your friends')).toBeInTheDocument();
  });

  it('should display feed items on mount', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByText('Marathon Ready')).toBeInTheDocument();
    });

    expect(screen.getByText('Just completed my first marathon training block! Feeling strong.')).toBeInTheDocument();
    expect(screen.getByText('Jane Doe')).toBeInTheDocument();
    expect(socialApi.getFeed).toHaveBeenCalled();
  });

  it('should show empty state when no feed items', async () => {
    vi.mocked(socialApi.getFeed).mockResolvedValue({
      items: [],
      next_cursor: null,
      has_more: false,
      metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
    });

    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByText('Your feed is empty')).toBeInTheDocument();
    });
  });

  it('should display insight type badge', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByText('Achievement')).toBeInTheDocument();
    });
  });

  it('should display context badges for sport type and training phase', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByText('Running')).toBeInTheDocument();
      expect(screen.getByText('build phase')).toBeInTheDocument();
    });
  });

  it('should show Share Insight button', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Share Insight/i })).toBeInTheDocument();
    });
  });

  it('should show reaction buttons with counts', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByText('Marathon Ready')).toBeInTheDocument();
    });

    // Check for reaction counts
    expect(screen.getByText('3')).toBeInTheDocument(); // like count
    expect(screen.getByText('2')).toBeInTheDocument(); // celebrate count
  });

  it('should add a reaction when clicking reaction button', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByText('Marathon Ready')).toBeInTheDocument();
    });

    // Find and click the like button (first reaction button)
    const reactionButtons = screen.getAllByRole('button').filter(
      btn => btn.textContent?.includes('👍')
    );
    fireEvent.click(reactionButtons[0]);

    await waitFor(() => {
      expect(socialApi.addReaction).toHaveBeenCalledWith('insight-1', 'like');
    });
  });

  it('should show Adapt to My Training button', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Adapt to My Training/i })).toBeInTheDocument();
    });
  });

  it('should show Adapted button when user has already adapted', async () => {
    vi.mocked(socialApi.getFeed).mockResolvedValue({
      ...mockFeedItems,
      items: [
        {
          ...mockFeedItems.items[0],
          user_has_adapted: true,
        },
      ],
    });

    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Adapted/i })).toBeInTheDocument();
    });
  });

  it('should open share modal when clicking Share Insight', async () => {
    render(<SocialFeedTab />);

    await waitFor(() => {
      expect(screen.getByText('Marathon Ready')).toBeInTheDocument();
    });

    const shareButton = screen.getByRole('button', { name: /Share Insight/i });
    fireEvent.click(shareButton);

    // Modal should appear - look for the modal title "Share Insight" in the modal header
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Share Insight' })).toBeInTheDocument();
    });
  });
});
