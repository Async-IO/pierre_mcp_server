// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for RejectedCoachesList component
// ABOUTME: Tests rejected coaches list, rejection details, and empty state

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockCoaches = [
  {
    id: 'coach-1',
    title: 'Problematic Coach',
    description: 'This coach had issues',
    category: 'Training',
    tags: ['fitness', 'workout'],
    sample_prompts: ['Sample prompt'],
    token_count: 500,
    install_count: 0,
    icon_url: null,
    published_at: null,
    author_id: 'author-1',
    author_email: 'problem@example.com',
    system_prompt: 'You are a coach...',
    created_at: '2024-01-10T00:00:00Z',
    rejected_at: '2024-01-15T14:30:00Z',
    rejection_reason: 'quality_standards',
    rejection_notes: 'The system prompt lacks specificity.',
    publish_status: 'rejected',
  },
  {
    id: 'coach-2',
    title: 'Duplicate Coach',
    description: 'Already exists',
    category: 'Nutrition',
    tags: ['diet', 'food', 'health', 'nutrition', 'meal'],
    sample_prompts: ['What to eat?'],
    token_count: 300,
    install_count: 0,
    icon_url: 'https://example.com/dup.png',
    published_at: null,
    author_id: 'author-2',
    author_email: 'dup@example.com',
    system_prompt: 'You are a nutrition coach...',
    created_at: '2024-01-12T00:00:00Z',
    rejected_at: '2024-01-18T09:00:00Z',
    rejection_reason: 'duplicate_submission',
    rejection_notes: null,
    publish_status: 'rejected',
  },
];

// Mock the API service - mocked implementation is set in beforeEach
vi.mock('../../services/api', () => ({
  apiService: {
    getRejectedStoreCoaches: vi.fn(),
  },
}));

import RejectedCoachesList from '../RejectedCoachesList';
import { apiService } from '../../services/api';

function renderRejectedCoachesList() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <RejectedCoachesList />
    </QueryClientProvider>
  );
}

describe('RejectedCoachesList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiService.getRejectedStoreCoaches).mockResolvedValue({
      coaches: mockCoaches,
      total: 2,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });
  });

  it('displays loading spinner initially', () => {
    vi.mocked(apiService.getRejectedStoreCoaches).mockImplementation(
      () => new Promise(() => {})
    );

    renderRejectedCoachesList();

    expect(document.querySelector('.pierre-spinner')).toBeInTheDocument();
  });

  it('renders rejected coaches', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Problematic Coach')).toBeInTheDocument();
    });

    expect(screen.getByText('Duplicate Coach')).toBeInTheDocument();
  });

  it('shows rejection count', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('2 rejected submissions')).toBeInTheDocument();
    });
  });

  it('displays category badges', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Training')).toBeInTheDocument();
      expect(screen.getByText('Nutrition')).toBeInTheDocument();
    });
  });

  it('shows author emails', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('by problem@example.com')).toBeInTheDocument();
      expect(screen.getByText('by dup@example.com')).toBeInTheDocument();
    });
  });

  it('displays rejection reasons with human-readable labels', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Quality standards not met')).toBeInTheDocument();
      expect(screen.getByText('Duplicate submission')).toBeInTheDocument();
    });
  });

  it('shows rejection notes when present', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      expect(
        screen.getByText('The system prompt lacks specificity.')
      ).toBeInTheDocument();
    });
  });

  it('shows token counts', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('500 tokens')).toBeInTheDocument();
      expect(screen.getByText('300 tokens')).toBeInTheDocument();
    });
  });

  it('shows tags with overflow indicator', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      // Coach 2 has 5 tags, should show 4 + overflow
      expect(screen.getByText('+1')).toBeInTheDocument();
    });
  });

  it('shows empty state when no rejected coaches', async () => {
    vi.mocked(apiService.getRejectedStoreCoaches).mockResolvedValue({
      coaches: [],
      total: 0,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });

    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('No Rejected Coaches')).toBeInTheDocument();
      expect(
        screen.getByText('Rejected coach submissions will appear here.')
      ).toBeInTheDocument();
    });
  });

  it('shows error state on API failure', async () => {
    vi.mocked(apiService.getRejectedStoreCoaches).mockRejectedValue(
      new Error('API Error')
    );

    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Failed to Load Rejected Coaches')).toBeInTheDocument();
    });
  });

  it('displays coach initial when no icon', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      // Problematic coach has no icon, should show 'P'
      expect(screen.getByText('P')).toBeInTheDocument();
    });
  });

  it('displays icon image when available', async () => {
    renderRejectedCoachesList();

    await waitFor(() => {
      const img = screen.getByAltText('Duplicate Coach');
      expect(img).toBeInTheDocument();
      expect(img).toHaveAttribute('src', 'https://example.com/dup.png');
    });
  });

  it('handles unknown rejection reason gracefully', async () => {
    vi.mocked(apiService.getRejectedStoreCoaches).mockResolvedValue({
      coaches: [
        {
          ...mockCoaches[0],
          rejection_reason: 'unknown_reason',
        },
      ],
      total: 1,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });

    renderRejectedCoachesList();

    await waitFor(() => {
      // Should display the raw reason when not in the labels map
      expect(screen.getByText('unknown_reason')).toBeInTheDocument();
    });
  });

  it('shows singular form for one rejection', async () => {
    vi.mocked(apiService.getRejectedStoreCoaches).mockResolvedValue({
      coaches: [mockCoaches[0]],
      total: 1,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });

    renderRejectedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('1 rejected submission')).toBeInTheDocument();
    });
  });
});
