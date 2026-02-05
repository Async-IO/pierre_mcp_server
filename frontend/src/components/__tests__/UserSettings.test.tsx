// ABOUTME: Unit tests for the UserSettings component
// ABOUTME: Tests tab navigation, profile display, change password modal, and about tab
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import UserSettings from '../UserSettings';

// Mock lazy-loaded components
vi.mock('../A2AClientList', () => ({
  default: () => <div data-testid="a2a-client-list">A2A Clients</div>,
}));

vi.mock('../CreateA2AClient', () => ({
  default: () => (
    <div data-testid="create-a2a-client">Create A2A Client</div>
  ),
}));

vi.mock('../LlmSettingsTab', () => ({
  default: () => <div data-testid="llm-settings">LLM Settings</div>,
}));

// Mock auth context
vi.mock('../../hooks/useAuth', () => ({
  useAuth: () => ({
    user: {
      id: 'user-1',
      email: 'test@pierre.dev',
      display_name: 'Test User',
      is_admin: false,
      role: 'user',
      tier: 'free',
      created_at: '2024-06-15T10:00:00Z',
    },
    logout: vi.fn(),
    isAuthenticated: true,
    isLoading: false,
  }),
}));

// Mock API service - factory must be self-contained (vi.mock is hoisted)
vi.mock('../../services/api', () => ({
  userApi: {
    getUserStats: vi.fn().mockResolvedValue({ connected_providers: 2, days_active: 45 }),
    getUserOAuthApps: vi.fn().mockResolvedValue({ apps: [] }),
    getMcpTokens: vi.fn().mockResolvedValue({ tokens: [] }),
    changePassword: vi.fn().mockResolvedValue({ message: 'Password changed successfully' }),
    updateProfile: vi.fn().mockResolvedValue({
      message: 'Profile updated',
      user: { id: 'user-1', email: 'test@pierre.dev', display_name: 'Test User' },
    }),
  },
  apiClient: {
    user: {
      getOAuthApps: vi.fn().mockResolvedValue({ apps: [] }),
      registerOAuthApp: vi.fn(),
      deleteOAuthApp: vi.fn(),
      createMcpToken: vi.fn(),
      revokeMcpToken: vi.fn(),
    },
  },
}));

function renderUserSettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <UserSettings />
    </QueryClientProvider>
  );
}

describe('UserSettings Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Tab Navigation', () => {
    it('should render settings tab bar with all tabs', async () => {
      await act(async () => {
        renderUserSettings();
      });

      expect(screen.getByText('Profile')).toBeInTheDocument();
      expect(screen.getByText('Connections')).toBeInTheDocument();
      expect(screen.getByText('API Tokens')).toBeInTheDocument();
      expect(screen.getByText('AI Settings')).toBeInTheDocument();
      expect(screen.getByText('About')).toBeInTheDocument();
      expect(screen.getByText('Account')).toBeInTheDocument();
    });

    it('should start on Profile tab by default', async () => {
      await act(async () => {
        renderUserSettings();
      });

      // Profile tab should show user display name and email
      expect(screen.getByText('Test User')).toBeInTheDocument();
      expect(screen.getAllByText('test@pierre.dev').length).toBeGreaterThan(0);
    });

    it('should switch to About tab when clicked', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('About'));

      await waitFor(() => {
        expect(screen.getByText('Version')).toBeInTheDocument();
        expect(screen.getByText('1.0.0')).toBeInTheDocument();
      });
    });

    it('should switch to Account tab when clicked', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('Account'));

      await waitFor(() => {
        expect(screen.getByText('Danger Zone')).toBeInTheDocument();
      });
    });
  });

  describe('Profile Tab', () => {
    it('should display user avatar with gradient ring', async () => {
      await act(async () => {
        renderUserSettings();
      });

      // The avatar should be present with gradient border
      const avatarContainer = document.querySelector('.bg-gradient-to-br');
      expect(avatarContainer).toBeInTheDocument();
    });

    it('should display user stats', async () => {
      await act(async () => {
        renderUserSettings();
      });

      await waitFor(() => {
        expect(screen.getByText('Connected Providers')).toBeInTheDocument();
        expect(screen.getByText('Days Active')).toBeInTheDocument();
      });
    });

    it('should show display name input', async () => {
      await act(async () => {
        renderUserSettings();
      });

      const displayNameInput = screen.getByLabelText('Display Name');
      expect(displayNameInput).toBeInTheDocument();
      expect(displayNameInput).toHaveValue('Test User');
    });
  });

  describe('About Tab', () => {
    it('should show version, help, and terms links', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('About'));

      await waitFor(() => {
        expect(screen.getByText('Version')).toBeInTheDocument();
        expect(screen.getByText('1.0.0')).toBeInTheDocument();
        expect(screen.getByText('Help Center')).toBeInTheDocument();
        expect(screen.getByText('Terms & Privacy')).toBeInTheDocument();
      });
    });
  });

  describe('Account Tab', () => {
    it('should display member since date', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('Account'));

      await waitFor(() => {
        // The created_at is 2024-06-15 which formats to "Jun 15, 2024"
        expect(screen.getByText('Jun 15, 2024')).toBeInTheDocument();
      });
    });

    it('should show change password button', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('Account'));

      await waitFor(() => {
        expect(screen.getByText('Change Password')).toBeInTheDocument();
      });
    });

    it('should open change password modal', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('Account'));

      await waitFor(() => {
        expect(screen.getByText('Change Password')).toBeInTheDocument();
      });

      // Click the Change Password button
      const changePasswordButtons = screen.getAllByText('Change Password');
      // The button in Account tab (not the modal title)
      await user.click(changePasswordButtons[0]);

      await waitFor(() => {
        // Modal should appear with password fields
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
        expect(screen.getByLabelText('New Password')).toBeInTheDocument();
        expect(screen.getByLabelText('Confirm New Password')).toBeInTheDocument();
      });
    });

    it('should validate passwords match in change password modal', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('Account'));

      await waitFor(() => {
        expect(screen.getByText('Change Password')).toBeInTheDocument();
      });

      // Open modal
      await user.click(screen.getByRole('button', { name: 'Change Password' }));

      await waitFor(() => {
        expect(screen.getByLabelText('Current Password')).toBeInTheDocument();
      });

      // Fill in mismatched passwords
      await user.type(screen.getByLabelText('Current Password'), 'password123');
      await user.type(screen.getByLabelText('New Password'), 'NewPass456');
      await user.type(screen.getByLabelText('Confirm New Password'), 'DifferentPass789');

      // The inline error should appear on the confirm field (no submit needed)
      await waitFor(() => {
        expect(screen.getByText(/passwords do not match/i)).toBeInTheDocument();
      });
    });

    it('should show sign out button in danger zone', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('Account'));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Sign Out' })).toBeInTheDocument();
      });
    });
  });

  describe('Connections Tab', () => {
    it('should switch to connections tab', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('Connections'));

      await waitFor(() => {
        // Should show the connections content with provider credentials section
        expect(screen.getByText('Provider Credentials')).toBeInTheDocument();
      });
    });
  });

  describe('API Tokens Tab', () => {
    it('should switch to tokens tab', async () => {
      const user = userEvent.setup();

      await act(async () => {
        renderUserSettings();
      });

      await user.click(screen.getByText('API Tokens'));

      await waitFor(() => {
        // Should show the tokens content with create button
        expect(screen.getByText('Create New Token')).toBeInTheDocument();
      });
    });
  });
});
