// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for CoachReviewQueue component
// ABOUTME: Tests pending coaches list, empty state, loading, and drawer opening

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockCoaches = [
  {
    id: 'coach-1',
    title: 'Marathon Training Coach',
    description: 'A comprehensive marathon training program',
    category: 'Training',
    tags: ['marathon', 'running', 'endurance'],
    sample_prompts: ['What should my weekly mileage be?'],
    token_count: 1200,
    install_count: 0,
    icon_url: null,
    published_at: null,
    author_id: 'author-1',
    author_email: 'coach@example.com',
    system_prompt: 'You are a marathon training coach...',
    created_at: '2024-01-10T00:00:00Z',
    submitted_at: '2024-01-15T10:30:00Z',
    publish_status: 'pending_review',
  },
  {
    id: 'coach-2',
    title: 'Nutrition Advisor',
    description: 'Expert nutrition guidance for athletes',
    category: 'Nutrition',
    tags: ['nutrition', 'diet', 'health', 'protein', 'carbs'],
    sample_prompts: ['What should I eat before a race?'],
    token_count: 800,
    install_count: 0,
    icon_url: null,
    published_at: null,
    author_id: 'author-2',
    author_email: 'nutrition@example.com',
    system_prompt: 'You are a nutrition advisor...',
    created_at: '2024-01-12T00:00:00Z',
    submitted_at: '2024-01-16T14:00:00Z',
    publish_status: 'pending_review',
  },
];

// Mock the admin API - mocked implementation is set in beforeEach
vi.mock('../../services/api', () => ({
  adminApi: {
    getStoreReviewQueue: vi.fn(),
    approveStoreCoach: vi.fn(),
    rejectStoreCoach: vi.fn(),
  },
}));

import CoachReviewQueue from '../CoachReviewQueue';
import { adminApi } from '../../services/api';

function renderCoachReviewQueue() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <CoachReviewQueue />
    </QueryClientProvider>
  );
}

describe('CoachReviewQueue', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(adminApi.getStoreReviewQueue).mockResolvedValue({
      coaches: mockCoaches,
      total: 2,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });
    vi.mocked(adminApi.approveStoreCoach).mockResolvedValue({ success: true });
    vi.mocked(adminApi.rejectStoreCoach).mockResolvedValue({ success: true });
  });

  it('displays loading spinner initially', () => {
    vi.mocked(adminApi.getStoreReviewQueue).mockImplementation(
      () => new Promise(() => {})
    );

    renderCoachReviewQueue();

    expect(document.querySelector('.pierre-spinner')).toBeInTheDocument();
  });

  it('renders coaches in the queue', async () => {
    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
    });

    expect(screen.getByText('Nutrition Advisor')).toBeInTheDocument();
  });

  it('shows queue position numbers', async () => {
    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('1')).toBeInTheDocument();
      expect(screen.getByText('2')).toBeInTheDocument();
    });
  });

  it('displays category badges', async () => {
    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('Training')).toBeInTheDocument();
      expect(screen.getByText('Nutrition')).toBeInTheDocument();
    });
  });

  it('shows author email', async () => {
    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('coach@example.com')).toBeInTheDocument();
      expect(screen.getByText('nutrition@example.com')).toBeInTheDocument();
    });
  });

  it('displays token count', async () => {
    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('1,200 tokens')).toBeInTheDocument();
      expect(screen.getByText('800 tokens')).toBeInTheDocument();
    });
  });

  it('shows tags with overflow indicator', async () => {
    renderCoachReviewQueue();

    await waitFor(() => {
      // Coach 2 has 5 tags, should show 4 + overflow
      expect(screen.getByText('+1')).toBeInTheDocument();
    });
  });

  it('shows empty state when no coaches pending', async () => {
    vi.mocked(adminApi.getStoreReviewQueue).mockResolvedValue({
      coaches: [],
      total: 0,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });

    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('All Caught Up!')).toBeInTheDocument();
      expect(
        screen.getByText('There are no coaches pending review at this time.')
      ).toBeInTheDocument();
    });
  });

  it('shows error state on API failure', async () => {
    vi.mocked(adminApi.getStoreReviewQueue).mockRejectedValue(
      new Error('API Error')
    );

    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('Failed to Load Review Queue')).toBeInTheDocument();
    });
  });

  it('opens review drawer when coach is clicked', async () => {
    const user = userEvent.setup();
    renderCoachReviewQueue();

    await waitFor(() => {
      expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
    });

    // Click on the first coach
    const coachButton = screen.getByText('Marathon Training Coach').closest('button');
    expect(coachButton).toBeInTheDocument();

    if (coachButton) {
      await user.click(coachButton);

      // Drawer should open - look for drawer header
      await waitFor(() => {
        expect(screen.getByText('Review Coach')).toBeInTheDocument();
      });
    }
  });
});
