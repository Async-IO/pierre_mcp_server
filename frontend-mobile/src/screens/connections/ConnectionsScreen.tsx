// ABOUTME: Provider connections screen for fitness data sources
// ABOUTME: Displays connection status and OAuth flow for Strava, Garmin, Fitbit, WHOOP, Terra

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
} from 'react-native';
import * as WebBrowser from 'expo-web-browser';
import * as Linking from 'expo-linking';
import { getOAuthCallbackUrl } from '../../utils/oauth';
import { LinearGradient } from 'expo-linear-gradient';
import { colors, spacing, glassCard, gradients } from '../../constants/theme';
import { Card, DragIndicator } from '../../components/ui';
import { oauthApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { ExtendedProviderStatus } from '../../types';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import type { SettingsStackParamList } from '../../navigation/MainTabs';

interface ConnectionsScreenProps {
  navigation: NativeStackNavigationProp<SettingsStackParamList>;
}

export function ConnectionsScreen({ navigation }: ConnectionsScreenProps) {
  const { isAuthenticated } = useAuth();
  const [providers, setProviders] = useState<ExtendedProviderStatus[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);

  const loadConnectionStatus = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await oauthApi.getProvidersStatus();
      setProviders(response.providers || []);
    } catch (error) {
      console.error('Failed to load connection status:', error);
      // Don't show alert on auth errors - screen will reload when auth is ready
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isAuthenticated) {
      loadConnectionStatus();
    }
  }, [isAuthenticated, loadConnectionStatus]);

  const handleConnect = async (providerId: string, providerName: string) => {
    try {
      setConnectingProvider(providerId);

      // Create return URL for the mobile app (deep link)
      // Server will redirect to this URL after OAuth completes
      // Uses custom scheme (pierre://) for consistent behavior in dev and prod
      const returnUrl = getOAuthCallbackUrl();

      // Call the mobile OAuth init endpoint which returns the authorization URL
      // and includes the redirect URL in the OAuth state for callback handling
      const oauthResponse = await oauthApi.initMobileOAuth(providerId, returnUrl);

      // Open OAuth in an in-app browser (ASWebAuthenticationSession on iOS)
      // The returnUrl is watched for redirects to close the browser automatically
      const result = await WebBrowser.openAuthSessionAsync(
        oauthResponse.authorization_url,
        returnUrl
      );

      if (result.type === 'success' && result.url) {
        // Validate the callback URL matches our expected scheme/host before processing
        const expectedPrefix = getOAuthCallbackUrl();
        if (!result.url.startsWith(expectedPrefix)) {
          console.error('OAuth callback URL does not match expected scheme:', result.url);
          Alert.alert('Connection Failed', 'Unexpected OAuth callback URL');
          return;
        }

        // Parse the return URL to check for success/error
        const parsedUrl = Linking.parse(result.url);
        const success = parsedUrl.queryParams?.success === 'true';
        const error = parsedUrl.queryParams?.error as string | undefined;

        if (success) {
          // OAuth completed successfully - refresh connection status
          await loadConnectionStatus();
          Alert.alert('Success', `Connected to ${providerName} successfully!`);
        } else if (error) {
          console.error('OAuth error from server:', error);
          Alert.alert('Connection Failed', `Failed to connect: ${error}`);
        } else {
          // No explicit success/error - refresh status to check
          await loadConnectionStatus();
          Alert.alert('Connection Complete', `${providerName} connection flow completed.`);
        }
      } else if (result.type === 'cancel') {
        console.log('OAuth cancelled by user');
      }
    } catch (error) {
      console.error('Failed to start OAuth flow:', error);
      Alert.alert('Error', 'Failed to start authentication. Please try again.');
    } finally {
      setConnectingProvider(null);
    }
  };

  const handleDisconnect = async (providerId: string, providerName: string) => {
    Alert.alert(
      `Disconnect ${providerName}`,
      `Are you sure you want to disconnect ${providerName}? You will need to reconnect to sync new data.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Disconnect',
          style: 'destructive',
          onPress: async () => {
            try {
              await oauthApi.disconnectProvider(providerId);
              await loadConnectionStatus();
              Alert.alert('Success', `${providerName} has been disconnected.`);
            } catch (error) {
              console.error('Failed to disconnect provider:', error);
              Alert.alert('Error', `Failed to disconnect ${providerName}. Please try again.`);
            }
          },
        },
      ]
    );
  };

  // Provider display config (colors, icons, descriptions)
  const getProviderConfig = (providerId: string) => {
    const configs: Record<string, { color: string; icon: string; description: string }> = {
      strava: { color: colors.providers.strava, icon: 'S', description: 'Running, cycling, and swimming activities' },
      garmin: { color: colors.providers.garmin, icon: 'G', description: 'Activities and health metrics from Garmin devices' },
      fitbit: { color: colors.providers.fitbit, icon: 'F', description: 'Activity, sleep, and heart rate data' },
      whoop: { color: colors.providers.whoop, icon: 'W', description: 'Recovery, strain, and sleep metrics' },
      terra: { color: colors.providers.terra, icon: 'T', description: 'Aggregate data from multiple fitness platforms' },
      coros: { color: '#E91E63', icon: 'C', description: 'Training and performance data from COROS devices' },
      synthetic: { color: '#9C27B0', icon: '🧪', description: 'Synthetic test data for development' },
      synthetic_sleep: { color: '#673AB7', icon: '😴', description: 'Synthetic sleep data for development' },
    };
    return configs[providerId] || { color: '#607D8B', icon: '?', description: 'Fitness data provider' };
  };

  const renderProvider = (provider: ExtendedProviderStatus) => {
    const config = getProviderConfig(provider.provider);
    const isConnected = provider.connected;
    const isConnecting = connectingProvider === provider.provider;
    const requiresOAuth = provider.requires_oauth;

    return (
      <Card key={provider.provider} className="mb-3">
        <View className="flex-row items-start mb-3">
          <View
            className="w-12 h-12 rounded-lg items-center justify-center mr-3"
            style={{ backgroundColor: config.color }}
          >
            <Text className="text-2xl font-bold text-text-primary">{config.icon}</Text>
          </View>
          <View className="flex-1">
            <Text className="text-lg font-semibold text-text-primary mb-0.5">{provider.display_name}</Text>
            <Text className="text-sm text-text-secondary leading-5">{config.description}</Text>
            {provider.capabilities.length > 0 && (
              <Text className="text-xs text-text-tertiary mt-1">
                Capabilities: {provider.capabilities.join(', ')}
              </Text>
            )}
          </View>
        </View>

        <View className="flex-row items-center justify-between">
          {isConnected ? (
            <>
              <View className="bg-success/20 px-2 py-1 rounded">
                <Text className="text-sm text-success font-medium">Connected</Text>
              </View>
              {requiresOAuth && (
                <TouchableOpacity
                  className="px-3 py-2"
                  onPress={() => handleDisconnect(provider.provider, provider.display_name)}
                >
                  <Text className="text-sm text-error font-medium">Disconnect</Text>
                </TouchableOpacity>
              )}
            </>
          ) : requiresOAuth ? (
            <TouchableOpacity
              className="flex-1 py-2 rounded-lg items-center"
              style={{ backgroundColor: config.color }}
              onPress={() => handleConnect(provider.provider, provider.display_name)}
              disabled={isConnecting}
            >
              {isConnecting ? (
                <ActivityIndicator size="small" color={colors.text.primary} />
              ) : (
                <Text className="text-base font-semibold text-text-primary">Connect</Text>
              )}
            </TouchableOpacity>
          ) : (
            <View className="bg-background-tertiary px-2 py-1 rounded">
              <Text className="text-sm text-text-tertiary font-medium">Not Available</Text>
            </View>
          )}
        </View>
      </Card>
    );
  };

  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      <DragIndicator testID="connections-drag-indicator" />
      {/* Header */}
      <View className="flex-row items-center px-3 py-2 border-b border-border-subtle">
        <TouchableOpacity
          className="w-10 h-10 items-center justify-center"
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-semibold text-text-primary text-center">Connections</Text>
        <View className="w-10" />
      </View>

      <ScrollView
        contentContainerStyle={{ padding: spacing.lg }}
        showsVerticalScrollIndicator={false}
      >
        <Text className="text-xl font-bold text-text-primary mb-1">Fitness Providers</Text>
        <Text className="text-base text-text-secondary mb-4 leading-[22px]">
          Connect your fitness accounts to sync activities, health metrics, and more.
        </Text>

        {isLoading ? (
          <View className="items-center py-12">
            <ActivityIndicator size="large" color={colors.primary[500]} />
            <Text className="mt-3 text-text-secondary text-base">Loading connections...</Text>
          </View>
        ) : (
          <View className="gap-3">
            {providers.map(renderProvider)}
          </View>
        )}

        {/* Privacy Note with glassmorphism */}
        <View className="rounded-xl overflow-hidden mt-6" style={{ ...glassCard, borderRadius: 16 }}>
          <LinearGradient
            colors={gradients.violetCyan as [string, string]}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 0 }}
            style={{ height: 3, width: '100%' }}
          />
          <View className="p-4">
            <Text className="text-sm font-semibold text-text-primary mb-1">Privacy Note</Text>
            <Text className="text-sm text-text-secondary leading-5">
              Pierre only accesses the data you authorize. We never share your
              fitness data with third parties. You can disconnect any provider at
              any time.
            </Text>
          </View>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}
