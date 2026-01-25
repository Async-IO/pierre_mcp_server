// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Authentication API methods - login, logout, register, token refresh
// ABOUTME: Handles OAuth2 ROPC flow and Firebase authentication

import { axios } from './client';

export const authApi = {
  async login(email: string, password: string) {
    // OAuth2 ROPC endpoint requires form-encoded body per RFC 6749
    const params = new URLSearchParams();
    params.append('grant_type', 'password');
    params.append('username', email);
    params.append('password', password);

    const response = await axios.post('/oauth/token', params, {
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
    });
    return response.data;
  },

  async loginWithFirebase(idToken: string) {
    const response = await axios.post('/api/auth/firebase', { id_token: idToken });
    return response.data;
  },

  async logout() {
    try {
      // Call logout endpoint to clear httpOnly cookies
      await axios.post('/api/auth/logout');
    } catch (error) {
      console.error('Logout API call failed:', error);
      // Don't throw - allow local cleanup to continue
    }
  },

  async register(email: string, password: string, displayName?: string) {
    const response = await axios.post('/api/auth/register', {
      email,
      password,
      display_name: displayName,
    });
    return response.data;
  },

  async refreshToken() {
    const response = await axios.post('/api/auth/refresh');
    return response.data;
  },
};
