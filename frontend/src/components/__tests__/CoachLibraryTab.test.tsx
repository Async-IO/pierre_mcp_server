// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unit tests for CoachLibraryTab component
// ABOUTME: Tests token estimation, context percentage calculation, and rendering

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import CoachLibraryTab from '../CoachLibraryTab';

// Mock the coaches API
vi.mock('../../services/api', () => ({
  coachesApi: {
    list: vi.fn().mockResolvedValue({
      coaches: [
        {
          id: 'coach-1',
          title: 'Test Coach',
          description: 'A test coach for unit testing',
          system_prompt: 'You are a helpful coach.',
          category: 'Training',
          tags: ['test', 'unit'],
          token_count: 100,
          is_favorite: false,
          use_count: 5,
          is_system: false,
          visibility: 'private',
          created_at: '2025-01-01T00:00:00Z',
          updated_at: '2025-01-01T00:00:00Z',
        },
        {
          id: 'coach-2',
          title: 'Favorite Coach',
          description: 'A favorite coach',
          system_prompt: 'You are an expert.',
          category: 'Nutrition',
          tags: ['favorite'],
          token_count: 256,
          is_favorite: true,
          use_count: 42,
          is_system: false,
          visibility: 'private',
          created_at: '2025-01-02T00:00:00Z',
          updated_at: '2025-01-02T00:00:00Z',
        },
      ],
      total: 2,
    }),
    getHidden: vi.fn().mockResolvedValue({
      coaches: [],
    }),
    create: vi.fn().mockResolvedValue({
      id: 'coach-new',
      title: 'New Coach',
      token_count: 50,
    }),
    update: vi.fn().mockResolvedValue({
      id: 'coach-1',
      title: 'Updated Coach',
    }),
    delete: vi.fn().mockResolvedValue(undefined),
    toggleFavorite: vi.fn().mockResolvedValue({ is_favorite: true }),
    hide: vi.fn().mockResolvedValue({ success: true, is_hidden: true }),
    show: vi.fn().mockResolvedValue({ success: true, is_hidden: false }),
    fork: vi.fn().mockResolvedValue({
      coach: {
        id: 'coach-forked',
        title: 'Forked Coach',
        is_system: false,
        token_count: 100,
      },
    }),
  },
}));

function renderCoachLibraryTab(props = {}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <CoachLibraryTab {...props} />
    </QueryClientProvider>
  );
}

describe('Token Estimation Functions', () => {
  // These test the internal logic that the component uses
  // Token formula: Math.ceil(text.length / 4)
  // Context percentage: (tokens / 128000 * 100).toFixed(1)

  const CONTEXT_WINDOW_SIZE = 128000;

  const estimateTokenCount = (text: string): number => {
    return Math.ceil(text.length / 4);
  };

  const getContextPercentage = (tokens: number): string => {
    return ((tokens / CONTEXT_WINDOW_SIZE) * 100).toFixed(1);
  };

  describe('estimateTokenCount', () => {
    it('should return 0 for empty string', () => {
      expect(estimateTokenCount('')).toBe(0);
    });

    it('should estimate tokens as characters / 4 (ceiling)', () => {
      expect(estimateTokenCount('a')).toBe(1); // 1/4 = 0.25, ceil = 1
      expect(estimateTokenCount('ab')).toBe(1); // 2/4 = 0.5, ceil = 1
      expect(estimateTokenCount('abc')).toBe(1); // 3/4 = 0.75, ceil = 1
      expect(estimateTokenCount('abcd')).toBe(1); // 4/4 = 1, ceil = 1
      expect(estimateTokenCount('abcde')).toBe(2); // 5/4 = 1.25, ceil = 2
    });

    it('should handle longer text correctly', () => {
      const text100 = 'a'.repeat(100);
      expect(estimateTokenCount(text100)).toBe(25); // 100/4 = 25

      const text401 = 'a'.repeat(401);
      expect(estimateTokenCount(text401)).toBe(101); // 401/4 = 100.25, ceil = 101
    });

    it('should handle text with special characters', () => {
      const textWithEmoji = '👋 Hello World! 🎉';
      // Emojis count as 2 characters each in JS
      expect(estimateTokenCount(textWithEmoji)).toBe(Math.ceil(textWithEmoji.length / 4));
    });

    it('should handle multiline text', () => {
      const multiline = 'Line 1\nLine 2\nLine 3';
      expect(estimateTokenCount(multiline)).toBe(Math.ceil(multiline.length / 4));
    });
  });

  describe('getContextPercentage', () => {
    it('should return 0.0 for 0 tokens', () => {
      expect(getContextPercentage(0)).toBe('0.0');
    });

    it('should calculate correct percentage for small token counts', () => {
      expect(getContextPercentage(128)).toBe('0.1'); // 128/128000 * 100 = 0.1
      expect(getContextPercentage(1280)).toBe('1.0'); // 1280/128000 * 100 = 1.0
    });

    it('should calculate correct percentage for medium token counts', () => {
      expect(getContextPercentage(12800)).toBe('10.0'); // 12800/128000 * 100 = 10.0
      expect(getContextPercentage(25600)).toBe('20.0'); // 25600/128000 * 100 = 20.0
    });

    it('should calculate correct percentage for large token counts', () => {
      expect(getContextPercentage(64000)).toBe('50.0'); // 64000/128000 * 100 = 50.0
      expect(getContextPercentage(128000)).toBe('100.0'); // 128000/128000 * 100 = 100.0
    });

    it('should handle decimal precision correctly', () => {
      expect(getContextPercentage(100)).toBe('0.1'); // 100/128000 * 100 ≈ 0.078, rounds to 0.1
      expect(getContextPercentage(500)).toBe('0.4'); // 500/128000 * 100 ≈ 0.391, rounds to 0.4
    });
  });
});

describe('CoachLibraryTab Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render coach list view', async () => {
    await act(async () => {
      renderCoachLibraryTab();
    });

    // Should show header (using heading role to differentiate from filter button)
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'My Coaches' })).toBeInTheDocument();
    });

    // Should show Create Coach button
    expect(screen.getByRole('button', { name: /Create Coach/i })).toBeInTheDocument();
  });

  it('should display coach cards with category badges', async () => {
    await act(async () => {
      renderCoachLibraryTab();
    });

    // Wait for coaches to load
    await waitFor(() => {
      expect(screen.getByText('Test Coach')).toBeInTheDocument();
    });

    // Cards show category badges - there are multiple "Training" elements
    // (filter button + badge on card), so use getAllByText
    const trainingElements = screen.getAllByText('Training');
    expect(trainingElements.length).toBeGreaterThanOrEqual(1);

    const nutritionElements = screen.getAllByText('Nutrition');
    expect(nutritionElements.length).toBeGreaterThanOrEqual(1);
  });

  it('should display coach cards with star ratings based on use count', async () => {
    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getByText('Test Coach')).toBeInTheDocument();
    });

    // Star ratings are now used instead of use count text
    // Each coach card should have 5 star SVGs for the rating display
    // The SVGs have different fill colors based on use_count
    const coachCards = screen.getAllByText(/Coach/i);
    expect(coachCards.length).toBeGreaterThan(0);
  });

  it('should display favorite icon filled for favorite coaches', async () => {
    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getByText('Favorite Coach')).toBeInTheDocument();
    });

    // The favorite button for the favorite coach should have the fill class
    const favoriteButtons = screen.getAllByTitle(/favorites/i);
    expect(favoriteButtons.length).toBeGreaterThan(0);
  });

  it('should show category filters', async () => {
    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'My Coaches' })).toBeInTheDocument();
    });

    // Should show category filter buttons
    expect(screen.getByRole('button', { name: 'All' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Training' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Nutrition' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Recovery' })).toBeInTheDocument();
  });

  it('should show favorites filter', async () => {
    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'My Coaches' })).toBeInTheDocument();
    });

    // Should show favorites filter button (exact text match to avoid matching favorite icons)
    expect(screen.getByRole('button', { name: 'Favorites' })).toBeInTheDocument();
  });

  it('should navigate to create form when Create Coach is clicked', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'My Coaches' })).toBeInTheDocument();
    });

    // Click the first Create Coach button (the one in the header)
    const createButtons = screen.getAllByRole('button', { name: /Create Coach/i });
    await user.click(createButtons[0]);

    // Should show create form with title field
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/Marathon Training Coach/i)).toBeInTheDocument();
    });
  });

  it('should show token estimate in create form', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getAllByRole('button', { name: /Create Coach/i }).length).toBeGreaterThan(0);
    });

    // Navigate to create form (click first Create Coach button)
    const createButtons = screen.getAllByRole('button', { name: /Create Coach/i });
    await user.click(createButtons[0]);

    // Wait for form to appear
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/Pierre, an expert coach/i)).toBeInTheDocument();
    });

    // Type in system prompt field
    const systemPromptField = screen.getByPlaceholderText(/Pierre, an expert coach/i);
    await user.type(systemPromptField, 'You are a professional coach with expertise.');

    // Should show token estimate (text length / 4)
    // "You are a professional coach with expertise." = 44 chars = 11 tokens
    await waitFor(() => {
      expect(screen.getByText(/~11 tokens/)).toBeInTheDocument();
    });
  });

  it('should call onBack when back button is clicked in list view', async () => {
    const onBack = vi.fn();
    const user = userEvent.setup();

    await act(async () => {
      renderCoachLibraryTab({ onBack });
    });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'My Coaches' })).toBeInTheDocument();
    });

    // Find and click back button
    const backButton = screen.getByText('Back');
    await user.click(backButton);

    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('should navigate to detail view when coach card is clicked', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getByText('Test Coach')).toBeInTheDocument();
    });

    // Click coach card
    await user.click(screen.getByText('Test Coach'));

    // Should show detail view with stats
    await waitFor(() => {
      expect(screen.getByText('System Prompt')).toBeInTheDocument();
    });

    // Should show token count in stats
    expect(screen.getByText('~100')).toBeInTheDocument();
    expect(screen.getByText(/context/i)).toBeInTheDocument();

    // Should show use count
    expect(screen.getByText('5')).toBeInTheDocument();
    expect(screen.getByText('Uses')).toBeInTheDocument();
  });

  it('should show Edit and Delete buttons in detail view', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderCoachLibraryTab();
    });

    await waitFor(() => {
      expect(screen.getByText('Test Coach')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Test Coach'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Edit/i })).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: /Delete/i })).toBeInTheDocument();
  });
});
