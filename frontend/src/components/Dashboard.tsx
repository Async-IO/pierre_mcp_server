import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import type { DashboardOverview, RateLimitOverview, TierUsage } from '../types/api';
import type { AnalyticsData, TimeSeriesPoint } from '../types/chart';
import UsageAnalytics from './UsageAnalytics';
import RequestMonitor from './RequestMonitor';
import ToolUsageBreakdown from './ToolUsageBreakdown';
import UnifiedConnections from './UnifiedConnections';
import UserManagement from './UserManagement';
import { Line } from 'react-chartjs-2';
import { useWebSocketContext } from '../hooks/useWebSocketContext';
import { useEffect } from 'react';
import { Button, Card, Badge } from './ui';
import RealTimeIndicator from './RealTimeIndicator';
import { clsx } from 'clsx';

export default function Dashboard() {
  const { user, logout } = useAuth();
  const [activeTab, setActiveTab] = useState('overview');
  const { lastMessage } = useWebSocketContext();

  const { data: overview, isLoading: overviewLoading, refetch: refetchOverview } = useQuery<DashboardOverview>({
    queryKey: ['dashboard-overview'],
    queryFn: () => apiService.getDashboardOverview(),
  });

  const { data: rateLimits } = useQuery<RateLimitOverview[]>({
    queryKey: ['rate-limits'],
    queryFn: () => apiService.getRateLimitOverview(),
  });

  const { data: weeklyUsage } = useQuery<AnalyticsData>({
    queryKey: ['usage-analytics', 7],
    queryFn: () => apiService.getUsageAnalytics(7),
  });

  const { data: a2aOverview } = useQuery({
    queryKey: ['a2a-dashboard-overview'],
    queryFn: () => apiService.getA2ADashboardOverview(),
  });

  const { data: pendingUsers = [] } = useQuery({
    queryKey: ['pending-users'],
    queryFn: () => apiService.getPendingUsers(),
    refetchInterval: 30000,
  });

  // Prepare mini chart data for the overview
  const miniChartData = {
    labels: weeklyUsage?.time_series?.slice(-7).map((point: TimeSeriesPoint) => {
      const date = new Date(point.date);
      return date.toLocaleDateString('en-US', { weekday: 'short' });
    }) || [],
    datasets: [
      {
        label: 'Requests',
        data: weeklyUsage?.time_series?.slice(-7).map((point: TimeSeriesPoint) => point.request_count) || [],
        borderColor: 'rgb(37, 99, 235)',
        backgroundColor: 'rgba(37, 99, 235, 0.1)',
        tension: 0.4,
        fill: true,
        pointRadius: 3,
      },
    ],
  };

  const miniChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: false,
      },
    },
    scales: {
      x: {
        display: false,
      },
      y: {
        display: false,
      },
    },
    elements: {
      point: {
        hoverRadius: 6,
      },
    },
  };

  // Refresh data when WebSocket updates are received
  useEffect(() => {
    if (lastMessage) {
      if (lastMessage.type === 'usage_update' || lastMessage.type === 'system_stats') {
        refetchOverview();
      }
    }
  }, [lastMessage, refetchOverview]);

  const tabs = [
    { id: 'overview', name: 'Overview', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    ) },
    { id: 'connections', name: 'Connections', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
      </svg>
    ) },
    { id: 'analytics', name: 'Analytics', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    ) },
    { id: 'monitor', name: 'Monitor', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
      </svg>
    ) },
    { id: 'tools', name: 'Tools', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
    { id: 'users', name: 'Users', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
      </svg>
    ), badge: pendingUsers.length > 0 ? pendingUsers.length : undefined },
  ];

  return (
    <div className="min-h-screen bg-pierre-gray-50 flex">
      {/* Sidebar Navigation */}
      <aside className="w-32 bg-white shadow-lg border-r border-pierre-gray-200 flex flex-col">
        {/* Logo */}
        <div className="flex items-center justify-center px-4 h-16 border-b border-pierre-gray-200">
          <span className="text-2xl">🗿</span>
        </div>
        
        {/* User section */}
        <div className="border-b border-pierre-gray-200 p-2">
          <div className="flex flex-col items-center mb-2">
            <div className="w-8 h-8 bg-pierre-blue-100 rounded-full flex items-center justify-center mb-1">
              <span className="text-xs font-medium text-pierre-blue-600">
                {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
              </span>
            </div>
            <p className="text-xs font-medium text-pierre-gray-900 text-center truncate w-full">
              {user?.display_name || user?.email?.split('@')[0]}
            </p>
          </div>
          <Button 
            variant="secondary" 
            onClick={logout}
            className="w-full text-xs py-1"
          >
            Sign out
          </Button>
        </div>
        
        {/* Navigation Items */}
        <nav className="flex-1 px-2 py-4 space-y-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={clsx(
                'w-full flex flex-col items-center px-2 py-2 rounded-lg text-sm font-medium transition-colors relative',
                {
                  'bg-pierre-blue-50 text-pierre-blue-600 border border-pierre-blue-200': activeTab === tab.id,
                  'text-pierre-gray-600 hover:text-pierre-gray-900 hover:bg-pierre-gray-50': activeTab !== tab.id,
                }
              )}
            >
              <div className="mb-1 relative">
                {tab.icon}
                {tab.badge && (
                  <span className="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full h-4 w-4 flex items-center justify-center">
                    {tab.badge}
                  </span>
                )}
              </div>
              <span className="text-xs text-center">{tab.name}</span>
            </button>
          ))}
        </nav>
      </aside>

      {/* Main Content */}
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <header className="bg-white shadow-sm border-b border-pierre-gray-200">
          <div className="px-6 py-4">
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-2xl font-bold text-pierre-gray-900">Pierre Fitness API</h1>
                <div className="flex items-center space-x-4 mt-1">
                  <p className="text-pierre-gray-600">Welcome back, {user?.display_name || user?.email}</p>
                  <RealTimeIndicator />
                </div>
              </div>
            </div>
          </div>
        </header>

        {/* Content Area */}
        <div className="flex-1 px-6 py-8 overflow-auto">

          {/* Content */}
        {activeTab === 'overview' && (
          <div className="space-y-6">
            {overviewLoading ? (
              <div className="flex justify-center py-8">
                <div className="pierre-spinner"></div>
              </div>
            ) : (
              <>
                {/* Unified Stats Cards */}
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-blue-600">
                      {(overview?.total_api_keys || 0) + (a2aOverview?.total_clients || 0)}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Total Connections</div>
                    <div className="text-xs text-pierre-gray-500 mt-1">
                      {overview?.total_api_keys || 0} API Keys • {a2aOverview?.total_clients || 0} Apps
                    </div>
                  </Card>
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-green-600">
                      {(overview?.active_api_keys || 0) + (a2aOverview?.active_clients || 0)}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Active Connections</div>
                    <div className="text-xs text-pierre-gray-500 mt-1">
                      {overview?.active_api_keys || 0} Keys • {a2aOverview?.active_clients || 0} Apps
                    </div>
                  </Card>
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-yellow-600">
                      {(overview?.total_requests_today || 0) + (a2aOverview?.requests_today || 0)}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Requests Today</div>
                    <div className="text-xs text-pierre-gray-500 mt-1">
                      All connections combined
                    </div>
                  </Card>
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-gray-700">
                      {(overview?.total_requests_this_month || 0) + (a2aOverview?.requests_this_month || 0)}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Requests This Month</div>
                    <div className="text-xs text-pierre-gray-500 mt-1">
                      All connections combined
                    </div>
                  </Card>
                </div>

                {/* Weekly Trend Mini Chart */}
                {weeklyUsage?.time_series && weeklyUsage.time_series.length > 0 && (
                  <Card>
                    <div className="flex justify-between items-center mb-4">
                      <h3 className="text-lg font-medium">7-Day Request Trend</h3>
                      <span className="text-sm text-pierre-gray-500">
                        Total: {weeklyUsage?.time_series?.reduce((sum: number, point: TimeSeriesPoint) => sum + point.request_count, 0) || 0}
                      </span>
                    </div>
                    <div style={{ height: '120px' }}>
                      <Line data={miniChartData} options={miniChartOptions} />
                    </div>
                  </Card>
                )}

                {/* Usage by Tier */}
                {overview?.current_month_usage_by_tier && overview.current_month_usage_by_tier.length > 0 && (
                  <Card>
                    <h3 className="text-lg font-medium mb-4">Usage by Tier</h3>
                    <div className="space-y-3">
                      {overview.current_month_usage_by_tier?.map((tier: TierUsage) => (
                        <div key={tier.tier} className="flex justify-between items-center">
                          <div>
                            <span className="font-medium capitalize">{tier.tier}</span>
                            <span className="text-pierre-gray-500 ml-2">({tier.key_count} keys)</span>
                          </div>
                          <div className="text-right">
                            <div className="font-bold">{tier.total_requests.toLocaleString()}</div>
                            <div className="text-sm text-pierre-gray-500">
                              Avg: {Math.round(tier.average_requests_per_key)}/key
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </Card>
                )}

                {/* Rate Limit Status */}
                {rateLimits && rateLimits.length > 0 && (
                  <Card>
                    <h3 className="text-lg font-medium mb-4">Rate Limit Status</h3>
                    <div className="space-y-4">
                      {rateLimits.map((item: RateLimitOverview) => (
                        <div key={item.api_key_id} className="border border-pierre-gray-200 rounded-lg p-4">
                          <div className="flex justify-between items-center mb-2">
                            <span className="font-medium">{item.api_key_name}</span>
                            <Badge variant={item.tier as 'starter' | 'professional' | 'enterprise' | 'trial'}>
                              {item.tier}
                            </Badge>
                          </div>
                          {item.limit ? (
                            <>
                              <div className="flex justify-between text-sm text-pierre-gray-600 mb-1">
                                <span>Usage: {item.current_usage.toLocaleString()} / {item.limit.toLocaleString()}</span>
                                <span>{Math.round(item.usage_percentage)}%</span>
                              </div>
                              <div className="w-full bg-pierre-gray-200 rounded-full h-2">
                                <div
                                  className={`h-2 rounded-full ${
                                    item.usage_percentage > 90 ? 'bg-pierre-red-600' :
                                    item.usage_percentage > 70 ? 'bg-pierre-yellow-600' : 'bg-pierre-green-600'
                                  }`}
                                  style={{ width: `${Math.min(item.usage_percentage, 100)}%` }}
                                ></div>
                              </div>
                            </>
                          ) : (
                            <div className="text-sm text-pierre-gray-600">
                              Unlimited usage - {item.current_usage.toLocaleString()} requests used
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </Card>
                )}
              </>
            )}
          </div>
        )}

        {activeTab === 'connections' && <UnifiedConnections />}
        {activeTab === 'analytics' && <UsageAnalytics />}
        {activeTab === 'monitor' && (
          <div className="space-y-6">
            <Card>
              <h2 className="text-xl font-semibold mb-4">Real-time Request Monitor</h2>
              <p className="text-pierre-gray-600 mb-4">
                Monitor API requests in real-time across all your connections. See request status, response times, and error details as they happen.
              </p>
            </Card>
            <RequestMonitor showAllKeys={true} />
          </div>
        )}
        {activeTab === 'tools' && (
          <div className="space-y-6">
            <Card>
              <h2 className="text-xl font-semibold mb-4">Tool Usage Analysis</h2>
              <p className="text-pierre-gray-600 mb-4">
                Analyze which fitness tools are being used most frequently, their performance metrics, and success rates.
              </p>
            </Card>
            <ToolUsageBreakdown />
          </div>
        )}
        {activeTab === 'users' && (
          <div className="space-y-6">
            <Card>
              <h2 className="text-xl font-semibold mb-4">User Management</h2>
              <p className="text-pierre-gray-600 mb-4">
                Manage user registrations, approve pending users, and monitor user activity across the platform.
              </p>
            </Card>
            <UserManagement />
          </div>
        )}
        </div>
      </div>
    </div>
  );
}