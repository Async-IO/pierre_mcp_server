// ABOUTME: Unit tests for AuthContext
// ABOUTME: Tests authentication state management, login, logout, and registration

import React from 'react';
import { render, waitFor, act, fireEvent } from '@testing-library/react-native';
import { Text, TouchableOpacity } from 'react-native';
import { AuthProvider, useAuth } from '../src/contexts/AuthContext';
import { apiService, onAuthFailure } from '../src/services/api';

// Mock the api service
jest.mock('../src/services/api', () => ({
  apiService: {
    initializeAuth: jest.fn(),
    getStoredUser: jest.fn(),
    login: jest.fn(),
    logout: jest.fn(),
    register: jest.fn(),
    storeAuth: jest.fn(),
  },
  onAuthFailure: jest.fn(() => jest.fn()),
}));

// Test component that uses the auth context
function TestAuthConsumer() {
  const { user, isAuthenticated, isLoading, login, logout, register } = useAuth();

  return (
    <>
      <Text testID="loading">{isLoading ? 'loading' : 'loaded'}</Text>
      <Text testID="authenticated">{isAuthenticated ? 'authenticated' : 'not-authenticated'}</Text>
      <Text testID="user-email">{user?.email || 'no-user'}</Text>
      <TouchableOpacity
        testID="login-btn"
        onPress={() => login('test@example.com', 'password123')}
      >
        <Text>Login</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="logout-btn"
        onPress={logout}
      >
        <Text>Logout</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="register-btn"
        onPress={() => register('new@example.com', 'password123', 'New User')}
      >
        <Text>Register</Text>
      </TouchableOpacity>
    </>
  );
}

describe('AuthContext', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('useAuth hook', () => {
    it('should throw error when used outside AuthProvider', () => {
      const consoleError = jest.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        render(<TestAuthConsumer />);
      }).toThrow('useAuth must be used within an AuthProvider');

      consoleError.mockRestore();
    });
  });

  describe('AuthProvider initialization', () => {
    it('should start in loading state', async () => {
      (apiService.initializeAuth as jest.Mock).mockImplementation(
        () => new Promise(() => {}) // Never resolves - simulates loading
      );

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      expect(getByTestId('loading').children[0]).toBe('loading');
    });

    it('should initialize with stored user if token exists', async () => {
      const mockUser = {
        user_id: '123',
        email: 'stored@example.com',
        is_admin: false,
        role: 'user',
        user_status: 'active',
      };

      (apiService.initializeAuth as jest.Mock).mockResolvedValue(true);
      (apiService.getStoredUser as jest.Mock).mockResolvedValue(mockUser);

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('loading').children[0]).toBe('loaded');
      });

      expect(getByTestId('authenticated').children[0]).toBe('authenticated');
      expect(getByTestId('user-email').children[0]).toBe('stored@example.com');
    });

    it('should initialize as unauthenticated if no token', async () => {
      (apiService.initializeAuth as jest.Mock).mockResolvedValue(false);

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('loading').children[0]).toBe('loaded');
      });

      expect(getByTestId('authenticated').children[0]).toBe('not-authenticated');
      expect(getByTestId('user-email').children[0]).toBe('no-user');
    });

    it('should handle initialization error gracefully', async () => {
      (apiService.initializeAuth as jest.Mock).mockRejectedValue(new Error('Init failed'));
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('loading').children[0]).toBe('loaded');
      });

      expect(getByTestId('authenticated').children[0]).toBe('not-authenticated');
      consoleSpy.mockRestore();
    });
  });

  describe('login', () => {
    it('should update state after successful login', async () => {
      const mockUser = {
        user_id: '123',
        email: 'test@example.com',
        is_admin: false,
        role: 'user',
        user_status: 'active',
      };

      const mockLoginResponse = {
        access_token: 'jwt-token',
        csrf_token: 'csrf-token',
        user: mockUser,
      };

      (apiService.initializeAuth as jest.Mock).mockResolvedValue(false);
      (apiService.login as jest.Mock).mockResolvedValue(mockLoginResponse);
      (apiService.storeAuth as jest.Mock).mockResolvedValue(undefined);

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('loading').children[0]).toBe('loaded');
      });

      // Initially not authenticated
      expect(getByTestId('authenticated').children[0]).toBe('not-authenticated');

      // Trigger login
      await act(async () => {
        fireEvent.press(getByTestId('login-btn'));
      });

      await waitFor(() => {
        expect(getByTestId('authenticated').children[0]).toBe('authenticated');
      });

      expect(apiService.login).toHaveBeenCalledWith('test@example.com', 'password123');
      expect(apiService.storeAuth).toHaveBeenCalledWith('jwt-token', 'csrf-token', mockUser);
    });
  });

  describe('logout', () => {
    it('should clear user state after logout', async () => {
      const mockUser = {
        user_id: '123',
        email: 'stored@example.com',
        is_admin: false,
        role: 'user',
        user_status: 'active',
      };

      (apiService.initializeAuth as jest.Mock).mockResolvedValue(true);
      (apiService.getStoredUser as jest.Mock).mockResolvedValue(mockUser);
      (apiService.logout as jest.Mock).mockResolvedValue(undefined);

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('authenticated').children[0]).toBe('authenticated');
      });

      // Trigger logout
      await act(async () => {
        fireEvent.press(getByTestId('logout-btn'));
      });

      await waitFor(() => {
        expect(getByTestId('authenticated').children[0]).toBe('not-authenticated');
      });

      expect(apiService.logout).toHaveBeenCalled();
    });
  });

  describe('register', () => {
    it('should call register API', async () => {
      (apiService.initializeAuth as jest.Mock).mockResolvedValue(false);
      (apiService.register as jest.Mock).mockResolvedValue({ success: true });

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('loading').children[0]).toBe('loaded');
      });

      // Trigger register
      await act(async () => {
        fireEvent.press(getByTestId('register-btn'));
      });

      expect(apiService.register).toHaveBeenCalledWith(
        'new@example.com',
        'password123',
        'New User'
      );
    });
  });

  describe('isAuthenticated', () => {
    it('should be false for pending users', async () => {
      const mockPendingUser = {
        user_id: '123',
        email: 'pending@example.com',
        is_admin: false,
        role: 'user',
        user_status: 'pending', // Not active
      };

      (apiService.initializeAuth as jest.Mock).mockResolvedValue(true);
      (apiService.getStoredUser as jest.Mock).mockResolvedValue(mockPendingUser);

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('loading').children[0]).toBe('loaded');
      });

      // User exists but isAuthenticated should be false due to pending status
      expect(getByTestId('user-email').children[0]).toBe('pending@example.com');
      expect(getByTestId('authenticated').children[0]).toBe('not-authenticated');
    });

    it('should be true for active users', async () => {
      const mockActiveUser = {
        user_id: '123',
        email: 'active@example.com',
        is_admin: false,
        role: 'user',
        user_status: 'active',
      };

      (apiService.initializeAuth as jest.Mock).mockResolvedValue(true);
      (apiService.getStoredUser as jest.Mock).mockResolvedValue(mockActiveUser);

      const { getByTestId } = render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      await waitFor(() => {
        expect(getByTestId('loading').children[0]).toBe('loaded');
      });

      expect(getByTestId('authenticated').children[0]).toBe('authenticated');
    });
  });

  describe('onAuthFailure listener', () => {
    it('should register auth failure listener on mount', async () => {
      (apiService.initializeAuth as jest.Mock).mockResolvedValue(false);

      render(
        <AuthProvider>
          <TestAuthConsumer />
        </AuthProvider>
      );

      expect(onAuthFailure).toHaveBeenCalled();
    });
  });
});
