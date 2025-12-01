// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

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
  Filler,
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
  ArcElement,
  Filler
);

export default function UsageAnalytics() {
  const [timeRange, setTimeRange] = useState<number>(30);

  const { data: analytics, isLoading } = useQuery<AnalyticsData>({
    queryKey: ['usage-analytics', timeRange],
    queryFn: () => apiService.getUsageAnalytics(timeRange),
  });

  // Pierre Design System colors for charts
  const pierreColors = {
    violet: '#7C3AED',
    cyan: '#06B6D4',
    activity: '#10B981',
    nutrition: '#F59E0B',
    recovery: '#6366F1',
    red: '#EF4444',
  };

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
        borderColor: pierreColors.violet,
        backgroundColor: `${pierreColors.violet}1A`,
        tension: 0.4,
        fill: true,
      },
      {
        label: 'Errors',
        data: analytics?.time_series?.map((point: TimeSeriesPoint) => point.error_count) || [],
        borderColor: pierreColors.red,
        backgroundColor: `${pierreColors.red}1A`,
        tension: 0.4,
        fill: true,
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
          `${pierreColors.violet}CC`,
          `${pierreColors.activity}CC`,
          `${pierreColors.nutrition}CC`,
          `${pierreColors.cyan}CC`,
          `${pierreColors.recovery}CC`,
        ],
        borderColor: [
          pierreColors.violet,
          pierreColors.activity,
          pierreColors.nutrition,
          pierreColors.cyan,
          pierreColors.recovery,
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
        backgroundColor: `${pierreColors.activity}99`,
        borderColor: pierreColors.activity,
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
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-pierre-violet"></div>
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
            <div className="text-2xl font-bold text-pierre-violet">
              {analytics?.time_series?.reduce((sum: number, point: TimeSeriesPoint) => sum + point.request_count, 0) || 0}
            </div>
            <div className="text-sm text-pierre-gray-600">Total Requests</div>
          </div>
          <div className="stat-card">
            <div className="text-2xl font-bold text-pierre-red-500">
              {analytics?.error_rate?.toFixed(1) || 0}%
            </div>
            <div className="text-sm text-pierre-gray-600">Error Rate</div>
          </div>
          <div className="stat-card">
            <div className="text-2xl font-bold text-pierre-activity">
              {analytics?.average_response_time?.toFixed(0) || 0}ms
            </div>
            <div className="text-sm text-pierre-gray-600">Avg Response Time</div>
          </div>
        </div>

        {/* Time Series Chart */}
        <div className="mb-8">
          <h3 className="text-lg font-medium mb-4">Request Volume Over Time</h3>
          <div className="bg-white rounded-lg p-4 border border-pierre-gray-200">
            {analytics?.time_series && analytics.time_series.length > 0 ? (
              <Line data={timeSeriesData} options={chartOptions} />
            ) : (
              <div className="bg-pierre-gray-100 rounded-lg p-8 text-center text-pierre-gray-500">
                No time series data available yet
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
              <div className="bg-white rounded-lg p-4 border border-pierre-gray-200">
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
              <div className="bg-white rounded-lg p-4 border border-pierre-gray-200">
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
                <div key={tool.tool_name} className="flex justify-between items-center p-3 bg-pierre-gray-50 rounded hover:bg-pierre-gray-100 transition-colors">
                  <div>
                    <span className="font-medium text-pierre-gray-900">{tool.tool_name}</span>
                    <span className="text-pierre-gray-500 ml-2">
                      {(tool.success_rate || 0).toFixed(1)}% success rate
                    </span>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-pierre-violet">{tool.request_count.toLocaleString()}</div>
                    <div className="text-sm text-pierre-gray-500">
                      {(tool.average_response_time || 0).toFixed(0)}ms avg
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {(!analytics?.time_series?.length && !analytics?.top_tools?.length) && (
          <div className="text-center py-8 text-pierre-gray-500">
            <p className="text-lg mb-2">No usage data yet</p>
            <p>Start making API calls to see analytics here</p>
          </div>
        )}
      </div>

    </div>
  );
}