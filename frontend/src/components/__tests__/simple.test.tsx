// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import Login from '../Login'
import { AuthProvider } from '../../contexts/AuthContext'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

// Mock the API service - AuthContext uses authApi, pierreApi, adminApi
vi.mock('../../services/api', () => ({
  authApi: {
    login: vi.fn(),
    logout: vi.fn().mockResolvedValue(undefined),
  },
  adminApi: {
    getSetupStatus: vi.fn().mockResolvedValue({ needs_setup: false, admin_exists: true }),
    endImpersonation: vi.fn(),
  },
  pierreApi: {
    adapter: {
      authStorage: {
        setCsrfToken: vi.fn().mockResolvedValue(undefined),
        getCsrfToken: vi.fn().mockResolvedValue(null),
        setUser: vi.fn().mockResolvedValue(undefined),
        getUser: vi.fn().mockResolvedValue(null),
        clear: vi.fn().mockResolvedValue(undefined),
        getToken: vi.fn().mockResolvedValue(null),
        setToken: vi.fn().mockResolvedValue(undefined),
        removeToken: vi.fn().mockResolvedValue(undefined),
        getRefreshToken: vi.fn().mockResolvedValue(null),
        setRefreshToken: vi.fn().mockResolvedValue(undefined),
      },
    },
  },
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

    expect(screen.getByRole('heading', { name: /pierre fitness platform/i })).toBeInTheDocument()
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/^password$/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
  })
})