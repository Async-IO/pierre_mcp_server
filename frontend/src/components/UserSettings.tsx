// ABOUTME: Comprehensive user settings with tabbed navigation
// ABOUTME: Includes Profile, Connections, Tokens, About, and Account sections
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { format } from 'date-fns';
import { useAuth } from '../hooks/useAuth';
import { userApi, pierreApi, apiService } from '../services/api';
import type { ProviderStatus } from '../services/api';
import { Card, Button, Badge, ConfirmDialog, Input, Modal, ModalActions } from './ui';
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
  { id: 'strava', name: 'Strava', color: 'bg-pierre-nutrition' },
  { id: 'fitbit', name: 'Fitbit', color: 'bg-pierre-cyan' },
  { id: 'garmin', name: 'Garmin', color: 'bg-pierre-blue-600' },
  { id: 'whoop', name: 'WHOOP', color: 'bg-black' },
  { id: 'terra', name: 'Terra', color: 'bg-pierre-green-600' },
];

const MIN_PASSWORD_LENGTH = 8;

type SettingsTab = 'profile' | 'connections' | 'tokens' | 'llm' | 'about' | 'account';

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
    name: 'Data Providers',
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
    id: 'about',
    name: 'About',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
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

  // Fitness provider connection state
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);
  const [providerToDisconnect, setProviderToDisconnect] = useState<string | null>(null);
  const [providerMessage, setProviderMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  // Change Password state
  const [showChangePassword, setShowChangePassword] = useState(false);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordMessage, setPasswordMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  // Fetch fitness provider connection status
  const { data: providersResponse, isLoading: isLoadingProviders, refetch: refetchProviders } = useQuery({
    queryKey: ['provider-connections'],
    queryFn: () => apiService.getProvidersStatus(),
    enabled: isAuthenticated,
  });

  const fitnessProviders: ProviderStatus[] = providersResponse?.providers || [];

  // Fetch OAuth apps
  const { data: oauthAppsResponse, isLoading: isLoadingApps } = useQuery({
    queryKey: ['user-oauth-apps'],
    queryFn: () => userApi.getOAuthApps(),
  });

  // Fetch user stats
  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ['userStats'],
    queryFn: () => userApi.getStats(),
    staleTime: 30000,
  });

  // Fetch MCP tokens
  const { data: tokensResponse, isLoading: tokensLoading } = useQuery({
    queryKey: ['mcp-tokens'],
    queryFn: () => userApi.getMcpTokens(),
    enabled: isAuthenticated,
  });

  const oauthApps: OAuthApp[] = oauthAppsResponse?.apps || [];
  const tokens: McpToken[] = tokensResponse?.tokens || [];
  const activeTokens = tokens.filter((t) => !t.is_revoked);

  // Register OAuth app mutation
  const registerMutation = useMutation({
    mutationFn: (data: { provider: string; client_id: string; client_secret: string; redirect_uri: string }) =>
      userApi.registerOAuthApp(data),
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
    mutationFn: (provider: string) => userApi.deleteOAuthApp(provider),
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
    mutationFn: (data: { display_name: string }) => userApi.updateProfile(data),
    onSuccess: (response) => {
      setMessage({ type: 'success', text: response.message });
      pierreApi.adapter.authStorage.setUser(response.user);
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
    mutationFn: (data: { name: string; expires_in_days?: number }) => userApi.createMcpToken(data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['mcp-tokens'] });
      setCreatedToken({ token_value: data.token_value ?? '', name: data.name });
      setShowCreateForm(false);
      setNewTokenName('');
      setExpiresInDays(undefined);
    },
  });

  const revokeTokenMutation = useMutation({
    mutationFn: (tokenId: string) => userApi.revokeMcpToken(tokenId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mcp-tokens'] });
      setTokenToRevoke(null);
    },
  });

  // Change password mutation
  const changePasswordMutation = useMutation({
    mutationFn: (data: { current_password: string; new_password: string }) =>
      userApi.changePassword(data.current_password, data.new_password),
    onSuccess: () => {
      setPasswordMessage({ type: 'success', text: 'Password changed successfully' });
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      setTimeout(() => {
        setShowChangePassword(false);
        setPasswordMessage(null);
      }, 2000);
    },
    onError: (error: Error) => {
      setPasswordMessage({ type: 'error', text: error.message || 'Failed to change password' });
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

  const handleChangePassword = () => {
    setPasswordMessage(null);
    if (!currentPassword || !newPassword || !confirmPassword) {
      setPasswordMessage({ type: 'error', text: 'All fields are required' });
      return;
    }
    if (newPassword.length < MIN_PASSWORD_LENGTH) {
      setPasswordMessage({ type: 'error', text: `Password must be at least ${MIN_PASSWORD_LENGTH} characters` });
      return;
    }
    if (newPassword !== confirmPassword) {
      setPasswordMessage({ type: 'error', text: 'Passwords do not match' });
      return;
    }
    changePasswordMutation.mutate({
      current_password: currentPassword,
      new_password: newPassword,
    });
  };

  // Connect to a fitness provider via OAuth popup
  const handleConnectProvider = async (providerId: string) => {
    try {
      setConnectingProvider(providerId);
      setProviderMessage(null);
      const authUrl = await apiService.getOAuthAuthorizeUrlForProvider(providerId);

      // Open OAuth in a popup window with noopener to prevent tabnabbing
      const popup = window.open(authUrl, `oauth_${providerId}`, 'width=600,height=700,left=200,top=100,noopener,noreferrer');

      // Listen for the OAuth callback result stored in localStorage by OAuthCallback
      const checkInterval = setInterval(() => {
        try {
          const resultStr = localStorage.getItem('pierre_oauth_result');
          if (resultStr) {
            const result = JSON.parse(resultStr);
            // Only process results less than 30 seconds old
            if (result.timestamp && Date.now() - result.timestamp < 30000 && result.provider === providerId) {
              localStorage.removeItem('pierre_oauth_result');
              clearInterval(checkInterval);
              if (popup && !popup.closed) popup.close();
              setConnectingProvider(null);

              if (result.success) {
                setProviderMessage({ type: 'success', text: `${providerId} connected successfully!` });
                refetchProviders();
              } else {
                setProviderMessage({ type: 'error', text: `Failed to connect ${providerId}` });
              }
            }
          }
          // Also check if popup was closed manually
          if (popup && popup.closed) {
            clearInterval(checkInterval);
            setConnectingProvider(null);
          }
        } catch {
          // Ignore localStorage parse errors
        }
      }, 500);

      // Safety timeout: stop checking after 5 minutes
      setTimeout(() => {
        clearInterval(checkInterval);
        setConnectingProvider(null);
      }, 300000);
    } catch (error) {
      setConnectingProvider(null);
      setProviderMessage({
        type: 'error',
        text: error instanceof Error ? error.message : 'Failed to start connection',
      });
    }
  };

  // Disconnect a fitness provider
  const handleDisconnectProvider = async (providerId: string) => {
    try {
      setProviderMessage(null);
      await apiService.disconnectProvider(providerId);
      setProviderToDisconnect(null);
      setProviderMessage({ type: 'success', text: `${providerId} disconnected` });
      refetchProviders();
    } catch (error) {
      setProviderToDisconnect(null);
      setProviderMessage({
        type: 'error',
        text: error instanceof Error ? error.message : 'Failed to disconnect provider',
      });
    }
  };

  // Display config for fitness providers (matching mobile)
  const PROVIDER_DISPLAY: Record<string, { color: string; description: string }> = {
    strava: { color: '#FC4C02', description: 'Running, cycling, and swimming activities' },
    garmin: { color: '#007CC3', description: 'Activities and health metrics from Garmin devices' },
    fitbit: { color: '#00B0B9', description: 'Activity, sleep, and heart rate data' },
    whoop: { color: '#000000', description: 'Recovery, strain, and sleep metrics' },
    terra: { color: '#16A34A', description: 'Aggregate data from multiple fitness platforms' },
    coros: { color: '#E91E63', description: 'Training and performance data from COROS devices' },
    synthetic: { color: '#9C27B0', description: 'Synthetic test data for development' },
    synthetic_sleep: { color: '#673AB7', description: 'Synthetic sleep data for development' },
  };

  const copyToClipboard = async (text: string) => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const getProviderInfo = (providerId: string) => {
    return PROVIDERS.find((p) => p.id === providerId) || { id: providerId, name: providerId, color: 'bg-pierre-gray-500' };
  };

  const configuredProviders = oauthApps.map((app) => app.provider);
  const availableProviders = PROVIDERS.filter((p) => !configuredProviders.includes(p.id));

  return (
    <div className="space-y-6">
      {/* Horizontal Tab Navigation */}
      <div className="border-b border-white/10">
        <nav className="flex gap-1 -mb-px overflow-x-auto" aria-label="Settings tabs">
          {SETTINGS_TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={clsx(
                'flex items-center gap-2 px-4 py-3 text-sm font-medium whitespace-nowrap transition-all duration-200 border-b-2',
                activeTab === tab.id
                  ? 'border-pierre-violet text-white'
                  : 'border-transparent text-zinc-400 hover:text-white hover:border-white/20'
              )}
            >
              <span className={clsx('flex-shrink-0', activeTab === tab.id ? 'text-pierre-violet' : '')}>{tab.icon}</span>
              {tab.name}
            </button>
          ))}
        </nav>
      </div>

      {/* Settings Content */}
      <div className="space-y-6">
        {/* Profile Tab */}
        {activeTab === 'profile' && (
          <>
            <Card variant="dark">
              <h2 className="text-lg font-semibold text-white mb-4">Profile Information</h2>
              <div className="space-y-4">
                {/* Gradient ring avatar */}
                <div className="flex items-center gap-4 pb-4 border-b border-white/10">
                  <div className="relative flex-shrink-0">
                    <div className="w-24 h-24 rounded-full p-[3px] bg-gradient-to-br from-pierre-violet to-pierre-cyan">
                      <div className="w-full h-full bg-pierre-slate rounded-full flex items-center justify-center">
                        <span className="text-3xl font-bold text-white">
                          {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
                        </span>
                      </div>
                    </div>
                  </div>
                  <div>
                    <p className="text-xl font-semibold text-white">{user?.display_name || 'No name set'}</p>
                    <p className="text-sm text-zinc-400">{user?.email}</p>
                    <Badge variant={user?.user_status === 'active' ? 'success' : 'warning'} className="mt-1">
                      {user?.user_status?.charAt(0).toUpperCase()}{user?.user_status?.slice(1)}
                    </Badge>
                  </div>
                </div>

                <Input
                  variant="dark"
                  label="Display Name"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  placeholder="Enter your display name"
                  size="lg"
                />

                <div>
                  <label className="block text-sm font-medium text-zinc-300 mb-2">Email</label>
                  <p className="text-zinc-400 bg-[#151520]/50 px-4 py-3 rounded-xl border border-white/10">{user?.email}</p>
                  <p className="text-xs text-zinc-500 mt-1">Email cannot be changed</p>
                </div>

                {message && (
                  <div
                    className={`p-3 rounded-lg text-sm ${
                      message.type === 'success'
                        ? 'bg-pierre-activity/20 text-pierre-activity border border-pierre-activity/30'
                        : 'bg-pierre-red-500/20 text-pierre-red-500 border border-pierre-red-500/30'
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
                  className="shadow-glow hover:shadow-glow-lg"
                >
                  Save Changes
                </Button>
              </div>
            </Card>

            {/* Quick Stats with gradient accent */}
            <div className="grid grid-cols-2 gap-4">
              <div className="stat-card-dark rounded-xl border border-white/10 p-6">
                <div className="text-center">
                  <div className="text-3xl font-bold bg-gradient-to-r from-pierre-violet to-pierre-cyan bg-clip-text text-transparent">
                    {statsLoading ? '...' : (stats?.connected_providers ?? 0)}
                  </div>
                  <div className="text-sm text-zinc-400 mt-1">Connected Providers</div>
                </div>
              </div>
              <div className="stat-card-dark rounded-xl border border-white/10 p-6">
                <div className="text-center">
                  <div className="text-3xl font-bold bg-gradient-to-r from-pierre-nutrition to-pierre-activity bg-clip-text text-transparent">
                    {statsLoading ? '...' : (stats?.days_active ?? 0)}
                  </div>
                  <div className="text-sm text-zinc-400 mt-1">Days Active</div>
                </div>
              </div>
            </div>
          </>
        )}

        {/* Connections Tab */}
        {activeTab === 'connections' && (
          <>
            {/* Fitness Providers - Connection Status */}
            <Card variant="dark">
              <h2 className="text-lg font-semibold text-white mb-1">Fitness Providers</h2>
              <p className="text-sm text-zinc-400 mb-4">
                Connect your fitness accounts to sync activities, health metrics, and more.
              </p>

              {providerMessage && (
                <div
                  className={clsx(
                    'p-3 rounded-lg text-sm mb-4',
                    providerMessage.type === 'success'
                      ? 'bg-pierre-activity/20 text-pierre-activity border border-pierre-activity/30'
                      : 'bg-pierre-red-500/20 text-pierre-red-500 border border-pierre-red-500/30'
                  )}
                >
                  {providerMessage.text}
                </div>
              )}

              {isLoadingProviders ? (
                <div className="flex justify-center py-8">
                  <div className="pierre-spinner w-6 h-6"></div>
                </div>
              ) : fitnessProviders.length === 0 ? (
                <div className="text-center py-8 text-zinc-400">
                  <p>No providers available</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {fitnessProviders.map((provider) => {
                    const display = PROVIDER_DISPLAY[provider.provider] || {
                      color: '#607D8B',
                      description: 'Fitness data provider',
                    };
                    const isConnecting = connectingProvider === provider.provider;

                    return (
                      <div
                        key={provider.provider}
                        className={clsx(
                          'p-4 rounded-xl border transition-all',
                          provider.connected
                            ? 'border-pierre-activity/30 bg-pierre-activity-light/10'
                            : 'border-white/10 bg-[#151520]'
                        )}
                      >
                        <div className="flex items-center gap-3">
                          <div
                            className="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0"
                            style={{ backgroundColor: display.color }}
                          >
                            <span className="text-white font-bold text-sm">
                              {provider.display_name.charAt(0)}
                            </span>
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
                              <p className="font-medium text-white">{provider.display_name}</p>
                              {provider.connected && (
                                <Badge variant="success">Connected</Badge>
                              )}
                            </div>
                            <p className="text-sm text-zinc-400 truncate">{display.description}</p>
                            {provider.capabilities.length > 0 && (
                              <p className="text-xs text-zinc-500 mt-0.5">
                                {provider.capabilities.join(', ')}
                              </p>
                            )}
                          </div>
                          <div className="flex-shrink-0">
                            {provider.connected ? (
                              provider.requires_oauth && (
                                <Button
                                  variant="secondary"
                                  size="sm"
                                  onClick={() => setProviderToDisconnect(provider.provider)}
                                  className="text-red-400 hover:bg-red-500/20"
                                >
                                  Disconnect
                                </Button>
                              )
                            ) : provider.requires_oauth ? (
                              <Button
                                variant="gradient"
                                size="sm"
                                onClick={() => handleConnectProvider(provider.provider)}
                                loading={isConnecting}
                              >
                                Connect
                              </Button>
                            ) : (
                              <Badge variant="secondary">Manual</Badge>
                            )}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}

              {/* Privacy note */}
              <div className="mt-4 p-3 bg-white/5 border border-white/10 rounded-lg">
                <p className="text-xs text-zinc-400">
                  Pierre only accesses the data you authorize. We never share your fitness data with third parties.
                  You can disconnect any provider at any time.
                </p>
              </div>
            </Card>

            {/* OAuth App Credentials (Advanced) */}
            <Card variant="dark">
            <div className="flex justify-between items-center mb-4">
              <div>
                <h2 className="text-lg font-semibold text-white">Custom API Credentials</h2>
                <p className="text-sm text-zinc-400 mt-1">
                  Use your own OAuth app credentials to avoid shared rate limits
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
                    ? 'bg-pierre-activity/20 text-pierre-activity border border-pierre-activity/30'
                    : 'bg-pierre-red-500/20 text-pierre-red-500 border border-pierre-red-500/30'
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
              <div className="text-center py-8 bg-[#151520] rounded-xl border border-white/10">
                <svg
                  className="w-12 h-12 text-zinc-600 mx-auto mb-3"
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
                <p className="text-white font-medium">No custom credentials configured</p>
                <p className="text-sm text-zinc-500 mt-1">
                  Add your own OAuth app credentials to use your personal API quotas
                </p>
              </div>
            ) : (
              <div className="space-y-3">
                {oauthApps.map((app) => {
                  const provider = getProviderInfo(app.provider);
                  return (
                    <div key={app.provider} className="flex items-center justify-between p-4 bg-[#151520] rounded-xl border border-white/10">
                      <div className="flex items-center gap-3">
                        <div className={`w-10 h-10 ${provider.color} rounded-lg flex items-center justify-center`}>
                          <span className="text-white font-bold text-sm">{provider.name.charAt(0)}</span>
                        </div>
                        <div>
                          <p className="font-medium text-white">{provider.name}</p>
                          <p className="text-xs text-zinc-500">Client ID: {app.client_id.substring(0, 8)}...</p>
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
              <div className="mt-4 p-4 border border-white/10 rounded-xl bg-[#151520]">
                <h3 className="font-medium text-white mb-4">Add Provider Credentials</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-zinc-300 mb-2">Provider</label>
                    <select
                      value={selectedProvider}
                      onChange={(e) => setSelectedProvider(e.target.value)}
                      className="select-dark w-full px-4 py-3 bg-[#0F0F1A] border border-white/10 rounded-lg text-white focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-30 focus:border-pierre-violet transition-all"
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
                    variant="dark"
                    label="Client ID"
                    value={clientId}
                    onChange={(e) => setClientId(e.target.value)}
                    placeholder="Enter your OAuth client ID"
                  />

                  <Input
                    variant="dark"
                    label="Client Secret"
                    type="password"
                    value={clientSecret}
                    onChange={(e) => setClientSecret(e.target.value)}
                    placeholder="Enter your OAuth client secret"
                  />

                  <Input
                    variant="dark"
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
          </>
        )}

        {/* Tokens Tab */}
        {activeTab === 'tokens' && (
          <>
            {/* Created Token Display */}
            {createdToken && (
              <div className="bg-emerald-500/10 border border-emerald-500/30 rounded-lg p-6">
                <div className="flex items-start gap-3">
                  <svg className="w-6 h-6 text-emerald-400 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  <div className="flex-1">
                    <h3 className="text-lg font-medium text-emerald-400">Token Created: {createdToken.name}</h3>
                    <p className="text-emerald-400/80 mt-1 mb-3">Copy this token now. You won&apos;t be able to see it again!</p>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 px-3 py-2 bg-[#151520] border border-emerald-500/30 rounded font-mono text-sm break-all text-white">
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

            <Card variant="dark">
              <div className="flex justify-between items-center mb-4">
                <div>
                  <h2 className="text-lg font-semibold text-white">API Tokens</h2>
                  <p className="text-sm text-zinc-400 mt-1">
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
                  <div className="bg-white/5 border border-white/10 rounded-lg p-4 space-y-4">
                    <h4 className="font-medium text-white">Create Token</h4>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <Input
                        variant="dark"
                        label="Token Name"
                        value={newTokenName}
                        onChange={(e) => setNewTokenName(e.target.value)}
                        placeholder="e.g., Claude Desktop, Cursor IDE"
                      />
                      <div>
                        <label className="block text-sm font-medium text-zinc-300 mb-1.5">Expires In (days)</label>
                        <select
                          value={expiresInDays || ''}
                          onChange={(e) => setExpiresInDays(e.target.value ? Number(e.target.value) : undefined)}
                          className="select-dark w-full px-4 py-2.5 bg-[#151520] border border-white/10 rounded-lg text-white text-sm focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-30 focus:border-pierre-violet transition-all"
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
                <div className="text-center py-8 text-zinc-400">
                  <svg className="w-12 h-12 text-zinc-600 mx-auto mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
                  </svg>
                  <p className="text-lg mb-2 text-white">No tokens yet</p>
                  <p>Create a token to connect AI clients like Claude Desktop or Cursor to Pierre</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {tokens.map((token) => (
                    <div key={token.id} className="p-4 bg-white/5 border border-white/10 rounded-lg">
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="flex items-center gap-2">
                            <h3 className="text-lg font-medium text-white">{token.name}</h3>
                            <Badge variant={token.is_revoked ? 'info' : 'success'}>
                              {token.is_revoked ? 'Revoked' : 'Active'}
                            </Badge>
                          </div>
                          <code className="inline-flex items-center gap-1 mt-1 px-2 py-0.5 bg-white/10 text-zinc-300 text-xs font-mono rounded border border-white/10">
                            {token.token_prefix}...
                          </code>
                          <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                            <div>
                              <span className="text-zinc-500">Created:</span>
                              <p className="font-medium text-white">{format(new Date(token.created_at), 'MMM d, yyyy')}</p>
                            </div>
                            <div>
                              <span className="text-zinc-500">Expires:</span>
                              <p className="font-medium text-white">
                                {token.expires_at ? format(new Date(token.expires_at), 'MMM d, yyyy') : 'Never'}
                              </p>
                            </div>
                            <div>
                              <span className="text-zinc-500">Usage:</span>
                              <p className="font-medium text-white">{token.usage_count} requests</p>
                            </div>
                            <div>
                              <span className="text-zinc-500">Last Used:</span>
                              <p className="font-medium text-white">
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
                            className="text-red-400 hover:bg-red-500/20"
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
              <div className="border-t border-white/10 mt-6 pt-4">
                <button
                  onClick={() => setShowSetupInstructions(!showSetupInstructions)}
                  className="flex items-center justify-between w-full text-left"
                >
                  <div className="flex items-center gap-2">
                    <svg className="w-5 h-5 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    <span className="font-medium text-white">Setup Instructions</span>
                    <span className="text-sm text-zinc-400">for Claude & ChatGPT</span>
                  </div>
                  <svg
                    className={`w-5 h-5 text-zinc-500 transition-transform ${showSetupInstructions ? 'rotate-180' : ''}`}
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                  </svg>
                </button>

                {showSetupInstructions && (
                  <div className="mt-4 space-y-4">
                    <div className="bg-white/5 border border-white/10 rounded-lg p-4">
                      <h4 className="font-medium text-white mb-2">Claude Desktop</h4>
                      <p className="text-sm text-zinc-400 mb-3">
                        Add the following to your Claude Desktop config file:
                      </p>
                      <pre className="text-xs bg-[#151520] text-zinc-300 p-3 rounded overflow-x-auto border border-white/10">
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

                    <div className="bg-white/5 border border-white/10 rounded-lg p-4">
                      <h4 className="font-medium text-white mb-2">ChatGPT</h4>
                      <p className="text-sm text-zinc-400 mb-3">Configure in ChatGPT MCP settings:</p>
                      <pre className="text-xs bg-[#151520] text-zinc-300 p-3 rounded overflow-x-auto border border-white/10">
                        {`Server URL: ${window.location.origin}/mcp
Authorization: Bearer <your-token-here>`}
                      </pre>
                    </div>
                  </div>
                )}
              </div>
            </Card>

            {/* Connected Apps Section */}
            <Card variant="dark">
              <div className="flex justify-between items-center mb-4">
                <div>
                  <h2 className="text-lg font-semibold text-white">Connected Apps</h2>
                  <p className="text-sm text-zinc-400 mt-1">
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

        {/* About Tab */}
        {activeTab === 'about' && (
          <Card variant="dark">
            <h2 className="text-lg font-semibold text-white mb-6">About Pierre</h2>
            <div className="space-y-3">
              {/* Version */}
              <div className="flex items-center gap-4 p-4 bg-white/5 rounded-xl border border-white/10">
                <div className="w-10 h-10 rounded-xl bg-pierre-violet/15 flex items-center justify-center flex-shrink-0">
                  <svg className="w-5 h-5 text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </div>
                <div className="flex-1">
                  <p className="text-sm text-zinc-400">Version</p>
                  <p className="text-white font-medium">1.0.0</p>
                </div>
              </div>

              {/* Help Center */}
              <a
                href="https://pierre.fitness/help"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-4 p-4 bg-white/5 rounded-xl border border-white/10 hover:bg-white/10 transition-colors group"
              >
                <div className="w-10 h-10 rounded-xl bg-pierre-cyan/15 flex items-center justify-center flex-shrink-0">
                  <svg className="w-5 h-5 text-pierre-cyan" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 5.636l-3.536 3.536m0 5.656l3.536 3.536M9.172 9.172L5.636 5.636m3.536 9.192l-3.536 3.536M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-5 0a4 4 0 11-8 0 4 4 0 018 0z" />
                  </svg>
                </div>
                <div className="flex-1">
                  <p className="text-white font-medium">Help Center</p>
                  <p className="text-sm text-zinc-400">Documentation and support</p>
                </div>
                <svg className="w-5 h-5 text-zinc-500 group-hover:text-white transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </a>

              {/* Terms & Privacy */}
              <a
                href="https://pierre.fitness/privacy"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-4 p-4 bg-white/5 rounded-xl border border-white/10 hover:bg-white/10 transition-colors group"
              >
                <div className="w-10 h-10 rounded-xl bg-pierre-activity/15 flex items-center justify-center flex-shrink-0">
                  <svg className="w-5 h-5 text-pierre-activity" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                  </svg>
                </div>
                <div className="flex-1">
                  <p className="text-white font-medium">Terms & Privacy</p>
                  <p className="text-sm text-zinc-400">Legal information and data policy</p>
                </div>
                <svg className="w-5 h-5 text-zinc-500 group-hover:text-white transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </a>
            </div>
          </Card>
        )}

        {/* Account Tab */}
        {activeTab === 'account' && (
          <>
            <Card variant="dark">
              <h2 className="text-lg font-semibold text-white mb-4">Account Status</h2>
              <div className="space-y-3">
                <div className="flex justify-between items-center py-2 border-b border-white/10">
                  <span className="text-zinc-400">Status</span>
                  <span
                    className={`px-2 py-1 rounded-full text-xs font-medium ${
                      user?.user_status === 'active'
                        ? 'bg-emerald-500/20 text-emerald-400'
                        : 'bg-amber-500/20 text-amber-400'
                    }`}
                  >
                    {user?.user_status?.charAt(0).toUpperCase()}
                    {user?.user_status?.slice(1)}
                  </span>
                </div>
                <div className="flex justify-between items-center py-2 border-b border-white/10">
                  <span className="text-zinc-400">Role</span>
                  <span className="text-white capitalize">{user?.role}</span>
                </div>
                <div className="flex justify-between items-center py-2">
                  <span className="text-zinc-400">Member Since</span>
                  <span className="text-white">
                    {user?.created_at
                      ? format(new Date(user.created_at), 'MMM d, yyyy')
                      : 'Unknown'}
                  </span>
                </div>
              </div>
            </Card>

            <Card variant="dark">
              <h2 className="text-lg font-semibold text-white mb-4">Security</h2>
              <div className="space-y-4">
                <div className="p-4 bg-white/5 border border-white/10 rounded-lg">
                  <h3 className="font-medium text-white mb-2">Password</h3>
                  <p className="text-sm text-zinc-400 mb-3">Change your password to keep your account secure.</p>
                  <Button variant="secondary" size="sm" onClick={() => setShowChangePassword(true)}>
                    Change Password
                  </Button>
                </div>
              </div>
            </Card>

            <Card variant="dark" className="border-red-500/30">
              <h2 className="text-lg font-semibold text-red-400 mb-4">Danger Zone</h2>
              <div className="space-y-4">
                <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-lg">
                  <h3 className="font-medium text-white mb-2">Sign Out</h3>
                  <p className="text-sm text-zinc-400 mb-3">Sign out of your account on this device.</p>
                  <Button variant="secondary" size="sm" onClick={logout}>
                    Sign Out
                  </Button>
                </div>
              </div>
            </Card>
          </>
        )}
      </div>

      {/* Change Password Modal */}
      <Modal
        isOpen={showChangePassword}
        onClose={() => {
          setShowChangePassword(false);
          setCurrentPassword('');
          setNewPassword('');
          setConfirmPassword('');
          setPasswordMessage(null);
        }}
        title="Change Password"
        size="sm"
        footer={
          <ModalActions>
            <Button
              variant="secondary"
              onClick={() => {
                setShowChangePassword(false);
                setCurrentPassword('');
                setNewPassword('');
                setConfirmPassword('');
                setPasswordMessage(null);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="gradient"
              onClick={handleChangePassword}
              loading={changePasswordMutation.isPending}
              disabled={!currentPassword || !newPassword || !confirmPassword}
            >
              Update Password
            </Button>
          </ModalActions>
        }
      >
        <div className="space-y-4">
          {passwordMessage && (
            <div
              className={`p-3 rounded-lg text-sm ${
                passwordMessage.type === 'success'
                  ? 'bg-pierre-activity/20 text-pierre-activity border border-pierre-activity/30'
                  : 'bg-pierre-red-500/20 text-pierre-red-500 border border-pierre-red-500/30'
              }`}
            >
              {passwordMessage.text}
            </div>
          )}
          <Input
            variant="dark"
            label="Current Password"
            type="password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            placeholder="Enter current password"
          />
          <Input
            variant="dark"
            label="New Password"
            type="password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            placeholder="Enter new password"
            helpText={`Minimum ${MIN_PASSWORD_LENGTH} characters`}
          />
          <Input
            variant="dark"
            label="Confirm New Password"
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            placeholder="Confirm new password"
            error={confirmPassword && newPassword !== confirmPassword ? 'Passwords do not match' : undefined}
          />
        </div>
      </Modal>

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

      {/* Disconnect Fitness Provider Confirmation */}
      <ConfirmDialog
        isOpen={providerToDisconnect !== null}
        onClose={() => setProviderToDisconnect(null)}
        onConfirm={() => providerToDisconnect && handleDisconnectProvider(providerToDisconnect)}
        title="Disconnect Provider"
        message={`Are you sure you want to disconnect ${providerToDisconnect}? You will need to reconnect to sync new data.`}
        confirmLabel="Disconnect"
        variant="danger"
      />
    </div>
  );
}
