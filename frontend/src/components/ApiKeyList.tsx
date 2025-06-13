import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';

export default function ApiKeyList() {
  const queryClient = useQueryClient();

  const { data: apiKeys, isLoading } = useQuery({
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
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-api-blue"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="card">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-xl font-semibold">Your API Keys</h2>
          <span className="text-gray-500">
            {apiKeys?.api_keys?.length || 0} total keys
          </span>
        </div>

        {!apiKeys?.api_keys?.length ? (
          <div className="text-center py-8 text-gray-500">
            <div className="text-4xl mb-4">🔑</div>
            <p className="text-lg mb-2">No API keys yet</p>
            <p>Create your first API key to get started</p>
          </div>
        ) : (
          <div className="space-y-4">
            {apiKeys.api_keys.map((key: any) => (
              <div key={key.id} className="border rounded-lg p-4">
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="font-medium text-lg">{key.name}</h3>
                      <span className={`px-2 py-1 rounded text-xs font-medium ${
                        key.is_active 
                          ? 'bg-green-100 text-green-800' 
                          : 'bg-red-100 text-red-800'
                      }`}>
                        {key.is_active ? 'Active' : 'Inactive'}
                      </span>
                      <span className="px-2 py-1 rounded text-xs font-medium bg-blue-100 text-blue-800 capitalize">
                        {key.tier}
                      </span>
                    </div>
                    
                    {key.description && (
                      <p className="text-gray-600 text-sm mb-2">{key.description}</p>
                    )}
                    
                    <div className="grid grid-cols-2 gap-4 text-sm text-gray-600">
                      <div>
                        <span className="font-medium">Key Prefix:</span>
                        <div className="font-mono bg-gray-100 px-2 py-1 rounded mt-1">
                          {key.key_prefix}****
                          <button
                            onClick={() => copyToClipboard(key.key_prefix)}
                            className="ml-2 text-api-blue hover:text-blue-700"
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
                    <button
                      onClick={() => {/* Navigate to usage details */}}
                      className="btn-secondary text-xs px-3 py-1"
                    >
                      View Usage
                    </button>
                    {key.is_active && (
                      <button
                        onClick={() => handleDeactivate(key.id, key.name)}
                        disabled={deactivateMutation.isPending}
                        className="btn-danger text-xs px-3 py-1 disabled:opacity-50"
                      >
                        {deactivateMutation.isPending ? 'Deactivating...' : 'Deactivate'}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}