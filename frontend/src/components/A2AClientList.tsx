// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { A2AClient, A2AUsageStats, A2ARateLimitStatus } from '../types/api';
import { Button, Card, CardHeader, Badge, StatusIndicator, StatusFilter, ConfirmDialog } from './ui';
import type { StatusFilterValue } from './ui';
// Helper functions for date formatting
const formatDistanceToNow = (date: Date) => {
  const now = new Date();
  const diffInMs = now.getTime() - date.getTime();
  const diffInDays = Math.floor(diffInMs / (1000 * 60 * 60 * 24));
  const diffInHours = Math.floor(diffInMs / (1000 * 60 * 60));
  const diffInMinutes = Math.floor(diffInMs / (1000 * 60));

  if (diffInDays > 0) {
    return `${diffInDays} day${diffInDays > 1 ? 's' : ''}`;
  } else if (diffInHours > 0) {
    return `${diffInHours} hour${diffInHours > 1 ? 's' : ''}`;
  } else if (diffInMinutes > 0) {
    return `${diffInMinutes} minute${diffInMinutes > 1 ? 's' : ''}`;
  } else {
    return 'just now';
  }
};

const format = (date: Date, pattern: string) => {
  if (pattern === 'MMM d, yyyy') {
    return date.toLocaleDateString('en-US', { 
      month: 'short', 
      day: 'numeric', 
      year: 'numeric' 
    });
  }
  return date.toLocaleDateString();
};

interface A2AClientListProps {
  onCreateClient?: () => void;
}

export default function A2AClientList({ onCreateClient }: A2AClientListProps) {
  const [selectedClient, setSelectedClient] = useState<string | null>(null);
  const [showCredentials, setShowCredentials] = useState<{ [key: string]: boolean }>({});
  const [statusFilter, setStatusFilter] = useState<StatusFilterValue>('active');
  const [clientToDeactivate, setClientToDeactivate] = useState<A2AClient | null>(null);
  const queryClient = useQueryClient();

  const { data: clients, isLoading, error } = useQuery<A2AClient[]>({
    queryKey: ['a2a-clients'],
    queryFn: () => apiService.getA2AClients(),
  });

  const { data: clientUsage } = useQuery<A2AUsageStats | null>({
    queryKey: ['a2a-client-usage', selectedClient],
    queryFn: () => selectedClient ? apiService.getA2AClientUsage(selectedClient) : Promise.resolve(null),
    enabled: !!selectedClient,
  });

  const { data: clientRateLimit } = useQuery<A2ARateLimitStatus | null>({
    queryKey: ['a2a-client-rate-limit', selectedClient],
    queryFn: () => selectedClient ? apiService.getA2AClientRateLimit(selectedClient) : Promise.resolve(null),
    enabled: !!selectedClient,
  });

  const deactivateMutation = useMutation({
    mutationFn: (clientId: string) => apiService.deactivateA2AClient(clientId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['a2a-clients'] });
      setSelectedClient(null);
      setClientToDeactivate(null);
    },
  });

  const allClients = useMemo(() => clients || [], [clients]);

  // Compute counts for the filter
  const activeCount = useMemo(() => allClients.filter(c => c.is_active).length, [allClients]);
  const inactiveCount = useMemo(() => allClients.filter(c => !c.is_active).length, [allClients]);

  // Filter clients based on status filter
  const filteredClients = useMemo(() => {
    switch (statusFilter) {
      case 'active':
        return allClients.filter(c => c.is_active);
      case 'inactive':
        return allClients.filter(c => !c.is_active);
      case 'all':
      default:
        return allClients;
    }
  }, [allClients, statusFilter]);

  const getTierBadgeColor = (tier: string) => {
    switch (tier.toLowerCase()) {
      case 'trial':
        return 'bg-yellow-100 text-yellow-800';
      case 'standard':
        return 'bg-blue-100 text-blue-800';
      case 'professional':
        return 'bg-green-100 text-green-800';
      case 'enterprise':
        return 'bg-purple-100 text-purple-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getCapabilityBadgeColor = (capability: string) => {
    const colorMap: { [key: string]: string } = {
      'fitness-data-analysis': 'bg-blue-100 text-blue-800',
      'activity-intelligence': 'bg-green-100 text-green-800',
      'goal-management': 'bg-purple-100 text-purple-800',
      'performance-prediction': 'bg-orange-100 text-orange-800',
      'training-analytics': 'bg-teal-100 text-teal-800',
      'provider-integration': 'bg-indigo-100 text-indigo-800',
    };
    return colorMap[capability] || 'bg-gray-100 text-gray-800';
  };

  const handleDeactivate = (client: A2AClient) => {
    setClientToDeactivate(client);
  };

  const confirmDeactivate = () => {
    if (clientToDeactivate) {
      deactivateMutation.mutate(clientToDeactivate.id);
    }
  };

  const toggleCredentials = (clientId: string) => {
    setShowCredentials(prev => ({
      ...prev,
      [clientId]: !prev[clientId]
    }));
  };

  if (isLoading) {
    return (
      <Card>
        <div className="animate-pulse">
          <div className="h-4 bg-pierre-gray-200 rounded w-1/4 mb-4"></div>
          <div className="space-y-3">
            <div className="h-16 bg-pierre-gray-200 rounded"></div>
            <div className="h-16 bg-pierre-gray-200 rounded"></div>
            <div className="h-16 bg-pierre-gray-200 rounded"></div>
          </div>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <div className="text-center py-8">
          <div className="text-red-500 mb-4">❌</div>
          <h3 className="text-lg font-medium text-pierre-gray-900 mb-2">Failed to load A2A clients</h3>
          <p className="text-pierre-gray-600 mb-4">There was an error loading your A2A clients.</p>
          <Button onClick={() => window.location.reload()}>
            Try Again
          </Button>
        </div>
      </Card>
    );
  }

  if (allClients.length === 0) {
    return (
      <div className="text-center py-16 bg-gray-50 rounded-lg border-2 border-dashed border-gray-300">
        <div className="text-6xl mb-4 text-gray-400">🤖</div>
        <h3 className="text-lg font-semibold text-gray-900 mb-2">No Connected Apps Yet</h3>
        <p className="text-gray-600 mb-6 max-w-md mx-auto">
          Register your first app to enable secure agent-to-agent communication with AI assistants and third-party integrations.
        </p>
        <Button
          onClick={onCreateClient}
          className="inline-flex items-center space-x-2"
        >
          <span>+</span>
          <span>Register Your First App</span>
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* A2A Client List */}
      <Card>
        <CardHeader
          title="Your Connected Apps"
          subtitle={`${allClients.length} total apps`}
        />

        {/* Status Filter */}
        <div className="px-6 pb-4">
          <StatusFilter
            value={statusFilter}
            onChange={setStatusFilter}
            activeCount={activeCount}
            inactiveCount={inactiveCount}
            totalCount={allClients.length}
          />
        </div>

        <div className="space-y-4 px-6 pb-6">
          {filteredClients.map((client) => (
            <div
              key={client.id}
              className={`border rounded-lg p-4 cursor-pointer transition-colors ${
                selectedClient === client.id
                  ? 'border-pierre-blue-500 bg-pierre-blue-50'
                  : 'border-pierre-gray-200 hover:border-pierre-gray-300'
              }`}
              onClick={() => setSelectedClient(selectedClient === client.id ? null : client.id)}
            >
              <div className="flex items-center justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <h3 className="text-lg font-medium text-pierre-gray-900">{client.name}</h3>
                    <StatusIndicator 
                      status={client.is_active ? 'online' : 'offline'} 
                      size="sm"
                    />
                    {client.is_verified && (
                      <Badge variant="success" className="bg-green-100 text-green-800">
                        Verified
                      </Badge>
                    )}
                  </div>
                  <p className="text-pierre-gray-600 mb-3">{client.description}</p>
                  
                  {/* Capabilities */}
                  <div className="flex flex-wrap gap-2 mb-3">
                    {client.capabilities.map((capability) => (
                      <Badge
                        key={capability}
                        variant="info"
                        className={getCapabilityBadgeColor(capability)}
                      >
                        {capability}
                      </Badge>
                    ))}
                  </div>

                  <div className="flex items-center gap-4 text-sm text-pierre-gray-500">
                    <span>Created {formatDistanceToNow(new Date(client.created_at))} ago</span>
                    {client.agent_version && <span>v{client.agent_version}</span>}
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleCredentials(client.id);
                    }}
                  >
                    {showCredentials[client.id] ? 'Hide' : 'Show'} Credentials
                  </Button>
                  {client.is_active && (
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeactivate(client);
                      }}
                      disabled={deactivateMutation.isPending}
                    >
                      Deactivate
                    </Button>
                  )}
                </div>
              </div>

              {/* Credentials (when expanded) */}
              {showCredentials[client.id] && (
                <div className="mt-4 pt-4 border-t border-pierre-gray-200">
                  <h4 className="text-sm font-medium text-pierre-gray-900 mb-2">Client Credentials</h4>
                  <div className="space-y-2 text-sm">
                    <div>
                      <label className="text-pierre-gray-600">Client ID:</label>
                      <code className="block bg-pierre-gray-100 p-2 rounded font-mono text-xs mt-1">
                        {client.id}
                      </code>
                    </div>
                    <div className="text-pierre-gray-600 text-xs">
                      ⚠️ Client secret and API key are only shown once during registration
                    </div>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      </Card>

      {/* Client Details */}
      {selectedClient && clientUsage && clientRateLimit && (
        <Card>
          <h3 className="text-lg font-semibold text-pierre-gray-900 mb-4">
            Client Usage & Rate Limits
          </h3>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            {/* Usage Stats */}
            <div>
              <h4 className="text-sm font-medium text-pierre-gray-700 mb-2">Usage Statistics</h4>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <span className="text-pierre-gray-600">Today:</span>
                  <span className="font-medium">{clientUsage?.requests_today?.toLocaleString() || 0}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-pierre-gray-600">This Month:</span>
                  <span className="font-medium">{clientUsage?.requests_this_month?.toLocaleString() || 0}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-pierre-gray-600">Total:</span>
                  <span className="font-medium">{clientUsage?.total_requests?.toLocaleString() || 0}</span>
                </div>
                {clientUsage?.last_request_at && (
                  <div className="flex justify-between">
                    <span className="text-pierre-gray-600">Last Request:</span>
                    <span className="font-medium">
                      {formatDistanceToNow(new Date(clientUsage.last_request_at))} ago
                    </span>
                  </div>
                )}
              </div>
            </div>

            {/* Rate Limits */}
            <div>
              <h4 className="text-sm font-medium text-pierre-gray-700 mb-2">Rate Limits</h4>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <span className="text-pierre-gray-600">Tier:</span>
                  <Badge variant="info" className={getTierBadgeColor(clientRateLimit?.tier || 'trial')}>
                    {clientRateLimit?.tier || 'Trial'}
                  </Badge>
                </div>
                {clientRateLimit?.limit && (
                  <>
                    <div className="flex justify-between">
                      <span className="text-pierre-gray-600">Monthly Limit:</span>
                      <span className="font-medium">{clientRateLimit.limit.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-pierre-gray-600">Remaining:</span>
                      <span className={`font-medium ${
                        clientRateLimit.remaining && clientRateLimit.remaining < clientRateLimit.limit * 0.1
                          ? 'text-red-600'
                          : 'text-green-600'
                      }`}>
                        {clientRateLimit.remaining?.toLocaleString() || 0}
                      </span>
                    </div>
                    {clientRateLimit.reset_at && (
                      <div className="flex justify-between">
                        <span className="text-pierre-gray-600">Resets:</span>
                        <span className="font-medium">
                          {format(new Date(clientRateLimit.reset_at), 'MMM d, yyyy')}
                        </span>
                      </div>
                    )}
                  </>
                )}
              </div>
            </div>

            {/* Tool Usage */}
            <div>
              <h4 className="text-sm font-medium text-pierre-gray-700 mb-2">Top Tools</h4>
              <div className="space-y-2">
                {clientUsage?.tool_usage_breakdown?.slice(0, 3).map((tool: { tool_name: string; usage_count: number }) => (
                  <div key={tool.tool_name} className="flex justify-between">
                    <span className="text-pierre-gray-600 truncate">{tool.tool_name}:</span>
                    <span className="font-medium">{tool.usage_count}</span>
                  </div>
                ))}
                {(!clientUsage?.tool_usage_breakdown || clientUsage.tool_usage_breakdown.length === 0) && (
                  <div className="text-pierre-gray-500 text-sm">No tool usage yet</div>
                )}
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* Deactivate Confirmation */}
      <ConfirmDialog
        isOpen={clientToDeactivate !== null}
        onClose={() => setClientToDeactivate(null)}
        onConfirm={confirmDeactivate}
        title="Deactivate A2A Client"
        message={`Are you sure you want to deactivate "${clientToDeactivate?.name}"? This action cannot be undone and any applications using this client will lose access.`}
        confirmLabel="Deactivate"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={deactivateMutation.isPending}
      />
    </div>
  );
}