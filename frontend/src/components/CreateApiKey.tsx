import React, { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { CreateApiKeyRequest } from '../types/api';

export default function CreateApiKey() {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [tier, setTier] = useState<'starter' | 'professional' | 'enterprise'>('professional');
  const [expiresInDays, setExpiresInDays] = useState<number | ''>('');
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  
  const queryClient = useQueryClient();

  const createMutation = useMutation({
    mutationFn: (data: CreateApiKeyRequest) => apiService.createApiKey(data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard-overview'] });
      setCreatedKey(data.api_key);
      // Reset form
      setName('');
      setDescription('');
      setTier('professional');
      setExpiresInDays('');
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    const data = {
      name,
      description: description || undefined,
      tier,
      expires_in_days: expiresInDays ? Number(expiresInDays) : undefined,
    };
    
    createMutation.mutate(data);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const tierInfo = {
    starter: {
      limit: '10,000 requests/month',
      description: 'Perfect for development and small projects',
      color: 'text-blue-600',
    },
    professional: {
      limit: '100,000 requests/month',
      description: 'Ideal for production applications',
      color: 'text-green-600',
    },
    enterprise: {
      limit: 'Unlimited requests',
      description: 'For high-volume enterprise applications',
      color: 'text-purple-600',
    },
  };

  return (
    <div className="space-y-6">
      {createdKey && (
        <div className="card bg-green-50 border-green-200">
          <div className="flex items-center gap-3 mb-4">
            <div className="text-green-600 text-2xl">✅</div>
            <h3 className="text-lg font-semibold text-green-800">API Key Created Successfully!</h3>
          </div>
          
          <div className="bg-white border rounded-lg p-4">
            <p className="text-sm text-gray-600 mb-2">
              <strong>Important:</strong> Copy this API key now. You won't be able to see it again.
            </p>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-gray-100 px-3 py-2 rounded font-mono text-sm">
                {createdKey}
              </code>
              <button
                onClick={() => copyToClipboard(createdKey)}
                className="btn-primary px-3 py-2 text-sm"
              >
                Copy
              </button>
            </div>
          </div>
          
          <button
            onClick={() => setCreatedKey(null)}
            className="mt-4 text-green-600 hover:text-green-700 text-sm"
          >
            Dismiss
          </button>
        </div>
      )}

      <div className="card">
        <h2 className="text-xl font-semibold mb-6">Create New API Key</h2>
        
        <form onSubmit={handleSubmit} className="space-y-6">
          <div>
            <label htmlFor="name" className="block text-sm font-medium text-gray-700 mb-1">
              API Key Name *
            </label>
            <input
              type="text"
              id="name"
              required
              className="input-field"
              placeholder="e.g., Production API Key"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <p className="text-xs text-gray-500 mt-1">
              Choose a descriptive name to help you identify this key
            </p>
          </div>

          <div>
            <label htmlFor="description" className="block text-sm font-medium text-gray-700 mb-1">
              Description
            </label>
            <textarea
              id="description"
              rows={3}
              className="input-field"
              placeholder="Optional description of what this key will be used for"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              API Key Tier *
            </label>
            <div className="space-y-3">
              {Object.entries(tierInfo).map(([tierKey, info]) => (
                <label key={tierKey} className="flex items-start gap-3 cursor-pointer">
                  <input
                    type="radio"
                    name="tier"
                    value={tierKey}
                    checked={tier === tierKey}
                    onChange={(e) => setTier(e.target.value as 'starter' | 'professional' | 'enterprise')}
                    className="mt-1"
                  />
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-medium capitalize">{tierKey}</span>
                      <span className={`text-sm font-medium ${info.color}`}>
                        {info.limit}
                      </span>
                    </div>
                    <p className="text-sm text-gray-600">{info.description}</p>
                  </div>
                </label>
              ))}
            </div>
          </div>

          <div>
            <label htmlFor="expires" className="block text-sm font-medium text-gray-700 mb-1">
              Expires In (Days)
            </label>
            <input
              type="number"
              id="expires"
              min="1"
              max="365"
              className="input-field"
              placeholder="Leave empty for no expiration"
              value={expiresInDays}
              onChange={(e) => setExpiresInDays(e.target.value ? Number(e.target.value) : '')}
            />
            <p className="text-xs text-gray-500 mt-1">
              Optional. Set an expiration date for enhanced security.
            </p>
          </div>

          <div className="flex gap-4">
            <button
              type="submit"
              disabled={createMutation.isPending || !name.trim()}
              className="btn-primary px-6 py-2 disabled:opacity-50"
            >
              {createMutation.isPending ? 'Creating...' : 'Create API Key'}
            </button>
            
            <button
              type="button"
              onClick={() => {
                setName('');
                setDescription('');
                setTier('professional');
                setExpiresInDays('');
              }}
              className="btn-secondary px-6 py-2"
            >
              Reset
            </button>
          </div>

          {createMutation.isError && (
            <div className="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded">
              {createMutation.error?.message || 'Failed to create API key'}
            </div>
          )}
        </form>
      </div>

      <div className="card bg-blue-50 border-blue-200">
        <h3 className="font-semibold text-blue-800 mb-3">💡 Best Practices</h3>
        <ul className="text-sm text-blue-700 space-y-2">
          <li>• Store API keys securely in environment variables</li>
          <li>• Use different keys for development and production</li>
          <li>• Set expiration dates for enhanced security</li>
          <li>• Monitor usage regularly to detect anomalies</li>
          <li>• Deactivate unused keys to minimize risk</li>
        </ul>
      </div>
    </div>
  );
}