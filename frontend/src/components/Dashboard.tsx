import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useAuth } from '../contexts/AuthContext';
import { apiService } from '../services/api';
import ApiKeyList from './ApiKeyList';
import CreateApiKey from './CreateApiKey';
import UsageAnalytics from './UsageAnalytics';
import RealTimeIndicator from './RealTimeIndicator';
import { Line } from 'react-chartjs-2';
import { useWebSocket } from '../hooks/useWebSocket';
import { useEffect } from 'react';

export default function Dashboard() {
  const { user, logout } = useAuth();
  const [activeTab, setActiveTab] = useState('overview');
  const { lastMessage } = useWebSocket();

  const { data: overview, isLoading: overviewLoading, refetch: refetchOverview } = useQuery({
    queryKey: ['dashboard-overview'],
    queryFn: () => apiService.getDashboardOverview(),
  });

  const { data: rateLimits } = useQuery({
    queryKey: ['rate-limits'],
    queryFn: () => apiService.getRateLimitOverview(),
  });

  const { data: weeklyUsage } = useQuery({
    queryKey: ['usage-analytics', 7],
    queryFn: () => apiService.getUsageAnalytics(7),
  });

  // Prepare mini chart data for the overview
  const miniChartData = {
    labels: weeklyUsage?.time_series?.slice(-7).map((point: any) => {
      const date = new Date(point.date);
      return date.toLocaleDateString('en-US', { weekday: 'short' });
    }) || [],
    datasets: [
      {
        label: 'Requests',
        data: weeklyUsage?.time_series?.slice(-7).map((point: any) => point.request_count) || [],
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
    { id: 'create-key', name: 'Create Key', icon: '➕' },
  ];

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <header className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-6">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">API Key Management</h1>
              <div className="flex items-center space-x-4">
                <p className="text-gray-600">Welcome back, {user?.display_name || user?.email}</p>
                <RealTimeIndicator />
              </div>
            </div>
            <button
              onClick={logout}
              className="btn-secondary"
            >
              Sign out
            </button>
          </div>
        </div>
      </header>

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Navigation Tabs */}
        <div className="border-b border-gray-200 mb-8">
          <nav className="-mb-px flex space-x-8">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`py-2 px-1 border-b-2 font-medium text-sm ${
                  activeTab === tab.id
                    ? 'border-api-blue text-api-blue'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                <span className="mr-2">{tab.icon}</span>
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
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-api-blue"></div>
              </div>
            ) : (
              <>
                {/* Stats Cards */}
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
                  <div className="stat-card">
                    <div className="text-2xl font-bold text-api-blue">
                      {overview?.total_api_keys || 0}
                    </div>
                    <div className="text-sm text-gray-600">Total API Keys</div>
                  </div>
                  <div className="stat-card">
                    <div className="text-2xl font-bold text-api-green">
                      {overview?.active_api_keys || 0}
                    </div>
                    <div className="text-sm text-gray-600">Active Keys</div>
                  </div>
                  <div className="stat-card">
                    <div className="text-2xl font-bold text-api-yellow">
                      {overview?.total_requests_today || 0}
                    </div>
                    <div className="text-sm text-gray-600">Requests Today</div>
                  </div>
                  <div className="stat-card">
                    <div className="text-2xl font-bold text-gray-700">
                      {overview?.total_requests_this_month || 0}
                    </div>
                    <div className="text-sm text-gray-600">Requests This Month</div>
                  </div>
                </div>

                {/* Weekly Trend Mini Chart */}
                {weeklyUsage?.time_series?.length > 0 && (
                  <div className="card">
                    <div className="flex justify-between items-center mb-4">
                      <h3 className="text-lg font-medium">7-Day Request Trend</h3>
                      <span className="text-sm text-gray-500">
                        Total: {weeklyUsage.time_series.reduce((sum: number, point: any) => sum + point.request_count, 0)}
                      </span>
                    </div>
                    <div style={{ height: '120px' }}>
                      <Line data={miniChartData} options={miniChartOptions} />
                    </div>
                  </div>
                )}

                {/* Usage by Tier */}
                {overview?.current_month_usage_by_tier?.length > 0 && (
                  <div className="card">
                    <h3 className="text-lg font-medium mb-4">Usage by Tier</h3>
                    <div className="space-y-3">
                      {overview.current_month_usage_by_tier.map((tier: any) => (
                        <div key={tier.tier} className="flex justify-between items-center">
                          <div>
                            <span className="font-medium capitalize">{tier.tier}</span>
                            <span className="text-gray-500 ml-2">({tier.key_count} keys)</span>
                          </div>
                          <div className="text-right">
                            <div className="font-bold">{tier.total_requests.toLocaleString()}</div>
                            <div className="text-sm text-gray-500">
                              Avg: {Math.round(tier.average_requests_per_key)}/key
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Rate Limit Status */}
                {rateLimits && rateLimits.length > 0 && (
                  <div className="card">
                    <h3 className="text-lg font-medium mb-4">Rate Limit Status</h3>
                    <div className="space-y-4">
                      {rateLimits.map((item: any) => (
                        <div key={item.api_key_id} className="border rounded-lg p-4">
                          <div className="flex justify-between items-center mb-2">
                            <span className="font-medium">{item.api_key_name}</span>
                            <span className="text-sm bg-gray-100 px-2 py-1 rounded capitalize">
                              {item.tier}
                            </span>
                          </div>
                          {item.limit ? (
                            <>
                              <div className="flex justify-between text-sm text-gray-600 mb-1">
                                <span>Usage: {item.current_usage.toLocaleString()} / {item.limit.toLocaleString()}</span>
                                <span>{Math.round(item.usage_percentage)}%</span>
                              </div>
                              <div className="w-full bg-gray-200 rounded-full h-2">
                                <div
                                  className={`h-2 rounded-full ${
                                    item.usage_percentage > 90 ? 'bg-api-red' :
                                    item.usage_percentage > 70 ? 'bg-api-yellow' : 'bg-api-green'
                                  }`}
                                  style={{ width: `${Math.min(item.usage_percentage, 100)}%` }}
                                ></div>
                              </div>
                            </>
                          ) : (
                            <div className="text-sm text-gray-600">
                              Unlimited usage - {item.current_usage.toLocaleString()} requests used
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </>
            )}
          </div>
        )}

        {activeTab === 'api-keys' && <ApiKeyList />}
        {activeTab === 'analytics' && <UsageAnalytics />}
        {activeTab === 'create-key' && <CreateApiKey />}
      </div>
    </div>
  );
}