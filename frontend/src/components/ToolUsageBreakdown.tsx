import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Doughnut, Bar } from 'react-chartjs-2';
import type { ToolUsageBreakdown } from '../types/api';

interface ToolUsageBreakdownProps {
  apiKeyId?: string;
  timeRange?: string;
}

export default function ToolUsageBreakdownComponent({ 
  apiKeyId, 
  timeRange = '7d' 
}: ToolUsageBreakdownProps) {
  const { data: toolUsage, isLoading } = useQuery<ToolUsageBreakdown[]>({
    queryKey: ['tool-usage-breakdown', apiKeyId, timeRange],
    queryFn: () => apiService.getToolUsageBreakdown(apiKeyId, timeRange),
  });

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-api-blue"></div>
      </div>
    );
  }

  if (!toolUsage || toolUsage.length === 0) {
    return (
      <div className="card">
        <div className="text-center py-8 text-gray-500">
          <div className="text-4xl mb-4">🔧</div>
          <p className="text-lg mb-2">No tool usage data</p>
          <p>Start making API calls to see tool usage breakdown</p>
        </div>
      </div>
    );
  }

  // Prepare chart data
  const colors = [
    '#2563EB', // Blue
    '#10B981', // Green  
    '#F59E0B', // Yellow
    '#EF4444', // Red
    '#8B5CF6', // Purple
    '#06B6D4', // Cyan
    '#F97316', // Orange
    '#84CC16', // Lime
  ];

  const doughnutData = {
    labels: toolUsage.map(tool => tool.tool_name.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase())),
    datasets: [
      {
        data: toolUsage.map(tool => tool.request_count),
        backgroundColor: colors.slice(0, toolUsage.length),
        borderColor: colors.slice(0, toolUsage.length),
        borderWidth: 2,
      },
    ],
  };

  const barData = {
    labels: toolUsage.map(tool => tool.tool_name.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase())),
    datasets: [
      {
        label: 'Avg Response Time (ms)',
        data: toolUsage.map(tool => tool.avg_response_time),
        backgroundColor: 'rgba(37, 99, 235, 0.6)',
        borderColor: 'rgb(37, 99, 235)',
        borderWidth: 1,
      },
    ],
  };

  const doughnutOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'bottom' as const,
      },
      tooltip: {
        callbacks: {
          label: function(context: { label: string; dataIndex: number }) {
            const tool = toolUsage[context.dataIndex];
            return `${context.label}: ${tool.request_count} requests (${tool.percentage_of_total.toFixed(1)}%)`;
          }
        }
      }
    },
  };

  const barOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: false,
      },
    },
    scales: {
      y: {
        beginAtZero: true,
        title: {
          display: true,
          text: 'Response Time (ms)'
        }
      },
      x: {
        ticks: {
          maxRotation: 45,
        }
      }
    },
  };

  const formatToolName = (toolName: string) => {
    return toolName.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase());
  };

  return (
    <div className="space-y-6">
      {/* Tool Usage Distribution */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="card">
          <h3 className="text-lg font-medium mb-4">Request Distribution</h3>
          <div style={{ height: '300px' }}>
            <Doughnut data={doughnutData} options={doughnutOptions} />
          </div>
        </div>

        <div className="card">
          <h3 className="text-lg font-medium mb-4">Average Response Time</h3>
          <div style={{ height: '300px' }}>
            <Bar data={barData} options={barOptions} />
          </div>
        </div>
      </div>

      {/* Detailed Breakdown Table */}
      <div className="card">
        <h3 className="text-lg font-medium mb-4">Tool Usage Details</h3>
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Tool Name
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Requests
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Success Rate
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Avg Response Time
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Errors
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Share
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {toolUsage.map((tool, index) => (
                <tr key={tool.tool_name} className="hover:bg-gray-50">
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center">
                      <div 
                        className="w-3 h-3 rounded-full mr-3"
                        style={{ backgroundColor: colors[index % colors.length] }}
                      />
                      <div className="text-sm font-medium text-gray-900">
                        {formatToolName(tool.tool_name)}
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                    {tool.request_count.toLocaleString()}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center">
                      <div className="text-sm text-gray-900">
                        {tool.success_rate.toFixed(1)}%
                      </div>
                      <div className="ml-2 w-16 bg-gray-200 rounded-full h-2">
                        <div
                          className={`h-2 rounded-full ${
                            tool.success_rate >= 95 ? 'bg-green-500' :
                            tool.success_rate >= 90 ? 'bg-yellow-500' : 'bg-red-500'
                          }`}
                          style={{ width: `${tool.success_rate}%` }}
                        />
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                    {tool.avg_response_time.toFixed(0)}ms
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className={`inline-flex px-2 py-1 text-xs font-medium rounded-full ${
                      tool.error_count === 0 
                        ? 'bg-green-100 text-green-800'
                        : tool.error_count < 10
                        ? 'bg-yellow-100 text-yellow-800'
                        : 'bg-red-100 text-red-800'
                    }`}>
                      {tool.error_count}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                    {tool.percentage_of_total.toFixed(1)}%
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="stat-card">
          <div className="text-2xl font-bold text-api-blue">
            {toolUsage.length}
          </div>
          <div className="text-sm text-gray-600">Tools Used</div>
        </div>
        <div className="stat-card">
          <div className="text-2xl font-bold text-api-green">
            {toolUsage.reduce((sum, tool) => sum + tool.request_count, 0).toLocaleString()}
          </div>
          <div className="text-sm text-gray-600">Total Requests</div>
        </div>
        <div className="stat-card">
          <div className="text-2xl font-bold text-api-yellow">
            {(toolUsage.reduce((sum, tool) => sum + tool.success_rate * tool.request_count, 0) / 
             toolUsage.reduce((sum, tool) => sum + tool.request_count, 0)).toFixed(1)}%
          </div>
          <div className="text-sm text-gray-600">Overall Success Rate</div>
        </div>
        <div className="stat-card">
          <div className="text-2xl font-bold text-gray-700">
            {(toolUsage.reduce((sum, tool) => sum + tool.avg_response_time * tool.request_count, 0) / 
             toolUsage.reduce((sum, tool) => sum + tool.request_count, 0)).toFixed(0)}ms
          </div>
          <div className="text-sm text-gray-600">Avg Response Time</div>
        </div>
      </div>
    </div>
  );
}