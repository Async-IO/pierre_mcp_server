import React, { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { CreateApiKeyRequest } from '../types/api';
import { Button, Card } from './ui';

export default function CreateApiKey() {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [rateLimitRequests, setRateLimitRequests] = useState<number>(10000);
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
      setRateLimitRequests(10000);
      setExpiresInDays('');
    },
  });


  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    const data = {
      name,
      description: description || undefined,
      rate_limit_requests: rateLimitRequests,
      expires_in_days: expiresInDays ? Number(expiresInDays) : undefined,
    };
    createMutation.mutate(data);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const rateLimitOptions = [
    { value: 500, label: '500 requests/month', description: 'Light usage' },
    { value: 1000, label: '1,000 requests/month', description: 'Basic usage' },
    { value: 5000, label: '5,000 requests/month', description: 'Regular usage' },
    { value: 10000, label: '10,000 requests/month', description: 'Professional usage' },
    { value: 50000, label: '50,000 requests/month', description: 'High usage' },
    { value: 100000, label: '100,000 requests/month', description: 'Enterprise usage' },
    { value: 0, label: 'Unlimited', description: 'No rate limits' },
  ];

  return (
    <div className="space-y-6">
      {createdKey && (
        <Card className="bg-pierre-green-50 border-pierre-green-200">
          <div className="flex items-center gap-3 mb-4">
            <div className="text-pierre-green-600 text-2xl">✅</div>
            <h3 className="text-lg font-semibold text-pierre-green-800">API Key Created Successfully!</h3>
          </div>
          
          <div className="bg-white border rounded-lg p-4">
            <p className="text-sm text-pierre-gray-600 mb-2">
              <strong>Important:</strong> Copy this API key now. You won't be able to see it again.
            </p>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-pierre-gray-100 px-3 py-2 rounded font-mono text-sm">
                {createdKey}
              </code>
              <Button
                variant="primary"
                size="sm"
                onClick={() => copyToClipboard(createdKey)}
              >
                Copy
              </Button>
            </div>
          </div>
          
          <button
            onClick={() => setCreatedKey(null)}
            className="mt-4 text-pierre-green-600 hover:text-pierre-green-700 text-sm"
          >
            Dismiss
          </button>
        </Card>
      )}

      <Card>
        <h2 className="text-xl font-semibold mb-6">Create New API Key</h2>
        
        <form onSubmit={handleSubmit} className="space-y-6">
          <div>
            <label htmlFor="name" className="label">
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
            <p className="help-text">
              Choose a descriptive name to help you identify this key
            </p>
          </div>

          <div>
            <label htmlFor="description" className="label">
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
            <label className="label">
              Rate Limit *
            </label>
            <select
              value={rateLimitRequests}
              onChange={(e) => setRateLimitRequests(Number(e.target.value))}
              className="input-field"
              required
            >
              {rateLimitOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label} - {option.description}
                </option>
              ))}
            </select>
            <p className="help-text">
              Choose your monthly request limit. Select "Unlimited" for no rate limiting.
            </p>
          </div>

          <div>
            <label htmlFor="expires" className="label">
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
            <p className="help-text">
              Optional. Set an expiration date for enhanced security.
            </p>
          </div>


          <div className="flex gap-4">
            <Button
              type="submit"
              variant="primary"
              disabled={createMutation.isPending || !name.trim()}
              loading={createMutation.isPending}
            >
              Create API Key
            </Button>
            
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setName('');
                setDescription('');
                setRateLimitRequests(10000);
                setExpiresInDays('');
              }}
            >
              Reset
            </Button>
          </div>

          {createMutation.isError && (
            <div className="bg-pierre-red-50 border border-pierre-red-200 text-pierre-red-600 px-4 py-3 rounded">
              {createMutation.error?.message || 'Failed to create API key'}
            </div>
          )}
        </form>
      </Card>

    </div>
  );
}