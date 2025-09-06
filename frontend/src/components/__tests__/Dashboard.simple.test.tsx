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
    user: { email: 'admin@test.com', display_name: 'Admin User' },
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

    expect(screen.getByText('Pierre Fitness API')).toBeInTheDocument();
    expect(screen.getByText('🗿')).toBeInTheDocument(); // Logo
  });

  it('should render navigation tabs', async () => {
    await act(async () => {
      renderDashboard();
    });

    expect(screen.getByText('Overview')).toBeInTheDocument();
    expect(screen.getByText('Connections')).toBeInTheDocument();
    expect(screen.getByText('Analytics')).toBeInTheDocument();
    expect(screen.getByText('Monitor')).toBeInTheDocument();
    expect(screen.getByText('Tools')).toBeInTheDocument();
    expect(screen.getByText('Users')).toBeInTheDocument();
  });

  it('should show user information', async () => {
    await act(async () => {
      renderDashboard();
    });

    expect(screen.getByText('Admin User')).toBeInTheDocument();
    expect(screen.getByText('Sign out')).toBeInTheDocument();
  });

  it('should show pending users badge', async () => {
    await act(async () => {
      renderDashboard();
    });

    // Wait for the pending users query to load and badge to appear
    await waitFor(() => {
      expect(screen.getByText('1')).toBeInTheDocument();
    }, { timeout: 2000 });
  });

  it('should switch to Analytics tab', async () => {
    const user = userEvent.setup();
    
    await act(async () => {
      renderDashboard();
    });

    await user.click(screen.getByText('Analytics'));
    
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

    await user.click(screen.getByText('Connections'));
    
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

    await user.click(screen.getByText('Monitor'));
    
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

    await user.click(screen.getByText('Tools'));
    
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

    await user.click(screen.getByText('Users'));
    
    // Wait for lazy component to load
    await waitFor(() => {
      expect(screen.getByTestId('user-management')).toBeInTheDocument();
    });
  });
});