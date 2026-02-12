// ABOUTME: Hook for managing OAuth provider connection status
// ABOUTME: Handles provider loading, connection checks, and OAuth flow initiation

import { useState, useCallback, useEffect } from 'react';
import { Alert, AppState } from 'react-native';
import * as Linking from 'expo-linking';
import * as WebBrowser from 'expo-web-browser';
import { getOAuthCallbackUrl } from '../../utils/oauth';
import { oauthApi } from '../../services/api';
import type { ExtendedProviderStatus } from '../../types';

export interface ProviderStatusState {
  connectedProviders: ExtendedProviderStatus[];
  selectedProvider: string | null;
  providerModalVisible: boolean;
  error: string | null;
}

export interface ProviderStatusActions {
  loadProviderStatus: () => Promise<void>;
  hasConnectedProvider: () => boolean;
  setSelectedProvider: (provider: string | null) => void;
  setProviderModalVisible: (visible: boolean) => void;
  handleConnectProvider: (
    provider: string,
    onSuccess?: () => Promise<void>
  ) => Promise<void>;
  getCachedConnectedProvider: () => ExtendedProviderStatus | undefined;
}

export function useProviderStatus(): ProviderStatusState & ProviderStatusActions {
  const [connectedProviders, setConnectedProviders] = useState<ExtendedProviderStatus[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
  const [providerModalVisible, setProviderModalVisible] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadProviderStatus = useCallback(async () => {
    try {
      setError(null);
      const response = await oauthApi.getProvidersStatus();
      setConnectedProviders(response.providers || []);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load provider status';
      setError(errorMessage);
      console.error('Failed to load provider status:', err);
    }
  }, []);

  // Refresh provider status when app returns from OAuth flow
  useEffect(() => {
    const subscription = AppState.addEventListener('change', (nextAppState) => {
      if (nextAppState === 'active') {
        loadProviderStatus();
      }
    });
    return () => subscription.remove();
  }, [loadProviderStatus]);

  const hasConnectedProvider = useCallback((): boolean => {
    return connectedProviders.some(p => p.connected);
  }, [connectedProviders]);

  const getCachedConnectedProvider = useCallback((): ExtendedProviderStatus | undefined => {
    if (selectedProvider) {
      const cached = connectedProviders.find(
        p => p.provider === selectedProvider && p.connected
      );
      if (cached) return cached;
    }
    return connectedProviders.find(p => p.connected);
  }, [connectedProviders, selectedProvider]);

  const handleConnectProvider = useCallback(async (
    provider: string,
    onSuccess?: () => Promise<void>
  ) => {
    setProviderModalVisible(false);
    try {
      setError(null);
      const returnUrl = getOAuthCallbackUrl();
      const oauthResponse = await oauthApi.initMobileOAuth(provider, returnUrl);

      const result = await WebBrowser.openAuthSessionAsync(
        oauthResponse.authorization_url,
        returnUrl
      );

      if (result.type === 'success' && result.url) {
        const expectedPrefix = getOAuthCallbackUrl();
        if (!result.url.startsWith(expectedPrefix)) {
          console.error('OAuth callback URL does not match expected scheme:', result.url);
          setError('Unexpected OAuth callback URL');
          Alert.alert('Connection Failed', 'Unexpected OAuth callback URL');
          return;
        }

        const parsedUrl = Linking.parse(result.url);
        const success = parsedUrl.queryParams?.success === 'true';
        const errorParam = parsedUrl.queryParams?.error as string | undefined;

        if (success) {
          await loadProviderStatus();
          setSelectedProvider(provider);
          if (onSuccess) {
            await onSuccess();
          }
        } else if (errorParam) {
          setError(`Failed to connect: ${errorParam}`);
          console.error('OAuth error from server:', errorParam);
          Alert.alert('Connection Failed', `Failed to connect: ${errorParam}`);
        } else {
          await loadProviderStatus();
          Alert.alert('Connection Complete', `${provider} connection flow completed.`);
        }
      } else if (result.type === 'cancel') {
        console.log('OAuth cancelled by user');
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to connect provider';
      setError(errorMessage);
      console.error('Failed to start OAuth:', err);
      Alert.alert('Error', 'Failed to connect provider. Please try again.');
    }
  }, [loadProviderStatus]);

  return {
    connectedProviders,
    selectedProvider,
    providerModalVisible,
    error,
    loadProviderStatus,
    hasConnectedProvider,
    setSelectedProvider,
    setProviderModalVisible,
    handleConnectProvider,
    getCachedConnectedProvider,
  };
}
