import React, { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { CreateApiKeyRequest } from '../types/api';
import { Button, Card, Badge } from './ui';

export default function CreateApiKey() {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [tier, setTier] = useState<'starter' | 'professional' | 'enterprise'>('professional');
  const [expiresInDays, setExpiresInDays] = useState<number | ''>('');
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [keyType, setKeyType] = useState<'regular' | 'trial'>('regular');
  
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

  const createTrialMutation = useMutation({
    mutationFn: (data: { name: string; description?: string }) => apiService.createTrialKey(data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard-overview'] });
      setCreatedKey(data.api_key);
      // Reset form
      setName('');
      setDescription('');
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (keyType === 'trial') {
      const trialData = {
        name,
        description: description || undefined,
      };
      createTrialMutation.mutate(trialData);
    } else {
      const data = {
        name,
        description: description || undefined,
        tier,
        expires_in_days: expiresInDays ? Number(expiresInDays) : undefined,
      };
      createMutation.mutate(data);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const tierInfo = {
    starter: {
      limit: '10,000 requests/month',
      description: 'Perfect for development and small projects',
      color: 'text-pierre-blue-600',
    },
    professional: {
      limit: '100,000 requests/month',
      description: 'Ideal for production applications',
      color: 'text-pierre-green-600',
    },
    enterprise: {
      limit: 'Unlimited requests',
      description: 'For high-volume enterprise applications',
      color: 'text-pierre-purple-600',
    },
  };

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
          {/* Key Type Selection */}
          <div>
            <label className="label">
              Key Type *
            </label>
            <div className="space-y-3">
              <label className="flex items-start gap-3 cursor-pointer">
                <input
                  type="radio"
                  name="keyType"
                  value="regular"
                  checked={keyType === 'regular'}
                  onChange={(e) => setKeyType(e.target.value as 'regular' | 'trial')}
                  className="mt-1"
                />
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">Production Key</span>
                    <Badge variant="info">
                      Full access
                    </Badge>
                  </div>
                  <p className="text-sm text-pierre-gray-600">Choose your tier and configure custom settings</p>
                </div>
              </label>
              
              <label className="flex items-start gap-3 cursor-pointer">
                <input
                  type="radio"
                  name="keyType"
                  value="trial"
                  checked={keyType === 'trial'}
                  onChange={(e) => setKeyType(e.target.value as 'regular' | 'trial')}
                  className="mt-1"
                />
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">Trial Key</span>
                    <Badge variant="success">
                      1,000 requests/month
                    </Badge>
                    <Badge variant="warning">
                      14-day expiry
                    </Badge>
                  </div>
                  <p className="text-sm text-pierre-gray-600">Perfect for testing the platform with automatic expiration</p>
                </div>
              </label>
            </div>
          </div>
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

          {keyType === 'regular' && (
            <div>
              <label className="label">
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
                      <p className="text-sm text-pierre-gray-600">{info.description}</p>
                    </div>
                  </label>
                ))}
              </div>
            </div>
          )}

          {keyType === 'regular' && (
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
          )}

          {keyType === 'trial' && (
            <div className="bg-pierre-yellow-50 border border-pierre-yellow-200 rounded-lg p-4">
              <div className="flex items-center gap-2 mb-2">
                <span className="text-pierre-yellow-600">ℹ️</span>
                <span className="font-medium text-pierre-yellow-800">Trial Key Information</span>
              </div>
              <ul className="text-sm text-pierre-yellow-700 space-y-1">
                <li>• Limited to 1,000 requests per month</li>
                <li>• Automatically expires in 14 days</li>
                <li>• Only one trial key per account</li>
                <li>• Perfect for testing and evaluation</li>
              </ul>
            </div>
          )}

          <div className="flex gap-4">
            <Button
              type="submit"
              variant="primary"
              disabled={createMutation.isPending || createTrialMutation.isPending || !name.trim()}
              loading={createMutation.isPending || createTrialMutation.isPending}
            >
              {keyType === 'trial' ? 'Create Trial Key' : 'Create API Key'}
            </Button>
            
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setName('');
                setDescription('');
                setTier('professional');
                setExpiresInDays('');
              }}
            >
              Reset
            </Button>
          </div>

          {(createMutation.isError || createTrialMutation.isError) && (
            <div className="bg-pierre-red-50 border border-pierre-red-200 text-pierre-red-600 px-4 py-3 rounded">
              {createMutation.error?.message || createTrialMutation.error?.message || 'Failed to create API key'}
            </div>
          )}
        </form>
      </Card>

      <Card className="bg-pierre-blue-50 border-pierre-blue-200">
        <h3 className="font-semibold text-pierre-blue-800 mb-3">💡 Best Practices</h3>
        <ul className="text-sm text-pierre-blue-700 space-y-2">
          <li>• Store API keys securely in environment variables</li>
          <li>• Use different keys for development and production</li>
          <li>• Set expiration dates for enhanced security</li>
          <li>• Monitor usage regularly to detect anomalies</li>
          <li>• Deactivate unused keys to minimize risk</li>
        </ul>
      </Card>
    </div>
  );
}