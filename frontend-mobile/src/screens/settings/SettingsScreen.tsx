// ABOUTME: User settings screen with profile, MCP tokens, and account options
// ABOUTME: Allows password reset, token management, and logout

import React, { useState, useEffect } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  Alert,
  Modal,
  ActivityIndicator,
} from 'react-native';
import { colors, spacing, borderRadius } from '../../constants/theme';
import { Card, Button, Input } from '../../components/ui';
import { useAuth } from '../../contexts/AuthContext';
import { apiService } from '../../services/api';
import type { McpToken } from '../../types';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { SettingsStackParamList } from '../../navigation/MainTabs';
import { OAuthCredentialsSection } from '../../components/OAuthCredentialsSection';
import { Feather } from '@expo/vector-icons';

interface SettingsScreenProps {
  navigation: NativeStackNavigationProp<SettingsStackParamList>;
}

export function SettingsScreen({ navigation }: SettingsScreenProps) {
  const { user, logout, isAuthenticated } = useAuth();
  const [tokens, setTokens] = useState<McpToken[]>([]);
  const [isLoadingTokens, setIsLoadingTokens] = useState(false);
  const [showCreateToken, setShowCreateToken] = useState(false);
  const [showChangePassword, setShowChangePassword] = useState(false);
  const [newTokenName, setNewTokenName] = useState('');
  const [isCreatingToken, setIsCreatingToken] = useState(false);
  const [newToken, setNewToken] = useState<string | null>(null);

  // Password change state
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isChangingPassword, setIsChangingPassword] = useState(false);

  // Developer settings state
  const [showDeveloperSettings, setShowDeveloperSettings] = useState(false);

  useEffect(() => {
    if (isAuthenticated) {
      loadTokens();
    }
  }, [isAuthenticated]);

  const loadTokens = async () => {
    try {
      setIsLoadingTokens(true);
      const response = await apiService.getMcpTokens();
      const tokenList = response.tokens || [];
      // Deduplicate by ID and filter out revoked tokens
      const seen = new Set<string>();
      const deduplicated = tokenList.filter((t) => {
        if (t.is_revoked || seen.has(t.id)) return false;
        seen.add(t.id);
        return true;
      });
      setTokens(deduplicated);
    } catch (error) {
      console.error('Failed to load tokens:', error);
      setTokens([]);
    } finally {
      setIsLoadingTokens(false);
    }
  };

  const handleCreateToken = async () => {
    if (!newTokenName.trim()) {
      Alert.alert('Error', 'Please enter a token name');
      return;
    }

    try {
      setIsCreatingToken(true);
      const token = await apiService.createMcpToken({
        name: newTokenName.trim(),
        expires_in_days: 365,
      });
      setNewToken(token.token_value || 'Token created successfully');
      await loadTokens();
      setNewTokenName('');
    } catch {
      Alert.alert('Error', 'Failed to create token');
    } finally {
      setIsCreatingToken(false);
    }
  };

  const handleRevokeToken = (tokenId: string, tokenName: string) => {
    Alert.alert(
      'Revoke Token',
      `Are you sure you want to revoke "${tokenName}"? This action cannot be undone.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Revoke',
          style: 'destructive',
          onPress: async () => {
            try {
              await apiService.revokeMcpToken(tokenId);
              await loadTokens();
            } catch {
              Alert.alert('Error', 'Failed to revoke token');
            }
          },
        },
      ]
    );
  };

  const handleChangePassword = async () => {
    if (!currentPassword || !newPassword || !confirmPassword) {
      Alert.alert('Error', 'Please fill in all fields');
      return;
    }

    if (newPassword !== confirmPassword) {
      Alert.alert('Error', 'New passwords do not match');
      return;
    }

    if (newPassword.length < 8) {
      Alert.alert('Error', 'Password must be at least 8 characters');
      return;
    }

    try {
      setIsChangingPassword(true);
      await apiService.changePassword(currentPassword, newPassword);
      Alert.alert('Success', 'Password changed successfully');
      setShowChangePassword(false);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } catch {
      Alert.alert('Error', 'Failed to change password. Please check your current password.');
    } finally {
      setIsChangingPassword(false);
    }
  };

  const handleLogout = () => {
    Alert.alert(
      'Sign Out',
      'Are you sure you want to sign out?',
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Sign Out',
          style: 'destructive',
          onPress: logout,
        },
      ]
    );
  };

  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      {/* Header */}
      <View className="flex-row items-center px-3 py-2 border-b border-border-subtle">
        <View className="w-10" />
        <Text className="flex-1 text-lg font-semibold text-text-primary text-center">Settings</Text>
        <View className="w-10" />
      </View>

      <ScrollView contentContainerStyle={{ padding: spacing.lg }}>
        {/* Profile Section */}
        <Text className="text-lg font-semibold text-text-primary mb-2">Profile</Text>
        <Card className="mb-5">
          <View className="flex-row items-center">
            <View className="w-14 h-14 rounded-full bg-primary-600 items-center justify-center mr-3">
              <Text className="text-2xl font-bold text-text-primary">
                {user?.display_name?.[0]?.toUpperCase() || user?.email?.[0]?.toUpperCase() || 'U'}
              </Text>
            </View>
            <View className="flex-1">
              <Text className="text-lg font-semibold text-text-primary">
                {user?.display_name || 'User'}
              </Text>
              <Text className="text-sm text-text-secondary">{user?.email}</Text>
            </View>
          </View>
        </Card>

        {/* Security Section */}
        <Text className="text-lg font-semibold text-text-primary mb-2">Security</Text>
        <Card className="mb-5">
          <TouchableOpacity
            className="flex-row justify-between items-center py-2"
            onPress={() => setShowChangePassword(true)}
          >
            <Text className="text-base text-text-primary">Change Password</Text>
            <Text className="text-lg text-text-tertiary">{'>'}</Text>
          </TouchableOpacity>
        </Card>

        {/* Connected Services Section */}
        <Text className="text-lg font-semibold text-text-primary mb-2">Connected Services</Text>
        <Card className="mb-5">
          <TouchableOpacity
            className="flex-row justify-between items-center py-2"
            onPress={() => navigation.navigate('Connections')}
            testID="connect-providers-button"
          >
            <View className="flex-row items-center">
              <Feather name="link" size={20} color={colors.text.secondary} style={{ marginRight: 12 }} />
              <View>
                <Text className="text-base text-text-primary">Connect Providers</Text>
                <Text className="text-sm text-text-secondary">Link Strava, Garmin, and more</Text>
              </View>
            </View>
            <Text className="text-lg text-text-tertiary">{'>'}</Text>
          </TouchableOpacity>
        </Card>

        {/* MCP Tokens Section */}
        <View className="flex-row justify-between items-center mb-2">
          <Text className="text-lg font-semibold text-text-primary">MCP Tokens</Text>
          <TouchableOpacity
            className="px-2 py-1"
            onPress={() => setShowCreateToken(true)}
          >
            <Text className="text-sm font-semibold text-primary-500">+ New</Text>
          </TouchableOpacity>
        </View>
        <Card className="mb-5">
          {isLoadingTokens ? (
            <ActivityIndicator size="small" color={colors.primary[500]} />
          ) : tokens.length === 0 ? (
            <Text className="text-sm text-text-secondary text-center py-3">
              No MCP tokens created yet
            </Text>
          ) : (
            tokens.map((token, index) => (
              <View
                key={`${token.id}-${index}`}
                className={`flex-row justify-between items-center py-2 ${
                  index > 0 ? 'border-t border-border-subtle' : ''
                }`}
              >
                <View className="flex-1">
                  <Text className="text-base font-medium text-text-primary">{token.name}</Text>
                  <Text className="text-sm text-text-tertiary font-mono">{token.token_prefix}...</Text>
                  {token.last_used_at && (
                    <Text className="text-xs text-text-tertiary mt-0.5">
                      Last used: {new Date(token.last_used_at).toLocaleDateString()}
                    </Text>
                  )}
                </View>
                <TouchableOpacity
                  onPress={() => handleRevokeToken(token.id, token.name)}
                >
                  <Text className="text-sm font-medium text-error">Revoke</Text>
                </TouchableOpacity>
              </View>
            ))
          )}
        </Card>

        {/* Developer Settings Section */}
        <TouchableOpacity
          className="flex-row justify-between items-center py-3 mt-3 border-t border-border-subtle"
          onPress={() => setShowDeveloperSettings(!showDeveloperSettings)}
        >
          <Text className="text-lg font-semibold text-text-primary">Developer Settings</Text>
          <Text className="text-lg text-text-tertiary">{showDeveloperSettings ? '▼' : '>'}</Text>
        </TouchableOpacity>

        {showDeveloperSettings && (
          <OAuthCredentialsSection />
        )}

        {/* Logout */}
        <Button
          title="Sign Out"
          onPress={handleLogout}
          variant="danger"
          fullWidth
          style={{ marginTop: spacing.lg }}
        />
      </ScrollView>

      {/* Create Token Modal */}
      <Modal
        visible={showCreateToken}
        animationType="slide"
        transparent
        onRequestClose={() => setShowCreateToken(false)}
      >
        <View
          className="flex-1 bg-black/70 justify-center"
          style={{ paddingHorizontal: spacing.lg }}
        >
          <View
            className="bg-background-secondary p-5"
            style={{ borderRadius: borderRadius.xl }}
          >
            <Text className="text-xl font-semibold text-text-primary mb-5 text-center">
              {newToken ? 'Token Created' : 'Create MCP Token'}
            </Text>

            {newToken ? (
              <>
                <Text className="text-sm text-warning text-center mb-3">
                  Copy this token now. You won't be able to see it again!
                </Text>
                <View className="bg-background-tertiary rounded-lg p-3 mb-5">
                  <Text className="text-sm text-text-primary font-mono" selectable>
                    {newToken}
                  </Text>
                </View>
                <Button
                  title="Done"
                  onPress={() => {
                    setShowCreateToken(false);
                    setNewToken(null);
                  }}
                  fullWidth
                />
              </>
            ) : (
              <>
                <Input
                  label="Token Name"
                  placeholder="e.g., Claude Desktop"
                  value={newTokenName}
                  onChangeText={setNewTokenName}
                />
                <View className="flex-row gap-3 mt-3">
                  <Button
                    title="Cancel"
                    onPress={() => setShowCreateToken(false)}
                    variant="secondary"
                    style={{ flex: 1 }}
                  />
                  <Button
                    title="Create"
                    onPress={handleCreateToken}
                    loading={isCreatingToken}
                    style={{ flex: 1 }}
                  />
                </View>
              </>
            )}
          </View>
        </View>
      </Modal>

      {/* Change Password Modal */}
      <Modal
        visible={showChangePassword}
        animationType="slide"
        transparent
        onRequestClose={() => setShowChangePassword(false)}
      >
        <View
          className="flex-1 bg-black/70 justify-center"
          style={{ paddingHorizontal: spacing.lg }}
        >
          <View
            className="bg-background-secondary p-5"
            style={{ borderRadius: borderRadius.xl }}
          >
            <Text className="text-xl font-semibold text-text-primary mb-5 text-center">
              Change Password
            </Text>

            <Input
              label="Current Password"
              value={currentPassword}
              onChangeText={setCurrentPassword}
              secureTextEntry
              showPasswordToggle
            />
            <Input
              label="New Password"
              value={newPassword}
              onChangeText={setNewPassword}
              secureTextEntry
              showPasswordToggle
            />
            <Input
              label="Confirm New Password"
              value={confirmPassword}
              onChangeText={setConfirmPassword}
              secureTextEntry
              showPasswordToggle
            />

            <View className="flex-row gap-3 mt-3">
              <Button
                title="Cancel"
                onPress={() => setShowChangePassword(false)}
                variant="secondary"
                style={{ flex: 1 }}
              />
              <Button
                title="Change"
                onPress={handleChangePassword}
                loading={isChangingPassword}
                style={{ flex: 1 }}
              />
            </View>
          </View>
        </View>
      </Modal>
    </SafeAreaView>
  );
}
