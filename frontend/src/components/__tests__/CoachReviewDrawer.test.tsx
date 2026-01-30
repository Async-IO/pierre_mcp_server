// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for CoachReviewDrawer component
// ABOUTME: Tests drawer display, approve/reject actions, and prompt expansion

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockCoach = {
  id: 'coach-1',
  title: 'Marathon Training Coach',
  description: 'A comprehensive marathon training program with weekly schedules',
  category: 'Training',
  tags: ['marathon', 'running', 'endurance'],
  sample_prompts: ['What should my weekly mileage be?', 'How do I taper for race day?'],
  token_count: 1200,
  install_count: 0,
  icon_url: null,
  published_at: null,
  author_id: 'author-123',
  author_email: 'coach@example.com',
  system_prompt: 'You are an expert marathon training coach. Help athletes prepare for marathon races with personalized training plans. This is a longer prompt that exceeds 500 characters to test the expansion feature. '.repeat(5),
  created_at: '2024-01-10T00:00:00Z',
  submitted_at: '2024-01-15T10:30:00Z',
  publish_status: 'pending_review',
};

// Mock the admin API
vi.mock('../../services/api', () => ({
  adminApi: {
    approveStoreCoach: vi.fn().mockResolvedValue({
      success: true,
      message: 'Coach approved',
      coach_id: 'coach-1',
    }),
    rejectStoreCoach: vi.fn().mockResolvedValue({
      success: true,
      message: 'Coach rejected',
      coach_id: 'coach-1',
    }),
  },
}));

import CoachReviewDrawer from '../CoachReviewDrawer';
import { adminApi } from '../../services/api';

const mockOnClose = vi.fn();

function renderCoachReviewDrawer(coach = mockCoach, isOpen = true) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <CoachReviewDrawer coach={coach} isOpen={isOpen} onClose={mockOnClose} />
    </QueryClientProvider>
  );
}

describe('CoachReviewDrawer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders nothing when not open', () => {
    renderCoachReviewDrawer(mockCoach, false);
    expect(screen.queryByText('Review Coach')).not.toBeInTheDocument();
  });

  it('renders nothing when coach is null', () => {
    renderCoachReviewDrawer(null, true);
    expect(screen.queryByText('Review Coach')).not.toBeInTheDocument();
  });

  it('displays coach details when open', () => {
    renderCoachReviewDrawer();

    expect(screen.getByText('Review Coach')).toBeInTheDocument();
    expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
    expect(screen.getByText('Training')).toBeInTheDocument();
  });

  it('shows coach description', () => {
    renderCoachReviewDrawer();
    expect(
      screen.getByText('A comprehensive marathon training program with weekly schedules')
    ).toBeInTheDocument();
  });

  it('displays author information', () => {
    renderCoachReviewDrawer();
    expect(screen.getByText('coach@example.com')).toBeInTheDocument();
    expect(screen.getByText('author-123')).toBeInTheDocument();
  });

  it('shows token count', () => {
    renderCoachReviewDrawer();
    expect(screen.getByText('1,200 tokens')).toBeInTheDocument();
  });

  it('displays tags', () => {
    renderCoachReviewDrawer();
    expect(screen.getByText('marathon')).toBeInTheDocument();
    expect(screen.getByText('running')).toBeInTheDocument();
    expect(screen.getByText('endurance')).toBeInTheDocument();
  });

  it('shows sample prompts', () => {
    renderCoachReviewDrawer();
    expect(screen.getByText('What should my weekly mileage be?')).toBeInTheDocument();
    expect(screen.getByText('How do I taper for race day?')).toBeInTheDocument();
  });

  it('truncates long system prompts', () => {
    renderCoachReviewDrawer();
    // Should show "Show full prompt" button for long prompts
    expect(screen.getByText('Show full prompt')).toBeInTheDocument();
  });

  it('expands system prompt when button clicked', async () => {
    const user = userEvent.setup();
    renderCoachReviewDrawer();

    const expandButton = screen.getByText('Show full prompt');
    await user.click(expandButton);

    expect(screen.getByText('Show less')).toBeInTheDocument();
  });

  it('calls onClose when close button clicked', async () => {
    const user = userEvent.setup();
    renderCoachReviewDrawer();

    const closeButton = screen.getByLabelText('Close drawer');
    await user.click(closeButton);

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('calls onClose when backdrop clicked', async () => {
    const user = userEvent.setup();
    renderCoachReviewDrawer();

    const backdrop = document.querySelector('[aria-hidden="true"]');
    expect(backdrop).toBeInTheDocument();

    if (backdrop) {
      await user.click(backdrop);
      expect(mockOnClose).toHaveBeenCalled();
    }
  });

  it('has Approve and Reject buttons', () => {
    renderCoachReviewDrawer();
    expect(screen.getByRole('button', { name: /Approve/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Reject/i })).toBeInTheDocument();
  });

  it('calls approve API when Approve button clicked', async () => {
    const user = userEvent.setup();
    renderCoachReviewDrawer();

    const approveButton = screen.getByRole('button', { name: /Approve/i });
    await user.click(approveButton);

    await waitFor(() => {
      expect(adminApi.approveStoreCoach).toHaveBeenCalledWith('coach-1');
    });
  });

  it('shows loading state during approval', async () => {
    const user = userEvent.setup();
    vi.mocked(adminApi.approveStoreCoach).mockImplementation(
      () => new Promise(() => {})
    );

    renderCoachReviewDrawer();

    const approveButton = screen.getByRole('button', { name: /Approve/i });
    await user.click(approveButton);

    await waitFor(() => {
      expect(screen.getByText('Approving...')).toBeInTheDocument();
    });
  });

  it('opens rejection modal when Reject button clicked', async () => {
    const user = userEvent.setup();
    renderCoachReviewDrawer();

    const rejectButton = screen.getByRole('button', { name: /Reject/i });
    await user.click(rejectButton);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Reject Coach' })).toBeInTheDocument();
    });
  });

  it('displays coach initial when no icon', () => {
    renderCoachReviewDrawer();
    // Coach name starts with 'M', so the initial should be 'M'
    expect(screen.getByText('M')).toBeInTheDocument();
  });
});
