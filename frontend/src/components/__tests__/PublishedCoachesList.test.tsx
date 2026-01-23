// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for PublishedCoachesList component
// ABOUTME: Tests published coaches grid, sorting, and unpublish functionality

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
    tags: ['marathon', 'running'],
    sample_prompts: ['What should my weekly mileage be?'],
    token_count: 1200,
    install_count: 75,
    icon_url: null,
    published_at: '2024-01-15T00:00:00Z',
    author_id: 'author-1',
    author_email: 'coach@example.com',
    system_prompt: 'You are a marathon training coach...',
    created_at: '2024-01-10T00:00:00Z',
    publish_status: 'published',
  },
  {
    id: 'coach-2',
    title: 'Nutrition Advisor',
    description: 'Expert nutrition guidance',
    category: 'Nutrition',
    tags: ['nutrition', 'diet'],
    sample_prompts: ['What should I eat?'],
    token_count: 800,
    install_count: 120,
    icon_url: 'https://example.com/icon.png',
    published_at: '2024-01-20T00:00:00Z',
    author_id: 'author-2',
    author_email: 'nutrition@example.com',
    system_prompt: 'You are a nutrition advisor...',
    created_at: '2024-01-12T00:00:00Z',
    publish_status: 'published',
  },
];

// Mock the API service - mocked implementation is set in beforeEach
vi.mock('../../services/api', () => ({
  apiService: {
    getPublishedStoreCoaches: vi.fn(),
    unpublishStoreCoach: vi.fn(),
  },
}));

import PublishedCoachesList from '../PublishedCoachesList';
import { apiService } from '../../services/api';

function renderPublishedCoachesList() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <PublishedCoachesList />
    </QueryClientProvider>
  );
}

describe('PublishedCoachesList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiService.getPublishedStoreCoaches).mockResolvedValue({
      coaches: mockCoaches,
      total: 2,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });
    vi.mocked(apiService.unpublishStoreCoach).mockResolvedValue({
      success: true,
      message: 'Coach unpublished',
      coach_id: 'coach-1',
    });
  });

  it('displays loading spinner initially', () => {
    vi.mocked(apiService.getPublishedStoreCoaches).mockImplementation(
      () => new Promise(() => {})
    );

    renderPublishedCoachesList();

    expect(document.querySelector('.pierre-spinner')).toBeInTheDocument();
  });

  it('renders coaches in grid', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
    });

    expect(screen.getByText('Nutrition Advisor')).toBeInTheDocument();
  });

  it('shows coach count', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('2 published coaches')).toBeInTheDocument();
    });
  });

  it('displays category badges', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Training')).toBeInTheDocument();
      expect(screen.getByText('Nutrition')).toBeInTheDocument();
    });
  });

  it('shows install counts', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('75 installs')).toBeInTheDocument();
      expect(screen.getByText('120 installs')).toBeInTheDocument();
    });
  });

  it('shows author emails', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('by coach@example.com')).toBeInTheDocument();
      expect(screen.getByText('by nutrition@example.com')).toBeInTheDocument();
    });
  });

  it('has sort dropdown with options', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Sort by:')).toBeInTheDocument();
    });

    const select = screen.getByRole('combobox');
    expect(select).toHaveValue('newest');
  });

  it('changes sort order when dropdown changed', async () => {
    const user = userEvent.setup();
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
    });

    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'most_installed');

    await waitFor(() => {
      expect(apiService.getPublishedStoreCoaches).toHaveBeenCalledWith({
        sort_by: 'most_installed',
      });
    });
  });

  it('shows empty state when no coaches', async () => {
    vi.mocked(apiService.getPublishedStoreCoaches).mockResolvedValue({
      coaches: [],
      total: 0,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });

    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('No Published Coaches')).toBeInTheDocument();
      expect(
        screen.getByText(/Coaches will appear here once they are approved/)
      ).toBeInTheDocument();
    });
  });

  it('shows error state on API failure', async () => {
    vi.mocked(apiService.getPublishedStoreCoaches).mockRejectedValue(
      new Error('API Error')
    );

    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Failed to Load Published Coaches')).toBeInTheDocument();
    });
  });

  it('has View and Unpublish buttons for each coach', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      const viewButtons = screen.getAllByRole('button', { name: /View/i });
      const unpublishButtons = screen.getAllByRole('button', { name: /Unpublish/i });

      expect(viewButtons).toHaveLength(2);
      expect(unpublishButtons).toHaveLength(2);
    });
  });

  it('opens confirmation dialog when Unpublish clicked', async () => {
    const user = userEvent.setup();
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
    });

    const unpublishButtons = screen.getAllByRole('button', { name: /Unpublish/i });
    await user.click(unpublishButtons[0]);

    await waitFor(() => {
      expect(screen.getByText('Unpublish Coach')).toBeInTheDocument();
      expect(screen.getByText(/Are you sure you want to unpublish/)).toBeInTheDocument();
    });
  });

  it('calls unpublish API when confirmed', async () => {
    const user = userEvent.setup();
    renderPublishedCoachesList();

    await waitFor(() => {
      expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
    });

    // Click unpublish
    const unpublishButtons = screen.getAllByRole('button', { name: /Unpublish/i });
    await user.click(unpublishButtons[0]);

    // Confirm in dialog
    await waitFor(() => {
      expect(screen.getByText('Unpublish Coach')).toBeInTheDocument();
    });

    // Find the confirm button in the dialog (it has btn-danger class)
    const allUnpublishButtons = screen.getAllByRole('button', { name: /^Unpublish$/i });
    const confirmButton = allUnpublishButtons.find(btn => btn.className.includes('btn-danger'));
    expect(confirmButton).toBeDefined();
    await user.click(confirmButton!);

    await waitFor(() => {
      expect(apiService.unpublishStoreCoach).toHaveBeenCalledWith('coach-1');
    });
  });

  it('displays coach initial when no icon', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      // Marathon coach has no icon, should show 'M'
      expect(screen.getByText('M')).toBeInTheDocument();
    });
  });

  it('displays icon image when available', async () => {
    renderPublishedCoachesList();

    await waitFor(() => {
      const img = screen.getByAltText('Nutrition Advisor');
      expect(img).toBeInTheDocument();
      expect(img).toHaveAttribute('src', 'https://example.com/icon.png');
    });
  });
});
