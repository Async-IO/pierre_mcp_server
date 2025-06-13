import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import Login from '../Login'
import { AuthProvider } from '../../contexts/AuthContext'

// Mock the API service
vi.mock('../../services/api', () => ({
  apiService: {
    login: vi.fn(),
    getToken: vi.fn(),
    setToken: vi.fn(),
    clearToken: vi.fn(),
    setAuthToken: vi.fn(),
  }
}))

function renderLogin() {
  return render(
    <AuthProvider>
      <Login />
    </AuthProvider>
  )
}

describe('Login Component', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should render login form', () => {
    renderLogin()

    expect(screen.getByRole('heading', { name: /api key management/i })).toBeInTheDocument()
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
    expect(screen.getByText(/test@example\.com/)).toBeInTheDocument()
  })

  it('should allow user to type in email and password fields', async () => {
    const user = userEvent.setup()
    renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/password/i)

    await user.type(emailInput, 'test@example.com')
    await user.type(passwordInput, 'password123')

    expect(emailInput).toHaveValue('test@example.com')
    expect(passwordInput).toHaveValue('password123')
  })

  it('should require email and password fields', async () => {
    const user = userEvent.setup()
    renderLogin()

    const submitButton = screen.getByRole('button', { name: /sign in/i })
    
    // Try to submit without filling fields
    await user.click(submitButton)

    // HTML5 validation should prevent submission
    expect(screen.getByLabelText(/email address/i)).toBeRequired()
    expect(screen.getByLabelText(/password/i)).toBeRequired()
  })

  it('should show loading state during login', async () => {
    const user = userEvent.setup()
    const { apiService } = await import('../../services/api')
    
    // Make login hang to test loading state
    vi.mocked(apiService.login).mockImplementation(() => new Promise(() => {}))
    
    renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/password/i)
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
    
    renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/password/i)
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
    
    renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/password/i)
    const submitButton = screen.getByRole('button', { name: /sign in/i })

    await user.type(emailInput, 'test@example.com')
    await user.type(passwordInput, 'password123')
    await user.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Login failed')).toBeInTheDocument()
    })
  })

  it('should have proper accessibility attributes', () => {
    renderLogin()

    const emailInput = screen.getByLabelText(/email address/i)
    const passwordInput = screen.getByLabelText(/password/i)

    expect(emailInput).toHaveAttribute('type', 'email')
    expect(emailInput).toHaveAttribute('required')
    expect(passwordInput).toHaveAttribute('type', 'password')
    expect(passwordInput).toHaveAttribute('required')
  })
})