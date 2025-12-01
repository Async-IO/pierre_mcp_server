// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import Dashboard from '../Dashboard';

// Mock all dependencies to avoid complex setup
vi.mock('../UsageAnalytics', () => ({
  default: () => <div data-testid="usage-analytics">Analytics Component</div>
}));

vi.mock('../RequestMonitor', () => ({
  default: () => <div data-testid="request-monitor">Monitor Component</div>
}));

vi.mock('../ToolUsageBreakdown', () => ({
  default: () => <div data-testid="tool-breakdown">Tools Component</div>
}));

vi.mock('../UnifiedConnections', () => ({
  default: () => <div data-testid="connections">Connections Component</div>
}));

vi.mock('../UserManagement', () => ({
  default: () => <div data-testid="user-management">User Management Component</div>
}));

vi.mock('react-chartjs-2', () => ({
  Line: () => <div data-testid="chart">Chart Component</div>
}));

// Mock contexts
vi.mock('../../hooks/useAuth', () => ({
  useAuth: () => ({
    user: { email: 'admin@test.com', display_name: 'Admin User', is_admin: true },
    logout: vi.fn(),
    isAuthenticated: true,
    isLoading: false
  })
}));

vi.mock('../../hooks/useWebSocketContext', () => ({
  useWebSocketContext: () => ({
    isConnected: true,
    lastMessage: null,
    sendMessage: vi.fn(),
    subscribe: vi.fn()
  })
}));

// Mock API with simple responses
vi.mock('../../services/api', () => ({
  apiService: {
    getDashboardOverview: vi.fn().mockResolvedValue({
      total_api_keys: 10,
      active_api_keys: 8,
      total_requests_today: 500,
      total_requests_this_month: 15000
    }),
    getRateLimitOverview: vi.fn().mockResolvedValue([]),
    getUsageAnalytics: vi.fn().mockResolvedValue({ time_series: [] }),
    getA2ADashboardOverview: vi.fn().mockResolvedValue({
      total_clients: 3,
      active_clients: 2,
      requests_today: 100,
      requests_this_month: 3000
    }),
    getPendingUsers: vi.fn().mockResolvedValue([
      { id: '1', email: 'user@test.com' }
    ])
  }
}));

function renderDashboard() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } }
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <Dashboard />
    </QueryClientProvider>
  );
}

describe('Dashboard Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render dashboard layout', async () => {
    await act(async () => {
      renderDashboard();
    });

    // Check for page title in header
    expect(screen.getByRole('heading', { level: 1, name: 'Overview' })).toBeInTheDocument();
    // Check for sign out button (icon button with title attribute)
    expect(screen.getByTitle('Sign out')).toBeInTheDocument();
  });

  it('should render navigation tabs', async () => {
    await act(async () => {
      renderDashboard();
    });

    // Use getAllByText since nav items appear in sidebar and may appear in header
    expect(screen.getAllByText('Overview').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Connections').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Analytics').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Monitor').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Tools').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Users').length).toBeGreaterThan(0);
  });

  it('should show user information', async () => {
    await act(async () => {
      renderDashboard();
    });

    expect(screen.getByText('Admin User')).toBeInTheDocument();
    expect(screen.getByTitle('Sign out')).toBeInTheDocument();
  });

  it('should show pending users badge', async () => {
    await act(async () => {
      renderDashboard();
    });

    // Wait for the pending users query to resolve and badge to appear
    const badge = await screen.findByTestId('pending-users-badge', {}, { timeout: 5000 });
    expect(badge).toHaveTextContent('1');
  });

  it('should switch to Analytics tab', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderDashboard();
    });

    // Click the sidebar nav button (first element found)
    const buttons = screen.getAllByText('Analytics');
    await user.click(buttons[0]);

    // Wait for lazy component to load
    await waitFor(() => {
      expect(screen.getByTestId('usage-analytics')).toBeInTheDocument();
    });
  });

  it('should switch to Connections tab', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderDashboard();
    });

    // Click the sidebar nav button (first element found)
    const buttons = screen.getAllByText('Connections');
    await user.click(buttons[0]);

    // Wait for lazy component to load
    await waitFor(() => {
      expect(screen.getByTestId('connections')).toBeInTheDocument();
    });
  });

  it('should switch to Monitor tab', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderDashboard();
    });

    // Click the sidebar nav button (first element found)
    const buttons = screen.getAllByText('Monitor');
    await user.click(buttons[0]);

    // Wait for lazy component to load
    await waitFor(() => {
      expect(screen.getByTestId('request-monitor')).toBeInTheDocument();
    });
  });

  it('should switch to Tools tab', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderDashboard();
    });

    // Click the sidebar nav button (first element found)
    const buttons = screen.getAllByText('Tools');
    await user.click(buttons[0]);

    // Wait for lazy component to load
    await waitFor(() => {
      expect(screen.getByTestId('tool-breakdown')).toBeInTheDocument();
    });
  });

  it('should switch to Users tab', async () => {
    const user = userEvent.setup();

    await act(async () => {
      renderDashboard();
    });

    // Click the sidebar nav button (first element found)
    const buttons = screen.getAllByText('Users');
    await user.click(buttons[0]);

    // Wait for lazy component to load
    await waitFor(() => {
      expect(screen.getByTestId('user-management')).toBeInTheDocument();
    });
  });
});