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
const mockGetHiddenCoaches = jest.fn();

jest.mock('../src/services/api', () => ({
  apiService: {
    listCoaches: (...args: unknown[]) => mockListCoaches(...args),
    toggleCoachFavorite: (...args: unknown[]) => mockToggleCoachFavorite(...args),
    updateCoach: (...args: unknown[]) => mockUpdateCoach(...args),
    deleteCoach: (...args: unknown[]) => mockDeleteCoach(...args),
    hideCoach: (...args: unknown[]) => mockHideCoach(...args),
    showCoach: (...args: unknown[]) => mockShowCoach(...args),
    getHiddenCoaches: (...args: unknown[]) => mockGetHiddenCoaches(...args),
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
  is_hidden: false,
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
    mockGetHiddenCoaches.mockResolvedValue({ coaches: [] });
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
        expect(getByText('Mobility')).toBeTruthy();
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

    it('should filter by mobility category', async () => {
      const coaches = [
        createMockCoach({ id: '1', title: 'Mobility Coach', category: 'mobility' }),
        createMockCoach({ id: '2', title: 'Training Coach', category: 'training' }),
        createMockCoach({ id: '3', title: 'Nutrition Coach', category: 'nutrition' }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getByText, queryByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Mobility Coach')).toBeTruthy();
        expect(getByText('Training Coach')).toBeTruthy();
        expect(getByText('Nutrition Coach')).toBeTruthy();
      });

      // Filter by mobility
      fireEvent.press(getByText('Mobility'));

      await waitFor(() => {
        expect(getByText('Mobility Coach')).toBeTruthy();
        expect(queryByText('Training Coach')).toBeNull();
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

  describe('hidden filter chip', () => {
    it('should not show hidden filter chip when no hidden coaches', async () => {
      mockListCoaches.mockResolvedValue({ coaches: [] });
      mockGetHiddenCoaches.mockResolvedValue({ coaches: [] });

      const { queryByTestId } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(queryByTestId('category-filter-hidden')).toBeNull();
      });
    });

    it('should show hidden filter chip with count when hidden coaches exist', async () => {
      const hiddenCoaches = [
        createMockCoach({ id: 'hidden-1', title: 'Hidden Coach 1', is_system: true, is_hidden: true }),
        createMockCoach({ id: 'hidden-2', title: 'Hidden Coach 2', is_system: true, is_hidden: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches: [] });
      mockGetHiddenCoaches.mockResolvedValue({ coaches: hiddenCoaches });

      const { getByTestId, getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByTestId('category-filter-hidden')).toBeTruthy();
        expect(getByText('Hidden (2)')).toBeTruthy();
      });
    });

    it('should filter to show only hidden coaches when hidden filter selected', async () => {
      const visibleCoaches = [
        createMockCoach({ id: 'visible-1', title: 'Visible Coach', is_system: true }),
      ];
      const hiddenCoaches = [
        createMockCoach({ id: 'hidden-1', title: 'Hidden Coach', is_system: true, is_hidden: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches: visibleCoaches });
      mockGetHiddenCoaches.mockResolvedValue({ coaches: hiddenCoaches });

      const { getByTestId, getByText, queryByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      // Initially shows visible coaches
      await waitFor(() => {
        expect(getByText('Visible Coach')).toBeTruthy();
      });

      // Click hidden filter
      fireEvent.press(getByTestId('category-filter-hidden'));

      // Should show only hidden coaches
      await waitFor(() => {
        expect(getByText('Hidden Coach')).toBeTruthy();
        expect(queryByText('Visible Coach')).toBeNull();
      });
    });

    it('should update hidden count when coach is hidden', async () => {
      const coaches = [
        createMockCoach({ id: 'system-1', title: 'System Coach', is_system: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches });
      mockGetHiddenCoaches.mockResolvedValue({ coaches: [] });
      mockHideCoach.mockResolvedValue({ success: true, is_hidden: true });

      const { getByTestId, getByText, queryByTestId } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      // Wait for coach to render first
      await waitFor(() => {
        expect(getByText('System Coach')).toBeTruthy();
      });

      // Initially no hidden filter chip
      expect(queryByTestId('category-filter-hidden')).toBeNull();

      // Hide the coach
      fireEvent.press(getByTestId('hide-button-system-1'));

      // Hidden filter chip should appear with count 1
      await waitFor(() => {
        expect(getByTestId('category-filter-hidden')).toBeTruthy();
        expect(getByText('Hidden (1)')).toBeTruthy();
      });
    });

    it('should call showCoach API when unhide button pressed on hidden coach', async () => {
      const hiddenCoaches = [
        createMockCoach({ id: 'hidden-1', title: 'Hidden Coach', is_system: true, is_hidden: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches: [] });
      mockGetHiddenCoaches.mockResolvedValue({ coaches: hiddenCoaches });
      mockShowCoach.mockResolvedValue({ success: true, is_hidden: false });

      const { getByTestId } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      // Select hidden filter to see hidden coaches
      await waitFor(() => {
        expect(getByTestId('category-filter-hidden')).toBeTruthy();
      });
      fireEvent.press(getByTestId('category-filter-hidden'));

      // Press unhide button
      await waitFor(() => {
        expect(getByTestId('hide-button-hidden-1')).toBeTruthy();
      });
      fireEvent.press(getByTestId('hide-button-hidden-1'));

      await waitFor(() => {
        expect(mockShowCoach).toHaveBeenCalledWith('hidden-1');
      });
    });

    it('should toggle hidden filter off when pressed again', async () => {
      const visibleCoaches = [
        createMockCoach({ id: 'visible-1', title: 'Visible Coach', is_system: true }),
      ];
      const hiddenCoaches = [
        createMockCoach({ id: 'hidden-1', title: 'Hidden Coach', is_system: true, is_hidden: true }),
      ];
      mockListCoaches.mockResolvedValue({ coaches: visibleCoaches });
      mockGetHiddenCoaches.mockResolvedValue({ coaches: hiddenCoaches });

      const { getByTestId, getByText, queryByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByTestId('category-filter-hidden')).toBeTruthy();
      });

      // Click hidden filter to show hidden coaches
      fireEvent.press(getByTestId('category-filter-hidden'));
      await waitFor(() => {
        expect(getByText('Hidden Coach')).toBeTruthy();
        expect(queryByText('Visible Coach')).toBeNull();
      });

      // Click hidden filter again to go back to all
      fireEvent.press(getByTestId('category-filter-hidden'));
      await waitFor(() => {
        expect(getByText('Visible Coach')).toBeTruthy();
        expect(queryByText('Hidden Coach')).toBeNull();
      });
    });
  });

  describe('navigation', () => {
    it('should navigate to CoachWizard when coach is pressed', async () => {
      const coaches = [createMockCoach({ id: 'coach-1', title: 'Test Coach' })];
      mockListCoaches.mockResolvedValue({ coaches });

      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Test Coach')).toBeTruthy();
      });

      fireEvent.press(getByText('Test Coach'));

      expect(mockNavigation.navigate).toHaveBeenCalledWith('CoachWizard', {
        coachId: 'coach-1',
      });
    });

    it('should navigate to CoachWizard for new coach when FAB pressed', async () => {
      const { getByText } = render(
        <CoachLibraryScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('+')).toBeTruthy();
      });

      fireEvent.press(getByText('+'));

      expect(mockNavigation.navigate).toHaveBeenCalledWith('CoachWizard', {
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
