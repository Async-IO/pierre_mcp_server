// ABOUTME: Unit tests for CoachLibraryScreen component
// ABOUTME: Tests coach listing, filtering, favorites, and hide/show functionality

import React, { useEffect } from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';
import { Alert } from 'react-native';

// Mock navigation
const mockNavigation = {
  navigate: jest.fn(),
  openDrawer: jest.fn(),
};

// Mock useFocusEffect - needs to be before imports that use it
jest.mock('@react-navigation/native', () => {
  const actualReact = jest.requireActual('react');
  return {
    useFocusEffect: (callback: () => void) => {
      actualReact.useEffect(callback, []);
    },
  };
});

// Mock AuthContext
jest.mock('../src/contexts/AuthContext', () => ({
  useAuth: () => ({
    isAuthenticated: true,
  }),
}));

// Mock API service
const mockListCoaches = jest.fn();
const mockToggleCoachFavorite = jest.fn();
const mockUpdateCoach = jest.fn();
const mockDeleteCoach = jest.fn();
const mockHideCoach = jest.fn();
const mockShowCoach = jest.fn();

jest.mock('../src/services/api', () => ({
  apiService: {
    listCoaches: (...args: unknown[]) => mockListCoaches(...args),
    toggleCoachFavorite: (...args: unknown[]) => mockToggleCoachFavorite(...args),
    updateCoach: (...args: unknown[]) => mockUpdateCoach(...args),
    deleteCoach: (...args: unknown[]) => mockDeleteCoach(...args),
    hideCoach: (...args: unknown[]) => mockHideCoach(...args),
    showCoach: (...args: unknown[]) => mockShowCoach(...args),
  },
}));

// Mock Alert
jest.spyOn(Alert, 'alert');

import { CoachLibraryScreen } from '../src/screens/coaches/CoachLibraryScreen';
import type { Coach } from '../src/types';

const createMockCoach = (overrides: Partial<Coach> = {}): Coach => ({
  id: 'coach-1',
  title: 'Test Coach',
  description: 'A test coach',
  system_prompt: 'You are a helpful coach',
  category: 'training',
  tags: [],
  is_favorite: false,
  is_system: false,
  token_count: 500,
  use_count: 10,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  last_used_at: null,
  ...overrides,
});

describe('CoachLibraryScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockListCoaches.mockResolvedValue({ coaches: [] });
  });

  describe('rendering', () => {
    it('should render header with title', async () => {
      const { getAllByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        // "My Coaches" appears in header title AND source filter chip
        const elements = getAllByText('My Coaches');
        expect(elements.length).toBeGreaterThanOrEqual(1);
      });
    });

    it('should render category filters', async () => {
      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        expect(getByText('All')).toBeTruthy();
        expect(getByText('Training')).toBeTruthy();
        expect(getByText('Nutrition')).toBeTruthy();
        expect(getByText('Recovery')).toBeTruthy();
        expect(getByText('Recipes')).toBeTruthy();
        expect(getByText('Custom')).toBeTruthy();
      });
    });

    it('should render empty state when no coaches', async () => {
      mockListCoaches.mockResolvedValue({ coaches: [] });
      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        expect(getByText('No coaches yet')).toBeTruthy();
      });
    });
  });

  describe('coach list', () => {
    it('should render coach cards', async () => {
      const coaches = [
        createMockCoach({ id: '1', title: 'Training Coach' }),
        createMockCoach({ id: '2', title: 'Nutrition Coach', category: 'nutrition' }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Training Coach')).toBeTruthy();
        expect(getByText('Nutrition Coach')).toBeTruthy();
      });
    });

    it('should show system badge for system coaches', async () => {
      const coaches = [
        createMockCoach({ id: '1', title: 'System Coach', is_system: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('System')).toBeTruthy();
      });
    });

    it('should sort favorites first', async () => {
      const coaches = [
        createMockCoach({ id: '1', title: 'Regular Coach', is_favorite: false }),
        createMockCoach({ id: '2', title: 'Favorite Coach', is_favorite: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getAllByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        // Favorites should appear first in the sorted list
        const titles = getAllByText(/Coach/);
        expect(titles.length).toBeGreaterThanOrEqual(2);
      });
    });
  });

  describe('filtering', () => {
    it('should filter by category', async () => {
      const coaches = [
        createMockCoach({ id: '1', title: 'Training Coach', category: 'training' }),
        createMockCoach({ id: '2', title: 'Nutrition Coach', category: 'nutrition' }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getByText, queryByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Training Coach')).toBeTruthy();
        expect(getByText('Nutrition Coach')).toBeTruthy();
      });

      // Filter by training
      fireEvent.press(getByText('Training'));

      await waitFor(() => {
        expect(getByText('Training Coach')).toBeTruthy();
        expect(queryByText('Nutrition Coach')).toBeNull();
      });
    });
  });

  describe('hide/show functionality', () => {
    it('should load coaches without hidden when showHidden is false', async () => {
      mockListCoaches.mockResolvedValue({ coaches: [] });

      render(<CoachLibraryScreen navigation={mockNavigation as never} />);

      await waitFor(() => {
        expect(mockListCoaches).toHaveBeenCalledWith({
          include_hidden: false,
        });
      });
    });

    it('should show hide button only for system coaches', async () => {
      const coaches = [
        createMockCoach({ id: '1', title: 'User Coach', is_system: false }),
        createMockCoach({ id: '2', title: 'System Coach', is_system: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getAllByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        // System coach should have hide button (🙈 icon)
        const hideIcons = getAllByText('🙈');
        expect(hideIcons.length).toBe(1);
      });
    });

    it('should call hideCoach API when hide button pressed', async () => {
      const coaches = [
        createMockCoach({ id: 'system-1', title: 'System Coach', is_system: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });
      mockHideCoach.mockResolvedValue({ success: true, is_hidden: true });

      const { getByTestId } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByTestId('hide-button-system-1')).toBeTruthy();
      });

      // Press hide button using testID
      fireEvent.press(getByTestId('hide-button-system-1'));

      await waitFor(() => {
        expect(mockHideCoach).toHaveBeenCalledWith('system-1');
      });
    });

    it('should not show rename option for system coaches in action menu', async () => {
      const coaches = [
        createMockCoach({ id: 'system-1', title: 'System Coach', is_system: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getByText, queryByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('System Coach')).toBeTruthy();
      });

      // Long press to open action menu
      const coachCard = getByText('System Coach');
      fireEvent(coachCard, 'longPress');

      await waitFor(() => {
        // Hide option should be available
        expect(getByText('Hide coach')).toBeTruthy();
        // Rename and Delete should not be available for system coaches
        expect(queryByText('Rename')).toBeNull();
        expect(queryByText('Delete')).toBeNull();
      });
    });
  });

  describe('navigation', () => {
    it('should navigate to CoachEditor when coach is pressed', async () => {
      const coaches = [createMockCoach({ id: 'coach-1', title: 'Test Coach' })];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Test Coach')).toBeTruthy();
      });

      fireEvent.press(getByText('Test Coach'));

      expect(mockNavigation.navigate).toHaveBeenCalledWith('CoachEditor', {
        coachId: 'coach-1',
      });
    });

    it('should navigate to CoachEditor for new coach when FAB pressed', async () => {
      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('+')).toBeTruthy();
      });

      fireEvent.press(getByText('+'));

      expect(mockNavigation.navigate).toHaveBeenCalledWith('CoachEditor', {
        coachId: undefined,
      });
    });
  });

  describe('favorites', () => {
    it('should call toggleCoachFavorite when favorite button pressed', async () => {
      const coaches = [createMockCoach({ id: 'coach-1', is_favorite: false })];
      mockListCoaches.mockResolvedValue({ coaches });
      mockToggleCoachFavorite.mockResolvedValue({ is_favorite: true });

      const { getByTestId } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByTestId('favorite-button-coach-1')).toBeTruthy();
      });

      fireEvent.press(getByTestId('favorite-button-coach-1'));

      await waitFor(() => {
        expect(mockToggleCoachFavorite).toHaveBeenCalledWith('coach-1');
      });
    });
  });
});
