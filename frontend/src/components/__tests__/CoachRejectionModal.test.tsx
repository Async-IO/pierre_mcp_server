// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Unit tests for CoachRejectionModal component
// ABOUTME: Tests reason selection, notes input, and rejection submission

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockCoach = {
  id: 'coach-1',
  title: 'Marathon Training Coach',
  author_email: 'coach@example.com',
};

// Mock the admin API
vi.mock('../../services/api', () => ({
  adminApi: {
    rejectStoreCoach: vi.fn().mockResolvedValue({
      success: true,
      message: 'Coach rejected',
      coach_id: 'coach-1',
    }),
  },
}));

import CoachRejectionModal from '../CoachRejectionModal';
import { adminApi } from '../../services/api';

const mockOnClose = vi.fn();
const mockOnComplete = vi.fn();

function renderCoachRejectionModal(coach = mockCoach, isOpen = true) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <CoachRejectionModal
        coach={coach}
        isOpen={isOpen}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    </QueryClientProvider>
  );
}

describe('CoachRejectionModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders nothing when not open', () => {
    renderCoachRejectionModal(mockCoach, false);
    expect(screen.queryByRole('heading', { name: 'Reject Coach' })).not.toBeInTheDocument();
  });

  it('renders nothing when coach is null', () => {
    renderCoachRejectionModal(null, true);
    expect(screen.queryByRole('heading', { name: 'Reject Coach' })).not.toBeInTheDocument();
  });

  it('displays modal with coach info when open', () => {
    renderCoachRejectionModal();

    expect(screen.getByRole('heading', { name: 'Reject Coach' })).toBeInTheDocument();
    expect(screen.getByText(/"Marathon Training Coach" by coach@example.com/)).toBeInTheDocument();
  });

  it('shows warning message', () => {
    renderCoachRejectionModal();

    expect(
      screen.getByText(/This action will reject the coach submission/)
    ).toBeInTheDocument();
  });

  it('has rejection reason dropdown with all options', () => {
    renderCoachRejectionModal();

    const select = screen.getByRole('combobox');
    expect(select).toBeInTheDocument();

    // Check for all options
    expect(screen.getByText('Select a reason...')).toBeInTheDocument();
    expect(screen.getByText('Inappropriate content')).toBeInTheDocument();
    expect(screen.getByText('Quality standards not met')).toBeInTheDocument();
    expect(screen.getByText('Duplicate submission')).toBeInTheDocument();
    expect(screen.getByText('Incomplete information')).toBeInTheDocument();
    expect(screen.getByText('Other')).toBeInTheDocument();
  });

  it('has notes textarea', () => {
    renderCoachRejectionModal();

    const textarea = screen.getByPlaceholderText(
      'Provide additional context for the rejection...'
    );
    expect(textarea).toBeInTheDocument();
  });

  it('reject button is disabled when no reason selected', () => {
    renderCoachRejectionModal();

    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    expect(rejectButton).toBeDisabled();
  });

  it('reject button is enabled when reason selected', async () => {
    const user = userEvent.setup();
    renderCoachRejectionModal();

    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'quality_standards');

    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    expect(rejectButton).not.toBeDisabled();
  });

  it('calls onClose when cancel clicked', async () => {
    const user = userEvent.setup();
    renderCoachRejectionModal();

    const cancelButton = screen.getByRole('button', { name: /Cancel/i });
    await user.click(cancelButton);

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('calls onClose when X button clicked', async () => {
    const user = userEvent.setup();
    renderCoachRejectionModal();

    const closeButton = screen.getByLabelText('Close modal');
    await user.click(closeButton);

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('submits rejection with reason only', async () => {
    const user = userEvent.setup();
    renderCoachRejectionModal();

    // Select a reason
    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'quality_standards');

    // Click reject
    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    await user.click(rejectButton);

    await waitFor(() => {
      expect(adminApi.rejectStoreCoach).toHaveBeenCalledWith(
        'coach-1',
        'quality_standards',
        undefined
      );
    });
  });

  it('submits rejection with reason and notes', async () => {
    const user = userEvent.setup();
    renderCoachRejectionModal();

    // Select a reason
    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'other');

    // Add notes
    const textarea = screen.getByPlaceholderText(
      'Provide additional context for the rejection...'
    );
    await user.type(textarea, 'This coach needs more detail');

    // Click reject
    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    await user.click(rejectButton);

    await waitFor(() => {
      expect(adminApi.rejectStoreCoach).toHaveBeenCalledWith(
        'coach-1',
        'other',
        'This coach needs more detail'
      );
    });
  });

  it('calls onComplete after successful rejection', async () => {
    const user = userEvent.setup();
    renderCoachRejectionModal();

    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'inappropriate_content');

    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    await user.click(rejectButton);

    await waitFor(() => {
      expect(mockOnComplete).toHaveBeenCalled();
    });
  });

  it('shows loading state during rejection', async () => {
    const user = userEvent.setup();
    vi.mocked(adminApi.rejectStoreCoach).mockImplementation(
      () => new Promise(() => {})
    );

    renderCoachRejectionModal();

    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'quality_standards');

    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    await user.click(rejectButton);

    await waitFor(() => {
      expect(screen.getByText('Rejecting...')).toBeInTheDocument();
    });
  });

  it('shows error message on API failure', async () => {
    const user = userEvent.setup();
    vi.mocked(adminApi.rejectStoreCoach).mockRejectedValue(new Error('API Error'));

    renderCoachRejectionModal();

    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'quality_standards');

    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    await user.click(rejectButton);

    await waitFor(() => {
      expect(screen.getByText('Failed to reject coach. Please try again.')).toBeInTheDocument();
    });
  });

  it('trims whitespace from notes', async () => {
    const user = userEvent.setup();
    renderCoachRejectionModal();

    const select = screen.getByRole('combobox');
    await user.selectOptions(select, 'other');

    const textarea = screen.getByPlaceholderText(
      'Provide additional context for the rejection...'
    );
    await user.type(textarea, '   trimmed note   ');

    const rejectButton = screen.getByRole('button', { name: /Reject Coach/i });
    await user.click(rejectButton);

    await waitFor(() => {
      expect(adminApi.rejectStoreCoach).toHaveBeenCalledWith(
        'coach-1',
        'other',
        'trimmed note'
      );
    });
  });
});
