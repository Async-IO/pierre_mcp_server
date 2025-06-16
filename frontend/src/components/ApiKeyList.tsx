import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { ApiKeysResponse, ApiKey } from '../types/api';
import RequestMonitor from './RequestMonitor';
import { Button, Card, CardHeader, Badge } from './ui';

export default function ApiKeyList() {
  const queryClient = useQueryClient();
  const [selectedKeyId, setSelectedKeyId] = useState<string | null>(null);
  const [showRequestMonitor, setShowRequestMonitor] = useState(false);

  const { data: apiKeys, isLoading } = useQuery<ApiKeysResponse>({
    queryKey: ['api-keys'],
    queryFn: () => apiService.getApiKeys(),
  });

  const deactivateMutation = useMutation({
    mutationFn: (keyId: string) => apiService.deactivateApiKey(keyId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard-overview'] });
    },
  });

  const handleDeactivate = (keyId: string, keyName: string) => {
    if (window.confirm(`Are you sure you want to deactivate "${keyName}"? This action cannot be undone.`)) {
      deactivateMutation.mutate(keyId);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    // You could add a toast notification here
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="pierre-spinner"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader 
          title="Your API Keys" 
          subtitle={`${apiKeys?.api_keys?.length || 0} total keys`}
        />

        {!apiKeys?.api_keys?.length ? (
          <div className="text-center py-8 text-pierre-gray-500">
            <div className="text-4xl mb-4">🔑</div>
            <p className="text-lg mb-2">No API keys yet</p>
            <p>Create your first API key to get started</p>
          </div>
        ) : (
          <div className="space-y-4">
            {apiKeys.api_keys.map((key: ApiKey) => (
              <div key={key.id} className="border border-pierre-gray-200 rounded-lg p-4">
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="font-medium text-lg">{key.name}</h3>
                      <Badge variant={key.is_active ? 'success' : 'error'}>
                        {key.is_active ? 'Active' : 'Inactive'}
                      </Badge>
                      <Badge variant={key.tier as any}>
                        {key.tier}
                      </Badge>
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
                        loading={deactivateMutation.isPending}
                        onClick={() => handleDeactivate(key.id, key.name)}
                      >
                        {deactivateMutation.isPending ? 'Deactivating...' : 'Deactivate'}
                      </Button>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Request Monitor Modal/Overlay */}
      {showRequestMonitor && (
        <div className="mt-8">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-medium">
              Request Monitor {selectedKeyId && `- ${apiKeys?.api_keys.find(k => k.id === selectedKeyId)?.name}`}
            </h3>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowRequestMonitor(false)}
            >
              Close Monitor
            </Button>
          </div>
          <RequestMonitor apiKeyId={selectedKeyId || undefined} />
        </div>
      )}
    </div>
  );
}