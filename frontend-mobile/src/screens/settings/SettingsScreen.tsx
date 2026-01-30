// ABOUTME: Profile & Settings screen with Stitch UX design
// ABOUTME: Shows profile header, stats, connected services, and settings sections

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  ScrollView,
  TouchableOpacity,
  Alert,
  Modal,
  ActivityIndicator,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useFocusEffect } from '@react-navigation/native';
import { LinearGradient } from 'expo-linear-gradient';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, borderRadius } from '../../constants/theme';
import { Input } from '../../components/ui';
import { useAuth } from '../../contexts/AuthContext';
import { userApi, oauthApi } from '../../services/api';
import type { McpToken, ProviderStatus } from '../../types';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { SettingsStackParamList } from '../../navigation/MainTabs';

interface SettingsScreenProps {
  navigation: NativeStackNavigationProp<SettingsStackParamList>;
}

// Glassmorphism card style
const glassCardStyle: ViewStyle = {
  backgroundColor: 'rgba(255, 255, 255, 0.05)',
  borderWidth: 1,
  borderColor: 'rgba(255, 255, 255, 0.1)',
  borderRadius: 16,
};

// Settings row style
const settingsRowStyle: ViewStyle = {
  flexDirection: 'row',
  alignItems: 'center',
  paddingVertical: 14,
  paddingHorizontal: 16,
};

export function SettingsScreen({ navigation }: SettingsScreenProps) {
  const { user, logout, isAuthenticated } = useAuth();
  const insets = useSafeAreaInsets();
  const [tokens, setTokens] = useState<McpToken[]>([]);
  const [showCreateToken, setShowCreateToken] = useState(false);
  const [showChangePassword, setShowChangePassword] = useState(false);
  const [newTokenName, setNewTokenName] = useState('');
  const [isCreatingToken, setIsCreatingToken] = useState(false);
  const [newToken, setNewToken] = useState<string | null>(null);
  const [connectedProviders, setConnectedProviders] = useState<ProviderStatus[]>([]);

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

  // Reload provider status when screen comes into focus (e.g., after OAuth connection)
  useFocusEffect(
    useCallback(() => {
      if (isAuthenticated) {
        loadProviderStatus();
      }
    }, [isAuthenticated])
  );

  const loadTokens = async () => {
    try {
      const response = await userApi.getMcpTokens();
      const tokenList = response.tokens || [];
      const seen = new Set<string>();
      const deduplicated = tokenList.filter((t: { id: string; is_revoked: boolean }) => {
        if (t.is_revoked || seen.has(t.id)) return false;
        seen.add(t.id);
        return true;
      });
      setTokens(deduplicated);
    } catch (error) {
      console.error('Failed to load tokens:', error);
      setTokens([]);
    }
  };

  const loadProviderStatus = async () => {
    try {
      const response = await oauthApi.getStatus();
      setConnectedProviders(response.providers || []);
    } catch (error) {
      console.error('Failed to load provider status:', error);
    }
  };

  const handleCreateToken = async () => {
    if (!newTokenName.trim()) {
      Alert.alert('Error', 'Please enter a token name');
      return;
    }

    try {
      setIsCreatingToken(true);
      const token = await userApi.createMcpToken({
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
      await userApi.changePassword(currentPassword, newPassword);
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

  const displayName = user?.display_name || user?.email?.split('@')[0] || 'Athlete';

  return (
    <View style={{ flex: 1, backgroundColor: colors.background.primary }} testID="settings-screen">
      <ScrollView
        style={{ flex: 1 }}
        contentContainerStyle={{
          paddingTop: insets.top + spacing.sm,
          paddingBottom: 100,
          paddingHorizontal: spacing.md,
        }}
        showsVerticalScrollIndicator={false}
      >
        {/* Profile Header with gradient-bordered avatar */}
        <View style={{ alignItems: 'center', paddingHorizontal: 16, paddingVertical: 24 }} testID="settings-profile-section">
          {/* Gradient-bordered Avatar */}
          <LinearGradient
            colors={[colors.pierre.violet, colors.pierre.cyan]}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 1 }}
            style={{
              width: 112,
              height: 112,
              borderRadius: 56,
              alignItems: 'center',
              justifyContent: 'center',
              marginBottom: 16,
              padding: 4,
            }}
          >
            <View style={{
              width: '100%',
              height: '100%',
              borderRadius: 56,
              backgroundColor: colors.background.primary,
              alignItems: 'center',
              justifyContent: 'center',
            }}>
              <Text style={{ fontSize: 36, fontWeight: 'bold', color: '#ffffff' }}>
                {displayName[0]?.toUpperCase() || 'U'}
              </Text>
            </View>
          </LinearGradient>

          <Text style={{ fontSize: 24, fontWeight: 'bold', color: '#ffffff', marginBottom: 4 }}>{displayName}</Text>
          <Text style={{ fontSize: 16, color: colors.text.tertiary, marginBottom: 16 }}>{user?.email}</Text>

          {/* Edit Profile Button with violet glow */}
          <TouchableOpacity
            style={{
              paddingHorizontal: 24,
              paddingVertical: 10,
              borderRadius: 9999,
              backgroundColor: colors.pierre.violet,
              shadowColor: colors.pierre.violet,
              shadowOffset: { width: 0, height: 0 },
              shadowOpacity: 0.4,
              shadowRadius: 12,
              elevation: 6,
            }}
          >
            <Text style={{ fontSize: 14, fontWeight: '600', color: '#ffffff' }}>Edit Profile</Text>
          </TouchableOpacity>
        </View>

        {/* Data Providers Section - navigates to Connections screen */}
        <View style={{ paddingHorizontal: 16, marginBottom: 24 }} testID="settings-data-section">
          <Text style={{ fontSize: 18, fontWeight: '600', color: '#ffffff', marginBottom: 12 }}>Data</Text>
          <View style={glassCardStyle}>
            <TouchableOpacity
              style={settingsRowStyle}
              onPress={() => navigation.navigate('Connections')}
              testID="settings-data-providers-button"
            >
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="link" size={20} color={colors.text.secondary} />
              </View>
              <View style={{ flex: 1 }}>
                <Text style={{ fontSize: 16, color: '#ffffff' }}>Data Providers</Text>
                <Text style={{ fontSize: 14, color: colors.text.tertiary }}>
                  {connectedProviders.filter(p => p.connected).length} connected
                </Text>
              </View>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>
        </View>

        {/* Account Settings Section */}
        <View style={{ paddingHorizontal: 16, marginBottom: 24 }} testID="settings-account-section">
          <Text style={{ fontSize: 18, fontWeight: '600', color: '#ffffff', marginBottom: 12 }}>Account</Text>
          <View style={glassCardStyle}>
            <TouchableOpacity style={[settingsRowStyle, { borderBottomWidth: 1, borderBottomColor: 'rgba(255, 255, 255, 0.05)' }]}>
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="user" size={20} color={colors.text.secondary} />
              </View>
              <Text style={{ flex: 1, fontSize: 16, color: '#ffffff' }}>Personal Information</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>

            <TouchableOpacity
              style={[settingsRowStyle, { borderBottomWidth: 1, borderBottomColor: 'rgba(255, 255, 255, 0.05)' }]}
              onPress={() => setShowChangePassword(true)}
            >
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="lock" size={20} color={colors.text.secondary} />
              </View>
              <Text style={{ flex: 1, fontSize: 16, color: '#ffffff' }}>Change Password</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>

            <TouchableOpacity style={settingsRowStyle} onPress={() => setShowCreateToken(true)}>
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="key" size={20} color={colors.text.secondary} />
              </View>
              <View style={{ flex: 1 }}>
                <Text style={{ fontSize: 16, color: '#ffffff' }}>MCP Tokens</Text>
                <Text style={{ fontSize: 14, color: colors.text.tertiary }}>{tokens.length} active</Text>
              </View>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>
        </View>

        {/* Privacy Section */}
        <View style={{ paddingHorizontal: 16, marginBottom: 24 }}>
          <Text style={{ fontSize: 18, fontWeight: '600', color: '#ffffff', marginBottom: 12 }}>Privacy</Text>
          <View style={glassCardStyle}>
            <TouchableOpacity style={settingsRowStyle}>
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="shield" size={20} color={colors.text.secondary} />
              </View>
              <Text style={{ flex: 1, fontSize: 16, color: '#ffffff' }}>Privacy Settings</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>
        </View>

        {/* About Section */}
        <View style={{ paddingHorizontal: 16, marginBottom: 24 }}>
          <Text style={{ fontSize: 18, fontWeight: '600', color: '#ffffff', marginBottom: 12 }}>About</Text>
          <View style={glassCardStyle}>
            <TouchableOpacity style={[settingsRowStyle, { borderBottomWidth: 1, borderBottomColor: 'rgba(255, 255, 255, 0.05)' }]}>
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="info" size={20} color={colors.text.secondary} />
              </View>
              <View style={{ flex: 1 }}>
                <Text style={{ fontSize: 16, color: '#ffffff' }}>Version</Text>
                <Text style={{ fontSize: 14, color: colors.text.tertiary }}>1.0.0</Text>
              </View>
            </TouchableOpacity>

            <TouchableOpacity style={[settingsRowStyle, { borderBottomWidth: 1, borderBottomColor: 'rgba(255, 255, 255, 0.05)' }]}>
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="help-circle" size={20} color={colors.text.secondary} />
              </View>
              <Text style={{ flex: 1, fontSize: 16, color: '#ffffff' }}>Help Center</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>

            <TouchableOpacity style={settingsRowStyle}>
              <View style={{ width: 40, height: 40, borderRadius: 12, backgroundColor: colors.background.secondary, alignItems: 'center', justifyContent: 'center', marginRight: 12 }}>
                <Feather name="file-text" size={20} color={colors.text.secondary} />
              </View>
              <Text style={{ flex: 1, fontSize: 16, color: '#ffffff' }}>Terms & Privacy</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>
        </View>

        {/* Log Out Button - soft red */}
        <View style={{ paddingHorizontal: 16, marginBottom: 24 }}>
          <TouchableOpacity
            style={[glassCardStyle, { borderColor: 'rgba(255, 107, 107, 0.3)', paddingVertical: 16, alignItems: 'center' }]}
            onPress={handleLogout}
            testID="settings-logout-button"
          >
            <Text style={{ fontSize: 16, fontWeight: '600', color: colors.pierre.red }}>Log Out</Text>
          </TouchableOpacity>
        </View>
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
            className="bg-pierre-slate p-5"
            style={{ borderRadius: borderRadius.xl }}
          >
            <Text className="text-xl font-semibold text-white mb-5 text-center">
              {newToken ? 'Token Created' : 'Create MCP Token'}
            </Text>

            {newToken ? (
              <>
                <Text className="text-sm text-amber-500 text-center mb-3">
                  Copy this token now. You won't be able to see it again!
                </Text>
                <View className="bg-pierre-dark rounded-lg p-3 mb-5">
                  <Text className="text-sm text-white font-mono" selectable>
                    {newToken}
                  </Text>
                </View>
                <TouchableOpacity
                  className="py-3 rounded-full items-center"
                  style={{ backgroundColor: colors.pierre.violet }}
                  onPress={() => {
                    setShowCreateToken(false);
                    setNewToken(null);
                  }}
                >
                  <Text className="text-base font-semibold text-white">Done</Text>
                </TouchableOpacity>
              </>
            ) : (
              <>
                <Input
                  label="Token Name"
                  placeholder="e.g., Claude Desktop"
                  value={newTokenName}
                  onChangeText={setNewTokenName}
                />
                <View className="flex-row gap-3 mt-4">
                  <TouchableOpacity
                    className="flex-1 py-3 rounded-full items-center"
                    style={{ backgroundColor: 'rgba(255, 255, 255, 0.1)' }}
                    onPress={() => setShowCreateToken(false)}
                  >
                    <Text className="text-base font-semibold text-white">Cancel</Text>
                  </TouchableOpacity>
                  <TouchableOpacity
                    className="flex-1 py-3 rounded-full items-center"
                    style={{ backgroundColor: colors.pierre.violet }}
                    onPress={handleCreateToken}
                    disabled={isCreatingToken}
                  >
                    {isCreatingToken ? (
                      <ActivityIndicator size="small" color="#ffffff" />
                    ) : (
                      <Text className="text-base font-semibold text-white">Create</Text>
                    )}
                  </TouchableOpacity>
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
            className="bg-pierre-slate p-5"
            style={{ borderRadius: borderRadius.xl }}
          >
            <Text className="text-xl font-semibold text-white mb-5 text-center">
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

            <View className="flex-row gap-3 mt-4">
              <TouchableOpacity
                className="flex-1 py-3 rounded-full items-center"
                style={{ backgroundColor: 'rgba(255, 255, 255, 0.1)' }}
                onPress={() => setShowChangePassword(false)}
              >
                <Text className="text-base font-semibold text-white">Cancel</Text>
              </TouchableOpacity>
              <TouchableOpacity
                className="flex-1 py-3 rounded-full items-center"
                style={{ backgroundColor: colors.pierre.violet }}
                onPress={handleChangePassword}
                disabled={isChangingPassword}
              >
                {isChangingPassword ? (
                  <ActivityIndicator size="small" color="#ffffff" />
                ) : (
                  <Text className="text-base font-semibold text-white">Change</Text>
                )}
              </TouchableOpacity>
            </View>
          </View>
        </View>
      </Modal>
    </View>
  );
}
