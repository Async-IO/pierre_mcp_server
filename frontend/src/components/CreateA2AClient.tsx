// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { a2aApi } from '../services/api';
import type { A2AClientRegistrationRequest, A2AClientCredentials } from '../types/api';
import { Button, Card } from './ui';
import { QUERY_KEYS } from '../constants/queryKeys';

interface CreateA2AClientProps {
  onSuccess?: () => void;
  onCancel?: () => void;
}

const AVAILABLE_CAPABILITIES = [
  {
    id: 'fitness-data-analysis',
    name: 'Fitness Data Analysis',
    description: 'Access to fitness data and performance analytics'
  },
  {
    id: 'activity-intelligence',
    name: 'Activity Intelligence',
    description: 'AI-powered activity analysis and insights'
  },
  {
    id: 'goal-management',
    name: 'Goal Management',
    description: 'Goal setting and tracking capabilities'
  },
  {
    id: 'performance-prediction',
    name: 'Performance Prediction',
    description: 'Predictive analytics for performance forecasting'
  },
  {
    id: 'training-analytics',
    name: 'Training Analytics',
    description: 'Training plan analysis and optimization'
  },
  {
    id: 'provider-integration',
    name: 'Provider Integration',
    description: 'Multi-provider data access and synchronization'
  }
];

export default function CreateA2AClient({ onSuccess, onCancel }: CreateA2AClientProps) {
  const [formData, setFormData] = useState<A2AClientRegistrationRequest>({
    name: '',
    description: '',
    capabilities: [],
    redirect_uris: [],
    contact_email: '',
    agent_version: '',
    documentation_url: ''
  });
  
  const [redirectUri, setRedirectUri] = useState('');
  const [showCredentials, setShowCredentials] = useState(false);
  const [credentials, setCredentials] = useState<A2AClientCredentials | null>(null);
  const [validationError, setValidationError] = useState<string | null>(null);
  
  const queryClient = useQueryClient();

  const createMutation = useMutation({
    mutationFn: (data: A2AClientRegistrationRequest) => a2aApi.registerA2AClient(data),
    onSuccess: (response: A2AClientCredentials) => {
      setCredentials(response);
      setShowCredentials(true);
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.a2a.clients() });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    // Clear previous validation error
    setValidationError(null);
    
    if (formData.capabilities.length === 0) {
      setValidationError('Please select at least one capability.');
      return;
    }

    createMutation.mutate(formData);
  };

  const handleCapabilityToggle = (capabilityId: string) => {
    setFormData(prev => ({
      ...prev,
      capabilities: prev.capabilities.includes(capabilityId)
        ? prev.capabilities.filter(id => id !== capabilityId)
        : [...prev.capabilities, capabilityId]
    }));
  };

  const handleAddRedirectUri = () => {
    if (redirectUri.trim() && !formData.redirect_uris?.includes(redirectUri.trim())) {
      setFormData(prev => ({
        ...prev,
        redirect_uris: [...(prev.redirect_uris || []), redirectUri.trim()]
      }));
      setRedirectUri('');
    }
  };

  const handleRemoveRedirectUri = (uri: string) => {
    setFormData(prev => ({
      ...prev,
      redirect_uris: prev.redirect_uris?.filter(u => u !== uri) || []
    }));
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    // You might want to show a toast notification here
  };

  const handleDone = () => {
    setShowCredentials(false);
    setCredentials(null);
    setFormData({
      name: '',
      description: '',
      capabilities: [],
      redirect_uris: [],
      contact_email: '',
      agent_version: '',
      documentation_url: ''
    });
    onSuccess?.();
  };

  if (showCredentials && credentials) {
    return (
      <Card variant="dark">
        <div className="text-center">
          <div className="text-6xl mb-4">🎉</div>
          <h2 className="text-2xl font-bold text-white mb-2">A2A Client Created!</h2>
          <p className="text-zinc-400 mb-6">
            Your A2A client has been successfully registered. Save these credentials securely - they won't be shown again.
          </p>
        </div>

        <div className="bg-pierre-nutrition/15 border border-pierre-nutrition/30 rounded-lg p-4 mb-6">
          <div className="flex items-center mb-2">
            <span className="text-pierre-nutrition mr-2">⚠️</span>
            <h3 className="text-sm font-medium text-pierre-nutrition">Important Security Notice</h3>
          </div>
          <p className="text-sm text-zinc-300">
            Store these credentials securely. The client secret and API key will not be displayed again for security reasons.
          </p>
        </div>

        <div className="space-y-4">
          {/* Client ID */}
          <div>
            <label className="block text-sm font-medium text-zinc-300 mb-2">
              Client ID
            </label>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-white/10 p-3 rounded font-mono text-sm break-all text-zinc-300 border border-white/10">
                {credentials.client_id}
              </code>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => copyToClipboard(credentials.client_id)}
              >
                Copy
              </Button>
            </div>
          </div>

          {/* Client Secret */}
          <div>
            <label className="block text-sm font-medium text-zinc-300 mb-2">
              Client Secret
            </label>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-white/10 p-3 rounded font-mono text-sm break-all text-zinc-300 border border-white/10">
                {credentials.client_secret}
              </code>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => copyToClipboard(credentials.client_secret)}
              >
                Copy
              </Button>
            </div>
          </div>

          {/* API Key */}
          <div>
            <label className="block text-sm font-medium text-zinc-300 mb-2">
              API Key
            </label>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-white/10 p-3 rounded font-mono text-sm break-all text-zinc-300 border border-white/10">
                {credentials.api_key}
              </code>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => copyToClipboard(credentials.api_key)}
              >
                Copy
              </Button>
            </div>
          </div>
        </div>

        <div className="mt-8 text-center">
          <Button onClick={handleDone}>
            Done
          </Button>
        </div>
      </Card>
    );
  }

  return (
    <Card variant="dark">
      <form onSubmit={handleSubmit} className="space-y-6">
        <div>
          <h2 className="text-xl font-semibold text-white mb-2">Register A2A Client</h2>
          <p className="text-zinc-400">
            Create a new Agent-to-Agent protocol client for AI agent communication.
          </p>
        </div>

        {/* Basic Information */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label htmlFor="name" className="block text-sm font-medium text-zinc-300 mb-2">
              Client Name *
            </label>
            <input
              type="text"
              id="name"
              value={formData.name}
              onChange={(e) => setFormData(prev => ({ ...prev, name: e.target.value }))}
              className="input-dark"
              placeholder="My AI Agent"
              required
            />
          </div>

          <div>
            <label htmlFor="contact_email" className="block text-sm font-medium text-zinc-300 mb-2">
              Contact Email *
            </label>
            <input
              type="email"
              id="contact_email"
              value={formData.contact_email}
              onChange={(e) => setFormData(prev => ({ ...prev, contact_email: e.target.value }))}
              className="input-dark"
              placeholder="contact@example.com"
              required
            />
          </div>
        </div>

        <div>
          <label htmlFor="description" className="block text-sm font-medium text-zinc-300 mb-2">
            Description *
          </label>
          <textarea
            id="description"
            value={formData.description}
            onChange={(e) => setFormData(prev => ({ ...prev, description: e.target.value }))}
            rows={3}
            className="input-dark"
            placeholder="Describe what your AI agent does..."
            required
          />
        </div>

        {/* Optional Fields */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label htmlFor="agent_version" className="block text-sm font-medium text-zinc-300 mb-2">
              Agent Version
            </label>
            <input
              type="text"
              id="agent_version"
              value={formData.agent_version}
              onChange={(e) => setFormData(prev => ({ ...prev, agent_version: e.target.value }))}
              className="input-dark"
              placeholder="1.0.0"
            />
          </div>

          <div>
            <label htmlFor="documentation_url" className="block text-sm font-medium text-zinc-300 mb-2">
              Documentation URL
            </label>
            <input
              type="url"
              id="documentation_url"
              value={formData.documentation_url}
              onChange={(e) => setFormData(prev => ({ ...prev, documentation_url: e.target.value }))}
              className="input-dark"
              placeholder="https://docs.example.com"
            />
          </div>
        </div>

        {/* Capabilities */}
        <div>
          <label className="block text-sm font-medium text-zinc-300 mb-2">
            Capabilities * (Select at least one)
          </label>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {AVAILABLE_CAPABILITIES.map((capability) => (
              <div
                key={capability.id}
                className={`border rounded-lg p-3 cursor-pointer transition-colors ${
                  formData.capabilities.includes(capability.id)
                    ? 'border-pierre-violet bg-pierre-violet/10'
                    : 'border-white/10 hover:border-white/20'
                }`}
                onClick={() => handleCapabilityToggle(capability.id)}
              >
                <div className="flex items-center">
                  <input
                    type="checkbox"
                    checked={formData.capabilities.includes(capability.id)}
                    onChange={() => handleCapabilityToggle(capability.id)}
                    className="mr-3 rounded border-white/20 bg-white/10 text-pierre-violet focus:ring-pierre-violet"
                    onClick={(e) => e.stopPropagation()}
                  />
                  <div>
                    <h4 className="font-medium text-white">{capability.name}</h4>
                    <p className="text-sm text-zinc-400">{capability.description}</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Redirect URIs */}
        <div>
          <label className="block text-sm font-medium text-zinc-300 mb-2">
            Redirect URIs (Optional)
          </label>
          <div className="space-y-2">
            <div className="flex gap-2">
              <input
                type="url"
                value={redirectUri}
                onChange={(e) => setRedirectUri(e.target.value)}
                className="input-dark flex-1"
                placeholder="https://example.com/callback"
              />
              <Button
                type="button"
                variant="secondary"
                onClick={handleAddRedirectUri}
                disabled={!redirectUri.trim()}
              >
                Add
              </Button>
            </div>
            {formData.redirect_uris && formData.redirect_uris.length > 0 && (
              <div className="space-y-1">
                {formData.redirect_uris.map((uri) => (
                  <div key={uri} className="flex items-center justify-between bg-white/5 p-2 rounded border border-white/10">
                    <code className="text-sm text-zinc-300">{uri}</code>
                    <Button
                      type="button"
                      variant="danger"
                      size="sm"
                      onClick={() => handleRemoveRedirectUri(uri)}
                    >
                      Remove
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Error Display */}
        {createMutation.error && (
          <div className="bg-pierre-red-500/15 border border-pierre-red-500/30 rounded-lg p-4">
            <div className="flex items-center">
              <span className="text-pierre-red-400 mr-2">❌</span>
              <div>
                <h3 className="text-sm font-medium text-pierre-red-400">Registration Failed</h3>
                <p className="text-sm text-zinc-300 mt-1">
                  {createMutation.error instanceof Error
                    ? createMutation.error.message
                    : 'An error occurred while registering the A2A client.'}
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Validation Error */}
        {validationError && (
          <div className="bg-pierre-red-500/15 border border-pierre-red-500/30 text-pierre-red-400 px-4 py-3 rounded">
            {validationError}
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-3 pt-4">
          <Button
            type="submit"
            disabled={createMutation.isPending || formData.capabilities.length === 0}
          >
            {createMutation.isPending ? 'Creating...' : 'Register Client'}
          </Button>
          <Button
            type="button"
            variant="secondary"
            onClick={onCancel}
            disabled={createMutation.isPending}
          >
            Cancel
          </Button>
        </div>
      </form>
    </Card>
  );
}