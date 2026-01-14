// ABOUTME: OAuth credentials management section for Settings screen
// ABOUTME: Allows users to register custom OAuth app credentials for providers

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  Pressable,
  Alert,
  Modal,
  ActivityIndicator,
  FlatList,
} from 'react-native';
import { colors, spacing, fontSize, borderRadius } from '../constants/theme';
import { Card, Button, Input } from './ui';
import { apiService } from '../services/api';
import type { OAuthApp, OAuthProvider } from '../types';

const PROVIDERS: OAuthProvider[] = [
  { id: 'strava', name: 'Strava', color: '#FC4C02' },
  { id: 'fitbit', name: 'Fitbit', color: '#00B0B9' },
  { id: 'garmin', name: 'Garmin', color: '#007CC3' },
  { id: 'whoop', name: 'WHOOP', color: '#00D46A' },
  { id: 'terra', name: 'Terra', color: '#6366F1' },
];

const DEFAULT_REDIRECT_URI = 'https://pierre.fit/api/oauth/callback';

export function OAuthCredentialsSection() {
  const [oauthApps, setOauthApps] = useState<OAuthApp[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showAddModal, setShowAddModal] = useState(false);
  const [showProviderPicker, setShowProviderPicker] = useState(false);

  // Form state
  const [selectedProvider, setSelectedProvider] = useState<OAuthProvider | null>(null);
  const [clientId, setClientId] = useState('');
  const [clientSecret, setClientSecret] = useState('');
  const [redirectUri, setRedirectUri] = useState(DEFAULT_REDIRECT_URI);
  const [isSaving, setIsSaving] = useState(false);

  const loadOAuthApps = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await apiService.getUserOAuthApps();
      setOauthApps(response.apps || []);
    } catch (error) {
      console.error('Failed to load OAuth apps:', error);
      setOauthApps([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadOAuthApps();
  }, [loadOAuthApps]);

  const resetForm = () => {
    setSelectedProvider(null);
    setClientId('');
    setClientSecret('');
    setRedirectUri(DEFAULT_REDIRECT_URI);
  };

  const handleCloseModal = () => {
    setShowAddModal(false);
    resetForm();
  };

  const getAvailableProviders = (): OAuthProvider[] => {
    const configuredIds = oauthApps.map(app => app.provider.toLowerCase());
    return PROVIDERS.filter(p => !configuredIds.includes(p.id.toLowerCase()));
  };

  const handleSave = async () => {
    if (!selectedProvider) {
      Alert.alert('Error', 'Please select a provider');
      return;
    }
    if (!clientId.trim()) {
      Alert.alert('Error', 'Please enter a Client ID');
      return;
    }
    if (!clientSecret.trim()) {
      Alert.alert('Error', 'Please enter a Client Secret');
      return;
    }

    try {
      setIsSaving(true);
      await apiService.registerUserOAuthApp({
        provider: selectedProvider.id,
        client_id: clientId.trim(),
        client_secret: clientSecret.trim(),
        redirect_uri: redirectUri.trim() || `${DEFAULT_REDIRECT_URI}/${selectedProvider.id}`,
      });
      Alert.alert('Success', `${selectedProvider.name} credentials saved successfully`);
      handleCloseModal();
      await loadOAuthApps();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to save credentials';
      Alert.alert('Error', message);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = (provider: string, providerName: string) => {
    Alert.alert(
      'Remove Credentials',
      `Are you sure you want to remove ${providerName} credentials? You'll need to re-enter them to use a custom OAuth app.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Remove',
          style: 'destructive',
          onPress: async () => {
            try {
              await apiService.deleteUserOAuthApp(provider);
              await loadOAuthApps();
            } catch (error) {
              Alert.alert('Error', 'Failed to remove credentials');
            }
          },
        },
      ]
    );
  };

  const getProviderInfo = (providerId: string): OAuthProvider => {
    return PROVIDERS.find(p => p.id.toLowerCase() === providerId.toLowerCase()) || {
      id: providerId,
      name: providerId.charAt(0).toUpperCase() + providerId.slice(1),
      color: colors.primary[500],
    };
  };

  const maskClientId = (clientId: string): string => {
    if (clientId.length <= 8) return clientId;
    return `${clientId.substring(0, 8)}...`;
  };

  const availableProviders = getAvailableProviders();

  return (
    <View style={styles.container}>
      <View style={styles.sectionHeader}>
        <Text style={styles.sectionTitle}>OAuth Credentials</Text>
        {availableProviders.length > 0 && (
          <Pressable
            style={styles.addButton}
            onPress={() => setShowAddModal(true)}
          >
            <Text style={styles.addButtonText}>+ Add</Text>
          </Pressable>
        )}
      </View>

      <Text style={styles.description}>
        Configure custom OAuth app credentials to use your own developer applications instead of the server defaults.
      </Text>

      <Card style={styles.section}>
        {isLoading ? (
          <ActivityIndicator size="small" color={colors.primary[500]} />
        ) : oauthApps.length === 0 ? (
          <Text style={styles.emptyText}>No custom OAuth credentials configured</Text>
        ) : (
          oauthApps.map((app, index) => {
            const providerInfo = getProviderInfo(app.provider);
            return (
              <View
                key={app.provider}
                style={[styles.credentialItem, index > 0 && styles.itemBorder]}
              >
                <View style={styles.credentialRow}>
                  <View style={[styles.providerIcon, { backgroundColor: providerInfo.color }]}>
                    <Text style={styles.providerInitial}>
                      {providerInfo.name.charAt(0).toUpperCase()}
                    </Text>
                  </View>
                  <View style={styles.credentialInfo}>
                    <View style={styles.providerNameRow}>
                      <Text style={styles.providerName}>{providerInfo.name}</Text>
                      <View style={styles.configuredBadge}>
                        <Text style={styles.configuredText}>Configured</Text>
                      </View>
                    </View>
                    <Text style={styles.clientIdText}>
                      Client ID: {maskClientId(app.client_id)}
                    </Text>
                  </View>
                </View>
                <TouchableOpacity
                  onPress={() => handleDelete(app.provider, providerInfo.name)}
                >
                  <Text style={styles.removeText}>Remove</Text>
                </TouchableOpacity>
              </View>
            );
          })
        )}
      </Card>

      {/* Add Credentials Modal */}
      <Modal
        visible={showAddModal}
        animationType="slide"
        transparent
        onRequestClose={handleCloseModal}
      >
        <View style={styles.modalOverlay}>
          <View style={styles.modalContent}>
            <Text style={styles.modalTitle}>Add OAuth Credentials</Text>

            {/* Provider Picker */}
            <Text style={styles.inputLabel}>Provider</Text>
            <Pressable
              style={styles.pickerButton}
              onPress={() => setShowProviderPicker(true)}
            >
              {selectedProvider ? (
                <View style={styles.selectedProviderRow}>
                  <View style={[styles.providerIconSmall, { backgroundColor: selectedProvider.color }]}>
                    <Text style={styles.providerInitialSmall}>
                      {selectedProvider.name.charAt(0)}
                    </Text>
                  </View>
                  <Text style={styles.pickerButtonText}>{selectedProvider.name}</Text>
                </View>
              ) : (
                <Text style={styles.pickerPlaceholder}>Select a provider...</Text>
              )}
              <Text style={styles.pickerChevron}>{'>'}</Text>
            </Pressable>

            <Input
              label="Client ID"
              placeholder="Enter your OAuth client ID"
              value={clientId}
              onChangeText={setClientId}
              autoCapitalize="none"
              autoCorrect={false}
            />

            <Input
              label="Client Secret"
              placeholder="Enter your OAuth client secret"
              value={clientSecret}
              onChangeText={setClientSecret}
              secureTextEntry
              showPasswordToggle
              autoCapitalize="none"
              autoCorrect={false}
            />

            <Input
              label="Redirect URI"
              placeholder={DEFAULT_REDIRECT_URI}
              value={redirectUri}
              onChangeText={setRedirectUri}
              autoCapitalize="none"
              autoCorrect={false}
            />

            <View style={styles.modalActions}>
              <Button
                title="Cancel"
                onPress={handleCloseModal}
                variant="secondary"
                style={styles.modalButton}
              />
              <Button
                title="Save"
                onPress={handleSave}
                loading={isSaving}
                style={styles.modalButton}
              />
            </View>
          </View>
        </View>
      </Modal>

      {/* Provider Picker Modal */}
      <Modal
        visible={showProviderPicker}
        animationType="slide"
        transparent
        onRequestClose={() => setShowProviderPicker(false)}
      >
        <View style={styles.modalOverlay}>
          <View style={styles.pickerModalContent}>
            <Text style={styles.modalTitle}>Select Provider</Text>
            <FlatList
              data={availableProviders}
              keyExtractor={(item) => item.id}
              renderItem={({ item }) => (
                <Pressable
                  style={styles.providerOption}
                  onPress={() => {
                    setSelectedProvider(item);
                    setRedirectUri(`${DEFAULT_REDIRECT_URI}/${item.id}`);
                    setShowProviderPicker(false);
                  }}
                >
                  <View style={[styles.providerIcon, { backgroundColor: item.color }]}>
                    <Text style={styles.providerInitial}>{item.name.charAt(0)}</Text>
                  </View>
                  <Text style={styles.providerOptionText}>{item.name}</Text>
                  {selectedProvider?.id === item.id && (
                    <Text style={styles.checkmark}>{'checkmark'}</Text>
                  )}
                </Pressable>
              )}
              ItemSeparatorComponent={() => <View style={styles.separator} />}
            />
            <Button
              title="Cancel"
              onPress={() => setShowProviderPicker(false)}
              variant="secondary"
              fullWidth
              style={styles.pickerCancelButton}
            />
          </View>
        </View>
      </Modal>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    marginTop: spacing.md,
  },
  sectionHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: spacing.xs,
  },
  sectionTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
  },
  addButton: {
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xs,
  },
  addButtonText: {
    color: colors.primary[500],
    fontSize: fontSize.sm,
    fontWeight: '600',
  },
  description: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    marginBottom: spacing.md,
  },
  section: {
    marginBottom: spacing.md,
  },
  emptyText: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    textAlign: 'center',
    paddingVertical: spacing.md,
  },
  credentialItem: {
    paddingVertical: spacing.sm,
  },
  itemBorder: {
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  credentialRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: spacing.xs,
  },
  providerIcon: {
    width: 40,
    height: 40,
    borderRadius: 8,
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.md,
  },
  providerIconSmall: {
    width: 24,
    height: 24,
    borderRadius: 4,
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.sm,
  },
  providerInitial: {
    fontSize: fontSize.lg,
    fontWeight: '700',
    color: '#FFFFFF',
  },
  providerInitialSmall: {
    fontSize: fontSize.sm,
    fontWeight: '700',
    color: '#FFFFFF',
  },
  credentialInfo: {
    flex: 1,
  },
  providerNameRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 2,
  },
  providerName: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
    marginRight: spacing.sm,
  },
  configuredBadge: {
    backgroundColor: colors.success + '20',
    paddingHorizontal: spacing.xs,
    paddingVertical: 2,
    borderRadius: borderRadius.sm,
  },
  configuredText: {
    fontSize: fontSize.xs,
    color: colors.success,
    fontWeight: '500',
  },
  clientIdText: {
    fontSize: fontSize.sm,
    color: colors.text.tertiary,
    fontFamily: 'monospace',
  },
  removeText: {
    fontSize: fontSize.sm,
    color: colors.error,
    fontWeight: '500',
  },
  modalOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.7)',
    justifyContent: 'center',
    paddingHorizontal: spacing.lg,
  },
  modalContent: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.xl,
    padding: spacing.lg,
    maxHeight: '80%',
  },
  pickerModalContent: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.xl,
    padding: spacing.lg,
    maxHeight: '60%',
  },
  modalTitle: {
    fontSize: fontSize.xl,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.lg,
    textAlign: 'center',
  },
  inputLabel: {
    fontSize: fontSize.sm,
    fontWeight: '500',
    color: colors.text.secondary,
    marginBottom: spacing.xs,
  },
  pickerButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: colors.background.tertiary,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    marginBottom: spacing.md,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  selectedProviderRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  pickerButtonText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  pickerPlaceholder: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
  },
  pickerChevron: {
    fontSize: fontSize.lg,
    color: colors.text.tertiary,
  },
  modalActions: {
    flexDirection: 'row',
    gap: spacing.md,
    marginTop: spacing.md,
  },
  modalButton: {
    flex: 1,
  },
  providerOption: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: spacing.md,
  },
  providerOptionText: {
    flex: 1,
    fontSize: fontSize.md,
    color: colors.text.primary,
    marginLeft: spacing.sm,
  },
  checkmark: {
    fontSize: fontSize.lg,
    color: colors.primary[500],
  },
  separator: {
    height: 1,
    backgroundColor: colors.border.subtle,
  },
  pickerCancelButton: {
    marginTop: spacing.md,
  },
});
