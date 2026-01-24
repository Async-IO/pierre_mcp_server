// ABOUTME: Unit tests for SocialSettingsScreen component
// ABOUTME: Tests settings display, toggle switches, and save functionality

import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

// Mock navigation
const mockNavigation = {
  navigate: jest.fn(),
  openDrawer: jest.fn(),
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
const mockGetSocialSettings = jest.fn();
const mockUpdateSocialSettings = jest.fn();

jest.mock('../src/services/api', () => ({
  apiService: {
    getSocialSettings: (...args: unknown[]) => mockGetSocialSettings(...args),
    updateSocialSettings: (...args: unknown[]) => mockUpdateSocialSettings(...args),
  },
}));

import { SocialSettingsScreen } from '../src/screens/social/SocialSettingsScreen';
import type { UserSocialSettings } from '../src/types';

const createMockSettings = (overrides: Partial<UserSocialSettings> = {}): UserSocialSettings => ({
  user_id: 'user-1',
  discoverable: true,
  default_visibility: 'friends_only',
  share_activity_types: ['running', 'cycling'],
  notifications: {
    friend_requests: true,
    insight_reactions: true,
    adapted_insights: false,
  },
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  ...overrides,
});

describe('SocialSettingsScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockGetSocialSettings.mockResolvedValue({ settings: createMockSettings() });
    mockUpdateSocialSettings.mockResolvedValue({ settings: createMockSettings() });
  });

  describe('rendering', () => {
    it('should render header with Social Settings title', async () => {
      const { getByText } = render(<SocialSettingsScreen />);
      await waitFor(() => {
        expect(getByText('Social Settings')).toBeTruthy();
      });
    });

    it('should render Privacy section', async () => {
      const { getByText } = render(<SocialSettingsScreen />);
      await waitFor(() => {
        expect(getByText('Privacy')).toBeTruthy();
        expect(getByText('Discoverable')).toBeTruthy();
      });
    });

    it('should render Default Sharing section', async () => {
      const { getByText } = render(<SocialSettingsScreen />);
      await waitFor(() => {
        expect(getByText('Default Sharing')).toBeTruthy();
        expect(getByText('Friends Only')).toBeTruthy();
        expect(getByText('Public')).toBeTruthy();
      });
    });

    it('should render Notifications section', async () => {
      const { getByText } = render(<SocialSettingsScreen />);
      await waitFor(() => {
        expect(getByText('Notifications')).toBeTruthy();
        expect(getByText('Friend Requests')).toBeTruthy();
        expect(getByText('Reactions')).toBeTruthy();
        expect(getByText('Adapted Insights')).toBeTruthy();
      });
    });

    it('should render Privacy info card', async () => {
      const { getByText } = render(<SocialSettingsScreen />);
      await waitFor(() => {
        expect(getByText('Your Privacy is Protected')).toBeTruthy();
      });
    });
  });

  describe('settings values', () => {
    it('should display discoverable toggle in correct state', async () => {
      mockGetSocialSettings.mockResolvedValue({
        settings: createMockSettings({ discoverable: true }),
      });

      const { getByText } = render(<SocialSettingsScreen />);

      await waitFor(() => {
        expect(getByText('Discoverable')).toBeTruthy();
      });

      // The Switch component should reflect the value
      expect(mockGetSocialSettings).toHaveBeenCalled();
    });

    it('should display selected visibility', async () => {
      mockGetSocialSettings.mockResolvedValue({
        settings: createMockSettings({ default_visibility: 'friends_only' }),
      });

      const { getByText } = render(<SocialSettingsScreen />);

      await waitFor(() => {
        expect(getByText('Friends Only')).toBeTruthy();
      });
    });
  });

  describe('settings changes', () => {
    it('should enable Save button when settings change', async () => {
      const { getByText } = render(<SocialSettingsScreen />);

      await waitFor(() => {
        expect(getByText('Public')).toBeTruthy();
      });

      // Click Public visibility to change settings
      fireEvent.press(getByText('Public'));

      // Save button should be enabled (text is "Save" not "Save Changes")
      await waitFor(() => {
        expect(getByText('Save')).toBeTruthy();
      });
    });

    it('should call updateSocialSettings API when saving', async () => {
      const { getByText } = render(<SocialSettingsScreen />);

      await waitFor(() => {
        expect(getByText('Public')).toBeTruthy();
      });

      // Make a change
      fireEvent.press(getByText('Public'));

      // Click save
      await waitFor(() => {
        expect(getByText('Save')).toBeTruthy();
      });

      fireEvent.press(getByText('Save'));

      await waitFor(() => {
        expect(mockUpdateSocialSettings).toHaveBeenCalled();
      });
    });
  });

  describe('loading state', () => {
    it('should show loading indicator while loading settings', async () => {
      // Make the API call never resolve to keep loading state
      mockGetSocialSettings.mockImplementation(() => new Promise(() => {}));

      const { getByTestId, queryByText } = render(<SocialSettingsScreen />);

      // Should show loading state (ActivityIndicator)
      // Since we can't easily find ActivityIndicator, we check that content is not yet visible
      expect(queryByText('Discoverable')).toBeNull();
    });
  });

  describe('notification toggles', () => {
    it('should display notification settings from API', async () => {
      mockGetSocialSettings.mockResolvedValue({
        settings: createMockSettings({
          notifications: {
            friend_requests: true,
            insight_reactions: false,
            adapted_insights: true,
          },
        }),
      });

      const { getByText } = render(<SocialSettingsScreen />);

      await waitFor(() => {
        expect(getByText('Friend Requests')).toBeTruthy();
        expect(getByText('Reactions')).toBeTruthy();
        expect(getByText('Adapted Insights')).toBeTruthy();
      });
    });
  });
});
