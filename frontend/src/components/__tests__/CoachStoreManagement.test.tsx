// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for CoachStoreManagement component
// ABOUTME: Tests stats dashboard, tab navigation, and lazy loading

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// Mock the API service
vi.mock('../../services/api', () => ({
  apiService: {
    getStoreStats: vi.fn().mockResolvedValue({
      pending_count: 5,
      published_count: 12,
      rejected_count: 3,
      total_installs: 150,
      rejection_rate: 0.2,
    }),
    getStoreReviewQueue: vi.fn().mockResolvedValue({
      coaches: [],
      total: 0,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
    getPublishedStoreCoaches: vi.fn().mockResolvedValue({
      coaches: [],
      total: 0,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
    getRejectedStoreCoaches: vi.fn().mockResolvedValue({
      coaches: [],
      total: 0,
      metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
    }),
  },
}));

import CoachStoreManagement from '../CoachStoreManagement';
import { apiService } from '../../services/api';

function renderCoachStoreManagement() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <CoachStoreManagement />
    </QueryClientProvider>
  );
}

describe('CoachStoreManagement', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiService.getStoreStats).mockResolvedValue({
      pending_count: 5,
      published_count: 12,
      rejected_count: 3,
      total_installs: 150,
      rejection_rate: 0.2,
    });
  });

  it('renders the header correctly', async () => {
    renderCoachStoreManagement();

    expect(screen.getByText('Coach Store Management')).toBeInTheDocument();
    expect(
      screen.getByText(/Review coach submissions, manage published coaches/)
    ).toBeInTheDocument();
  });

  it('displays stats cards with correct values', async () => {
    renderCoachStoreManagement();

    await waitFor(() => {
      // '5' appears multiple times (badge and stat), use getAllByText
      const fives = screen.getAllByText('5');
      expect(fives.length).toBeGreaterThanOrEqual(1);
    });

    expect(screen.getByText('12')).toBeInTheDocument(); // published_count
    expect(screen.getByText('150')).toBeInTheDocument(); // total_installs
    expect(screen.getByText('20.0%')).toBeInTheDocument(); // rejection_rate (0.2 * 100)
  });

  it('shows loading state for stats', () => {
    vi.mocked(apiService.getStoreStats).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    renderCoachStoreManagement();

    // Should show loading placeholders (animated divs)
    const loadingDivs = document.querySelectorAll('.animate-pulse');
    expect(loadingDivs.length).toBeGreaterThan(0);
  });

  it('renders all three tabs', async () => {
    renderCoachStoreManagement();

    // Use getAllByRole since stat cards also contain similar text
    const reviewQueueButtons = screen.getAllByRole('button', { name: /Review Queue/i });
    const publishedButtons = screen.getAllByRole('button', { name: /Published$/i });
    const rejectedButtons = screen.getAllByRole('button', { name: /Rejected$/i });

    expect(reviewQueueButtons.length).toBeGreaterThanOrEqual(1);
    expect(publishedButtons.length).toBeGreaterThanOrEqual(1);
    expect(rejectedButtons.length).toBeGreaterThanOrEqual(1);
  });

  it('shows pending count badge on Review Queue tab', async () => {
    renderCoachStoreManagement();

    await waitFor(() => {
      // The badge shows the count
      const badges = screen.getAllByText('5');
      expect(badges.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('switches tabs when clicked', async () => {
    const user = userEvent.setup();
    renderCoachStoreManagement();

    // Click Published tab (ends with "Published" to avoid matching "Published Coaches" stat card)
    const publishedTabs = screen.getAllByRole('button', { name: /Published$/i });
    const publishedTab = publishedTabs.find(btn => btn.textContent?.trim() === 'Published');
    expect(publishedTab).toBeInTheDocument();

    if (publishedTab) {
      await user.click(publishedTab);
      // Tab should be active (check for active styling class)
      expect(publishedTab).toHaveClass('border-pierre-violet');
    }
  });

  it('clicking stats cards switches to corresponding tab', async () => {
    const user = userEvent.setup();
    renderCoachStoreManagement();

    await waitFor(() => {
      expect(screen.getByText('Pending Reviews')).toBeInTheDocument();
    });

    // Find and click the Published Coaches stat card
    const publishedCard = screen.getByText('Published Coaches').closest('button');
    expect(publishedCard).toBeInTheDocument();

    if (publishedCard) {
      await user.click(publishedCard);
      // The Published tab should now be active
      const publishedTabs = screen.getAllByRole('button', { name: /Published$/i });
      const publishedTab = publishedTabs.find(btn => btn.textContent?.trim() === 'Published');
      expect(publishedTab).toHaveClass('border-pierre-violet');
    }
  });

  it('formats numbers with locale formatting', async () => {
    vi.mocked(apiService.getStoreStats).mockResolvedValue({
      pending_count: 1234,
      published_count: 5678,
      rejected_count: 90,
      total_installs: 12345,
      rejection_rate: 0.156,
    });

    renderCoachStoreManagement();

    await waitFor(() => {
      // Numbers should be formatted with commas
      expect(screen.getByText('1,234')).toBeInTheDocument();
      expect(screen.getByText('5,678')).toBeInTheDocument();
      expect(screen.getByText('12,345')).toBeInTheDocument();
      expect(screen.getByText('15.6%')).toBeInTheDocument();
    });
  });

  it('handles zero stats gracefully', async () => {
    vi.mocked(apiService.getStoreStats).mockResolvedValue({
      pending_count: 0,
      published_count: 0,
      rejected_count: 0,
      total_installs: 0,
      rejection_rate: 0,
    });

    renderCoachStoreManagement();

    await waitFor(() => {
      const zeros = screen.getAllByText('0');
      expect(zeros.length).toBeGreaterThanOrEqual(3);
    });

    expect(screen.getByText('0.0%')).toBeInTheDocument();
  });
});
