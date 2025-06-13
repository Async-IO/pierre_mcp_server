import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { ChartData, ChartOptions, AnalyticsData, TimeSeriesPoint, TopTool } from '../types/chart';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  BarElement,
  Title,
  Tooltip,
  Legend,
  ArcElement,
} from 'chart.js';
import { Line, Bar, Doughnut } from 'react-chartjs-2';

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  BarElement,
  Title,
  Tooltip,
  Legend,
  ArcElement
);

export default function UsageAnalytics() {
  const [timeRange, setTimeRange] = useState<number>(30);

  const { data: analytics, isLoading } = useQuery<AnalyticsData>({
    queryKey: ['usage-analytics', timeRange],
    queryFn: () => apiService.getUsageAnalytics(timeRange),
  });

  // Prepare chart data
  const timeSeriesData: ChartData = {
    labels: analytics?.time_series?.map((point: TimeSeriesPoint) => {
      const date = new Date(point.date);
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    }) || [],
    datasets: [
      {
        label: 'API Requests',
        data: analytics?.time_series?.map((point: TimeSeriesPoint) => point.request_count) || [],
        borderColor: 'rgb(37, 99, 235)',
        backgroundColor: 'rgba(37, 99, 235, 0.1)',
        tension: 0.4,
      },
      {
        label: 'Errors',
        data: analytics?.time_series?.map((point: TimeSeriesPoint) => point.error_count) || [],
        borderColor: 'rgb(220, 38, 38)',
        backgroundColor: 'rgba(220, 38, 38, 0.1)',
        tension: 0.4,
      },
    ],
  };

  const toolUsageData: ChartData = {
    labels: analytics?.top_tools?.map((tool: TopTool) => tool.tool_name) || [],
    datasets: [
      {
        label: 'Request Count',
        data: analytics?.top_tools?.map((tool: TopTool) => tool.request_count) || [],
        backgroundColor: [
          'rgba(37, 99, 235, 0.8)',
          'rgba(5, 150, 105, 0.8)',
          'rgba(217, 119, 6, 0.8)',
          'rgba(220, 38, 38, 0.8)',
          'rgba(147, 51, 234, 0.8)',
        ],
        borderColor: [
          'rgb(37, 99, 235)',
          'rgb(5, 150, 105)',
          'rgb(217, 119, 6)',
          'rgb(220, 38, 38)',
          'rgb(147, 51, 234)',
        ],
        borderWidth: 1,
      },
    ],
  };

  const responseTimeData: ChartData = {
    labels: analytics?.top_tools?.map((tool: TopTool) => tool.tool_name) || [],
    datasets: [
      {
        label: 'Average Response Time (ms)',
        data: analytics?.top_tools?.map((tool: TopTool) => tool.average_response_time || 0) || [],
        backgroundColor: 'rgba(5, 150, 105, 0.6)',
        borderColor: 'rgb(5, 150, 105)',
        borderWidth: 1,
      },
    ],
  };

  const chartOptions: ChartOptions = {
    responsive: true,
    plugins: {
      legend: {
        position: 'top',
      },
    },
    scales: {
      y: {
        beginAtZero: true,
      },
    },
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-api-blue"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="card">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-xl font-semibold">Usage Analytics</h2>
          <select
            value={timeRange}
            onChange={(e) => setTimeRange(Number(e.target.value))}
            className="input-field w-auto"
          >
            <option value={7}>Last 7 days</option>
            <option value={30}>Last 30 days</option>
            <option value={90}>Last 90 days</option>
          </select>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <div className="stat-card">
            <div className="text-2xl font-bold text-api-blue">
              {analytics?.time_series?.reduce((sum: number, point: TimeSeriesPoint) => sum + point.request_count, 0) || 0}
            </div>
            <div className="text-sm text-gray-600">Total Requests</div>
          </div>
          <div className="stat-card">
            <div className="text-2xl font-bold text-api-red">
              {analytics?.error_rate?.toFixed(1) || 0}%
            </div>
            <div className="text-sm text-gray-600">Error Rate</div>
          </div>
          <div className="stat-card">
            <div className="text-2xl font-bold text-api-green">
              {analytics?.average_response_time?.toFixed(0) || 0}ms
            </div>
            <div className="text-sm text-gray-600">Avg Response Time</div>
          </div>
        </div>

        {/* Time Series Chart */}
        <div className="mb-8">
          <h3 className="text-lg font-medium mb-4">Request Volume Over Time</h3>
          <div className="bg-white rounded-lg p-4 border border-gray-200">
            {analytics?.time_series && analytics.time_series.length > 0 ? (
              <Line data={timeSeriesData} options={chartOptions} />
            ) : (
              <div className="bg-gray-100 rounded-lg p-8 text-center text-gray-500">
                📈 No time series data available yet
                <br />
                <small>Make some API calls to see request patterns</small>
              </div>
            )}
          </div>
        </div>

        {/* Tool Usage Charts */}
        {analytics?.top_tools && analytics.top_tools.length > 0 && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
            <div>
              <h3 className="text-lg font-medium mb-4">Tool Usage Distribution</h3>
              <div className="bg-white rounded-lg p-4 border border-gray-200">
                <Doughnut 
                  data={toolUsageData} 
                  options={{
                    responsive: true,
                    plugins: {
                      legend: {
                        position: 'bottom' as const,
                      },
                    },
                  }} 
                />
              </div>
            </div>
            <div>
              <h3 className="text-lg font-medium mb-4">Response Time by Tool</h3>
              <div className="bg-white rounded-lg p-4 border border-gray-200">
                <Bar data={responseTimeData} options={chartOptions} />
              </div>
            </div>
          </div>
        )}

        {/* Top Tools Table */}
        {analytics?.top_tools && analytics.top_tools.length > 0 && (
          <div>
            <h3 className="text-lg font-medium mb-4">Most Used Tools</h3>
            <div className="space-y-3">
              {analytics.top_tools.map((tool: TopTool) => (
                <div key={tool.tool_name} className="flex justify-between items-center p-3 bg-gray-50 rounded">
                  <div>
                    <span className="font-medium">{tool.tool_name}</span>
                    <span className="text-gray-500 ml-2">
                      {((tool.success_rate || 0) * 100).toFixed(1)}% success rate
                    </span>
                  </div>
                  <div className="text-right">
                    <div className="font-bold">{tool.request_count.toLocaleString()}</div>
                    <div className="text-sm text-gray-500">
                      {(tool.average_response_time || 0).toFixed(0)}ms avg
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {(!analytics?.time_series?.length && !analytics?.top_tools?.length) && (
          <div className="text-center py-8 text-gray-500">
            <div className="text-4xl mb-4">📊</div>
            <p className="text-lg mb-2">No usage data yet</p>
            <p>Start making API calls to see analytics here</p>
          </div>
        )}
      </div>

      <div className="card bg-green-50 border-green-200">
        <h3 className="font-semibold text-green-800 mb-3">✅ Analytics Features</h3>
        <ul className="text-sm text-green-700 space-y-2">
          <li>• ✅ Interactive charts and graphs</li>
          <li>• ✅ Real-time usage statistics</li>
          <li>• ✅ Time-based filtering</li>
          <li>• ✅ Tool usage distribution</li>
          <li>• ✅ Response time monitoring</li>
        </ul>
        
        <h4 className="font-semibold text-blue-800 mt-4 mb-2">🚀 Available Features</h4>
        <ul className="text-sm text-blue-700 space-y-1">
          <li>• Real-time usage monitoring via WebSocket</li>
          <li>• Interactive charts and visualizations</li>
          <li>• Time-based filtering (7, 30, 90 days)</li>
          <li>• API rate limit tracking</li>
          <li>• Tool usage distribution analysis</li>
        </ul>
      </div>
    </div>
  );
}