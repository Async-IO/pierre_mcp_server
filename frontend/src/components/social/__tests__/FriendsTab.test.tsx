// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unit tests for FriendsTab component
// ABOUTME: Tests friend list display, search, pending requests, and actions

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import FriendsTab from '../FriendsTab';
import { apiService } from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  apiService: {
    listFriends: vi.fn(),
    searchUsers: vi.fn(),
    getPendingFriendRequests: vi.fn(),
    sendFriendRequest: vi.fn(),
    acceptFriendRequest: vi.fn(),
    rejectFriendRequest: vi.fn(),
    removeFriend: vi.fn(),
  },
}));

const mockFriends = {
  friends: [
    {
      id: 'conn-1',
      initiator_id: 'user-1',
      receiver_id: 'user-2',
      status: 'accepted',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
      accepted_at: '2024-01-02T00:00:00Z',
      friend_display_name: 'Jane Doe',
      friend_email: 'jane@example.com',
      friend_user_id: 'user-2',
    },
  ],
  total: 1,
  metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
};

const mockSearchResults = {
  users: [
    {
      user_id: 'user-3',
      display_name: 'Bob Smith',
      email: 'bob@example.com',
      mutual_friends_count: 2,
      is_friend: false,
      pending_request: false,
    },
  ],
  query: 'bob',
  metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
};

const mockPendingRequests = {
  sent: [],
  received: [
    {
      id: 'conn-2',
      initiator_id: 'user-4',
      receiver_id: 'user-1',
      status: 'pending',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
      accepted_at: null,
    },
  ],
  metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
};

describe('FriendsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiService.listFriends).mockResolvedValue(mockFriends);
    vi.mocked(apiService.getPendingFriendRequests).mockResolvedValue(mockPendingRequests);
    vi.mocked(apiService.searchUsers).mockResolvedValue(mockSearchResults);
    vi.mocked(apiService.sendFriendRequest).mockResolvedValue({
      id: 'conn-3',
      initiator_id: 'user-1',
      receiver_id: 'user-3',
      status: 'pending',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
      accepted_at: null,
    });
    vi.mocked(apiService.acceptFriendRequest).mockResolvedValue({
      id: 'conn-2',
      initiator_id: 'user-4',
      receiver_id: 'user-1',
      status: 'accepted',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
      accepted_at: '2024-01-02T00:00:00Z',
    });
    vi.mocked(apiService.rejectFriendRequest).mockResolvedValue(undefined);
    vi.mocked(apiService.removeFriend).mockResolvedValue(undefined);
  });

  it('should render the Friends tab with title', async () => {
    render(<FriendsTab />);

    // Use getByRole to get the specific heading element, not the tab button
    expect(screen.getByRole('heading', { name: 'Friends', level: 2 })).toBeInTheDocument();
    expect(screen.getByText('Connect with other athletes and share coach insights')).toBeInTheDocument();
  });

  it('should display friend list on mount', async () => {
    render(<FriendsTab />);

    await waitFor(() => {
      expect(screen.getByText('Jane Doe')).toBeInTheDocument();
    });

    expect(apiService.listFriends).toHaveBeenCalled();
  });

  it('should show empty state when no friends', async () => {
    vi.mocked(apiService.listFriends).mockResolvedValue({
      friends: [],
      total: 0,
      metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
    });

    render(<FriendsTab />);

    await waitFor(() => {
      expect(screen.getByText('No friends yet')).toBeInTheDocument();
    });
  });

  it('should switch to search tab and show search results', async () => {
    render(<FriendsTab />);

    // Wait for initial load
    await waitFor(() => {
      expect(apiService.listFriends).toHaveBeenCalled();
    });

    // Click on Find Friends tab
    const findFriendsTab = screen.getByText('Find Friends');
    fireEvent.click(findFriendsTab);

    // Search for a user
    const searchInput = screen.getByPlaceholderText('Search by name or email...');
    fireEvent.change(searchInput, { target: { value: 'bob' } });

    const searchButton = screen.getByRole('button', { name: 'Search' });
    fireEvent.click(searchButton);

    await waitFor(() => {
      expect(apiService.searchUsers).toHaveBeenCalledWith('bob');
    });

    await waitFor(() => {
      expect(screen.getByText('Bob Smith')).toBeInTheDocument();
    });
  });

  it('should send friend request from search results', async () => {
    render(<FriendsTab />);

    // Switch to search tab
    const findFriendsTab = screen.getByText('Find Friends');
    fireEvent.click(findFriendsTab);

    // Search and get results
    const searchInput = screen.getByPlaceholderText('Search by name or email...');
    fireEvent.change(searchInput, { target: { value: 'bob' } });
    fireEvent.click(screen.getByRole('button', { name: 'Search' }));

    await waitFor(() => {
      expect(screen.getByText('Bob Smith')).toBeInTheDocument();
    });

    // Click Add Friend button
    const addButton = screen.getByRole('button', { name: 'Add Friend' });
    fireEvent.click(addButton);

    await waitFor(() => {
      expect(apiService.sendFriendRequest).toHaveBeenCalledWith('user-3');
    });
  });

  it('should switch to pending tab and show received requests', async () => {
    render(<FriendsTab />);

    await waitFor(() => {
      expect(apiService.listFriends).toHaveBeenCalled();
    });

    // Click on Pending tab
    const pendingTab = screen.getByRole('button', { name: /Pending/i });
    fireEvent.click(pendingTab);

    await waitFor(() => {
      expect(apiService.getPendingFriendRequests).toHaveBeenCalled();
    });

    expect(screen.getByText('Received Requests (1)')).toBeInTheDocument();
  });

  it('should accept a friend request', async () => {
    render(<FriendsTab />);

    // Switch to pending tab
    const pendingTab = screen.getByRole('button', { name: /Pending/i });
    fireEvent.click(pendingTab);

    await waitFor(() => {
      expect(screen.getByText('Received Requests (1)')).toBeInTheDocument();
    });

    // Click Accept button
    const acceptButton = screen.getByRole('button', { name: 'Accept' });
    fireEvent.click(acceptButton);

    await waitFor(() => {
      expect(apiService.acceptFriendRequest).toHaveBeenCalledWith('conn-2');
    });
  });

  it('should remove a friend', async () => {
    render(<FriendsTab />);

    await waitFor(() => {
      expect(screen.getByText('Jane Doe')).toBeInTheDocument();
    });

    // Click Remove button
    const removeButton = screen.getByRole('button', { name: 'Remove' });
    fireEvent.click(removeButton);

    await waitFor(() => {
      expect(apiService.removeFriend).toHaveBeenCalledWith('user-2');
    });
  });
});
