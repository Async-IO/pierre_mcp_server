import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
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
  it('should render Login component', () => {
    renderWithProviders(<Login />)
    
    expect(screen.getByRole('heading', { name: /api key management/i })).toBeInTheDocument()
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
  })

  it('should show test user hint', () => {
    renderWithProviders(<Login />)
    
    expect(screen.getByText(/test@example\.com/)).toBeInTheDocument()
  })
})