// ABOUTME: Provider connections screen for fitness data sources
// ABOUTME: Displays connection status and OAuth flow for Strava, Garmin, Fitbit, WHOOP, Terra

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
} from 'react-native';
import * as WebBrowser from 'expo-web-browser';
import * as Linking from 'expo-linking';
import { getOAuthCallbackUrl } from '../../utils/oauth';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { Card } from '../../components/ui';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { ProviderStatus } from '../../types';
import type { DrawerNavigationProp } from '@react-navigation/drawer';

interface ConnectionsScreenProps {
  navigation: DrawerNavigationProp<Record<string, undefined>>;
}

interface ProviderConfig {
  id: string;
  name: string;
  description: string;
  color: string;
  icon: string;
}

const PROVIDERS: ProviderConfig[] = [
  {
    id: 'strava',
    name: 'Strava',
    description: 'Running, cycling, and swimming activities',
    color: colors.providers.strava,
    icon: 'S',
  },
  {
    id: 'garmin',
    name: 'Garmin',
    description: 'Activities and health metrics from Garmin devices',
    color: colors.providers.garmin,
    icon: 'G',
  },
  {
    id: 'fitbit',
    name: 'Fitbit',
    description: 'Activity, sleep, and heart rate data',
    color: colors.providers.fitbit,
    icon: 'F',
  },
  {
    id: 'whoop',
    name: 'WHOOP',
    description: 'Recovery, strain, and sleep metrics',
    color: colors.providers.whoop,
    icon: 'W',
  },
  {
    id: 'terra',
    name: 'Terra',
    description: 'Aggregate data from multiple fitness platforms',
    color: colors.providers.terra,
    icon: 'T',
  },
];

export function ConnectionsScreen({ navigation }: ConnectionsScreenProps) {
  const { isAuthenticated } = useAuth();
  const [providerStatuses, setProviderStatuses] = useState<Map<string, ProviderStatus>>(new Map());
  const [isLoading, setIsLoading] = useState(true);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);

  const loadConnectionStatus = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await apiService.getOAuthStatus();
      const statusMap = new Map<string, ProviderStatus>();
      const providers = response.providers || [];
      providers.forEach((status) => {
        statusMap.set(status.provider, status);
      });
      setProviderStatuses(statusMap);
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
      const oauthResponse = await apiService.initMobileOAuth(providerId, returnUrl);

      // Open OAuth in an in-app browser (ASWebAuthenticationSession on iOS)
      // The returnUrl is watched for redirects to close the browser automatically
      const result = await WebBrowser.openAuthSessionAsync(
        oauthResponse.authorization_url,
        returnUrl
      );

      if (result.type === 'success' && result.url) {
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

  const handleDisconnect = (providerId: string, providerName: string) => {
    Alert.alert(
      `Disconnect ${providerName}`,
      `Are you sure you want to disconnect ${providerName}? You will need to reconnect to sync new data.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Disconnect',
          style: 'destructive',
          onPress: async () => {
            // TODO: Implement disconnect API call
            Alert.alert('Info', 'Disconnect functionality coming soon');
          },
        },
      ]
    );
  };

  const renderProvider = (provider: ProviderConfig) => {
    const status = providerStatuses.get(provider.id);
    const isConnected = status?.connected || false;
    const isConnecting = connectingProvider === provider.id;

    return (
      <Card key={provider.id} style={styles.providerCard}>
        <View style={styles.providerContent}>
          <View style={[styles.providerIcon, { backgroundColor: provider.color }]}>
            <Text style={styles.iconText}>{provider.icon}</Text>
          </View>
          <View style={styles.providerInfo}>
            <Text style={styles.providerName}>{provider.name}</Text>
            <Text style={styles.providerDescription}>{provider.description}</Text>
            {isConnected && status?.last_sync && (
              <Text style={styles.lastSync}>
                Last synced: {new Date(status.last_sync).toLocaleDateString()}
              </Text>
            )}
          </View>
        </View>

        <View style={styles.providerActions}>
          {isConnected ? (
            <>
              <View style={styles.connectedBadge}>
                <Text style={styles.connectedText}>Connected</Text>
              </View>
              <TouchableOpacity
                style={styles.disconnectButton}
                onPress={() => handleDisconnect(provider.id, provider.name)}
              >
                <Text style={styles.disconnectText}>Disconnect</Text>
              </TouchableOpacity>
            </>
          ) : (
            <TouchableOpacity
              style={[styles.connectButton, { backgroundColor: provider.color }]}
              onPress={() => handleConnect(provider.id, provider.name)}
              disabled={isConnecting}
            >
              {isConnecting ? (
                <ActivityIndicator size="small" color={colors.text.primary} />
              ) : (
                <Text style={styles.connectText}>Connect</Text>
              )}
            </TouchableOpacity>
          )}
        </View>
      </Card>
    );
  };

  return (
    <SafeAreaView style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
        >
          <Text style={styles.menuIcon}>{'☰'}</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Connections</Text>
        <View style={styles.headerSpacer} />
      </View>

      <ScrollView
        contentContainerStyle={styles.scrollContent}
        showsVerticalScrollIndicator={false}
      >
        <Text style={styles.sectionTitle}>Fitness Providers</Text>
        <Text style={styles.sectionDescription}>
          Connect your fitness accounts to sync activities, health metrics, and more.
        </Text>

        {isLoading ? (
          <View style={styles.loadingContainer}>
            <ActivityIndicator size="large" color={colors.primary[500]} />
            <Text style={styles.loadingText}>Loading connections...</Text>
          </View>
        ) : (
          <View style={styles.providersContainer}>
            {PROVIDERS.map(renderProvider)}
          </View>
        )}

        <View style={styles.infoBox}>
          <Text style={styles.infoTitle}>Privacy Note</Text>
          <Text style={styles.infoText}>
            Pierre only accesses the data you authorize. We never share your
            fitness data with third parties. You can disconnect any provider at
            any time.
          </Text>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  menuButton: {
    width: 40,
    height: 40,
    alignItems: 'center',
    justifyContent: 'center',
  },
  menuIcon: {
    fontSize: 20,
    color: colors.text.primary,
  },
  headerTitle: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
  },
  headerSpacer: {
    width: 40,
  },
  scrollContent: {
    padding: spacing.lg,
  },
  sectionTitle: {
    fontSize: fontSize.xl,
    fontWeight: '700',
    color: colors.text.primary,
    marginBottom: spacing.xs,
  },
  sectionDescription: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    marginBottom: spacing.lg,
    lineHeight: 22,
  },
  loadingContainer: {
    alignItems: 'center',
    paddingVertical: spacing.xxl,
  },
  loadingText: {
    marginTop: spacing.md,
    color: colors.text.secondary,
    fontSize: fontSize.md,
  },
  providersContainer: {
    gap: spacing.md,
  },
  providerCard: {
    marginBottom: spacing.md,
  },
  providerContent: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    marginBottom: spacing.md,
  },
  providerIcon: {
    width: 48,
    height: 48,
    borderRadius: borderRadius.lg,
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.md,
  },
  iconText: {
    fontSize: 24,
    fontWeight: '700',
    color: colors.text.primary,
  },
  providerInfo: {
    flex: 1,
  },
  providerName: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: 2,
  },
  providerDescription: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    lineHeight: 20,
  },
  lastSync: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
    marginTop: spacing.xs,
  },
  providerActions: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  connectedBadge: {
    backgroundColor: colors.success + '20',
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.sm,
  },
  connectedText: {
    fontSize: fontSize.sm,
    color: colors.success,
    fontWeight: '500',
  },
  disconnectButton: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
  },
  disconnectText: {
    fontSize: fontSize.sm,
    color: colors.error,
    fontWeight: '500',
  },
  connectButton: {
    flex: 1,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    alignItems: 'center',
  },
  connectText: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
  },
  infoBox: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.lg,
    padding: spacing.md,
    marginTop: spacing.xl,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  infoTitle: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.xs,
  },
  infoText: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    lineHeight: 20,
  },
});
