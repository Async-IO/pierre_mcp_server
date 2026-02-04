// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unit tests for ShareChatMessageModal component
// ABOUTME: Tests content sync when prop changes and share functionality

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import ShareChatMessageModal from '../ShareChatMessageModal';
import { socialApi } from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  socialApi: {
    shareFromActivity: vi.fn(),
  },
}));

describe('ShareChatMessageModal', () => {
  const mockOnClose = vi.fn();
  const mockOnSuccess = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(socialApi.shareFromActivity).mockResolvedValue({
      insight: {
        id: 'insight-1',
        user_id: 'user-1',
        visibility: 'friends_only',
        insight_type: 'coaching_insight',
        content: 'Test content',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      },
      metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
    });
  });

  it('should render modal with content', () => {
    render(
      <ShareChatMessageModal
        content="Test insight content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    expect(screen.getByRole('heading', { name: 'Share Insight' })).toBeInTheDocument();
    expect(screen.getByText(/Test insight content/)).toBeInTheDocument();
  });

  it('should sync editedContent when content prop changes', async () => {
    const { rerender } = render(
      <ShareChatMessageModal
        content="Initial content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    // Verify initial content is shown
    expect(screen.getByText(/Initial content/)).toBeInTheDocument();

    // Re-render with new content (simulating prop change while mounted)
    rerender(
      <ShareChatMessageModal
        content="Updated content from new insight"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    // Verify updated content is shown (this tests the useEffect sync)
    await waitFor(() => {
      expect(screen.getByText(/Updated content from new insight/)).toBeInTheDocument();
    });
  });

  it('should share insight with correct content', async () => {
    render(
      <ShareChatMessageModal
        content="My shareable insight content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    // Click share button
    const shareButton = screen.getByRole('button', { name: /Share/ });
    fireEvent.click(shareButton);

    await waitFor(() => {
      expect(socialApi.shareFromActivity).toHaveBeenCalledWith({
        content: 'My shareable insight content',
        insight_type: 'coaching_insight',
        visibility: 'friends_only',
      });
    });

    expect(mockOnSuccess).toHaveBeenCalled();
    expect(mockOnClose).toHaveBeenCalled();
  });

  it('should share with updated content after prop change', async () => {
    const { rerender } = render(
      <ShareChatMessageModal
        content=""
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    // Simulate modal receiving new content (the bug scenario)
    rerender(
      <ShareChatMessageModal
        content="Great training insight!"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    // Click share button
    const shareButton = screen.getByRole('button', { name: /Share/ });
    fireEvent.click(shareButton);

    // Verify the API was called with the NEW content, not empty string
    await waitFor(() => {
      expect(socialApi.shareFromActivity).toHaveBeenCalledWith({
        content: 'Great training insight!',
        insight_type: 'coaching_insight',
        visibility: 'friends_only',
      });
    });
  });

  it('should allow editing content before sharing', async () => {
    render(
      <ShareChatMessageModal
        content="Original content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    // Click edit button
    const editButton = screen.getByRole('button', { name: /Edit/ });
    fireEvent.click(editButton);

    // Find textarea and modify content
    const textarea = screen.getByPlaceholderText('Edit your insight...');
    fireEvent.change(textarea, { target: { value: 'Edited content' } });

    // Click done editing
    const doneButton = screen.getByRole('button', { name: /Done Editing/ });
    fireEvent.click(doneButton);

    // Click share
    const shareButton = screen.getByRole('button', { name: /Share/ });
    fireEvent.click(shareButton);

    await waitFor(() => {
      expect(socialApi.shareFromActivity).toHaveBeenCalledWith({
        content: 'Edited content',
        insight_type: 'coaching_insight',
        visibility: 'friends_only',
      });
    });
  });

  it('should allow changing visibility to public', async () => {
    render(
      <ShareChatMessageModal
        content="Test content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    // Select public visibility
    const publicRadio = screen.getByRole('radio', { name: /Public/i });
    fireEvent.click(publicRadio);

    // Click share
    const shareButton = screen.getByRole('button', { name: /Share/ });
    fireEvent.click(shareButton);

    await waitFor(() => {
      expect(socialApi.shareFromActivity).toHaveBeenCalledWith({
        content: 'Test content',
        insight_type: 'coaching_insight',
        visibility: 'public',
      });
    });
  });

  it('should show error message on share failure', async () => {
    vi.mocked(socialApi.shareFromActivity).mockRejectedValue(new Error('Network error'));

    render(
      <ShareChatMessageModal
        content="Test content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    const shareButton = screen.getByRole('button', { name: /Share/ });
    fireEvent.click(shareButton);

    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeInTheDocument();
    });

    expect(mockOnSuccess).not.toHaveBeenCalled();
    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('should close modal when cancel is clicked', () => {
    render(
      <ShareChatMessageModal
        content="Test content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    const cancelButton = screen.getByRole('button', { name: /Cancel/ });
    fireEvent.click(cancelButton);

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('should disable buttons while submitting', async () => {
    // Make API call hang
    vi.mocked(socialApi.shareFromActivity).mockImplementation(
      () => new Promise(() => {})
    );

    render(
      <ShareChatMessageModal
        content="Test content"
        onClose={mockOnClose}
        onSuccess={mockOnSuccess}
      />
    );

    const shareButton = screen.getByRole('button', { name: /Share/ });
    fireEvent.click(shareButton);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Sharing.../ })).toBeDisabled();
      expect(screen.getByRole('button', { name: /Cancel/ })).toBeDisabled();
    });
  });
});
