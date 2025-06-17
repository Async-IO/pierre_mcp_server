import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { A2ADashboardOverview } from '../types/api';
import { Button, Card, Badge, StatusIndicator } from './ui';
import A2AClientList from './A2AClientList';
import CreateA2AClient from './CreateA2AClient';

export default function A2AManagement() {
  const [activeView, setActiveView] = useState<'overview' | 'clients' | 'create'>('overview');

  const { data: overview, isLoading: overviewLoading } = useQuery<A2ADashboardOverview>({
    queryKey: ['a2a-dashboard-overview'],
    queryFn: () => apiService.getA2ADashboardOverview(),
  });

  const { data: agentCard } = useQuery({
    queryKey: ['a2a-agent-card'],
    queryFn: () => apiService.getA2AAgentCard(),
  });

  const renderOverview = () => (
    <div className="space-y-6">
      {/* A2A Overview Stats */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-pierre-blue-600">
              {overview?.total_clients || 0}
            </div>
            <div className="text-sm text-pierre-gray-600 mt-1">Total Clients</div>
            <div className="text-xs text-pierre-gray-500 mt-1">
              {overview?.active_clients || 0} active
            </div>
          </div>
        </Card>

        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-green-600">
              {overview?.active_sessions || 0}
            </div>
            <div className="text-sm text-pierre-gray-600 mt-1">Active Sessions</div>
            <div className="text-xs text-pierre-gray-500 mt-1">
              {overview?.total_sessions || 0} total
            </div>
          </div>
        </Card>

        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-purple-600">
              {overview?.requests_today?.toLocaleString() || 0}
            </div>
            <div className="text-sm text-pierre-gray-600 mt-1">Requests Today</div>
            <div className="text-xs text-pierre-gray-500 mt-1">
              {overview?.requests_this_month?.toLocaleString() || 0} this month
            </div>
          </div>
        </Card>

        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-orange-600">
              {overview?.error_rate ? `${(overview.error_rate * 100).toFixed(1)}%` : '0%'}
            </div>
            <div className="text-sm text-pierre-gray-600 mt-1">Error Rate</div>
            <StatusIndicator 
              status={
                !overview?.error_rate || overview.error_rate < 0.01 ? 'online' : 
                overview.error_rate < 0.05 ? 'offline' : 'error'
              } 
              size="sm"
              className="mt-1"
            />
          </div>
        </Card>
      </div>

      {/* Agent Card Information */}
      {agentCard && (
        <Card>
          <h3 className="text-lg font-semibold text-pierre-gray-900 mb-4">Pierre Agent Card</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <h4 className="text-sm font-medium text-pierre-gray-700 mb-2">Agent Information</h4>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-pierre-gray-600">Name:</span>
                  <span>{agentCard.agent?.name || 'Pierre Fitness Intelligence'}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-pierre-gray-600">Version:</span>
                  <span>{agentCard.agent?.version || '1.0.0'}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-pierre-gray-600">Protocol:</span>
                  <span>{agentCard.protocol?.version || 'A2A v0.1.0'}</span>
                </div>
              </div>
            </div>
            <div>
              <h4 className="text-sm font-medium text-pierre-gray-700 mb-2">Available Tools</h4>
              <div className="text-sm text-pierre-gray-600">
                {agentCard.tools?.length || 0} tools available
              </div>
              <div className="flex flex-wrap gap-1 mt-2">
                {agentCard.tools?.slice(0, 3).map((tool: { name: string }) => (
                  <Badge key={tool.name} variant="info" className="text-xs">
                    {tool.name}
                  </Badge>
                ))}
                {agentCard.tools?.length > 3 && (
                  <Badge variant="info" className="text-xs bg-pierre-gray-100 text-pierre-gray-600">
                    +{agentCard.tools.length - 3} more
                  </Badge>
                )}
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* Usage by Tier */}
      {overview?.usage_by_tier && overview.usage_by_tier.length > 0 && (
        <Card>
          <h3 className="text-lg font-semibold text-pierre-gray-900 mb-4">Usage by Tier</h3>
          <div className="space-y-3">
            {overview.usage_by_tier.map((tier) => (
              <div key={tier.tier} className="flex items-center justify-between p-3 bg-pierre-gray-50 rounded-lg">
                <div className="flex items-center gap-3">
                  <Badge 
                    variant="info"
                    className={
                      tier.tier === 'Enterprise' ? 'bg-purple-100 text-purple-800' :
                      tier.tier === 'Professional' ? 'bg-green-100 text-green-800' :
                      tier.tier === 'Standard' ? 'bg-blue-100 text-blue-800' :
                      'bg-yellow-100 text-yellow-800'
                    }
                  >
                    {tier.tier}
                  </Badge>
                  <span className="text-pierre-gray-700">{tier.client_count} clients</span>
                </div>
                <div className="text-right">
                  <div className="font-medium">{tier.request_count.toLocaleString()} requests</div>
                  <div className="text-sm text-pierre-gray-600">{tier.percentage.toFixed(1)}% of total</div>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Most Used Capability */}
      {overview?.most_used_capability && (
        <Card>
          <h3 className="text-lg font-semibold text-pierre-gray-900 mb-4">Most Popular Capability</h3>
          <div className="text-center">
            <Badge variant="info" className="text-lg px-4 py-2 bg-pierre-blue-100 text-pierre-blue-800">
              {overview.most_used_capability}
            </Badge>
            <p className="text-pierre-gray-600 mt-2">
              This capability is being used most frequently by A2A clients
            </p>
          </div>
        </Card>
      )}

      {/* Quick Actions */}
      <Card>
        <h3 className="text-lg font-semibold text-pierre-gray-900 mb-4">Quick Actions</h3>
        <div className="flex flex-wrap gap-3">
          <Button onClick={() => setActiveView('create')}>
            Register New A2A Client
          </Button>
          <Button variant="secondary" onClick={() => setActiveView('clients')}>
            Manage Clients
          </Button>
          <Button 
            variant="secondary" 
            onClick={() => window.open('/a2a/agent-card', '_blank')}
          >
            View Agent Card
          </Button>
        </div>
      </Card>
    </div>
  );

  if (overviewLoading) {
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {[...Array(4)].map((_, i) => (
            <Card key={i}>
              <div className="animate-pulse">
                <div className="h-8 bg-pierre-gray-200 rounded w-16 mx-auto mb-2"></div>
                <div className="h-4 bg-pierre-gray-200 rounded w-24 mx-auto mb-1"></div>
                <div className="h-3 bg-pierre-gray-200 rounded w-16 mx-auto"></div>
              </div>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-pierre-gray-900">A2A Protocol Management</h1>
          <p className="text-pierre-gray-600">Manage Agent-to-Agent communications and monitor usage</p>
        </div>
        
        {/* Navigation */}
        <div className="flex gap-2">
          <Button
            variant={activeView === 'overview' ? 'primary' : 'secondary'}
            onClick={() => setActiveView('overview')}
          >
            Overview
          </Button>
          <Button
            variant={activeView === 'clients' ? 'primary' : 'secondary'}
            onClick={() => setActiveView('clients')}
          >
            Clients
          </Button>
          <Button
            variant={activeView === 'create' ? 'primary' : 'secondary'}
            onClick={() => setActiveView('create')}
          >
            Register Client
          </Button>
        </div>
      </div>

      {/* Content */}
      {activeView === 'overview' && renderOverview()}
      
      {activeView === 'clients' && (
        <A2AClientList onCreateClient={() => setActiveView('create')} />
      )}
      
      {activeView === 'create' && (
        <CreateA2AClient
          onSuccess={() => setActiveView('clients')}
          onCancel={() => setActiveView('overview')}
        />
      )}
    </div>
  );
}