// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for StoreCoachDetail component
// ABOUTME: Tests coach detail display, add/remove actions, and navigation

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import StoreCoachDetail from '../StoreCoachDetail';

// Mock the API service - define mock data inline to avoid hoisting issues
vi.mock('../../services/api', () => ({
  apiService: {
    getStoreCoach: vi.fn().mockResolvedValue({
      id: 'coach-1',
      title: 'Marathon Training Coach',
      description: 'A comprehensive marathon training program with weekly schedules',
      category: 'training',
      tags: ['marathon', 'running', 'endurance', 'long-distance'],
      sample_prompts: ['What should my weekly mileage be?', 'How do I taper for race day?'],
      token_count: 1200,
      install_count: 75,
      icon_url: null,
      published_at: '2024-01-15T00:00:00Z',
      author_id: 'author-123',
      system_prompt:
        'You are an expert marathon training coach. Help athletes prepare for marathon races with personalized training plans.',
      created_at: '2024-01-10T00:00:00Z',
      publish_status: 'published',
    }),
    getStoreInstallations: vi.fn().mockResolvedValue({
      coaches: [],
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
    installStoreCoach: vi.fn().mockResolvedValue({
      message: 'Coach installed successfully',
      coach_id: 'coach-1',
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
    uninstallStoreCoach: vi.fn().mockResolvedValue({
      message: 'Coach uninstalled successfully',
      coach_id: 'coach-1',
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
  },
}));

import { apiService } from '../../services/api';

const mockOnBack = vi.fn();
const mockOnNavigateToLibrary = vi.fn();

function renderStoreCoachDetail(coachId = 'coach-1') {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <StoreCoachDetail
        coachId={coachId}
        onBack={mockOnBack}
        onNavigateToLibrary={mockOnNavigateToLibrary}
      />
    </QueryClientProvider>
  );
}

describe('StoreCoachDetail', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset to default mock implementations
    vi.mocked(apiService.getStoreCoach).mockResolvedValue({
      id: 'coach-1',
      title: 'Marathon Training Coach',
      description: 'A comprehensive marathon training program with weekly schedules',
      category: 'training',
      tags: ['marathon', 'running', 'endurance', 'long-distance'],
      sample_prompts: ['What should my weekly mileage be?', 'How do I taper for race day?'],
      token_count: 1200,
      install_count: 75,
      icon_url: null,
      published_at: '2024-01-15T00:00:00Z',
      author_id: 'author-123',
      system_prompt:
        'You are an expert marathon training coach. Help athletes prepare for marathon races with personalized training plans.',
      created_at: '2024-01-10T00:00:00Z',
      publish_status: 'published',
    });
    vi.mocked(apiService.getStoreInstallations).mockResolvedValue({
      coaches: [],
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    });
  });

  describe('rendering', () => {
    it('should show loading state initially', () => {
      renderStoreCoachDetail();
      expect(screen.getByText('Loading coach details...')).toBeInTheDocument();
    });

    it('should render coach title', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        // Title appears in both header and content, so use getAllByText
        expect(screen.getAllByText('Marathon Training Coach').length).toBeGreaterThanOrEqual(1);
      });
    });

    it('should render coach description', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(
          screen.getByText('A comprehensive marathon training program with weekly schedules')
        ).toBeInTheDocument();
      });
    });

    it('should render coach category badge', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByText('training')).toBeInTheDocument();
      });
    });

    it('should render user count', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByText('75 users')).toBeInTheDocument();
      });
    });

    it('should render coach tags', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByText('marathon')).toBeInTheDocument();
        expect(screen.getByText('running')).toBeInTheDocument();
        expect(screen.getByText('endurance')).toBeInTheDocument();
      });
    });

    it('should render sample prompts', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByText('What should my weekly mileage be?')).toBeInTheDocument();
        expect(screen.getByText('How do I taper for race day?')).toBeInTheDocument();
      });
    });

    it('should render system prompt preview', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByText(/You are an expert marathon training coach/)).toBeInTheDocument();
      });
    });

    it('should render token count in details', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByText('Token Count')).toBeInTheDocument();
        expect(screen.getByText('1,200')).toBeInTheDocument();
      });
    });

    it('should render published date', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByText('Published')).toBeInTheDocument();
      });
    });
  });

  describe('add functionality', () => {
    it('should show Add button when not added', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Coach' })).toBeInTheDocument();
      });
    });

    it('should call installStoreCoach when Add button is clicked', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Coach' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Add Coach' }));

      await waitFor(() => {
        expect(apiService.installStoreCoach).toHaveBeenCalledWith('coach-1');
      });
    });

    it('should show success message after adding', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Coach' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Add Coach' }));

      await waitFor(() => {
        expect(
          screen.getByText(/"Marathon Training Coach" has been added to your coaches/)
        ).toBeInTheDocument();
      });
    });
  });

  describe('remove functionality', () => {
    beforeEach(() => {
      // Mock as installed
      vi.mocked(apiService.getStoreInstallations).mockResolvedValue({
        coaches: [{ id: 'coach-1', title: 'Marathon Training Coach' }],
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      });
    });

    it('should show Remove button when installed', async () => {
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Remove' })).toBeInTheDocument();
      });
    });

    it('should show confirmation dialog when Remove is clicked', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Remove' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Remove' }));

      await waitFor(() => {
        expect(screen.getByText('Remove Coach?')).toBeInTheDocument();
      });
    });

    it('should call uninstallStoreCoach when confirmed', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Remove' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Remove' }));

      await waitFor(() => {
        expect(screen.getByText('Remove Coach?')).toBeInTheDocument();
      });

      // Click confirm button in the dialog - there are now two Remove buttons
      // (one on the page, one in the dialog), so we get all and click the second (dialog button)
      const removeButtons = screen.getAllByRole('button', { name: 'Remove' });
      await user.click(removeButtons[1]);

      await waitFor(() => {
        expect(apiService.uninstallStoreCoach).toHaveBeenCalledWith('coach-1');
      });
    });
  });

  describe('navigation', () => {
    it('should call onBack when back button is clicked', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        // Title appears in both header and content, so use getAllByText
        expect(screen.getAllByText('Marathon Training Coach').length).toBeGreaterThanOrEqual(1);
      });

      const backButton = screen.getByTitle('Back to Store');
      await user.click(backButton);

      expect(mockOnBack).toHaveBeenCalled();
    });

    it('should show View My Coaches link after adding', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Coach' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Add Coach' }));

      await waitFor(() => {
        expect(screen.getByText('View My Coaches →')).toBeInTheDocument();
      });
    });

    it('should call onNavigateToLibrary when View My Coaches is clicked', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Coach' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Add Coach' }));

      await waitFor(() => {
        expect(screen.getByText('View My Coaches →')).toBeInTheDocument();
      });

      await user.click(screen.getByText('View My Coaches →'));

      expect(mockOnNavigateToLibrary).toHaveBeenCalled();
    });
  });

  describe('error handling', () => {
    it('should show error state when coach not found', async () => {
      vi.mocked(apiService.getStoreCoach).mockRejectedValueOnce(new Error('Not found'));

      renderStoreCoachDetail('nonexistent-coach');

      await waitFor(() => {
        expect(screen.getByText('Coach not found')).toBeInTheDocument();
      });
    });

    it('should show Go Back button in error state', async () => {
      vi.mocked(apiService.getStoreCoach).mockRejectedValueOnce(new Error('Not found'));

      renderStoreCoachDetail('nonexistent-coach');

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Go Back' })).toBeInTheDocument();
      });
    });

    it('should call onBack when Go Back is clicked in error state', async () => {
      const user = userEvent.setup();
      vi.mocked(apiService.getStoreCoach).mockRejectedValueOnce(new Error('Not found'));

      renderStoreCoachDetail('nonexistent-coach');

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Go Back' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Go Back' }));

      expect(mockOnBack).toHaveBeenCalled();
    });
  });

  describe('success message dismissal', () => {
    it('should dismiss success message when close button is clicked', async () => {
      const user = userEvent.setup();
      renderStoreCoachDetail();

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Coach' })).toBeInTheDocument();
      });

      await user.click(screen.getByRole('button', { name: 'Add Coach' }));

      await waitFor(() => {
        expect(
          screen.getByText(/"Marathon Training Coach" has been added to your coaches/)
        ).toBeInTheDocument();
      });

      // Find and click the close button on the success message
      const closeButtons = screen.getAllByRole('button');
      const closeButton = closeButtons.find(
        (btn) => btn.querySelector('svg path[d*="6 18L18 6"]') !== null
      );

      if (closeButton) {
        await user.click(closeButton);
        await waitFor(() => {
          expect(
            screen.queryByText(/"Marathon Training Coach" has been added to your coaches/)
          ).not.toBeInTheDocument();
        });
      }
    });
  });
});
