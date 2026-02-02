// ABOUTME: Unit tests for CoachVersionHistory component
// ABOUTME: Tests version list display, selection, and revert functionality

import React from 'react';
import { render, fireEvent, waitFor, act } from '@testing-library/react-native';
import { Alert } from 'react-native';

// Mock navigation
jest.mock('@react-navigation/native', () => {
  const actualReact = jest.requireActual('react');
  return {
    useFocusEffect: (callback: () => void) => {
      actualReact.useEffect(callback, []);
    },
  };
});

// Mock SafeAreaInsets
jest.mock('react-native-safe-area-context', () => ({
  useSafeAreaInsets: () => ({ top: 44, bottom: 34, left: 0, right: 0 }),
}));

// Mock Alert
jest.spyOn(Alert, 'alert');

// Mock API service
const mockGetVersions = jest.fn();
const mockRevertToVersion = jest.fn();

jest.mock('../src/services/api', () => ({
  coachesApi: {
    getVersions: (id: string, limit: number) => mockGetVersions(id, limit),
    revertToVersion: (id: string, version: number) => mockRevertToVersion(id, version),
  },
}));

import { CoachVersionHistory } from '../src/components/coaches/CoachVersionHistory';

const createMockVersion = (overrides: Partial<{
  version: number;
  content_snapshot: Record<string, unknown>;
  change_summary: string | null;
  created_at: string;
  created_by_name: string | null;
}> = {}) => ({
  version: 1,
  content_snapshot: {
    title: 'Test Coach',
    description: 'A test coach',
    system_prompt: 'You are helpful',
    category: 'training',
  },
  change_summary: 'Initial version',
  created_at: '2024-01-15T10:30:00Z',
  created_by_name: 'Test User',
  ...overrides,
});

describe('CoachVersionHistory', () => {
  const defaultProps = {
    coachId: 'coach-123',
    coachTitle: 'Test Coach',
    isOpen: true,
    onClose: jest.fn(),
    onReverted: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    mockGetVersions.mockResolvedValue({
      versions: [
        createMockVersion({ version: 3, change_summary: 'Latest update' }),
        createMockVersion({ version: 2, change_summary: 'Second update' }),
        createMockVersion({ version: 1, change_summary: 'Initial version' }),
      ],
      total: 3,
      current_version: 3,
    });
  });

  it('should render header with coach title', async () => {
    const { getByText } = render(<CoachVersionHistory {...defaultProps} />);

    await waitFor(() => {
      expect(getByText('Version History')).toBeTruthy();
      expect(getByText('Test Coach')).toBeTruthy();
    });
  });

  it('should show loading state initially', () => {
    const { getByText } = render(<CoachVersionHistory {...defaultProps} />);
    expect(getByText('Loading versions...')).toBeTruthy();
  });

  it('should load and display versions', async () => {
    const { getByText, getByTestId } = render(<CoachVersionHistory {...defaultProps} />);

    await waitFor(() => {
      expect(mockGetVersions).toHaveBeenCalledWith('coach-123', 50);
    });

    await waitFor(() => {
      // Should show all versions
      expect(getByTestId('version-item-3')).toBeTruthy();
      expect(getByTestId('version-item-2')).toBeTruthy();
      expect(getByTestId('version-item-1')).toBeTruthy();
      // Should show version numbers
      expect(getByText('v3')).toBeTruthy();
      expect(getByText('v2')).toBeTruthy();
      expect(getByText('v1')).toBeTruthy();
    });
  });

  it('should show stats bar with version count and current version', async () => {
    const { getByText } = render(<CoachVersionHistory {...defaultProps} />);

    await waitFor(() => {
      expect(getByText('3 versions saved')).toBeTruthy();
      expect(getByText('Current: v3')).toBeTruthy();
    });
  });

  it('should mark current version with badge', async () => {
    const { getByText } = render(<CoachVersionHistory {...defaultProps} />);

    await waitFor(() => {
      expect(getByText('Current')).toBeTruthy();
    });
  });

  it('should expand version details when pressed', async () => {
    const { getByTestId, getByText, queryByText } = render(
      <CoachVersionHistory {...defaultProps} />
    );

    await waitFor(() => {
      expect(getByTestId('version-item-2')).toBeTruthy();
    });

    // Initially snapshot content shouldn't be visible
    expect(queryByText('Snapshot Content')).toBeNull();

    // Tap to expand
    await act(async () => {
      fireEvent.press(getByTestId('version-item-2'));
    });

    // Should show snapshot content section
    expect(getByText('Snapshot Content')).toBeTruthy();
    // Should show revert button (since it's not the current version)
    expect(getByTestId('revert-button-2')).toBeTruthy();
  });

  it('should not show revert button for current version', async () => {
    const { getByTestId, queryByTestId, getByText } = render(
      <CoachVersionHistory {...defaultProps} />
    );

    await waitFor(() => {
      expect(getByTestId('version-item-3')).toBeTruthy();
    });

    // Expand the current version
    await act(async () => {
      fireEvent.press(getByTestId('version-item-3'));
    });

    expect(getByText('Snapshot Content')).toBeTruthy();

    // Should NOT show revert button for current version
    expect(queryByTestId('revert-button-3')).toBeNull();
  });

  it('should show confirmation alert when revert is pressed', async () => {
    const { getByTestId } = render(<CoachVersionHistory {...defaultProps} />);

    await waitFor(() => {
      expect(getByTestId('version-item-2')).toBeTruthy();
    });

    // Expand version 2
    await act(async () => {
      fireEvent.press(getByTestId('version-item-2'));
    });

    // Press revert
    fireEvent.press(getByTestId('revert-button-2'));

    expect(Alert.alert).toHaveBeenCalledWith(
      'Confirm Revert',
      expect.stringContaining('Are you sure you want to revert to version 2'),
      expect.arrayContaining([
        expect.objectContaining({ text: 'Cancel' }),
        expect.objectContaining({ text: 'Revert' }),
      ])
    );
  });

  it('should call onClose when close button is pressed', async () => {
    const onClose = jest.fn();
    const { getByTestId } = render(
      <CoachVersionHistory {...defaultProps} onClose={onClose} />
    );

    await waitFor(() => {
      expect(getByTestId('close-version-history')).toBeTruthy();
    });

    fireEvent.press(getByTestId('close-version-history'));

    expect(onClose).toHaveBeenCalled();
  });

  it('should show empty state when no versions', async () => {
    mockGetVersions.mockResolvedValue({
      versions: [],
      total: 0,
      current_version: 0,
    });

    const { getByText } = render(<CoachVersionHistory {...defaultProps} />);

    await waitFor(() => {
      expect(getByText('No version history yet')).toBeTruthy();
      expect(getByText('Versions are created automatically when you update the coach.')).toBeTruthy();
    });
  });

  it('should call revertToVersion API on confirm', async () => {
    mockRevertToVersion.mockResolvedValue({ success: true });

    const onReverted = jest.fn();
    const onClose = jest.fn();
    const { getByTestId } = render(
      <CoachVersionHistory {...defaultProps} onReverted={onReverted} onClose={onClose} />
    );

    await waitFor(() => {
      expect(getByTestId('version-item-2')).toBeTruthy();
    });

    // Expand and press revert
    await act(async () => {
      fireEvent.press(getByTestId('version-item-2'));
    });

    fireEvent.press(getByTestId('revert-button-2'));

    // Get the onPress handler from the Alert mock and call it
    const alertCalls = (Alert.alert as jest.Mock).mock.calls;
    const lastCall = alertCalls[alertCalls.length - 1];
    const buttons = lastCall[2];
    const revertButton = buttons.find((b: { text: string }) => b.text === 'Revert');

    // Trigger the revert - wrap in act() to handle async state updates
    await act(async () => {
      await revertButton.onPress();
    });

    await waitFor(() => {
      expect(mockRevertToVersion).toHaveBeenCalledWith('coach-123', 2);
    });
  });
});
