// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unit tests for SocialSettingsTab component
// ABOUTME: Tests settings display, toggle switches, and save functionality

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import SocialSettingsTab from '../SocialSettingsTab';
import { apiService } from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  apiService: {
    getSocialSettings: vi.fn(),
    updateSocialSettings: vi.fn(),
  },
}));

const mockSettings = {
  settings: {
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
  },
  metadata: { timestamp: '2024-01-01T00:00:00Z', api_version: 'v1' },
};

describe('SocialSettingsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiService.getSocialSettings).mockResolvedValue(mockSettings);
    vi.mocked(apiService.updateSocialSettings).mockResolvedValue(mockSettings);
  });

  it('should render the Social Settings tab with title', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Social Settings')).toBeInTheDocument();
    });

    expect(screen.getByText('Manage your privacy and notification preferences')).toBeInTheDocument();
  });

  it('should load and display settings on mount', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Discoverable')).toBeInTheDocument();
    });

    expect(apiService.getSocialSettings).toHaveBeenCalled();
  });

  it('should display privacy section with discoverable toggle', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Privacy')).toBeInTheDocument();
      expect(screen.getByText('Discoverable')).toBeInTheDocument();
      expect(screen.getByText('Allow others to find you when searching for friends')).toBeInTheDocument();
    });
  });

  it('should display default sharing section', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Default Sharing')).toBeInTheDocument();
      expect(screen.getByText('Friends Only')).toBeInTheDocument();
      expect(screen.getByText('Public')).toBeInTheDocument();
    });
  });

  it('should display notifications section', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Notifications')).toBeInTheDocument();
      expect(screen.getByText('Friend Requests')).toBeInTheDocument();
      expect(screen.getByText('Reactions')).toBeInTheDocument();
      expect(screen.getByText('Adapted Insights')).toBeInTheDocument();
    });
  });

  it('should display privacy info card', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Your Privacy is Protected')).toBeInTheDocument();
    });
  });

  it('should enable Save Changes button when settings are modified', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Discoverable')).toBeInTheDocument();
    });

    // Click the Public visibility option to make a change
    const publicButton = screen.getByRole('button', { name: /Public/i });
    fireEvent.click(publicButton);

    // Save Changes button should now be enabled
    const saveButton = screen.getByRole('button', { name: /Save Changes/i });
    expect(saveButton).not.toBeDisabled();
  });

  it('should save settings when clicking Save Changes', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Discoverable')).toBeInTheDocument();
    });

    // Make a change
    const publicButton = screen.getByRole('button', { name: /Public/i });
    fireEvent.click(publicButton);

    // Click Save
    const saveButton = screen.getByRole('button', { name: /Save Changes/i });
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(apiService.updateSocialSettings).toHaveBeenCalled();
    });
  });

  it('should show Saved indicator after successful save', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Discoverable')).toBeInTheDocument();
    });

    // Make a change and save
    const publicButton = screen.getByRole('button', { name: /Public/i });
    fireEvent.click(publicButton);

    const saveButton = screen.getByRole('button', { name: /Save Changes/i });
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(screen.getByText('Saved')).toBeInTheDocument();
    });
  });

  it('should toggle notification setting when clicking switch', async () => {
    render(<SocialSettingsTab />);

    await waitFor(() => {
      expect(screen.getByText('Adapted Insights')).toBeInTheDocument();
    });

    // Find the Adapted Insights setting row and its toggle button
    const adaptedInsightsText = screen.getByText('Adapted Insights');
    // Get the parent flex container which contains both the text and the toggle button
    const settingRow = adaptedInsightsText.closest('.flex.items-center.justify-between');
    const toggleButton = settingRow?.querySelector('button');

    expect(toggleButton).toBeTruthy();
    if (toggleButton) {
      fireEvent.click(toggleButton);
    }

    // Save should now be enabled
    const saveButton = screen.getByRole('button', { name: /Save Changes/i });
    expect(saveButton).not.toBeDisabled();
  });
});
