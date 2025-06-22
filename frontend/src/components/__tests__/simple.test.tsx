import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import Login from '../Login'
import { AuthProvider } from '../../contexts/AuthContext'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

// Mock the API service
vi.mock('../../services/api', () => ({
  apiService: {
    login: vi.fn(),
    getToken: vi.fn(() => null),
    setToken: vi.fn(),
    clearToken: vi.fn(),
    setAuthToken: vi.fn(),
    getSetupStatus: vi.fn().mockResolvedValue({ needs_setup: false, admin_exists: true }),
  }
}))

function renderWithProviders(component: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })

  return render(
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        {component}
      </AuthProvider>
    </QueryClientProvider>
  )
}

describe('Component Tests', () => {
  it('should render Login component', async () => {
    renderWithProviders(<Login />)
    
    expect(screen.getByRole('heading', { name: /pierre mcp admin/i })).toBeInTheDocument()
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
    
    // Wait for setup status check to complete
    await waitFor(() => {
      expect(screen.getByText(/admin setup complete/i)).toBeInTheDocument()
    })
  })

  it('should show admin setup complete', async () => {
    renderWithProviders(<Login />)
    
    // Wait for setup status check to complete
    await waitFor(() => {
      expect(screen.getByText(/admin setup complete/i)).toBeInTheDocument()
    })
  })
})