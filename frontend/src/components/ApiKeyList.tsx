import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { ApiKeysResponse, ApiKey } from '../types/api';
import RequestMonitor from './RequestMonitor';
import { Button, Card, CardHeader, Badge, StatusFilter, ConfirmDialog } from './ui';
import type { StatusFilterValue } from './ui';

export default function ApiKeyList() {
  const queryClient = useQueryClient();
  const [selectedKeyId, setSelectedKeyId] = useState<string | null>(null);
  const [showRequestMonitor, setShowRequestMonitor] = useState(false);
  const [statusFilter, setStatusFilter] = useState<StatusFilterValue>('active');
  const [keyToDeactivate, setKeyToDeactivate] = useState<ApiKey | null>(null);

  const { data: apiKeys, isLoading } = useQuery<ApiKeysResponse>({
    queryKey: ['api-keys'],
    queryFn: () => apiService.getApiKeys(),
  });

  const deactivateMutation = useMutation({
    mutationFn: (keyId: string) => apiService.deactivateApiKey(keyId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard-overview'] });
      setKeyToDeactivate(null);
    },
  });

  const allKeys = useMemo(() => apiKeys?.api_keys || [], [apiKeys?.api_keys]);

  // Compute counts for the filter
  const activeCount = useMemo(() => allKeys.filter(k => k.is_active).length, [allKeys]);
  const inactiveCount = useMemo(() => allKeys.filter(k => !k.is_active).length, [allKeys]);

  // Filter keys based on status filter
  const filteredKeys = useMemo(() => {
    switch (statusFilter) {
      case 'active':
        return allKeys.filter(k => k.is_active);
      case 'inactive':
        return allKeys.filter(k => !k.is_active);
      case 'all':
      default:
        return allKeys;
    }
  }, [allKeys, statusFilter]);

  const handleDeactivate = (key: ApiKey) => {
    setKeyToDeactivate(key);
  };

  const confirmDeactivate = () => {
    if (keyToDeactivate) {
      deactivateMutation.mutate(keyToDeactivate.id);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    // You could add a toast notification here
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="pierre-spinner w-8 h-8"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader
          title="Your API Keys"
          subtitle={`${allKeys.length} total keys`}
        />

        {/* Status Filter */}
        <div className="px-6 pb-4">
          <StatusFilter
            value={statusFilter}
            onChange={setStatusFilter}
            activeCount={activeCount}
            inactiveCount={inactiveCount}
            totalCount={allKeys.length}
          />
        </div>

        {!filteredKeys.length ? (
          <div className="text-center py-8 text-pierre-gray-500">
            <div className="text-4xl mb-4">🔑</div>
            <p className="text-lg mb-2">No API keys yet</p>
            <p>Create your first API key to get started</p>
          </div>
        ) : (
          <div className="space-y-4 px-6 pb-6">
            {filteredKeys.map((key: ApiKey) => (
              <Card key={key.id} className="hover:shadow-md transition-shadow p-4">
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="font-medium text-lg">{key.name}</h3>
                      <Badge variant={key.is_active ? 'success' : 'error'}>
                        {key.is_active ? 'Active' : 'Inactive'}
                      </Badge>
                      {key.rate_limit_requests > 0 && (
                        <Badge variant="info">
                          {key.rate_limit_requests.toLocaleString()} req/month
                        </Badge>
                      )}
                      {key.rate_limit_requests === 0 && (
                        <Badge variant="enterprise">
                          Unlimited
                        </Badge>
                      )}
                    </div>
                    
                    {key.description && (
                      <p className="text-pierre-gray-600 text-sm mb-2">{key.description}</p>
                    )}
                    
                    <div className="grid grid-cols-2 gap-4 text-sm text-pierre-gray-600">
                      <div>
                        <span className="font-medium">Key Prefix:</span>
                        <div className="font-mono bg-pierre-gray-100 px-2 py-1 rounded mt-1">
                          {key.key_prefix}****
                          <button
                            onClick={() => copyToClipboard(key.key_prefix)}
                            className="ml-2 text-pierre-blue-600 hover:text-pierre-blue-700"
                            title="Copy prefix"
                          >
                            📋
                          </button>
                        </div>
                      </div>
                      <div>
                        <span className="font-medium">Created:</span>
                        <div className="mt-1">
                          {new Date(key.created_at).toLocaleDateString()}
                        </div>
                      </div>
                      <div>
                        <span className="font-medium">Last Used:</span>
                        <div className="mt-1">
                          {key.last_used_at 
                            ? new Date(key.last_used_at).toLocaleDateString()
                            : 'Never'
                          }
                        </div>
                      </div>
                      <div>
                        <span className="font-medium">Expires:</span>
                        <div className="mt-1">
                          {key.expires_at 
                            ? new Date(key.expires_at).toLocaleDateString()
                            : 'Never'
                          }
                        </div>
                      </div>
                    </div>
                  </div>
                  
                  <div className="flex gap-2 ml-4">
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => {
                        setSelectedKeyId(key.id);
                        setShowRequestMonitor(true);
                      }}
                    >
                      View Usage
                    </Button>
                    {key.is_active && (
                      <Button
                        variant="danger"
                        size="sm"
                        disabled={deactivateMutation.isPending}
                        onClick={() => handleDeactivate(key)}
                      >
                        Deactivate
                      </Button>
                    )}
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </Card>

      {/* Request Monitor Modal/Overlay */}
      {showRequestMonitor && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl max-w-4xl w-full m-4 max-h-[90vh] overflow-hidden flex flex-col">
            <div className="flex justify-between items-center p-4 border-b border-pierre-gray-200">
              <h3 className="text-lg font-semibold text-pierre-gray-900">
                API Key Usage {selectedKeyId && `- ${allKeys.find(k => k.id === selectedKeyId)?.name}`}
              </h3>
              <button
                onClick={() => setShowRequestMonitor(false)}
                className="text-pierre-gray-400 hover:text-pierre-gray-600"
              >
                <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
            <div className="p-4 overflow-y-auto flex-1">
              <RequestMonitor apiKeyId={selectedKeyId || undefined} />
            </div>
          </div>
        </div>
      )}

      {/* Deactivate Confirmation */}
      <ConfirmDialog
        isOpen={keyToDeactivate !== null}
        onClose={() => setKeyToDeactivate(null)}
        onConfirm={confirmDeactivate}
        title="Deactivate API Key"
        message={`Are you sure you want to deactivate "${keyToDeactivate?.name}"? This action cannot be undone and any applications using this key will lose access.`}
        confirmLabel="Deactivate"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={deactivateMutation.isPending}
      />
    </div>
  );
}