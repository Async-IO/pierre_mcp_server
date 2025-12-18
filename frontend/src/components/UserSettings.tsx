// ABOUTME: User settings tab for regular users
// ABOUTME: Displays profile information, account settings, and OAuth provider credentials
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import { Card, Button, Input, Badge, ConfirmDialog } from './ui';

interface OAuthApp {
  provider: string;
  client_id: string;
  redirect_uri: string;
  created_at: string;
}

const PROVIDERS = [
  { id: 'strava', name: 'Strava', color: 'bg-orange-500' },
  { id: 'fitbit', name: 'Fitbit', color: 'bg-teal-500' },
  { id: 'garmin', name: 'Garmin', color: 'bg-blue-600' },
  { id: 'whoop', name: 'WHOOP', color: 'bg-black' },
  { id: 'terra', name: 'Terra', color: 'bg-green-600' },
];

export default function UserSettings() {
  const { user, logout } = useAuth();
  const queryClient = useQueryClient();
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

  // Fetch OAuth apps
  const { data: oauthAppsResponse, isLoading: isLoadingApps } = useQuery({
    queryKey: ['user-oauth-apps'],
    queryFn: () => apiService.getUserOAuthApps(),
  });

  const oauthApps: OAuthApp[] = oauthAppsResponse?.apps || [];

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
      // Update the user in localStorage and invalidate relevant queries
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

  const getProviderInfo = (providerId: string) => {
    return PROVIDERS.find(p => p.id === providerId) || { id: providerId, name: providerId, color: 'bg-gray-500' };
  };

  const configuredProviders = oauthApps.map(app => app.provider);
  const availableProviders = PROVIDERS.filter(p => !configuredProviders.includes(p.id));

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Profile Section */}
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
            <div className={`p-3 rounded-lg text-sm ${
              message.type === 'success'
                ? 'bg-pierre-activity-light/30 text-pierre-activity'
                : 'bg-red-50 text-red-600'
            }`}>
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

      {/* OAuth Provider Credentials Section */}
      <Card>
        <div className="flex justify-between items-center mb-4">
          <div>
            <h2 className="text-lg font-semibold text-pierre-gray-900">Provider Credentials</h2>
            <p className="text-sm text-pierre-gray-500 mt-1">
              Configure your own OAuth app credentials to avoid rate limits
            </p>
          </div>
          {availableProviders.length > 0 && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowAddCredentials(true)}
            >
              Add Provider
            </Button>
          )}
        </div>

        {credentialMessage && (
          <div className={`p-3 rounded-lg text-sm mb-4 ${
            credentialMessage.type === 'success'
              ? 'bg-pierre-activity-light/30 text-pierre-activity'
              : 'bg-red-50 text-red-600'
          }`}>
            {credentialMessage.text}
          </div>
        )}

        {isLoadingApps ? (
          <div className="flex justify-center py-6">
            <div className="pierre-spinner w-6 h-6"></div>
          </div>
        ) : oauthApps.length === 0 ? (
          <div className="text-center py-8 bg-pierre-gray-50 rounded-lg">
            <svg className="w-12 h-12 text-pierre-gray-400 mx-auto mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
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
                <div
                  key={app.provider}
                  className="flex items-center justify-between p-4 bg-pierre-gray-50 rounded-lg"
                >
                  <div className="flex items-center gap-3">
                    <div className={`w-10 h-10 ${provider.color} rounded-lg flex items-center justify-center`}>
                      <span className="text-white font-bold text-sm">
                        {provider.name.charAt(0)}
                      </span>
                    </div>
                    <div>
                      <p className="font-medium text-pierre-gray-900">{provider.name}</p>
                      <p className="text-xs text-pierre-gray-500">
                        Client ID: {app.client_id.substring(0, 8)}...
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant="success">Configured</Badge>
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => setProviderToDelete(app.provider)}
                    >
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
                  {availableProviders.map(provider => (
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

              <div className="bg-pierre-gray-50 p-3 rounded-lg text-sm text-pierre-gray-600">
                <p className="font-medium text-pierre-gray-700 mb-1">How to get credentials:</p>
                <ol className="list-decimal list-inside space-y-1">
                  <li>Create an app on the provider&apos;s developer portal</li>
                  <li>Set the redirect URI to match your Pierre server</li>
                  <li>Copy the client ID and secret here</li>
                </ol>
              </div>

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

      {/* Account Status Section */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Account Status</h2>

        <div className="space-y-3">
          <div className="flex justify-between items-center py-2 border-b border-pierre-gray-100">
            <span className="text-pierre-gray-600">Status</span>
            <span className={`px-2 py-1 rounded-full text-xs font-medium ${
              user?.user_status === 'active'
                ? 'bg-pierre-activity-light/30 text-pierre-activity'
                : 'bg-pierre-nutrition-light/30 text-pierre-nutrition'
            }`}>
              {user?.user_status?.charAt(0).toUpperCase()}{user?.user_status?.slice(1)}
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

      {/* Security Section */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Security</h2>

        <div className="space-y-4">
          <div className="p-4 bg-pierre-gray-50 rounded-lg">
            <h3 className="font-medium text-pierre-gray-900 mb-2">Password</h3>
            <p className="text-sm text-pierre-gray-600 mb-3">
              Change your password to keep your account secure.
            </p>
            <Button variant="secondary" size="sm" disabled>
              Change Password
            </Button>
            <p className="text-xs text-pierre-gray-400 mt-2">Coming soon</p>
          </div>
        </div>
      </Card>

      {/* Danger Zone */}
      <Card className="border-red-200">
        <h2 className="text-lg font-semibold text-red-600 mb-4">Danger Zone</h2>

        <div className="space-y-4">
          <div className="p-4 bg-red-50 rounded-lg">
            <h3 className="font-medium text-pierre-gray-900 mb-2">Sign Out</h3>
            <p className="text-sm text-pierre-gray-600 mb-3">
              Sign out of your account on this device.
            </p>
            <Button variant="secondary" size="sm" onClick={logout}>
              Sign Out
            </Button>
          </div>
        </div>
      </Card>

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
    </div>
  );
}
