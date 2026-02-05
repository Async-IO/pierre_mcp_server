// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect, useCallback } from 'react';
import { authApi, adminApi, apiClient } from '../services/api';
import { AuthContext } from './auth';
import type { User, ImpersonationState } from './auth';

const STORAGE_KEYS = {
  TOKEN: 'pierre_auth_token',
  USER: 'pierre_user',
  IMPERSONATION: 'pierre_impersonation',
} as const;

const defaultImpersonationState: ImpersonationState = {
  isImpersonating: false,
  targetUser: null,
  sessionId: null,
  originalUser: null,
};

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [impersonation, setImpersonation] = useState<ImpersonationState>(defaultImpersonationState);

  useEffect(() => {
    // Check for stored user info on app start
    const storedUser = localStorage.getItem(STORAGE_KEYS.USER);
    const storedToken = localStorage.getItem(STORAGE_KEYS.TOKEN);
    const storedImpersonation = localStorage.getItem(STORAGE_KEYS.IMPERSONATION);

    if (storedUser) {
      setUser(JSON.parse(storedUser));
    }
    if (storedToken) {
      setToken(storedToken);
    }
    if (storedImpersonation) {
      setImpersonation(JSON.parse(storedImpersonation));
    }

    setIsLoading(false);

    // Listen for auth failures from API service
    const handleAuthFailure = () => {
      logout();
    };

    window.addEventListener('auth-failure', handleAuthFailure);

    return () => {
      window.removeEventListener('auth-failure', handleAuthFailure);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const login = async (email: string, password: string) => {
    const response = await authApi.login({ email, password });
    // OAuth2 ROPC response uses access_token, csrf_token, and user
    const { access_token, csrf_token, user: userData } = response;

    // Store CSRF token in API service
    apiClient.setCsrfToken(csrf_token);

    // Store JWT token for WebSocket authentication
    if (access_token) {
      setToken(access_token);
      localStorage.setItem(STORAGE_KEYS.TOKEN, access_token);
    }

    // Store user info in state and localStorage
    setUser(userData);
    localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(userData));
  };

  const loginWithFirebase = async (idToken: string) => {
    const response = await authApi.loginWithFirebase({ idToken });
    const { csrf_token, jwt_token, user: userData } = response;

    // Store CSRF token in API service
    apiClient.setCsrfToken(csrf_token);

    // Store JWT token for WebSocket authentication
    if (jwt_token) {
      setToken(jwt_token);
      localStorage.setItem(STORAGE_KEYS.TOKEN, jwt_token);
    }

    // Store user info in state and localStorage
    setUser(userData);
    localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(userData));

    return response;
  };

  const logout = useCallback(() => {
    // If impersonating, also clear impersonation state
    if (impersonation.isImpersonating) {
      setImpersonation(defaultImpersonationState);
      localStorage.removeItem(STORAGE_KEYS.IMPERSONATION);
    }

    setUser(null);
    setToken(null);

    // Clear user info and token from localStorage
    localStorage.removeItem(STORAGE_KEYS.USER);
    localStorage.removeItem(STORAGE_KEYS.TOKEN);

    // Clear CSRF token from API service
    apiClient.clearCsrfToken();
    apiClient.clearUser();

    // Optionally call logout endpoint to clear cookies
    authApi.logout().catch((error) => {
      console.error('Logout API call failed:', error);
      // Continue with local cleanup even if API fails
    });
  }, [impersonation.isImpersonating]);

  const startImpersonation = useCallback(async (targetUserId: string, reason?: string) => {
    if (!user || user.role !== 'super_admin') {
      throw new Error('Only super admins can impersonate users');
    }

    const response = await adminApi.startImpersonation(targetUserId, reason);

    // Store original user before switching
    const newImpersonationState: ImpersonationState = {
      isImpersonating: true,
      targetUser: response.target_user,
      sessionId: response.session_id,
      originalUser: user,
    };

    setImpersonation(newImpersonationState);
    localStorage.setItem(STORAGE_KEYS.IMPERSONATION, JSON.stringify(newImpersonationState));

    // Update token to impersonation token
    setToken(response.token);
    localStorage.setItem(STORAGE_KEYS.TOKEN, response.token);
  }, [user]);

  const endImpersonation = useCallback(async () => {
    if (!impersonation.isImpersonating) {
      return;
    }

    try {
      await adminApi.endImpersonation();
    } catch (error) {
      console.error('Failed to end impersonation on server:', error);
      // Continue with local cleanup even if API fails
    }

    // Restore original user
    if (impersonation.originalUser) {
      setUser(impersonation.originalUser);
      localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(impersonation.originalUser));
    }

    // Clear impersonation state
    setImpersonation(defaultImpersonationState);
    localStorage.removeItem(STORAGE_KEYS.IMPERSONATION);

    // Re-login to get fresh tokens for the original user
    // The user will need to log in again for simplicity
    // In a more sophisticated implementation, we could store the original token
    window.location.reload();
  }, [impersonation]);

  const value = {
    user,
    token,
    isAuthenticated: !!user,
    isLoading,
    loading: isLoading, // For test compatibility
    login,
    loginWithFirebase,
    logout,
    impersonation,
    startImpersonation,
    endImpersonation,
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
}

