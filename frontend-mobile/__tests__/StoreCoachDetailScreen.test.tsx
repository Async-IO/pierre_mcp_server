// ABOUTME: Unit tests for StoreCoachDetailScreen component
// ABOUTME: Tests coach detail display, install/uninstall functionality

import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';
import { Alert } from 'react-native';

// Mock navigation
const mockNavigation = {
  navigate: jest.fn(),
  goBack: jest.fn(),
};

const mockRoute = {
  params: { coachId: 'test-coach-id' },
};

// Mock AuthContext
jest.mock('../src/contexts/AuthContext', () => ({
  useAuth: () => ({
    isAuthenticated: true,
  }),
}));

// Mock API service
const mockGetStoreCoach = jest.fn();
const mockInstallStoreCoach = jest.fn();
const mockUninstallStoreCoach = jest.fn();
const mockGetInstalledCoaches = jest.fn();

jest.mock('../src/services/api', () => ({
  apiService: {
    getStoreCoach: (...args: unknown[]) => mockGetStoreCoach(...args),
    installStoreCoach: (...args: unknown[]) => mockInstallStoreCoach(...args),
    uninstallStoreCoach: (...args: unknown[]) => mockUninstallStoreCoach(...args),
    getInstalledCoaches: (...args: unknown[]) => mockGetInstalledCoaches(...args),
  },
}));

// Mock Alert
jest.spyOn(Alert, 'alert');

import { StoreCoachDetailScreen } from '../src/screens/store/StoreCoachDetailScreen';
import type { StoreCoachDetail, CoachCategory } from '../src/types';

const createMockStoreCoachDetail = (overrides: Partial<StoreCoachDetail> = {}): StoreCoachDetail => ({
  id: 'test-coach-id',
  title: 'Marathon Training Coach',
  description: 'A comprehensive marathon training program',
  category: 'training' as CoachCategory,
  tags: ['marathon', 'running', 'endurance'],
  sample_prompts: [
    'What should my weekly mileage be?',
    'How do I prevent injuries?',
    'What pace should I run my long runs?',
  ],
  system_prompt: 'You are an expert marathon coach with years of experience...',
  token_count: 1200,
  install_count: 75,
  icon_url: null,
  published_at: '2024-01-15T00:00:00Z',
  author_id: 'author-123',
  created_at: '2024-01-10T00:00:00Z',
  publish_status: 'published',
  ...overrides,
});

describe('StoreCoachDetailScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockGetStoreCoach.mockResolvedValue(createMockStoreCoachDetail());
    mockGetInstalledCoaches.mockResolvedValue({ coaches: [] });
  });

  describe('rendering', () => {
    it('should show loading state initially', async () => {
      // Delay the API response
      let resolvePromise: (value: unknown) => void;
      mockGetStoreCoach.mockReturnValue(
        new Promise((resolve) => {
          resolvePromise = resolve;
        })
      );

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      expect(getByText('Loading coach details...')).toBeTruthy();

      // Cleanup
      resolvePromise!(createMockStoreCoachDetail());
      await waitFor(() => {});
    });

    it('should render coach title', async () => {
      const { getAllByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        // Title appears in both header and content area
        expect(getAllByText('Marathon Training Coach').length).toBeGreaterThan(0);
      });
    });

    it('should render coach description', async () => {
      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('A comprehensive marathon training program')).toBeTruthy();
      });
    });

    it('should render category badge', async () => {
      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('training')).toBeTruthy();
      });
    });

    it('should render install count', async () => {
      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('75 installs')).toBeTruthy();
      });
    });

    it('should render tags', async () => {
      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('marathon')).toBeTruthy();
        expect(getByText('running')).toBeTruthy();
        expect(getByText('endurance')).toBeTruthy();
      });
    });

    it('should render sample prompts', async () => {
      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('What should my weekly mileage be?')).toBeTruthy();
        expect(getByText('How do I prevent injuries?')).toBeTruthy();
      });
    });

    it('should render system prompt section', async () => {
      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('System Prompt')).toBeTruthy();
      });
    });

    it('should render token count in details', async () => {
      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Token Count')).toBeTruthy();
        expect(getByText('1200')).toBeTruthy();
      });
    });

    it('should show error state when coach not found', async () => {
      mockGetStoreCoach.mockResolvedValue(null);

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Coach not found')).toBeTruthy();
      });
    });
  });

  describe('install functionality', () => {
    it('should show Install button when coach is not installed', async () => {
      mockGetInstalledCoaches.mockResolvedValue({ coaches: [] });

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Install Coach')).toBeTruthy();
      });
    });

    it('should call installStoreCoach when Install button is pressed', async () => {
      mockGetInstalledCoaches.mockResolvedValue({ coaches: [] });
      mockInstallStoreCoach.mockResolvedValue({
        coach_id: 'new-coach-id',
        message: 'Successfully installed',
      });

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Install Coach')).toBeTruthy();
      });

      fireEvent.press(getByText('Install Coach'));

      await waitFor(() => {
        expect(mockInstallStoreCoach).toHaveBeenCalledWith('test-coach-id');
      });
    });

    it('should show success alert after installation', async () => {
      mockGetInstalledCoaches.mockResolvedValue({ coaches: [] });
      mockInstallStoreCoach.mockResolvedValue({
        coach_id: 'new-coach-id',
        message: 'Successfully installed',
      });

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Install Coach')).toBeTruthy();
      });

      fireEvent.press(getByText('Install Coach'));

      await waitFor(() => {
        expect(Alert.alert).toHaveBeenCalledWith(
          'Installed!',
          expect.stringContaining('Marathon Training Coach'),
          expect.any(Array)
        );
      });
    });

    it('should show error alert on installation failure', async () => {
      mockGetInstalledCoaches.mockResolvedValue({ coaches: [] });
      mockInstallStoreCoach.mockRejectedValue(new Error('Installation failed'));

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Install Coach')).toBeTruthy();
      });

      fireEvent.press(getByText('Install Coach'));

      await waitFor(() => {
        expect(Alert.alert).toHaveBeenCalledWith(
          'Error',
          'Failed to install coach. Please try again.'
        );
      });
    });
  });

  describe('uninstall functionality', () => {
    it('should show Uninstall button when coach is installed', async () => {
      mockGetInstalledCoaches.mockResolvedValue({
        coaches: [{ id: 'test-coach-id' }],
      });

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Uninstall')).toBeTruthy();
      });
    });

    it('should show confirmation dialog when Uninstall is pressed', async () => {
      mockGetInstalledCoaches.mockResolvedValue({
        coaches: [{ id: 'test-coach-id' }],
      });

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Uninstall')).toBeTruthy();
      });

      fireEvent.press(getByText('Uninstall'));

      await waitFor(() => {
        expect(Alert.alert).toHaveBeenCalledWith(
          'Uninstall Coach?',
          expect.stringContaining('Marathon Training Coach'),
          expect.any(Array)
        );
      });
    });

    it('should call uninstallStoreCoach when confirmed', async () => {
      mockGetInstalledCoaches.mockResolvedValue({
        coaches: [{ id: 'test-coach-id' }],
      });
      mockUninstallStoreCoach.mockResolvedValue({ message: 'Uninstalled' });

      // Mock Alert to automatically call the destructive action
      (Alert.alert as jest.Mock).mockImplementation(
        (title, message, buttons) => {
          const uninstallButton = buttons?.find(
            (b: { text: string }) => b.text === 'Uninstall'
          );
          if (uninstallButton?.onPress) {
            uninstallButton.onPress();
          }
        }
      );

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Uninstall')).toBeTruthy();
      });

      fireEvent.press(getByText('Uninstall'));

      await waitFor(() => {
        expect(mockUninstallStoreCoach).toHaveBeenCalledWith('test-coach-id');
      });
    });
  });

  describe('navigation', () => {
    it('should go back when back button is pressed', async () => {
      const { getAllByText, getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        // Title appears in both header and content area
        expect(getAllByText('Marathon Training Coach').length).toBeGreaterThan(0);
      });

      // Find and press back button (back arrow text)
      fireEvent.press(getByText('←'));

      expect(mockNavigation.navigate).toHaveBeenCalledWith('Store');
    });

    it('should navigate to CoachLibrary after successful install', async () => {
      mockGetInstalledCoaches.mockResolvedValue({ coaches: [] });
      mockInstallStoreCoach.mockResolvedValue({
        coach_id: 'new-coach-id',
        message: 'Successfully installed',
      });

      // Mock Alert to call the "View My Coaches" action
      (Alert.alert as jest.Mock).mockImplementation(
        (title, message, buttons) => {
          const viewButton = buttons?.find(
            (b: { text: string }) => b.text === 'View My Coaches'
          );
          if (viewButton?.onPress) {
            viewButton.onPress();
          }
        }
      );

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('Install Coach')).toBeTruthy();
      });

      fireEvent.press(getByText('Install Coach'));

      await waitFor(() => {
        expect(mockNavigation.navigate).toHaveBeenCalledWith('CoachLibrary');
      });
    });
  });

  describe('edge cases', () => {
    it('should handle coach with no tags', async () => {
      mockGetStoreCoach.mockResolvedValue(
        createMockStoreCoachDetail({ tags: [] })
      );

      const { queryByText, getAllByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        // Title appears in both header and content area
        expect(getAllByText('Marathon Training Coach').length).toBeGreaterThan(0);
        // Tags section should not show empty tags
        expect(queryByText('marathon')).toBeNull();
      });
    });

    it('should handle coach with no sample prompts', async () => {
      mockGetStoreCoach.mockResolvedValue(
        createMockStoreCoachDetail({ sample_prompts: [] })
      );

      const { getAllByText, queryByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        // Title appears in both header and content area
        expect(getAllByText('Marathon Training Coach').length).toBeGreaterThan(0);
        expect(queryByText('What should my weekly mileage be?')).toBeNull();
      });
    });

    it('should handle singular install count', async () => {
      mockGetStoreCoach.mockResolvedValue(
        createMockStoreCoachDetail({ install_count: 1 })
      );

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(getByText('1 install')).toBeTruthy();
      });
    });

    it('should handle API error gracefully', async () => {
      mockGetStoreCoach.mockRejectedValue(new Error('Network error'));

      const { getByText } = render(
        <StoreCoachDetailScreen
          navigation={mockNavigation as never}
          route={mockRoute as never}
        />
      );

      await waitFor(() => {
        expect(Alert.alert).toHaveBeenCalledWith(
          'Error',
          'Failed to load coach details'
        );
      });
    });
  });
});
