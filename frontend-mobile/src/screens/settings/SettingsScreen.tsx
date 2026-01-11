// ABOUTME: User settings screen with profile, MCP tokens, and account options
// ABOUTME: Allows password reset, token management, and logout

import React, { useState, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  Alert,
  TextInput,
  Modal,
  ActivityIndicator,
} from 'react-native';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { Card, Button, Input } from '../../components/ui';
import { useAuth } from '../../contexts/AuthContext';
import { apiService } from '../../services/api';
import type { McpToken } from '../../types';
import type { DrawerNavigationProp } from '@react-navigation/drawer';

interface SettingsScreenProps {
  navigation: DrawerNavigationProp<Record<string, undefined>>;
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
    } catch (error) {
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
            } catch (error) {
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
    } catch (error) {
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
    <SafeAreaView style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
        >
          <Text style={styles.menuIcon}>{'☰'}</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Settings</Text>
        <View style={styles.headerSpacer} />
      </View>

      <ScrollView contentContainerStyle={styles.scrollContent}>
        {/* Profile Section */}
        <Text style={styles.sectionTitle}>Profile</Text>
        <Card style={styles.profileCard}>
          <View style={styles.profileHeader}>
            <View style={styles.avatar}>
              <Text style={styles.avatarText}>
                {user?.display_name?.[0]?.toUpperCase() || user?.email?.[0]?.toUpperCase() || 'U'}
              </Text>
            </View>
            <View style={styles.profileInfo}>
              <Text style={styles.displayName}>
                {user?.display_name || 'User'}
              </Text>
              <Text style={styles.email}>{user?.email}</Text>
            </View>
          </View>
        </Card>

        {/* Security Section */}
        <Text style={styles.sectionTitle}>Security</Text>
        <Card style={styles.section}>
          <TouchableOpacity
            style={styles.menuItem}
            onPress={() => setShowChangePassword(true)}
          >
            <Text style={styles.menuItemText}>Change Password</Text>
            <Text style={styles.chevron}>{'>'}</Text>
          </TouchableOpacity>
        </Card>

        {/* MCP Tokens Section */}
        <View style={styles.sectionHeader}>
          <Text style={styles.sectionTitle}>MCP Tokens</Text>
          <TouchableOpacity
            style={styles.addButton}
            onPress={() => setShowCreateToken(true)}
          >
            <Text style={styles.addButtonText}>+ New</Text>
          </TouchableOpacity>
        </View>
        <Card style={styles.section}>
          {isLoadingTokens ? (
            <ActivityIndicator size="small" color={colors.primary[500]} />
          ) : tokens.length === 0 ? (
            <Text style={styles.emptyText}>No MCP tokens created yet</Text>
          ) : (
            tokens.map((token, index) => (
              <View
                key={`${token.id}-${index}`}
                style={[styles.tokenItem, index > 0 && styles.tokenBorder]}
              >
                <View style={styles.tokenInfo}>
                  <Text style={styles.tokenName}>{token.name}</Text>
                  <Text style={styles.tokenPrefix}>{token.token_prefix}...</Text>
                  {token.last_used_at && (
                    <Text style={styles.tokenUsage}>
                      Last used: {new Date(token.last_used_at).toLocaleDateString()}
                    </Text>
                  )}
                </View>
                <TouchableOpacity
                  onPress={() => handleRevokeToken(token.id, token.name)}
                >
                  <Text style={styles.revokeText}>Revoke</Text>
                </TouchableOpacity>
              </View>
            ))
          )}
        </Card>

        {/* Logout */}
        <Button
          title="Sign Out"
          onPress={handleLogout}
          variant="danger"
          fullWidth
          style={styles.logoutButton}
        />
      </ScrollView>

      {/* Create Token Modal */}
      <Modal
        visible={showCreateToken}
        animationType="slide"
        transparent
        onRequestClose={() => setShowCreateToken(false)}
      >
        <View style={styles.modalOverlay}>
          <View style={styles.modalContent}>
            <Text style={styles.modalTitle}>
              {newToken ? 'Token Created' : 'Create MCP Token'}
            </Text>

            {newToken ? (
              <>
                <Text style={styles.tokenWarning}>
                  Copy this token now. You won't be able to see it again!
                </Text>
                <View style={styles.tokenDisplay}>
                  <Text style={styles.tokenValue} selectable>
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
                <View style={styles.modalActions}>
                  <Button
                    title="Cancel"
                    onPress={() => setShowCreateToken(false)}
                    variant="secondary"
                    style={styles.modalButton}
                  />
                  <Button
                    title="Create"
                    onPress={handleCreateToken}
                    loading={isCreatingToken}
                    style={styles.modalButton}
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
        <View style={styles.modalOverlay}>
          <View style={styles.modalContent}>
            <Text style={styles.modalTitle}>Change Password</Text>

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

            <View style={styles.modalActions}>
              <Button
                title="Cancel"
                onPress={() => setShowChangePassword(false)}
                variant="secondary"
                style={styles.modalButton}
              />
              <Button
                title="Change"
                onPress={handleChangePassword}
                loading={isChangingPassword}
                style={styles.modalButton}
              />
            </View>
          </View>
        </View>
      </Modal>
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
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.sm,
  },
  sectionHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: spacing.sm,
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
  section: {
    marginBottom: spacing.lg,
  },
  profileCard: {
    marginBottom: spacing.lg,
  },
  profileHeader: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  avatar: {
    width: 56,
    height: 56,
    borderRadius: 28,
    backgroundColor: colors.primary[600],
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.md,
  },
  avatarText: {
    fontSize: 24,
    fontWeight: '700',
    color: colors.text.primary,
  },
  profileInfo: {
    flex: 1,
  },
  displayName: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
  },
  email: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  menuItem: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: spacing.sm,
  },
  menuItemText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  chevron: {
    fontSize: fontSize.lg,
    color: colors.text.tertiary,
  },
  emptyText: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    textAlign: 'center',
    paddingVertical: spacing.md,
  },
  tokenItem: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: spacing.sm,
  },
  tokenBorder: {
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  tokenInfo: {
    flex: 1,
  },
  tokenName: {
    fontSize: fontSize.md,
    fontWeight: '500',
    color: colors.text.primary,
  },
  tokenPrefix: {
    fontSize: fontSize.sm,
    color: colors.text.tertiary,
    fontFamily: 'monospace',
  },
  tokenUsage: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
    marginTop: 2,
  },
  revokeText: {
    fontSize: fontSize.sm,
    color: colors.error,
    fontWeight: '500',
  },
  logoutButton: {
    marginTop: spacing.lg,
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
  },
  modalTitle: {
    fontSize: fontSize.xl,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.lg,
    textAlign: 'center',
  },
  modalActions: {
    flexDirection: 'row',
    gap: spacing.md,
    marginTop: spacing.md,
  },
  modalButton: {
    flex: 1,
  },
  tokenWarning: {
    fontSize: fontSize.sm,
    color: colors.warning,
    textAlign: 'center',
    marginBottom: spacing.md,
  },
  tokenDisplay: {
    backgroundColor: colors.background.tertiary,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    marginBottom: spacing.lg,
  },
  tokenValue: {
    fontSize: fontSize.sm,
    color: colors.text.primary,
    fontFamily: 'monospace',
  },
});
