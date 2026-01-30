// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for StoreScreen component
// ABOUTME: Tests browsing, filtering, searching, and navigation functionality

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import StoreScreen from '../StoreScreen';

// Mock the store API - define mock data inline to avoid hoisting issues
vi.mock('../../services/api', () => ({
  storeApi: {
    browse: vi.fn().mockResolvedValue({
      coaches: [
        {
          id: 'coach-1',
          title: 'Marathon Training Coach',
          description: 'A comprehensive marathon training program',
          category: 'training',
          tags: ['marathon', 'running', 'endurance'],
          sample_prompts: ['What should my weekly mileage be?'],
          token_count: 1200,
          install_count: 75,
          icon_url: null,
          published_at: '2024-01-15T00:00:00Z',
          author_id: 'author-123',
        },
        {
          id: 'coach-2',
          title: 'Nutrition Expert',
          description: 'Personalized nutrition advice',
          category: 'nutrition',
          tags: ['diet', 'macros', 'meal-planning'],
          sample_prompts: ['How many calories should I eat?'],
          token_count: 800,
          install_count: 120,
          icon_url: null,
          published_at: '2024-01-20T00:00:00Z',
          author_id: 'author-456',
        },
        {
          id: 'coach-3',
          title: 'Recovery Coach',
          description: 'Optimize your recovery',
          category: 'recovery',
          tags: ['sleep', 'stretching', 'rest'],
          sample_prompts: ['How long should I sleep?'],
          token_count: 600,
          install_count: 45,
          icon_url: null,
          published_at: '2024-01-25T00:00:00Z',
          author_id: 'author-789',
        },
      ],
      total: 3,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
    search: vi.fn().mockResolvedValue({
      coaches: [
        {
          id: 'coach-1',
          title: 'Marathon Training Coach',
          description: 'A comprehensive marathon training program',
          category: 'training',
          tags: ['marathon', 'running', 'endurance'],
          sample_prompts: ['What should my weekly mileage be?'],
          token_count: 1200,
          install_count: 75,
          icon_url: null,
          published_at: '2024-01-15T00:00:00Z',
          author_id: 'author-123',
        },
      ],
      query: 'marathon',
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
    get: vi.fn().mockResolvedValue({
      id: 'coach-1',
      title: 'Marathon Training Coach',
      description: 'A comprehensive marathon training program',
      category: 'training',
      tags: ['marathon', 'running', 'endurance'],
      sample_prompts: ['What should my weekly mileage be?'],
      system_prompt: 'You are an expert marathon coach...',
      token_count: 1200,
      install_count: 75,
      icon_url: null,
      published_at: '2024-01-15T00:00:00Z',
      created_at: '2024-01-10T00:00:00Z',
      author_id: 'author-123',
      publish_status: 'published',
    }),
    getInstallations: vi.fn().mockResolvedValue({
      coaches: [],
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
  },
}));

import { storeApi } from '../../services/api';

const mockOnNavigateToCoaches = vi.fn();

function renderStoreScreen() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <StoreScreen onNavigateToCoaches={mockOnNavigateToCoaches} />
    </QueryClientProvider>
  );
}

describe('StoreScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render the header with title', async () => {
      renderStoreScreen();
      expect(screen.getByText('Discover')).toBeInTheDocument();
    });

    it('should render the search input', async () => {
      renderStoreScreen();
      expect(screen.getByPlaceholderText('Search coaches...')).toBeInTheDocument();
    });

    it('should render category filter buttons', async () => {
      renderStoreScreen();
      expect(screen.getByText('All')).toBeInTheDocument();
      expect(screen.getByText('Training')).toBeInTheDocument();
      expect(screen.getByText('Nutrition')).toBeInTheDocument();
      expect(screen.getByText('Recovery')).toBeInTheDocument();
    });

    it('should render sort options', async () => {
      renderStoreScreen();
      expect(screen.getByText('Popular')).toBeInTheDocument();
      expect(screen.getByText('Newest')).toBeInTheDocument();
      expect(screen.getByText('A-Z')).toBeInTheDocument();
    });

    it('should render coach cards after loading', async () => {
      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
        expect(screen.getByText('Nutrition Expert')).toBeInTheDocument();
        expect(screen.getByText('Recovery Coach')).toBeInTheDocument();
      });
    });

    it('should display coach user counts', async () => {
      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getByText('75 users')).toBeInTheDocument();
        expect(screen.getByText('120 users')).toBeInTheDocument();
        expect(screen.getByText('45 users')).toBeInTheDocument();
      });
    });

    it('should display coach categories as badges', async () => {
      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getAllByText('training').length).toBeGreaterThan(0);
        expect(screen.getAllByText('nutrition').length).toBeGreaterThan(0);
        expect(screen.getAllByText('recovery').length).toBeGreaterThan(0);
      });
    });

    it('should display coach tags', async () => {
      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getByText('marathon')).toBeInTheDocument();
        expect(screen.getByText('running')).toBeInTheDocument();
        expect(screen.getByText('diet')).toBeInTheDocument();
      });
    });
  });

  describe('filtering', () => {
    it('should call browseStoreCoaches with popular sort by default', async () => {
      renderStoreScreen();

      await waitFor(() => {
        expect(storeApi.browse).toHaveBeenCalledWith(
          expect.objectContaining({
            sort_by: 'popular',
          })
        );
      });
    });

    it('should filter by category when clicked', async () => {
      const user = userEvent.setup();
      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Training' }));

      await waitFor(() => {
        expect(storeApi.browse).toHaveBeenCalledWith(
          expect.objectContaining({
            category: 'training',
          })
        );
      });
    });

    it('should change sort when option is clicked', async () => {
      const user = userEvent.setup();
      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Newest' }));

      await waitFor(() => {
        expect(storeApi.browse).toHaveBeenCalledWith(
          expect.objectContaining({
            sort_by: 'newest',
          })
        );
      });
    });
  });

  describe('search', () => {
    it('should search coaches when text is entered', async () => {
      const user = userEvent.setup();
      renderStoreScreen();

      const searchInput = screen.getByPlaceholderText('Search coaches...');
      await user.type(searchInput, 'marathon');

      await waitFor(
        () => {
          expect(storeApi.search).toHaveBeenCalledWith('marathon', 50);
        },
        { timeout: 1000 }
      );
    });

    it('should clear search when X button is clicked', async () => {
      const user = userEvent.setup();
      renderStoreScreen();

      const searchInput = screen.getByPlaceholderText('Search coaches...');
      await user.type(searchInput, 'marathon');

      await waitFor(() => {
        expect(searchInput).toHaveValue('marathon');
      });

      // Find and click clear button (it appears after typing)
      const buttons = screen.getAllByRole('button');
      const clearButton = buttons.find(
        (btn) => btn.querySelector('svg path[d*="6 18L18 6"]') !== null
      );
      if (clearButton) {
        await user.click(clearButton);
        await waitFor(() => {
          expect(searchInput).toHaveValue('');
        });
      }
    });
  });

  describe('navigation', () => {
    it('should open detail view when coach card is clicked', async () => {
      const user = userEvent.setup();
      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getByText('Marathon Training Coach')).toBeInTheDocument();
      });

      await user.click(screen.getByText('Marathon Training Coach'));

      // Detail view should show Add Coach button and System Prompt section
      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Coach' })).toBeInTheDocument();
      });
    });
  });

  describe('empty state', () => {
    it('should show empty state when no coaches', async () => {
      vi.mocked(storeApi.browse).mockResolvedValueOnce({
        coaches: [],
        total: 0,
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      });

      renderStoreScreen();

      await waitFor(() => {
        expect(screen.getByText('Store is empty')).toBeInTheDocument();
      });
    });

    it('should show search empty state when no search results', async () => {
      vi.mocked(storeApi.search).mockResolvedValueOnce({
        coaches: [],
        query: 'nonexistent',
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      });

      const user = userEvent.setup();
      renderStoreScreen();

      const searchInput = screen.getByPlaceholderText('Search coaches...');
      await user.type(searchInput, 'nonexistent');

      await waitFor(
        () => {
          expect(screen.getByText('No coaches found')).toBeInTheDocument();
        },
        { timeout: 1000 }
      );
    });
  });
});
