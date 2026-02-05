// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Authentication domain API - login, logout, register, token refresh
// ABOUTME: Platform-agnostic auth logic using the adapter for token storage

import type { AxiosInstance } from 'axios';
import type { User, LoginResponse, RegisterResponse, FirebaseLoginResponse, SessionResponse } from '@pierre/shared-types';
import type { AuthStorage } from '../types/platform';
import { ENDPOINTS } from '../core/endpoints';

export interface LoginCredentials {
  email: string;
  password: string;
}

export interface RegisterCredentials {
  email: string;
  password: string;
  display_name?: string;
}

export interface FirebaseLoginData {
  idToken: string;
}

/**
 * Creates the auth API methods bound to an axios instance.
 */
export function createAuthApi(axios: AxiosInstance, authStorage: AuthStorage) {
  return {
    /**
     * Login with email and password.
     */
    async login(credentials: LoginCredentials): Promise<LoginResponse> {
      const formData = new URLSearchParams();
      formData.append('grant_type', 'password');
      formData.append('username', credentials.email);
      formData.append('password', credentials.password);

      const response = await axios.post<LoginResponse>(ENDPOINTS.AUTH.TOKEN, formData, {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      });

      const data = response.data;

      // Store tokens
      if (data.access_token) {
        await authStorage.setToken(data.access_token);
      }
      if (data.refresh_token) {
        await authStorage.setRefreshToken(data.refresh_token);
      }
      if (data.user) {
        await authStorage.setUser(data.user);
      }

      return data;
    },

    /**
     * Login with Firebase ID token.
     */
    async loginWithFirebase(data: FirebaseLoginData): Promise<FirebaseLoginResponse> {
      // Backend expects id_token (snake_case)
      const response = await axios.post<FirebaseLoginResponse>(ENDPOINTS.AUTH.FIREBASE, {
        id_token: data.idToken,
      });
      const result = response.data;

      // Store tokens - Firebase uses jwt_token, not access_token
      if (result.jwt_token) {
        await authStorage.setToken(result.jwt_token);
      }
      if (result.csrf_token) {
        await authStorage.setCsrfToken(result.csrf_token);
      }
      if (result.user) {
        await authStorage.setUser(result.user);
      }

      return result;
    },

    /**
     * Logout the current user.
     */
    async logout(): Promise<void> {
      try {
        await axios.post(ENDPOINTS.AUTH.LOGOUT);
      } finally {
        // Always clear local auth data
        await authStorage.clear();
      }
    },

    /**
     * Register a new user.
     */
    async register(credentials: RegisterCredentials): Promise<RegisterResponse> {
      const response = await axios.post<RegisterResponse>(ENDPOINTS.AUTH.REGISTER, credentials);
      return response.data;
    },

    /**
     * Refresh the access token using the refresh token.
     */
    async refreshToken(): Promise<LoginResponse> {
      const refreshToken = await authStorage.getRefreshToken();
      if (!refreshToken) {
        throw new Error('No refresh token available');
      }

      const formData = new URLSearchParams();
      formData.append('grant_type', 'refresh_token');
      formData.append('refresh_token', refreshToken);

      const response = await axios.post<LoginResponse>(ENDPOINTS.AUTH.TOKEN, formData, {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      });

      const data = response.data;

      // Update stored tokens
      if (data.access_token) {
        await authStorage.setToken(data.access_token);
      }
      if (data.refresh_token) {
        await authStorage.setRefreshToken(data.refresh_token);
      }
      if (data.user) {
        await authStorage.setUser(data.user);
      }

      return data;
    },

    /**
     * Restore session using httpOnly cookie authentication.
     * Returns user info and a fresh JWT for WebSocket auth.
     * Throws on 401 if no valid session exists.
     */
    async getSession(): Promise<SessionResponse> {
      const response = await axios.get<SessionResponse>(ENDPOINTS.AUTH.SESSION);
      return response.data;
    },

    /**
     * Get the currently stored user (from local storage, not server).
     */
    async getStoredUser(): Promise<User | null> {
      return authStorage.getUser<User>();
    },

    /**
     * Store user data locally.
     */
    async storeUser(user: User): Promise<void> {
      await authStorage.setUser(user);
    },

    /**
     * Store all auth data (token, CSRF token, and user).
     * Mobile-compatible convenience method.
     */
    async storeAuth(token: string, csrfToken: string, user: User): Promise<void> {
      await authStorage.setToken(token);
      await authStorage.setCsrfToken(csrfToken);
      await authStorage.setUser(user);
    },

    /**
     * Clear all stored auth data.
     */
    async clearStoredAuth(): Promise<void> {
      await authStorage.clear();
    },

    /**
     * Initialize auth state from storage.
     * Returns true if valid auth data was found.
     */
    async initializeAuth(): Promise<boolean> {
      const token = await authStorage.getToken();
      const user = await authStorage.getUser<User>();
      return !!(token && user);
    },
  };
}

export type AuthApi = ReturnType<typeof createAuthApi>;
