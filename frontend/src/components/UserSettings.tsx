// ABOUTME: Comprehensive user settings with tabbed navigation
// ABOUTME: Includes Profile, Connections, Tokens, and Account sections
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { format } from 'date-fns';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import { Card, Button, Input, Badge, ConfirmDialog } from './ui';
import { clsx } from 'clsx';
import A2AClientList from './A2AClientList';
import CreateA2AClient from './CreateA2AClient';
import LlmSettingsTab from './LlmSettingsTab';

interface OAuthApp {
  provider: string;
  client_id: string;
  redirect_uri: string;
  created_at: string;
}

interface McpToken {
  id: string;
  name: string;
  token_prefix: string;
  expires_at: string | null;
  last_used_at: string | null;
  usage_count: number;
  is_revoked: boolean;
  created_at: string;
}

const PROVIDERS = [
  { id: 'strava', name: 'Strava', color: 'bg-orange-500' },
  { id: 'fitbit', name: 'Fitbit', color: 'bg-teal-500' },
  { id: 'garmin', name: 'Garmin', color: 'bg-blue-600' },
  { id: 'whoop', name: 'WHOOP', color: 'bg-black' },
  { id: 'terra', name: 'Terra', color: 'bg-green-600' },
];

type SettingsTab = 'profile' | 'connections' | 'tokens' | 'llm' | 'account';

const SETTINGS_TABS: { id: SettingsTab; name: string; icon: React.ReactNode }[] = [
  {
    id: 'profile',
    name: 'Profile',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
      </svg>
    ),
  },
  {
    id: 'connections',
    name: 'Connections',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
      </svg>
    ),
  },
  {
    id: 'tokens',
    name: 'API Tokens',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
      </svg>
    ),
  },
  {
    id: 'llm',
    name: 'AI Settings',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
      </svg>
    ),
  },
  {
    id: 'account',
    name: 'Account',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ),
  },
];

export default function UserSettings() {
  const { user, logout, isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<SettingsTab>('profile');

  // Profile state
  const [displayName, setDisplayName] = useState(user?.display_name || '');
  const [isSaving, setIsSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  // OAuth App state
  const [showAddCredentials, setShowAddCredentials] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState('');
  const [clientId, setClientId] = useState('');
  const [clientSecret, setClientSecret] = useState('');
  const [redirectUri, setRedirectUri] = useState('');
  const [credentialMessage, setCredentialMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [providerToDelete, setProviderToDelete] = useState<string | null>(null);

  // Token state
  const [tokenToRevoke, setTokenToRevoke] = useState<McpToken | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newTokenName, setNewTokenName] = useState('');
  const [expiresInDays, setExpiresInDays] = useState<number | undefined>(undefined);
  const [createdToken, setCreatedToken] = useState<{ token_value: string; name: string } | null>(null);
  const [copied, setCopied] = useState(false);
  const [showCreateA2AClient, setShowCreateA2AClient] = useState(false);
  const [showSetupInstructions, setShowSetupInstructions] = useState(false);

  // Fetch OAuth apps
  const { data: oauthAppsResponse, isLoading: isLoadingApps } = useQuery({
    queryKey: ['user-oauth-apps'],
    queryFn: () => apiService.getUserOAuthApps(),
  });

  // Fetch user stats
  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ['userStats'],
    queryFn: () => apiService.getUserStats(),
    staleTime: 30000,
  });

  // Fetch MCP tokens
  const { data: tokensResponse, isLoading: tokensLoading } = useQuery({
    queryKey: ['mcp-tokens'],
    queryFn: () => apiService.getMcpTokens(),
    enabled: isAuthenticated,
  });

  const oauthApps: OAuthApp[] = oauthAppsResponse?.apps || [];
  const tokens: McpToken[] = tokensResponse?.tokens || [];
  const activeTokens = tokens.filter((t) => !t.is_revoked);

  // Register OAuth app mutation
  const registerMutation = useMutation({
    mutationFn: (data: { provider: string; client_id: string; client_secret: string; redirect_uri: string }) =>
      apiService.registerUserOAuthApp(data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['user-oauth-apps'] });
      setCredentialMessage({ type: 'success', text: data.message });
      setShowAddCredentials(false);
      setSelectedProvider('');
      setClientId('');
      setClientSecret('');
      setRedirectUri('');
    },
    onError: (error: Error) => {
      setCredentialMessage({ type: 'error', text: error.message || 'Failed to save credentials' });
    },
  });

  // Delete OAuth app mutation
  const deleteMutation = useMutation({
    mutationFn: (provider: string) => apiService.deleteUserOAuthApp(provider),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-oauth-apps'] });
      setCredentialMessage({ type: 'success', text: 'Provider credentials removed' });
      setProviderToDelete(null);
    },
    onError: (error: Error) => {
      setCredentialMessage({ type: 'error', text: error.message || 'Failed to remove credentials' });
      setProviderToDelete(null);
    },
  });

  // Profile update mutation
  const profileMutation = useMutation({
    mutationFn: (data: { display_name: string }) => apiService.updateProfile(data),
    onSuccess: (response) => {
      setMessage({ type: 'success', text: response.message });
      apiService.setUser(response.user);
      queryClient.invalidateQueries({ queryKey: ['user'] });
    },
    onError: (error: Error) => {
      setMessage({ type: 'error', text: error.message || 'Failed to update profile' });
    },
    onSettled: () => {
      setIsSaving(false);
    },
  });

  // Token mutations
  const createTokenMutation = useMutation({
    mutationFn: (data: { name: string; expires_in_days?: number }) => apiService.createMcpToken(data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['mcp-tokens'] });
      setCreatedToken({ token_value: data.token_value, name: data.name });
      setShowCreateForm(false);
      setNewTokenName('');
      setExpiresInDays(undefined);
    },
  });

  const revokeTokenMutation = useMutation({
    mutationFn: (tokenId: string) => apiService.revokeMcpToken(tokenId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mcp-tokens'] });
      setTokenToRevoke(null);
    },
  });

  const handleSaveProfile = async () => {
    setIsSaving(true);
    setMessage(null);
    profileMutation.mutate({ display_name: displayName.trim() });
  };

  const handleAddCredentials = () => {
    if (!selectedProvider || !clientId.trim() || !clientSecret.trim() || !redirectUri.trim()) {
      setCredentialMessage({ type: 'error', text: 'All fields are required' });
      return;
    }
    registerMutation.mutate({
      provider: selectedProvider,
      client_id: clientId.trim(),
      client_secret: clientSecret.trim(),
      redirect_uri: redirectUri.trim(),
    });
  };

  const handleCreateToken = () => {
    if (!newTokenName.trim()) return;
    createTokenMutation.mutate({
      name: newTokenName.trim(),
      expires_in_days: expiresInDays,
    });
  };

  const copyToClipboard = async (text: string) => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const getProviderInfo = (providerId: string) => {
    return PROVIDERS.find((p) => p.id === providerId) || { id: providerId, name: providerId, color: 'bg-gray-500' };
  };

  const configuredProviders = oauthApps.map((app) => app.provider);
  const availableProviders = PROVIDERS.filter((p) => !configuredProviders.includes(p.id));

  return (
    <div className="flex gap-6">
      {/* Settings Navigation Sidebar */}
      <div className="w-56 flex-shrink-0">
        <Card className="sticky top-6">
          <nav className="space-y-1">
            {SETTINGS_TABS.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={clsx(
                  'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all',
                  activeTab === tab.id
                    ? 'bg-gradient-to-r from-pierre-violet/10 to-pierre-cyan/5 text-pierre-violet'
                    : 'text-pierre-gray-600 hover:bg-pierre-gray-50 hover:text-pierre-violet'
                )}
              >
                {tab.icon}
                {tab.name}
              </button>
            ))}
          </nav>
        </Card>
      </div>

      {/* Settings Content */}
      <div className="flex-1 space-y-6">
        {/* Profile Tab */}
        {activeTab === 'profile' && (
          <>
            <Card>
              <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Profile Information</h2>
              <div className="space-y-4">
                <div className="flex items-center gap-4 pb-4 border-b border-pierre-gray-100">
                  <div className="w-16 h-16 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-full flex items-center justify-center flex-shrink-0">
                    <span className="text-2xl font-bold text-white">
                      {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
                    </span>
                  </div>
                  <div>
                    <p className="font-medium text-pierre-gray-900">{user?.display_name || 'No name set'}</p>
                    <p className="text-sm text-pierre-gray-500">{user?.email}</p>
                  </div>
                </div>

                <div>
                  <Input
                    label="Display Name"
                    value={displayName}
                    onChange={(e) => setDisplayName(e.target.value)}
                    placeholder="Enter your display name"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">Email</label>
                  <p className="text-pierre-gray-900 bg-pierre-gray-50 px-3 py-2 rounded-lg">{user?.email}</p>
                  <p className="text-xs text-pierre-gray-500 mt-1">Email cannot be changed</p>
                </div>

                {message && (
                  <div
                    className={`p-3 rounded-lg text-sm ${
                      message.type === 'success'
                        ? 'bg-pierre-activity-light/30 text-pierre-activity'
                        : 'bg-red-50 text-red-600'
                    }`}
                  >
                    {message.text}
                  </div>
                )}

                <Button
                  variant="gradient"
                  onClick={handleSaveProfile}
                  loading={isSaving}
                  disabled={displayName === user?.display_name}
                >
                  Save Changes
                </Button>
              </div>
            </Card>

            {/* Quick Stats */}
            <div className="grid grid-cols-2 gap-4">
              <Card>
                <div className="text-center">
                  <div className="text-3xl font-bold text-pierre-violet">
                    {statsLoading ? '...' : (stats?.connected_providers ?? 0)}
                  </div>
                  <div className="text-sm text-pierre-gray-600 mt-1">Connected Providers</div>
                </div>
              </Card>
              <Card>
                <div className="text-center">
                  <div className="text-3xl font-bold text-pierre-nutrition">
                    {statsLoading ? '...' : (stats?.days_active ?? 0)}
                  </div>
                  <div className="text-sm text-pierre-gray-600 mt-1">Days Active</div>
                </div>
              </Card>
            </div>
          </>
        )}

        {/* Connections Tab */}
        {activeTab === 'connections' && (
          <Card>
            <div className="flex justify-between items-center mb-4">
              <div>
                <h2 className="text-lg font-semibold text-pierre-gray-900">Provider Credentials</h2>
                <p className="text-sm text-pierre-gray-500 mt-1">
                  Configure your own OAuth app credentials to avoid rate limits
                </p>
              </div>
              {availableProviders.length > 0 && (
                <Button variant="secondary" size="sm" onClick={() => setShowAddCredentials(true)}>
                  Add Provider
                </Button>
              )}
            </div>

            {credentialMessage && (
              <div
                className={`p-3 rounded-lg text-sm mb-4 ${
                  credentialMessage.type === 'success'
                    ? 'bg-pierre-activity-light/30 text-pierre-activity'
                    : 'bg-red-50 text-red-600'
                }`}
              >
                {credentialMessage.text}
              </div>
            )}

            {isLoadingApps ? (
              <div className="flex justify-center py-6">
                <div className="pierre-spinner w-6 h-6"></div>
              </div>
            ) : oauthApps.length === 0 ? (
              <div className="text-center py-8 bg-pierre-gray-50 rounded-lg">
                <svg
                  className="w-12 h-12 text-pierre-gray-400 mx-auto mb-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={1.5}
                    d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
                  />
                </svg>
                <p className="text-pierre-gray-600 font-medium">No custom credentials configured</p>
                <p className="text-sm text-pierre-gray-500 mt-1">
                  Add your own OAuth app credentials to use your personal API quotas
                </p>
              </div>
            ) : (
              <div className="space-y-3">
                {oauthApps.map((app) => {
                  const provider = getProviderInfo(app.provider);
                  return (
                    <div key={app.provider} className="flex items-center justify-between p-4 bg-pierre-gray-50 rounded-lg">
                      <div className="flex items-center gap-3">
                        <div className={`w-10 h-10 ${provider.color} rounded-lg flex items-center justify-center`}>
                          <span className="text-white font-bold text-sm">{provider.name.charAt(0)}</span>
                        </div>
                        <div>
                          <p className="font-medium text-pierre-gray-900">{provider.name}</p>
                          <p className="text-xs text-pierre-gray-500">Client ID: {app.client_id.substring(0, 8)}...</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant="success">Configured</Badge>
                        <Button variant="danger" size="sm" onClick={() => setProviderToDelete(app.provider)}>
                          Remove
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Add Credentials Form */}
            {showAddCredentials && (
              <div className="mt-4 p-4 border border-pierre-gray-200 rounded-lg bg-white">
                <h3 className="font-medium text-pierre-gray-900 mb-4">Add Provider Credentials</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-pierre-gray-700 mb-1">Provider</label>
                    <select
                      value={selectedProvider}
                      onChange={(e) => setSelectedProvider(e.target.value)}
                      className="w-full px-3 py-2 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                    >
                      <option value="">Select a provider</option>
                      {availableProviders.map((provider) => (
                        <option key={provider.id} value={provider.id}>
                          {provider.name}
                        </option>
                      ))}
                    </select>
                  </div>

                  <Input
                    label="Client ID"
                    value={clientId}
                    onChange={(e) => setClientId(e.target.value)}
                    placeholder="Enter your OAuth client ID"
                  />

                  <Input
                    label="Client Secret"
                    type="password"
                    value={clientSecret}
                    onChange={(e) => setClientSecret(e.target.value)}
                    placeholder="Enter your OAuth client secret"
                  />

                  <Input
                    label="Redirect URI"
                    value={redirectUri}
                    onChange={(e) => setRedirectUri(e.target.value)}
                    placeholder="e.g., http://localhost:8081/api/oauth/callback/strava"
                  />

                  <div className="flex gap-2 justify-end">
                    <Button
                      variant="secondary"
                      onClick={() => {
                        setShowAddCredentials(false);
                        setSelectedProvider('');
                        setClientId('');
                        setClientSecret('');
                        setRedirectUri('');
                        setCredentialMessage(null);
                      }}
                    >
                      Cancel
                    </Button>
                    <Button
                      variant="gradient"
                      onClick={handleAddCredentials}
                      loading={registerMutation.isPending}
                      disabled={!selectedProvider || !clientId || !clientSecret || !redirectUri}
                    >
                      Save Credentials
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </Card>
        )}

        {/* Tokens Tab */}
        {activeTab === 'tokens' && (
          <>
            {/* Created Token Display */}
            {createdToken && (
              <div className="bg-green-50 border border-green-200 rounded-lg p-6">
                <div className="flex items-start gap-3">
                  <svg className="w-6 h-6 text-green-600 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  <div className="flex-1">
                    <h3 className="text-lg font-medium text-green-900">Token Created: {createdToken.name}</h3>
                    <p className="text-green-700 mt-1 mb-3">Copy this token now. You won&apos;t be able to see it again!</p>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 px-3 py-2 bg-white border border-green-300 rounded font-mono text-sm break-all">
                        {createdToken.token_value}
                      </code>
                      <Button onClick={() => copyToClipboard(createdToken.token_value)} variant="secondary" size="sm">
                        {copied ? 'Copied!' : 'Copy'}
                      </Button>
                    </div>
                    <Button onClick={() => setCreatedToken(null)} variant="secondary" size="sm" className="mt-3">
                      Dismiss
                    </Button>
                  </div>
                </div>
              </div>
            )}

            <Card>
              <div className="flex justify-between items-center mb-4">
                <div>
                  <h2 className="text-lg font-semibold text-pierre-gray-900">API Tokens</h2>
                  <p className="text-sm text-pierre-gray-500 mt-1">
                    {activeTokens.length} active tokens for AI client connections
                  </p>
                </div>
              </div>

              {/* Create Token Section */}
              <div className="mb-6">
                {!showCreateForm ? (
                  <Button onClick={() => setShowCreateForm(true)} variant="primary">
                    Create New Token
                  </Button>
                ) : (
                  <div className="bg-pierre-gray-50 rounded-lg p-4 space-y-4">
                    <h4 className="font-medium text-pierre-gray-900">Create Token</h4>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div>
                        <label className="block text-sm font-medium text-pierre-gray-700 mb-1">Token Name</label>
                        <input
                          type="text"
                          value={newTokenName}
                          onChange={(e) => setNewTokenName(e.target.value)}
                          placeholder="e.g., Claude Desktop, Cursor IDE"
                          className="w-full px-3 py-2 border border-pierre-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-pierre-blue-500"
                        />
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-pierre-gray-700 mb-1">Expires In (days)</label>
                        <select
                          value={expiresInDays || ''}
                          onChange={(e) => setExpiresInDays(e.target.value ? Number(e.target.value) : undefined)}
                          className="w-full px-3 py-2 border border-pierre-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-pierre-blue-500"
                        >
                          <option value="">Never expires</option>
                          <option value="30">30 days</option>
                          <option value="90">90 days</option>
                          <option value="180">180 days</option>
                          <option value="365">1 year</option>
                        </select>
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        onClick={handleCreateToken}
                        disabled={!newTokenName.trim() || createTokenMutation.isPending}
                        variant="primary"
                      >
                        {createTokenMutation.isPending ? 'Creating...' : 'Create Token'}
                      </Button>
                      <Button onClick={() => setShowCreateForm(false)} variant="secondary">
                        Cancel
                      </Button>
                    </div>
                  </div>
                )}
              </div>

              {/* Token List */}
              {tokensLoading ? (
                <div className="flex justify-center py-8">
                  <div className="pierre-spinner w-8 h-8"></div>
                </div>
              ) : tokens.length === 0 ? (
                <div className="text-center py-8 text-pierre-gray-500">
                  <div className="text-4xl mb-4">🔑</div>
                  <p className="text-lg mb-2">No tokens yet</p>
                  <p>Create a token to connect AI clients like Claude Desktop or Cursor to Pierre</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {tokens.map((token) => (
                    <div key={token.id} className="p-4 bg-pierre-gray-50 rounded-lg">
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="flex items-center gap-2">
                            <h3 className="text-lg font-medium text-pierre-gray-900">{token.name}</h3>
                            <Badge variant={token.is_revoked ? 'info' : 'success'}>
                              {token.is_revoked ? 'Revoked' : 'Active'}
                            </Badge>
                          </div>
                          <code className="inline-flex items-center gap-1 mt-1 px-2 py-0.5 bg-pierre-gray-100 text-pierre-gray-700 text-xs font-mono rounded border border-pierre-gray-200">
                            {token.token_prefix}...
                          </code>
                          <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                            <div>
                              <span className="text-pierre-gray-500">Created:</span>
                              <p className="font-medium">{format(new Date(token.created_at), 'MMM d, yyyy')}</p>
                            </div>
                            <div>
                              <span className="text-pierre-gray-500">Expires:</span>
                              <p className="font-medium">
                                {token.expires_at ? format(new Date(token.expires_at), 'MMM d, yyyy') : 'Never'}
                              </p>
                            </div>
                            <div>
                              <span className="text-pierre-gray-500">Usage:</span>
                              <p className="font-medium">{token.usage_count} requests</p>
                            </div>
                            <div>
                              <span className="text-pierre-gray-500">Last Used:</span>
                              <p className="font-medium">
                                {token.last_used_at ? format(new Date(token.last_used_at), 'MMM d, yyyy') : 'Never'}
                              </p>
                            </div>
                          </div>
                        </div>
                        {!token.is_revoked && (
                          <Button
                            onClick={() => setTokenToRevoke(token)}
                            disabled={revokeTokenMutation.isPending}
                            variant="secondary"
                            className="text-red-600 hover:bg-red-50"
                            size="sm"
                          >
                            Revoke
                          </Button>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {/* Setup Instructions - Collapsible */}
              <div className="border-t border-pierre-gray-200 mt-6 pt-4">
                <button
                  onClick={() => setShowSetupInstructions(!showSetupInstructions)}
                  className="flex items-center justify-between w-full text-left"
                >
                  <div className="flex items-center gap-2">
                    <svg className="w-5 h-5 text-pierre-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    <span className="font-medium text-pierre-gray-900">Setup Instructions</span>
                    <span className="text-sm text-pierre-gray-500">for Claude & ChatGPT</span>
                  </div>
                  <svg
                    className={`w-5 h-5 text-pierre-gray-400 transition-transform ${showSetupInstructions ? 'rotate-180' : ''}`}
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                  </svg>
                </button>

                {showSetupInstructions && (
                  <div className="mt-4 space-y-4">
                    <div className="bg-pierre-gray-50 rounded-lg p-4">
                      <h4 className="font-medium text-pierre-gray-900 mb-2">Claude Desktop</h4>
                      <p className="text-sm text-pierre-gray-600 mb-3">
                        Add the following to your Claude Desktop config file:
                      </p>
                      <pre className="text-xs bg-pierre-gray-800 text-pierre-gray-100 p-3 rounded overflow-x-auto">
                        {`{
  "mcpServers": {
    "pierre": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-client"],
      "env": {
        "MCP_SERVER_URL": "${window.location.origin}/mcp",
        "MCP_TOKEN": "<your-token-here>"
      }
    }
  }
}`}
                      </pre>
                    </div>

                    <div className="bg-pierre-gray-50 rounded-lg p-4">
                      <h4 className="font-medium text-pierre-gray-900 mb-2">ChatGPT</h4>
                      <p className="text-sm text-pierre-gray-600 mb-3">Configure in ChatGPT MCP settings:</p>
                      <pre className="text-xs bg-pierre-gray-800 text-pierre-gray-100 p-3 rounded overflow-x-auto">
                        {`Server URL: ${window.location.origin}/mcp
Authorization: Bearer <your-token-here>`}
                      </pre>
                    </div>
                  </div>
                )}
              </div>
            </Card>

            {/* Connected Apps Section */}
            <Card>
              <div className="flex justify-between items-center mb-4">
                <div>
                  <h2 className="text-lg font-semibold text-pierre-gray-900">Connected Apps</h2>
                  <p className="text-sm text-pierre-gray-500 mt-1">
                    Third-party applications authorized to access your fitness data via OAuth
                  </p>
                </div>
              </div>
              {showCreateA2AClient ? (
                <CreateA2AClient
                  onSuccess={() => setShowCreateA2AClient(false)}
                  onCancel={() => setShowCreateA2AClient(false)}
                />
              ) : (
                <A2AClientList onCreateClient={() => setShowCreateA2AClient(true)} />
              )}
            </Card>
          </>
        )}

        {/* AI Settings Tab */}
        {activeTab === 'llm' && <LlmSettingsTab />}

        {/* Account Tab */}
        {activeTab === 'account' && (
          <>
            <Card>
              <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Account Status</h2>
              <div className="space-y-3">
                <div className="flex justify-between items-center py-2 border-b border-pierre-gray-100">
                  <span className="text-pierre-gray-600">Status</span>
                  <span
                    className={`px-2 py-1 rounded-full text-xs font-medium ${
                      user?.user_status === 'active'
                        ? 'bg-pierre-activity-light/30 text-pierre-activity'
                        : 'bg-pierre-nutrition-light/30 text-pierre-nutrition'
                    }`}
                  >
                    {user?.user_status?.charAt(0).toUpperCase()}
                    {user?.user_status?.slice(1)}
                  </span>
                </div>
                <div className="flex justify-between items-center py-2 border-b border-pierre-gray-100">
                  <span className="text-pierre-gray-600">Role</span>
                  <span className="text-pierre-gray-900 capitalize">{user?.role}</span>
                </div>
                <div className="flex justify-between items-center py-2">
                  <span className="text-pierre-gray-600">Member Since</span>
                  <span className="text-pierre-gray-900">N/A</span>
                </div>
              </div>
            </Card>

            <Card>
              <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Security</h2>
              <div className="space-y-4">
                <div className="p-4 bg-pierre-gray-50 rounded-lg">
                  <h3 className="font-medium text-pierre-gray-900 mb-2">Password</h3>
                  <p className="text-sm text-pierre-gray-600 mb-3">Change your password to keep your account secure.</p>
                  <Button variant="secondary" size="sm" disabled>
                    Change Password
                  </Button>
                  <p className="text-xs text-pierre-gray-400 mt-2">Coming soon</p>
                </div>
              </div>
            </Card>

            <Card className="border-red-200">
              <h2 className="text-lg font-semibold text-red-600 mb-4">Danger Zone</h2>
              <div className="space-y-4">
                <div className="p-4 bg-red-50 rounded-lg">
                  <h3 className="font-medium text-pierre-gray-900 mb-2">Sign Out</h3>
                  <p className="text-sm text-pierre-gray-600 mb-3">Sign out of your account on this device.</p>
                  <Button variant="secondary" size="sm" onClick={logout}>
                    Sign Out
                  </Button>
                </div>
              </div>
            </Card>
          </>
        )}
      </div>

      {/* Delete Provider Confirmation Dialog */}
      <ConfirmDialog
        isOpen={!!providerToDelete}
        onClose={() => setProviderToDelete(null)}
        onConfirm={() => providerToDelete && deleteMutation.mutate(providerToDelete)}
        title="Remove Provider Credentials"
        message={`Are you sure you want to remove the ${getProviderInfo(providerToDelete || '').name} credentials? You'll need to use the shared server credentials after this.`}
        confirmLabel="Remove"
        variant="danger"
        isLoading={deleteMutation.isPending}
      />

      {/* Revoke Token Confirmation */}
      <ConfirmDialog
        isOpen={tokenToRevoke !== null}
        onClose={() => setTokenToRevoke(null)}
        onConfirm={() => tokenToRevoke && revokeTokenMutation.mutate(tokenToRevoke.id)}
        title="Revoke Token"
        message={`Are you sure you want to revoke "${tokenToRevoke?.name}"? Any AI clients using this token will lose access immediately.`}
        confirmLabel="Revoke Token"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={revokeTokenMutation.isPending}
      />
    </div>
  );
}
