// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import Login from '../Login'
import { AuthProvider } from '../../contexts/AuthContext'

// Mock the API service
vi.mock('../../services/api', () => ({
  apiService: {
    login: vi.fn(),
    logout: vi.fn().mockResolvedValue(undefined),
    getCsrfToken: vi.fn(),
    setCsrfToken: vi.fn(),
    clearCsrfToken: vi.fn(),
    getUser: vi.fn(),
    setUser: vi.fn(),
    clearUser: vi.fn(),
    getSetupStatus: vi.fn().mockResolvedValue({ needs_setup: false, admin_exists: true }),
  }
}))

async function renderLogin() {
  let result;
  await act(async () => {
    result = render(
      <AuthProvider>
        <Login />
      </AuthProvider>
    );
    // Wait for setup status check to complete
    await waitFor(() => {
      expect(screen.queryByText('Checking setup...')).not.toBeInTheDocument();
    }, { timeout: 1000 });
  });
  return result;
}

describe('Login Component', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should render login form', async () => {
    await renderLogin()

    expect(screen.getByRole('heading', { name: /pierre fitness platform/i })).toBeInTheDocument()
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/^password$/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
  })

  it('should allow user to type in email and password fields', async () => {
    const user = userEvent.setup()
    await renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/^password$/i)

    await user.type(emailInput, 'test@example.com')
    await user.type(passwordInput, 'password123')

    expect(emailInput).toHaveValue('test@example.com')
    expect(passwordInput).toHaveValue('password123')
  })

  it('should require email and password fields', async () => {
    const user = userEvent.setup()
    await renderLogin()

    const submitButton = screen.getByRole('button', { name: /sign in/i })
    
    // Try to submit without filling fields
    await user.click(submitButton)

    // HTML5 validation should prevent submission
    expect(screen.getByLabelText(/email address/i)).toBeRequired()
    expect(screen.getByLabelText(/^password$/i)).toBeRequired()
  })

  it('should show loading state during login', async () => {
    const user = userEvent.setup()
    const { apiService } = await import('../../services/api')
    
    // Make login hang to test loading state
    vi.mocked(apiService.login).mockImplementation(() => new Promise(() => {}))
    
    await renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/^password$/i)
    const submitButton = screen.getByRole('button', { name: /sign in/i })

    await user.type(emailInput, 'test@example.com')
    await user.type(passwordInput, 'password123')
    await user.click(submitButton)

    expect(screen.getByText(/signing in\.\.\./i)).toBeInTheDocument()
    expect(submitButton).toBeDisabled()
  })

  it('should display error message on login failure', async () => {
    const user = userEvent.setup()
    const { apiService } = await import('../../services/api')
    
    const mockError = {
      response: {
        data: {
          error: 'Invalid credentials'
        }
      }
    }
    
    vi.mocked(apiService.login).mockRejectedValue(mockError)
    
    await renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/^password$/i)
    const submitButton = screen.getByRole('button', { name: /sign in/i })

    await user.type(emailInput, 'test@example.com')
    await user.type(passwordInput, 'wrongpassword')
    await user.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Invalid credentials')).toBeInTheDocument()
    })

    // Should not be loading anymore
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
    expect(submitButton).not.toBeDisabled()
  })

  it('should handle generic error when no specific error message', async () => {
    const user = userEvent.setup()
    const { apiService } = await import('../../services/api')
    
    vi.mocked(apiService.login).mockRejectedValue(new Error('Network error'))
    
    await renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/^password$/i)
    const submitButton = screen.getByRole('button', { name: /sign in/i })

    await user.type(emailInput, 'test@example.com')
    await user.type(passwordInput, 'password123')
    await user.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Login failed')).toBeInTheDocument()
    })
  })

  it('should have proper accessibility attributes', async () => {
    await renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/^password$/i)

    expect(emailInput).toHaveAttribute('type', 'email')
    expect(emailInput).toHaveAttribute('required')
    expect(passwordInput).toHaveAttribute('type', 'password')
    expect(passwordInput).toHaveAttribute('required')
  })
})