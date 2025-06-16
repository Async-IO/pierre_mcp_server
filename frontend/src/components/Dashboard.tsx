import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import type { DashboardOverview, RateLimitOverview, TierUsage } from '../types/api';
import type { AnalyticsData, TimeSeriesPoint } from '../types/chart';
import ApiKeyList from './ApiKeyList';
import CreateApiKey from './CreateApiKey';
import UsageAnalytics from './UsageAnalytics';
import RequestMonitor from './RequestMonitor';
import ToolUsageBreakdown from './ToolUsageBreakdown';
import { Line } from 'react-chartjs-2';
import { useWebSocket } from '../hooks/useWebSocket';
import { useEffect } from 'react';
import { Button, Card, Badge, StatusIndicator } from './ui';
import { clsx } from 'clsx';

export default function Dashboard() {
  const { user, logout } = useAuth();
  const [activeTab, setActiveTab] = useState('overview');
  const { lastMessage } = useWebSocket();

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
    { id: 'overview', name: 'Overview', icon: '📊' },
    { id: 'api-keys', name: 'API Keys', icon: '🔑' },
    { id: 'analytics', name: 'Analytics', icon: '📈' },
    { id: 'monitor', name: 'Monitor', icon: '📡' },
    { id: 'tools', name: 'Tools', icon: '🔧' },
    { id: 'create-key', name: 'Create Key', icon: '➕' },
  ];

  return (
    <div className="min-h-screen bg-pierre-gray-50">
      {/* Header */}
      <header className="bg-white shadow-sm border-b border-pierre-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-6">
            <div>
              <div className="flex items-center gap-3 mb-2">
                <span className="text-2xl">🗿</span>
                <h1 className="text-3xl font-bold text-pierre-gray-900">Pierre MCP Server</h1>
              </div>
              <div className="flex items-center space-x-4">
                <p className="text-pierre-gray-600">Welcome back, {user?.display_name || user?.email}</p>
                <StatusIndicator status="online" label="Real-time Updates" />
              </div>
            </div>
            <Button variant="secondary" onClick={logout}>
              Sign out
            </Button>
          </div>
        </div>
      </header>

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Navigation Tabs */}
        <div className="border-b border-pierre-gray-200 mb-8">
          <nav className="flex space-x-8">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={clsx('tab', {
                  'tab-active': activeTab === tab.id,
                })}
              >
                <span>{tab.icon}</span>
                {tab.name}
              </button>
            ))}
          </nav>
        </div>

        {/* Content */}
        {activeTab === 'overview' && (
          <div className="space-y-6">
            {overviewLoading ? (
              <div className="flex justify-center py-8">
                <div className="pierre-spinner"></div>
              </div>
            ) : (
              <>
                {/* Stats Cards */}
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-blue-600">
                      {overview?.total_api_keys || 0}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Total API Keys</div>
                  </Card>
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-green-600">
                      {overview?.active_api_keys || 0}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Active Keys</div>
                  </Card>
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-yellow-600">
                      {overview?.total_requests_today || 0}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Requests Today</div>
                  </Card>
                  <Card variant="stat">
                    <div className="text-2xl font-bold text-pierre-gray-700">
                      {overview?.total_requests_this_month || 0}
                    </div>
                    <div className="text-sm text-pierre-gray-600">Requests This Month</div>
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

        {activeTab === 'api-keys' && <ApiKeyList />}
        {activeTab === 'analytics' && <UsageAnalytics />}
        {activeTab === 'monitor' && (
          <div className="space-y-6">
            <Card>
              <h2 className="text-xl font-semibold mb-4">Real-time Request Monitor</h2>
              <p className="text-pierre-gray-600 mb-4">
                Monitor API requests in real-time across all your keys. See request status, response times, and error details as they happen.
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
                Analyze which MCP tools are being used most frequently, their performance metrics, and success rates.
              </p>
            </Card>
            <ToolUsageBreakdown />
          </div>
        )}
        {activeTab === 'create-key' && <CreateApiKey />}
      </div>
    </div>
  );
}