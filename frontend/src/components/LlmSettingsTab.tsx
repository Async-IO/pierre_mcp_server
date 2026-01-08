// ABOUTME: LLM provider settings tab for user configuration
// ABOUTME: Allows users to configure API keys for Gemini, Groq, and local LLM providers
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card, Button, Input, Badge, ConfirmDialog } from './ui';
import { clsx } from 'clsx';

const PROVIDER_INFO: Record<string, { description: string; docsUrl: string }> = {
  gemini: {
    description: 'Google Gemini API for advanced reasoning and multimodal capabilities',
    docsUrl: 'https://ai.google.dev/docs',
  },
  groq: {
    description: 'Groq cloud for fast inference with Llama, Mixtral, and other open-source models',
    docsUrl: 'https://console.groq.com/docs',
  },
  local: {
    description: 'Local LLM server via OpenAI-compatible API (Ollama, vLLM, LocalAI)',
    docsUrl: 'https://github.com/ollama/ollama',
  },
};

export default function LlmSettingsTab() {
  const queryClient = useQueryClient();
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [defaultModel, setDefaultModel] = useState('');
  const [isValidating, setIsValidating] = useState(false);
  const [validationResult, setValidationResult] = useState<{
    valid: boolean;
    models?: string[];
    error?: string;
  } | null>(null);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [providerToDelete, setProviderToDelete] = useState<string | null>(null);

  // Fetch current LLM settings
  const { data: settings, isLoading } = useQuery({
    queryKey: ['llm-settings'],
    queryFn: () => apiService.getLlmSettings(),
  });

  // Save credentials mutation
  const saveMutation = useMutation({
    mutationFn: (data: {
      provider: string;
      api_key: string;
      base_url?: string;
      default_model?: string;
    }) => apiService.saveLlmCredentials(data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['llm-settings'] });
      setMessage({ type: 'success', text: data.message });
      resetForm();
    },
    onError: (error: Error) => {
      setMessage({ type: 'error', text: error.message || 'Failed to save credentials' });
    },
  });

  // Delete credentials mutation
  const deleteMutation = useMutation({
    mutationFn: (provider: string) => apiService.deleteLlmCredentials(provider),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['llm-settings'] });
      setMessage({ type: 'success', text: data.message });
      setProviderToDelete(null);
    },
    onError: (error: Error) => {
      setMessage({ type: 'error', text: error.message || 'Failed to delete credentials' });
      setProviderToDelete(null);
    },
  });

  const resetForm = () => {
    setSelectedProvider(null);
    setApiKey('');
    setBaseUrl('');
    setDefaultModel('');
    setValidationResult(null);
  };

  const handleValidate = async () => {
    if (!selectedProvider || !apiKey.trim()) return;

    setIsValidating(true);
    setValidationResult(null);

    try {
      const result = await apiService.validateLlmCredentials({
        provider: selectedProvider,
        api_key: apiKey.trim(),
        base_url: selectedProvider === 'local' ? baseUrl.trim() || undefined : undefined,
      });

      setValidationResult({
        valid: result.valid,
        models: result.models || undefined,
        error: result.error || undefined,
      });
    } catch (error) {
      setValidationResult({
        valid: false,
        error: error instanceof Error ? error.message : 'Validation failed',
      });
    } finally {
      setIsValidating(false);
    }
  };

  const handleSave = () => {
    if (!selectedProvider || !apiKey.trim()) return;

    saveMutation.mutate({
      provider: selectedProvider,
      api_key: apiKey.trim(),
      base_url: selectedProvider === 'local' ? baseUrl.trim() || undefined : undefined,
      default_model: defaultModel.trim() || undefined,
    });
  };

  const getSourceBadge = (source: string | null) => {
    switch (source) {
      case 'user_specific':
        return <Badge variant="info">Your Key</Badge>;
      case 'tenant_default':
        return <Badge variant="secondary">Organization</Badge>;
      case 'environment':
        return <Badge variant="warning">System</Badge>;
      default:
        return null;
    }
  };

  if (isLoading) {
    return (
      <Card>
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-pierre-gray-200 rounded w-1/3"></div>
          <div className="h-20 bg-pierre-gray-100 rounded"></div>
          <div className="h-20 bg-pierre-gray-100 rounded"></div>
        </div>
      </Card>
    );
  }

  const providers = settings?.providers || [];
  const currentProvider = settings?.current_provider;

  return (
    <>
      {/* Current Status */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">AI Provider Configuration</h2>
        <p className="text-sm text-pierre-gray-600 mb-6">
          Configure your preferred AI provider for chat conversations. You can use your own API keys
          or rely on organization-wide settings.
        </p>

        {currentProvider && (
          <div className="mb-6 p-4 bg-pierre-activity-light/20 border border-pierre-activity/30 rounded-lg">
            <div className="flex items-center gap-2">
              <svg
                className="w-5 h-5 text-pierre-activity"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <span className="text-sm font-medium text-pierre-activity">
                Active Provider:{' '}
                {providers.find((p) => p.name === currentProvider)?.display_name || currentProvider}
              </span>
            </div>
          </div>
        )}

        {/* Provider List */}
        <div className="space-y-4">
          {providers.map((provider) => (
            <div
              key={provider.name}
              className={clsx(
                'p-4 rounded-lg border transition-all',
                provider.has_credentials
                  ? 'border-pierre-activity/30 bg-pierre-activity-light/10'
                  : 'border-pierre-gray-200 bg-white hover:border-pierre-gray-300'
              )}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <h3 className="font-medium text-pierre-gray-900">{provider.display_name}</h3>
                    {provider.has_credentials && getSourceBadge(provider.credential_source)}
                    {provider.name === currentProvider && (
                      <Badge variant="success">Active</Badge>
                    )}
                  </div>
                  <p className="text-sm text-pierre-gray-600">
                    {PROVIDER_INFO[provider.name]?.description}
                  </p>
                  <a
                    href={PROVIDER_INFO[provider.name]?.docsUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm text-pierre-violet hover:underline mt-1 inline-block"
                  >
                    Documentation
                  </a>
                </div>
                <div className="flex gap-2 ml-4">
                  {provider.has_credentials && provider.credential_source === 'user_specific' && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setProviderToDelete(provider.name)}
                    >
                      Remove
                    </Button>
                  )}
                  <Button
                    variant={provider.has_credentials ? 'outline' : 'gradient'}
                    size="sm"
                    onClick={() => {
                      setSelectedProvider(provider.name);
                      setMessage(null);
                      setValidationResult(null);
                    }}
                  >
                    {provider.has_credentials ? 'Update' : 'Configure'}
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      </Card>

      {/* Configuration Form */}
      {selectedProvider && (
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-pierre-gray-900">
              Configure {providers.find((p) => p.name === selectedProvider)?.display_name}
            </h2>
            <button
              onClick={resetForm}
              className="text-pierre-gray-500 hover:text-pierre-gray-700"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          <div className="space-y-4">
            <Input
              label="API Key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={`Enter your ${selectedProvider.toUpperCase()} API key`}
            />

            {selectedProvider === 'local' && (
              <Input
                label="Base URL"
                value={baseUrl}
                onChange={(e) => setBaseUrl(e.target.value)}
                placeholder="http://localhost:11434/v1"
                helpText="The base URL of your local LLM server (defaults to Ollama)"
              />
            )}

            <Input
              label="Default Model (Optional)"
              value={defaultModel}
              onChange={(e) => setDefaultModel(e.target.value)}
              placeholder={
                selectedProvider === 'gemini'
                  ? 'gemini-2.0-flash-exp'
                  : selectedProvider === 'groq'
                    ? 'llama-3.1-70b-versatile'
                    : 'qwen2.5:14b-instruct'
              }
              helpText="Override the default model for this provider"
            />

            {/* Validation Result */}
            {validationResult && (
              <div
                className={clsx(
                  'p-4 rounded-lg',
                  validationResult.valid
                    ? 'bg-pierre-activity-light/30 border border-pierre-activity/30'
                    : 'bg-red-50 border border-red-200'
                )}
              >
                {validationResult.valid ? (
                  <div>
                    <div className="flex items-center gap-2 text-pierre-activity font-medium mb-2">
                      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                        />
                      </svg>
                      API key is valid!
                    </div>
                    {validationResult.models && validationResult.models.length > 0 && (
                      <div className="text-sm text-pierre-gray-600">
                        Available models: {validationResult.models.slice(0, 5).join(', ')}
                        {validationResult.models.length > 5 && ` (+${validationResult.models.length - 5} more)`}
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="flex items-center gap-2 text-red-600">
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    {validationResult.error || 'Invalid API key'}
                  </div>
                )}
              </div>
            )}

            {/* General Message */}
            {message && (
              <div
                className={clsx(
                  'p-3 rounded-lg text-sm',
                  message.type === 'success'
                    ? 'bg-pierre-activity-light/30 text-pierre-activity'
                    : 'bg-red-50 text-red-600'
                )}
              >
                {message.text}
              </div>
            )}

            {/* Action Buttons */}
            <div className="flex gap-3 pt-2">
              <Button
                variant="outline"
                onClick={handleValidate}
                loading={isValidating}
                disabled={!apiKey.trim()}
              >
                Test Connection
              </Button>
              <Button
                variant="gradient"
                onClick={handleSave}
                loading={saveMutation.isPending}
                disabled={!apiKey.trim() || (validationResult !== null && !validationResult.valid)}
              >
                Save API Key
              </Button>
            </div>
          </div>
        </Card>
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        isOpen={providerToDelete !== null}
        onClose={() => setProviderToDelete(null)}
        onConfirm={() => providerToDelete && deleteMutation.mutate(providerToDelete)}
        title="Remove API Key"
        message={`Are you sure you want to remove your ${providerToDelete?.toUpperCase()} API key? You'll fall back to organization or system defaults if available.`}
        confirmLabel="Remove"
        variant="danger"
        isLoading={deleteMutation.isPending}
      />
    </>
  );
}
