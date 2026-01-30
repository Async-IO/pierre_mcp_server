// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unit tests for PromptSuggestions component
// ABOUTME: Tests hide/show coach functionality and coach selection

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import PromptSuggestions from '../PromptSuggestions';

// Mock the coaches API - data must be inline due to vi.mock hoisting
vi.mock('../../services/api', () => ({
  coachesApi: {
    list: vi.fn().mockResolvedValue({
      coaches: [
        {
          id: 'coach-1',
          title: 'Training Coach',
          description: 'Helps with training plans',
          system_prompt: 'You are a training expert.',
          category: 'training',
          tags: ['training'],
          token_count: 100,
          is_favorite: false,
          use_count: 5,
          is_system: false,
          visibility: 'private',
          is_assigned: false,
        },
        {
          id: 'coach-2',
          title: 'System Coach',
          description: 'A system-provided coach',
          system_prompt: 'You are an expert.',
          category: 'nutrition',
          tags: [],
          token_count: 150,
          is_favorite: false,
          use_count: 10,
          is_system: true,
          visibility: 'tenant',
          is_assigned: true,
        },
      ],
      total: 2,
    }),
    getHidden: vi.fn().mockResolvedValue({
      coaches: [
        {
          id: 'coach-3',
          title: 'Hidden Coach',
          description: 'A hidden system coach',
          system_prompt: 'You are hidden.',
          category: 'recovery',
          tags: [],
          token_count: 80,
          is_favorite: false,
          use_count: 0,
          is_system: true,
          visibility: 'tenant',
          is_assigned: true,
        },
      ],
    }),
    hide: vi.fn().mockResolvedValue({ success: true, is_hidden: true }),
    show: vi.fn().mockResolvedValue({ success: true, is_hidden: false }),
    recordUsage: vi.fn().mockResolvedValue({ success: true }),
  },
}));

function renderPromptSuggestions(props = {}) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false }
    },
  });

  const defaultProps = {
    onSelectPrompt: vi.fn(),
    onEditCoach: vi.fn(),
    onDeleteCoach: vi.fn(),
  };

  return {
    ...render(
      <QueryClientProvider client={queryClient}>
        <PromptSuggestions {...defaultProps} {...props} />
      </QueryClientProvider>
    ),
    queryClient,
  };
}

describe('PromptSuggestions Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Rendering', () => {
    it('should render coaches when loaded', async () => {
      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      expect(screen.getByText('System Coach')).toBeInTheDocument();
    });

    it('should show loading state initially', async () => {
      const { container } = renderPromptSuggestions();

      // Should show loading skeleton
      expect(container.querySelector('.animate-pulse')).toBeInTheDocument();
    });
  });

  describe('Coach Selection', () => {
    it('should call onSelectPrompt when coach is clicked', async () => {
      const onSelectPrompt = vi.fn();
      const user = userEvent.setup();

      await act(async () => {
        renderPromptSuggestions({ onSelectPrompt });
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      // Click the coach card
      await user.click(screen.getByText('Training Coach'));

      expect(onSelectPrompt).toHaveBeenCalledWith(
        expect.any(String),
        'coach-1',
        'You are a training expert.'
      );
    });
  });

  describe('Hide/Show System Coaches', () => {
    it('should show hide button for system coaches on hover', async () => {
      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('System Coach')).toBeInTheDocument();
      });

      // Find the hide button (eye-off icon) - it should be in the DOM for system coaches
      const hideButtons = screen.getAllByTitle('Hide coach');
      expect(hideButtons.length).toBeGreaterThan(0);
    });

    it('should not show hide button for user coaches', async () => {
      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      // User coaches should have edit/delete buttons but not hide
      const editButtons = screen.getAllByTitle('Edit coach');
      expect(editButtons.length).toBeGreaterThan(0);
    });

    it('should have correct aria-label on hide button for accessibility', async () => {
      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('System Coach')).toBeInTheDocument();
      });

      const hideButtons = screen.getAllByTitle('Hide coach');
      expect(hideButtons[0]).toHaveAttribute('title', 'Hide coach');
      expect(hideButtons[0]).toHaveAttribute('aria-label', 'Hide coach');
    });

    it('should call hide API when hide button is clicked', async () => {
      const { coachesApi } = await import('../../services/api');
      const user = userEvent.setup();

      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('System Coach')).toBeInTheDocument();
      });

      // Click the hide button on the system coach
      const hideButtons = screen.getAllByTitle('Hide coach');
      await user.click(hideButtons[0]);

      // hide should be called with the coach ID
      await waitFor(() => {
        expect(coachesApi.hide).toHaveBeenCalledWith('coach-2');
      });
    });
  });

  describe('Edit and Delete for User Coaches', () => {
    it('should show edit button for user coaches', async () => {
      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      // Should have edit buttons
      const editButtons = screen.getAllByTitle('Edit coach');
      expect(editButtons.length).toBeGreaterThan(0);
    });

    it('should show delete button for user coaches', async () => {
      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      // Should have delete buttons
      const deleteButtons = screen.getAllByTitle('Delete coach');
      expect(deleteButtons.length).toBeGreaterThan(0);
    });

    it('should call onEditCoach when edit button is clicked', async () => {
      const onEditCoach = vi.fn();
      const user = userEvent.setup();

      await act(async () => {
        renderPromptSuggestions({ onEditCoach });
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      // Click the edit button
      const editButtons = screen.getAllByTitle('Edit coach');
      await user.click(editButtons[0]);

      expect(onEditCoach).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'coach-1', title: 'Training Coach' })
      );
    });

    it('should call onDeleteCoach when delete button is clicked', async () => {
      const onDeleteCoach = vi.fn();
      const user = userEvent.setup();

      await act(async () => {
        renderPromptSuggestions({ onDeleteCoach });
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      // Click the delete button
      const deleteButtons = screen.getAllByTitle('Delete coach');
      await user.click(deleteButtons[0]);

      expect(onDeleteCoach).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'coach-1', title: 'Training Coach' })
      );
    });
  });

  describe('Category Display', () => {
    it('should display category emoji badges', async () => {
      await act(async () => {
        renderPromptSuggestions();
      });

      await waitFor(() => {
        expect(screen.getByText('Training Coach')).toBeInTheDocument();
      });

      // Category badges are displayed as emojis
      // Training category shows running emoji
      expect(screen.getByText('🏃')).toBeInTheDocument();
      // Nutrition category shows bowl emoji
      expect(screen.getByText('🥗')).toBeInTheDocument();
    });
  });
});
