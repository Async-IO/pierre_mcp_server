// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { useWebSocketContext } from '../hooks/useWebSocketContext';
import RealTimeIndicator from './RealTimeIndicator';
import type { RequestLog, RequestStats } from '../types/api';

interface RequestMonitorProps {
  apiKeyId?: string;
  showAllKeys?: boolean;
}

export default function RequestMonitor({ apiKeyId, showAllKeys = false }: RequestMonitorProps) {
  const [filter, setFilter] = useState({
    timeRange: '1h',
    status: 'all',
    tool: 'all'
  });
  const [liveRequests, setLiveRequests] = useState<RequestLog[]>([]);
  const { lastMessage } = useWebSocketContext();

  const { data: requestLogs, isLoading } = useQuery<RequestLog[]>({
    queryKey: ['request-logs', apiKeyId, filter],
    queryFn: () => apiService.getRequestLogs(apiKeyId, filter),
    refetchInterval: 5000, // Refresh every 5 seconds
  });

  const { data: requestStats } = useQuery<RequestStats>({
    queryKey: ['request-stats', apiKeyId, filter.timeRange],
    queryFn: () => apiService.getRequestStats(apiKeyId, filter.timeRange),
    refetchInterval: 10000,
  });

  // Handle real-time WebSocket updates
  useEffect(() => {
    if (lastMessage?.type === 'request_update') {
      const newRequest = lastMessage.data as RequestLog;
      if (!apiKeyId || newRequest.api_key_id === apiKeyId) {
        setLiveRequests(prev => [newRequest, ...prev.slice(0, 49)]); // Keep last 50
      }
    }
  }, [lastMessage, apiKeyId]);

  const getStatusIcon = (status: number) => {
    if (status >= 200 && status < 300) return '✅';
    if (status >= 400 && status < 500) return '⚠️';
    if (status >= 500) return '❌';
    return '⏳';
  };

  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return 'text-pierre-green-400';
    if (status >= 400 && status < 500) return 'text-pierre-yellow-400';
    if (status >= 500) return 'text-pierre-red-400';
    return 'text-zinc-400';
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${Math.round(ms)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const allRequests = [...liveRequests, ...(requestLogs || [])];
  const uniqueRequests = allRequests.filter((request, index, arr) =>
    arr.findIndex(r => r.id === request.id) === index
  );

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="pierre-spinner w-8 h-8"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Real-time Stats */}
      {requestStats && (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="stat-card-dark">
            <div className="text-2xl font-bold text-pierre-violet-light">
              {requestStats.total_requests}
            </div>
            <div className="text-sm text-zinc-400">Total Requests</div>
          </div>
          <div className="stat-card-dark">
            <div className="text-2xl font-bold text-pierre-activity">
              {requestStats.total_requests > 0
                ? ((requestStats.successful_requests / requestStats.total_requests) * 100).toFixed(1)
                : '0.0'}%
            </div>
            <div className="text-sm text-zinc-400">Success Rate</div>
          </div>
          <div className="stat-card-dark">
            <div className="text-2xl font-bold text-pierre-nutrition">
              {formatDuration(requestStats.average_response_time)}
            </div>
            <div className="text-sm text-zinc-400">Avg Response Time</div>
          </div>
          <div className="stat-card-dark">
            <div className="text-2xl font-bold text-white">
              {requestStats.requests_per_minute.toFixed(1)}
            </div>
            <div className="text-sm text-zinc-400">Requests/min</div>
          </div>
        </div>
      )}

      {/* Filters */}
      <div className="card-admin">
        <div className="flex flex-wrap gap-4 items-center">
          <div>
            <label className="block text-sm font-medium text-zinc-400 mb-1">
              Time Range
            </label>
            <select
              value={filter.timeRange}
              onChange={(e) => setFilter(prev => ({ ...prev, timeRange: e.target.value }))}
              className="select-dark text-sm py-1"
            >
              <option value="1h">Last Hour</option>
              <option value="24h">Last 24 Hours</option>
              <option value="7d">Last 7 Days</option>
              <option value="30d">Last 30 Days</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-zinc-400 mb-1">
              Status
            </label>
            <select
              value={filter.status}
              onChange={(e) => setFilter(prev => ({ ...prev, status: e.target.value }))}
              className="select-dark text-sm py-1"
            >
              <option value="all">All Status</option>
              <option value="success">Success (2xx)</option>
              <option value="error">Error (4xx/5xx)</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-zinc-400 mb-1">
              Tool
            </label>
            <select
              value={filter.tool}
              onChange={(e) => setFilter(prev => ({ ...prev, tool: e.target.value }))}
              className="select-dark text-sm py-1"
            >
              <option value="all">All Tools</option>
              <option value="get_activities">Get Activities</option>
              <option value="get_athlete">Get Athlete</option>
              <option value="get_stats">Get Stats</option>
              <option value="get_activity_intelligence">Activity Intelligence</option>
            </select>
          </div>

          <RealTimeIndicator className="ml-auto" />
        </div>
      </div>

      {/* Request Log */}
      <div className="card-admin">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-lg font-medium text-white">Request Log</h3>
          <span className="text-sm text-zinc-500">
            Showing {uniqueRequests.length} requests
          </span>
        </div>

        {uniqueRequests.length === 0 ? (
          <div className="text-center py-8 text-zinc-500">
            <div className="text-4xl mb-4">📊</div>
            <p className="text-lg mb-2">No requests yet</p>
            <p>Start making API calls to see request logs here</p>
          </div>
        ) : (
          <div className="space-y-2 max-h-96 overflow-y-auto scrollbar-dark">
            {uniqueRequests.map((request) => (
              <div
                key={request.id}
                className="flex items-center justify-between p-3 border border-white/10 rounded-lg hover:bg-white/5 transition-colors"
              >
                <div className="flex items-center space-x-4 flex-1">
                  <div className="text-lg">
                    {getStatusIcon(request.status_code)}
                  </div>

                  <div className="flex-1">
                    <div className="flex items-center space-x-2">
                      <span className="font-medium text-sm text-white">{request.tool_name}</span>
                      <span className={`text-sm font-mono ${getStatusColor(request.status_code)}`}>
                        {request.status_code}
                      </span>
                      {request.error_message && (
                        <span className="text-xs text-pierre-red-400 truncate max-w-xs">
                          {request.error_message}
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-zinc-500">
                      {new Date(request.timestamp).toLocaleString()}
                    </div>
                  </div>
                </div>

                <div className="text-right text-sm">
                  <div className="font-medium text-white">
                    {request.response_time_ms ? formatDuration(request.response_time_ms) : 'N/A'}
                  </div>
                  {showAllKeys && (
                    <div className="text-xs text-zinc-500 font-mono">
                      {request.api_key_name}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
