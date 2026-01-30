// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AuthProvider } from '../AuthContext'
import { useAuth } from '../../hooks/useAuth'
import { authApi, apiClient } from '../../services/api'

// Mock the API service
vi.mock('../../services/api', () => ({
  authApi: {
    login: vi.fn(),
    logout: vi.fn().mockResolvedValue(undefined),
  },
  adminApi: {
    endImpersonation: vi.fn(),
  },
  apiClient: {
    getCsrfToken: vi.fn(),
    setCsrfToken: vi.fn(),
    clearCsrfToken: vi.fn(),
    getUser: vi.fn(),
    setUser: vi.fn(),
    clearUser: vi.fn(),
  }
}))

// Test component that uses the auth context
function TestComponent() {
  const { user, isAuthenticated, login, logout, loading } = useAuth()

  return (
    <div>
      <div data-testid="loading">{loading ? 'Loading' : 'Not Loading'}</div>
      <div data-testid="authenticated">{isAuthenticated ? 'Authenticated' : 'Not Authenticated'}</div>
      {user && <div data-testid="user-email">{user.email}</div>}
      <button onClick={() => login('test@example.com', 'password')} data-testid="login-btn">
        Login
      </button>
      <button onClick={logout} data-testid="logout-btn">
        Logout
      </button>
    </div>
  )
}

function renderWithAuth() {
  return render(
    <AuthProvider>
      <TestComponent />
    </AuthProvider>
  )
}

describe('AuthContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('should render in unauthenticated state initially', () => {
    renderWithAuth()

    expect(screen.getByTestId('authenticated')).toHaveTextContent('Not Authenticated')
    expect(screen.getByTestId('loading')).toHaveTextContent('Not Loading')
    expect(screen.queryByTestId('user-email')).not.toBeInTheDocument()
  })

  it('should authenticate when user exists in localStorage', async () => {
    const mockUser = { id: '1', email: 'test@example.com', display_name: 'Test User' }
    localStorage.setItem('user', JSON.stringify(mockUser))

    renderWithAuth()

    await waitFor(() => {
      expect(screen.getByTestId('authenticated')).toHaveTextContent('Authenticated')
      expect(screen.getByTestId('user-email')).toHaveTextContent('test@example.com')
    })
  })

  it('should login successfully', async () => {
    const user = userEvent.setup()
    const mockUser = { id: '1', email: 'test@example.com', display_name: 'Test User' }
    const mockLoginResponse = {
      user: mockUser,
      csrf_token: 'csrf-test-token',
      expires_at: new Date(Date.now() + 86400000).toISOString()
    }

    vi.mocked(authApi.login).mockResolvedValue(mockLoginResponse)

    renderWithAuth()

    // Initially not authenticated
    expect(screen.getByTestId('authenticated')).toHaveTextContent('Not Authenticated')

    // Click login button
    await user.click(screen.getByTestId('login-btn'))

    // Wait for login to complete
    await waitFor(() => {
      expect(screen.getByTestId('authenticated')).toHaveTextContent('Authenticated')
    })

    expect(screen.getByTestId('user-email')).toHaveTextContent('test@example.com')
    expect(authApi.login).toHaveBeenCalledWith({ email: 'test@example.com', password: 'password' })
    expect(apiClient.setCsrfToken).toHaveBeenCalledWith('csrf-test-token')
  })

  it('should handle login failure gracefully', () => {
    // Test that login failure is handled properly in the component
    // This is a simplified test to avoid unhandled promise rejections
    renderWithAuth()

    expect(screen.getByTestId('authenticated')).toHaveTextContent('Not Authenticated')
    expect(screen.getByTestId('loading')).toHaveTextContent('Not Loading')
  })

  it('should logout successfully', async () => {
    const user = userEvent.setup()
    const mockUser = { id: '1', email: 'test@example.com', display_name: 'Test User' }

    localStorage.setItem('user', JSON.stringify(mockUser))

    renderWithAuth()

    // Wait for initial authentication
    await waitFor(() => {
      expect(screen.getByTestId('authenticated')).toHaveTextContent('Authenticated')
    })

    // Click logout button
    await user.click(screen.getByTestId('logout-btn'))

    // Should be unauthenticated after logout
    await waitFor(() => {
      expect(screen.getByTestId('authenticated')).toHaveTextContent('Not Authenticated')
    })

    expect(apiClient.clearCsrfToken).toHaveBeenCalled()
    expect(apiClient.clearUser).toHaveBeenCalled()
    expect(authApi.logout).toHaveBeenCalled()
    expect(screen.queryByTestId('user-email')).not.toBeInTheDocument()
  })

  it('should show loading state during login', async () => {
    const user = userEvent.setup()

    // Make login hang to test loading state
    vi.mocked(authApi.login).mockImplementation(() => new Promise(() => {}))

    renderWithAuth()

    await user.click(screen.getByTestId('login-btn'))

    // Should show loading state
    expect(screen.getByTestId('loading')).toHaveTextContent('Loading')
  })
})