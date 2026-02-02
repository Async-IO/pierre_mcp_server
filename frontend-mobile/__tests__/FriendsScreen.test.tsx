// ABOUTME: Unit tests for FriendsScreen component
// ABOUTME: Tests friends list display, search, navigation, and friend removal

import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

// Mock navigation
const mockNavigation = {
  navigate: jest.fn(),
  goBack: jest.fn(),
};

// Mock useFocusEffect - needs to be before imports that use it
jest.mock('@react-navigation/native', () => {
  const actualReact = jest.requireActual('react');
  return {
    useFocusEffect: (callback: () => (() => void) | void) => {
      actualReact.useEffect(() => {
        return callback();
      }, [callback]);
    },
    useNavigation: () => mockNavigation,
  };
});

// Mock AuthContext
jest.mock('../src/contexts/AuthContext', () => ({
  useAuth: () => ({
    isAuthenticated: true,
  }),
}));

// Mock API service
const mockListFriends = jest.fn();
const mockGetPendingRequests = jest.fn();
const mockRemoveFriend = jest.fn();

jest.mock('../src/services/api', () => ({
  socialApi: {
    listFriends: (...args: unknown[]) => mockListFriends(...args),
    getPendingRequests: (...args: unknown[]) => mockGetPendingRequests(...args),
    removeFriend: (...args: unknown[]) => mockRemoveFriend(...args),
  },
}));

import { FriendsScreen } from '../src/screens/social/FriendsScreen';
import type { FriendWithInfo } from '../src/types';

const createMockFriend = (overrides: Partial<FriendWithInfo> = {}): FriendWithInfo => ({
  id: 'friend-1',
  initiator_id: 'user-1',
  receiver_id: 'user-2',
  status: 'accepted',
  friend_user_id: 'user-2',
  friend_display_name: 'Jane Doe',
  friend_email: 'jane@example.com',
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-02T00:00:00Z',
  accepted_at: '2024-01-02T00:00:00Z',
  ...overrides,
});

describe('FriendsScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockListFriends.mockResolvedValue({ friends: [], total: 0 });
    mockGetPendingRequests.mockResolvedValue({ received: [], sent: [] });
    mockRemoveFriend.mockResolvedValue(undefined);
  });

  describe('rendering', () => {
    it('should render header with Friends title', async () => {
      const { getByText } = render(<FriendsScreen />);
      await waitFor(() => {
        expect(getByText('Friends')).toBeTruthy();
      });
    });

    it('should render empty state when no friends', async () => {
      mockListFriends.mockResolvedValue({ friends: [], total: 0 });
      const { getByText } = render(<FriendsScreen />);
      await waitFor(() => {
        expect(getByText('No Friends Yet')).toBeTruthy();
        expect(getByText('Find and connect with other athletes to share coach insights')).toBeTruthy();
      });
    });

    it('should render Find Friends button in empty state', async () => {
      mockListFriends.mockResolvedValue({ friends: [], total: 0 });
      const { getByText } = render(<FriendsScreen />);
      await waitFor(() => {
        expect(getByText('Find Friends')).toBeTruthy();
      });
    });
  });

  describe('friends list', () => {
    it('should render friend cards', async () => {
      const friends = [
        createMockFriend({ id: '1', friend_display_name: 'Alice Smith' }),
        createMockFriend({ id: '2', friend_display_name: 'Bob Jones', friend_email: 'bob@example.com' }),
      ];
      mockListFriends.mockResolvedValue({ friends, total: 2 });

      const { getByText } = render(<FriendsScreen />);

      await waitFor(() => {
        expect(getByText('Alice Smith')).toBeTruthy();
        expect(getByText('Bob Jones')).toBeTruthy();
      });
    });

    it('should render search bar when friends exist', async () => {
      const friends = [createMockFriend()];
      mockListFriends.mockResolvedValue({ friends, total: 1 });

      const { getByPlaceholderText } = render(<FriendsScreen />);

      await waitFor(() => {
        expect(getByPlaceholderText('Search friends...')).toBeTruthy();
      });
    });

    it('should filter friends when searching', async () => {
      const friends = [
        createMockFriend({ id: '1', friend_display_name: 'Alice Smith' }),
        createMockFriend({ id: '2', friend_display_name: 'Bob Jones' }),
      ];
      mockListFriends.mockResolvedValue({ friends, total: 2 });

      const { getByPlaceholderText, getByText, queryByText } = render(<FriendsScreen />);

      await waitFor(() => {
        expect(getByText('Alice Smith')).toBeTruthy();
        expect(getByText('Bob Jones')).toBeTruthy();
      });

      const searchInput = getByPlaceholderText('Search friends...');
      fireEvent.changeText(searchInput, 'Alice');

      await waitFor(() => {
        expect(getByText('Alice Smith')).toBeTruthy();
        expect(queryByText('Bob Jones')).toBeNull();
      });
    });
  });

  describe('pending requests badge', () => {
    it('should show pending badge when there are pending requests', async () => {
      mockListFriends.mockResolvedValue({ friends: [], total: 0 });
      mockGetPendingRequests.mockResolvedValue({
        received: [{ id: 'req-1', status: 'pending' }],
        sent: [],
      });

      const { getByText } = render(<FriendsScreen />);

      await waitFor(() => {
        expect(getByText('1')).toBeTruthy();
      });
    });

    it('should show 9+ when more than 9 pending requests', async () => {
      mockListFriends.mockResolvedValue({ friends: [], total: 0 });
      mockGetPendingRequests.mockResolvedValue({
        received: Array(15).fill({ id: 'req', status: 'pending' }),
        sent: [],
      });

      const { getByText } = render(<FriendsScreen />);

      await waitFor(() => {
        expect(getByText('9+')).toBeTruthy();
      });
    });
  });

  describe('navigation', () => {
    it('should navigate to SearchFriends when Find Friends button pressed', async () => {
      mockListFriends.mockResolvedValue({ friends: [], total: 0 });

      const { getByText } = render(<FriendsScreen />);

      await waitFor(() => {
        expect(getByText('Find Friends')).toBeTruthy();
      });

      fireEvent.press(getByText('Find Friends'));
      expect(mockNavigation.navigate).toHaveBeenCalledWith('SearchFriends');
    });
  });

  describe('friend display', () => {
    it('should display friend connected date', async () => {
      const friends = [
        createMockFriend({
          id: 'friend-1',
          accepted_at: '2024-01-02T00:00:00Z',
        }),
      ];
      mockListFriends.mockResolvedValue({ friends, total: 1 });

      const { getByText } = render(<FriendsScreen />);

      await waitFor(() => {
        expect(getByText('Jane Doe')).toBeTruthy();
      });

      // Should show connected since date
      expect(getByText(/Friends since/)).toBeTruthy();
    });
  });
});
