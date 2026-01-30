// ABOUTME: Unit tests for StoreScreen component
// ABOUTME: Tests coach store browsing, filtering, search, and navigation

import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

// Mock navigation
const mockNavigation = {
  navigate: jest.fn(),
  goBack: jest.fn(),
  canGoBack: jest.fn().mockReturnValue(true),
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
    useNavigation: () => ({
      goBack: jest.fn(),
      canGoBack: jest.fn().mockReturnValue(true),
    }),
  };
});

// Mock AuthContext
jest.mock('../src/contexts/AuthContext', () => ({
  useAuth: () => ({
    isAuthenticated: true,
  }),
}));

// Mock API service
const mockBrowseStoreCoaches = jest.fn();
const mockSearchStoreCoaches = jest.fn();
const mockGetStoreCategories = jest.fn();

jest.mock('../src/services/api', () => ({
  storeApi: {
    browse: (...args: unknown[]) => mockBrowseStoreCoaches(...args),
    search: (...args: unknown[]) => mockSearchStoreCoaches(...args),
    getCategories: (...args: unknown[]) => mockGetStoreCategories(...args),
  },
}));

import { StoreScreen } from '../src/screens/store/StoreScreen';
import type { StoreCoach, CoachCategory } from '../src/types';

const createMockStoreCoach = (overrides: Partial<StoreCoach> = {}): StoreCoach => ({
  id: 'store-coach-1',
  title: 'Test Store Coach',
  description: 'A published coach for the store',
  category: 'training' as CoachCategory,
  tags: ['running', 'marathon'],
  sample_prompts: ['How do I improve my pace?'],
  token_count: 800,
  install_count: 25,
  icon_url: null,
  published_at: '2024-01-15T00:00:00Z',
  author_id: null,
  ...overrides,
});

describe('StoreScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockBrowseStoreCoaches.mockResolvedValue({ coaches: [], total: 0 });
    mockGetStoreCategories.mockResolvedValue({
      categories: [
        { category: 'training', count: 5 },
        { category: 'nutrition', count: 3 },
        { category: 'recovery', count: 2 },
      ],
    });
  });

  describe('rendering', () => {
    it('should render header with Discover title', async () => {
      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        expect(getByText('Discover')).toBeTruthy();
      });
    });

    it('should render category filters', async () => {
      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        expect(getByText('All')).toBeTruthy();
        expect(getByText('Training')).toBeTruthy();
        expect(getByText('Nutrition')).toBeTruthy();
        expect(getByText('Recovery')).toBeTruthy();
      });
    });

    it('should render sort options', async () => {
      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        expect(getByText('Popular')).toBeTruthy();
        expect(getByText('Newest')).toBeTruthy();
        expect(getByText('A-Z')).toBeTruthy();
      });
    });

    it('should render search input', async () => {
      const { getByPlaceholderText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        expect(getByPlaceholderText('Search coaches...')).toBeTruthy();
      });
    });

    it('should render empty state when no coaches', async () => {
      mockBrowseStoreCoaches.mockResolvedValue({ coaches: [], total: 0 });
      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );
      await waitFor(() => {
        expect(getByText('No coaches available')).toBeTruthy();
      });
    });
  });

  describe('coach list', () => {
    it('should render coach cards', async () => {
      const coaches = [
        createMockStoreCoach({ id: '1', title: 'Marathon Training Coach' }),
        createMockStoreCoach({ id: '2', title: 'Nutrition Guide', category: 'nutrition' as CoachCategory }),
      ];
      mockBrowseStoreCoaches.mockResolvedValue({ coaches, total: 2 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Marathon Training Coach')).toBeTruthy();
        expect(getByText('Nutrition Guide')).toBeTruthy();
      });
    });

    it('should show install count on coach cards', async () => {
      const coaches = [
        createMockStoreCoach({ id: '1', title: 'Popular Coach', install_count: 150 }),
      ];
      mockBrowseStoreCoaches.mockResolvedValue({ coaches, total: 1 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('150 installs')).toBeTruthy();
      });
    });

    it('should show category badge on coach cards', async () => {
      const coaches = [
        createMockStoreCoach({ id: '1', title: 'Training Coach', category: 'training' as CoachCategory }),
      ];
      mockBrowseStoreCoaches.mockResolvedValue({ coaches, total: 1 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('training')).toBeTruthy();
      });
    });

    it('should show tags on coach cards', async () => {
      const coaches = [
        createMockStoreCoach({ id: '1', title: 'Tagged Coach', tags: ['beginner', 'cardio'] }),
      ];
      mockBrowseStoreCoaches.mockResolvedValue({ coaches, total: 1 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('beginner')).toBeTruthy();
        expect(getByText('cardio')).toBeTruthy();
      });
    });
  });

  describe('filtering', () => {
    it('should filter by category when chip is pressed', async () => {
      const coaches = [
        createMockStoreCoach({ id: '1', title: 'Training Coach', category: 'training' as CoachCategory }),
      ];
      mockBrowseStoreCoaches.mockResolvedValue({ coaches, total: 1 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Training')).toBeTruthy();
      });

      // Clear previous calls and press Training filter
      mockBrowseStoreCoaches.mockClear();
      fireEvent.press(getByText('Training'));

      await waitFor(() => {
        expect(mockBrowseStoreCoaches).toHaveBeenCalledWith(
          expect.objectContaining({ category: 'training' })
        );
      });
    });

    it('should clear category filter when All is pressed', async () => {
      mockBrowseStoreCoaches.mockResolvedValue({ coaches: [], total: 0 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      // Wait for initial load
      await waitFor(() => {
        expect(getByText('All')).toBeTruthy();
      });

      // First select a category and wait for the load triggered by that
      fireEvent.press(getByText('Training'));
      await waitFor(() => {
        expect(mockBrowseStoreCoaches).toHaveBeenCalledWith(
          expect.objectContaining({ category: 'training' })
        );
      });

      // Then clear with All
      mockBrowseStoreCoaches.mockClear();
      fireEvent.press(getByText('All'));

      await waitFor(() => {
        expect(mockBrowseStoreCoaches).toHaveBeenCalledWith(
          expect.objectContaining({ category: undefined })
        );
      });
    });
  });

  describe('sorting', () => {
    it('should sort by popular by default', async () => {
      mockBrowseStoreCoaches.mockResolvedValue({ coaches: [], total: 0 });

      render(<StoreScreen navigation={mockNavigation as never} />);

      await waitFor(() => {
        expect(mockBrowseStoreCoaches).toHaveBeenCalledWith(
          expect.objectContaining({ sort_by: 'popular' })
        );
      });
    });

    it('should change sort when option is pressed', async () => {
      mockBrowseStoreCoaches.mockResolvedValue({ coaches: [], total: 0 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Newest')).toBeTruthy();
      });

      mockBrowseStoreCoaches.mockClear();
      fireEvent.press(getByText('Newest'));

      await waitFor(() => {
        expect(mockBrowseStoreCoaches).toHaveBeenCalledWith(
          expect.objectContaining({ sort_by: 'newest' })
        );
      });
    });
  });

  describe('search', () => {
    it('should search coaches when text is entered', async () => {
      mockBrowseStoreCoaches.mockResolvedValue({ coaches: [], total: 0 });
      mockSearchStoreCoaches.mockResolvedValue({ coaches: [], total: 0 });

      const { getByPlaceholderText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByPlaceholderText('Search coaches...')).toBeTruthy();
      });

      const searchInput = getByPlaceholderText('Search coaches...');
      fireEvent.changeText(searchInput, 'marathon');

      // Search is debounced, so wait for it
      await waitFor(
        () => {
          expect(mockSearchStoreCoaches).toHaveBeenCalledWith('marathon', expect.any(Number));
        },
        { timeout: 1000 }
      );
    });
  });

  describe('navigation', () => {
    it('should navigate to StoreCoachDetail when coach is pressed', async () => {
      const coaches = [
        createMockStoreCoach({ id: 'coach-123', title: 'Clickable Coach' }),
      ];
      mockBrowseStoreCoaches.mockResolvedValue({ coaches, total: 1 });

      const { getByText } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByText('Clickable Coach')).toBeTruthy();
      });

      fireEvent.press(getByText('Clickable Coach'));

      expect(mockNavigation.navigate).toHaveBeenCalledWith('StoreCoachDetail', {
        coachId: 'coach-123',
      });
    });

  });

  describe('loading states', () => {
    it('should show loading indicator while fetching coaches', async () => {
      // Create a promise that doesn't resolve immediately
      let resolvePromise: (value: unknown) => void;
      const pendingPromise = new Promise((resolve) => {
        resolvePromise = resolve;
      });
      mockBrowseStoreCoaches.mockReturnValue(pendingPromise);

      const { getByTestId } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      // Should show loading state
      expect(getByTestId('loading-indicator')).toBeTruthy();

      // Resolve the promise
      resolvePromise!({ coaches: [], total: 0 });

      await waitFor(() => {
        // Loading should be done
      });
    });
  });

  describe('pull to refresh', () => {
    it('should refresh coaches on pull down', async () => {
      const coaches = [
        createMockStoreCoach({ id: '1', title: 'Initial Coach' }),
      ];
      mockBrowseStoreCoaches.mockResolvedValue({ coaches, total: 1 });

      const { getByTestId } = render(
        <StoreScreen navigation={mockNavigation as never} />
      );

      await waitFor(() => {
        expect(getByTestId('coach-list')).toBeTruthy();
      });

      // Clear mock to track refresh call
      mockBrowseStoreCoaches.mockClear();

      // Trigger refresh
      const flatList = getByTestId('coach-list');
      const { refreshControl } = flatList.props;
      if (refreshControl?.props?.onRefresh) {
        refreshControl.props.onRefresh();
      }

      await waitFor(() => {
        expect(mockBrowseStoreCoaches).toHaveBeenCalled();
      });
    });
  });
});
