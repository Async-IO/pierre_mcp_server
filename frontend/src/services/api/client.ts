// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Base API client with axios setup, interceptors, and CSRF token management
// ABOUTME: Shared by all domain-specific API modules

import axios, { type AxiosError, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios';

// In development, use empty string to leverage Vite proxy (avoids CORS issues)
// In production, use VITE_API_BASE_URL environment variable
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

class ApiClient {
  private csrfToken: string | null = null;
  private initialized = false;

  constructor() {
    this.initialize();
  }

  private initialize() {
    if (this.initialized) return;

    // Set up axios defaults
    axios.defaults.baseURL = API_BASE_URL;
    axios.defaults.headers.common['Content-Type'] = 'application/json';

    // Enable sending cookies with requests
    axios.defaults.withCredentials = true;

    // Set up interceptors
    this.setupInterceptors();
    this.initialized = true;
  }

  private setupInterceptors() {
    // Request interceptor to add CSRF token
    axios.interceptors.request.use(
      (config: InternalAxiosRequestConfig) => {
        // Add CSRF token for state-changing operations
        if (this.csrfToken && config.headers && ['POST', 'PUT', 'DELETE', 'PATCH'].includes(config.method?.toUpperCase() || '')) {
          config.headers['X-CSRF-Token'] = this.csrfToken;
        }
        return config;
      },
      (error) => Promise.reject(error)
    );

    // Response interceptor to handle 401 errors
    axios.interceptors.response.use(
      (response: AxiosResponse) => response,
      async (error: AxiosError) => {
        if (error.response?.status === 401) {
          this.handleAuthFailure();
        }
        return Promise.reject(error);
      }
    );
  }

  private handleAuthFailure() {
    // Clear CSRF token
    this.csrfToken = null;

    // Trigger logout event for components to react
    window.dispatchEvent(new CustomEvent('auth-failure'));

    // Redirect to login if not already there
    if (!window.location.pathname.includes('/login')) {
      window.location.href = '/login';
    }
  }

  // CSRF Token management
  getCsrfToken(): string | null {
    return this.csrfToken;
  }

  setCsrfToken(token: string) {
    this.csrfToken = token;
  }

  clearCsrfToken() {
    this.csrfToken = null;
  }

  // User info management (still using localStorage for user data, not auth tokens)
  getUser(): { id: string; email: string; display_name?: string } | null {
    const user = localStorage.getItem('user');
    return user ? JSON.parse(user) : null;
  }

  setUser(user: { id: string; email: string; display_name?: string }) {
    localStorage.setItem('user', JSON.stringify(user));
  }

  clearUser() {
    localStorage.removeItem('user');
  }
}

// Export singleton instance
export const apiClient = new ApiClient();

// Re-export axios for use in domain modules
export { axios };
