// ABOUTME: Profile & Settings screen with Stitch UX design
// ABOUTME: Shows profile header, stats, connected services, and settings sections

import React, { useState, useEffect } from 'react';
import {
  View,
  Text,
  ScrollView,
  TouchableOpacity,
  Alert,
  Modal,
  ActivityIndicator,
  Switch,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { LinearGradient } from 'expo-linear-gradient';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, borderRadius } from '../../constants/theme';
import { Input } from '../../components/ui';
import { useAuth } from '../../contexts/AuthContext';
import { apiService } from '../../services/api';
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

  // Mock user stats - in production would come from API
  const [userStats] = useState({
    totalActivities: 127,
    activeDays: 89,
    currentStreak: 12,
  });

  useEffect(() => {
    if (isAuthenticated) {
      loadTokens();
      loadProviderStatus();
    }
  }, [isAuthenticated]);

  const loadTokens = async () => {
    try {
      const response = await apiService.getMcpTokens();
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
      const response = await apiService.getOAuthStatus();
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

  const isProviderConnected = (provider: string): boolean => {
    return connectedProviders.some(p => p.provider === provider && p.connected);
  };

  const displayName = user?.display_name || user?.email?.split('@')[0] || 'Athlete';

  return (
    <View className="flex-1 bg-pierre-dark">
      <ScrollView
        className="flex-1"
        contentContainerStyle={{
          paddingTop: insets.top + spacing.sm,
          paddingBottom: 100,
        }}
        showsVerticalScrollIndicator={false}
      >
        {/* Profile Header with gradient-bordered avatar */}
        <View className="items-center px-4 py-6">
          {/* Gradient-bordered Avatar */}
          <LinearGradient
            colors={[colors.pierre.violet, colors.pierre.cyan]}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 1 }}
            className="w-28 h-28 rounded-full items-center justify-center mb-4 p-1"
          >
            <View className="w-full h-full rounded-full bg-pierre-dark items-center justify-center">
              <Text className="text-4xl font-bold text-white">
                {displayName[0]?.toUpperCase() || 'U'}
              </Text>
            </View>
          </LinearGradient>

          <Text className="text-2xl font-bold text-white mb-1">{displayName}</Text>
          <Text className="text-base text-zinc-500 mb-4">{user?.email}</Text>

          {/* Edit Profile Button with violet glow */}
          <TouchableOpacity
            className="px-6 py-2.5 rounded-full"
            style={{
              backgroundColor: colors.pierre.violet,
              shadowColor: colors.pierre.violet,
              shadowOffset: { width: 0, height: 0 },
              shadowOpacity: 0.4,
              shadowRadius: 12,
              elevation: 6,
            }}
          >
            <Text className="text-sm font-semibold text-white">Edit Profile</Text>
          </TouchableOpacity>
        </View>

        {/* Stats Cards - horizontal with cyan accents */}
        <View className="flex-row px-4 gap-3 mb-6">
          <View style={glassCardStyle} className="flex-1 p-4 items-center">
            <Text className="text-2xl font-bold text-pierre-cyan">{userStats.totalActivities}</Text>
            <Text className="text-xs text-zinc-500 mt-1">Total Activities</Text>
          </View>
          <View style={glassCardStyle} className="flex-1 p-4 items-center">
            <Text className="text-2xl font-bold text-pierre-cyan">{userStats.activeDays}</Text>
            <Text className="text-xs text-zinc-500 mt-1">Active Days</Text>
          </View>
          <View style={glassCardStyle} className="flex-1 p-4 items-center">
            <Text className="text-2xl font-bold text-pierre-cyan">{userStats.currentStreak}</Text>
            <Text className="text-xs text-zinc-500 mt-1">Day Streak</Text>
          </View>
        </View>

        {/* Connected Services Section */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-3">Connected Services</Text>
          <View style={glassCardStyle}>
            {/* Strava */}
            <View style={settingsRowStyle} className="border-b border-white/5">
              <View className="w-10 h-10 rounded-xl items-center justify-center mr-3" style={{ backgroundColor: '#FC4C0220' }}>
                <Text className="text-lg">🏃</Text>
              </View>
              <View className="flex-1">
                <Text className="text-base font-medium text-white">Strava</Text>
                <View className="flex-row items-center mt-0.5">
                  <View
                    className="px-2 py-0.5 rounded-full mr-2"
                    style={{
                      backgroundColor: isProviderConnected('strava') ? 'rgba(74, 222, 128, 0.2)' : 'rgba(113, 113, 122, 0.2)',
                    }}
                  >
                    <Text
                      className="text-xs font-medium"
                      style={{ color: isProviderConnected('strava') ? colors.pierre.activity : colors.text.tertiary }}
                    >
                      {isProviderConnected('strava') ? 'Connected' : 'Disconnected'}
                    </Text>
                  </View>
                </View>
              </View>
              <Switch
                value={isProviderConnected('strava')}
                trackColor={{ false: '#3f3f46', true: colors.pierre.activity }}
                thumbColor="#ffffff"
                onValueChange={() => navigation.navigate('Connections')}
              />
            </View>

            {/* Garmin */}
            <View style={settingsRowStyle} className="border-b border-white/5">
              <View className="w-10 h-10 rounded-xl items-center justify-center mr-3" style={{ backgroundColor: '#007CC320' }}>
                <Text className="text-lg">⌚</Text>
              </View>
              <View className="flex-1">
                <Text className="text-base font-medium text-white">Garmin</Text>
                <View className="flex-row items-center mt-0.5">
                  <View
                    className="px-2 py-0.5 rounded-full mr-2"
                    style={{
                      backgroundColor: isProviderConnected('garmin') ? 'rgba(74, 222, 128, 0.2)' : 'rgba(113, 113, 122, 0.2)',
                    }}
                  >
                    <Text
                      className="text-xs font-medium"
                      style={{ color: isProviderConnected('garmin') ? colors.pierre.activity : colors.text.tertiary }}
                    >
                      {isProviderConnected('garmin') ? 'Connected' : 'Disconnected'}
                    </Text>
                  </View>
                </View>
              </View>
              <Switch
                value={isProviderConnected('garmin')}
                trackColor={{ false: '#3f3f46', true: colors.pierre.activity }}
                thumbColor="#ffffff"
                onValueChange={() => navigation.navigate('Connections')}
              />
            </View>

            {/* Apple Health */}
            <View style={settingsRowStyle}>
              <View className="w-10 h-10 rounded-xl items-center justify-center mr-3" style={{ backgroundColor: '#FF375F20' }}>
                <Text className="text-lg">❤️</Text>
              </View>
              <View className="flex-1">
                <Text className="text-base font-medium text-white">Apple Health</Text>
                <View className="flex-row items-center mt-0.5">
                  <View className="px-2 py-0.5 rounded-full mr-2" style={{ backgroundColor: 'rgba(113, 113, 122, 0.2)' }}>
                    <Text className="text-xs font-medium" style={{ color: colors.text.tertiary }}>Disconnected</Text>
                  </View>
                </View>
              </View>
              <Switch
                value={false}
                trackColor={{ false: '#3f3f46', true: colors.pierre.activity }}
                thumbColor="#ffffff"
              />
            </View>
          </View>
        </View>

        {/* Account Settings Section */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-3">Account</Text>
          <View style={glassCardStyle}>
            <TouchableOpacity style={settingsRowStyle} className="border-b border-white/5">
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="user" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Personal Information</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>

            <TouchableOpacity
              style={settingsRowStyle}
              className="border-b border-white/5"
              onPress={() => setShowChangePassword(true)}
            >
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="lock" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Change Password</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>

            <TouchableOpacity style={settingsRowStyle} onPress={() => setShowCreateToken(true)}>
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="key" size={20} color={colors.text.secondary} />
              </View>
              <View className="flex-1">
                <Text className="text-base text-white">MCP Tokens</Text>
                <Text className="text-sm text-zinc-500">{tokens.length} active</Text>
              </View>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>
        </View>

        {/* Notifications Section */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-3">Notifications</Text>
          <View style={glassCardStyle}>
            <View style={settingsRowStyle} className="border-b border-white/5">
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="bell" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Push Notifications</Text>
              <Switch
                value={true}
                trackColor={{ false: '#3f3f46', true: colors.pierre.violet }}
                thumbColor="#ffffff"
              />
            </View>

            <View style={settingsRowStyle}>
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="mail" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Email Updates</Text>
              <Switch
                value={false}
                trackColor={{ false: '#3f3f46', true: colors.pierre.violet }}
                thumbColor="#ffffff"
              />
            </View>
          </View>
        </View>

        {/* Privacy Section */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-3">Privacy</Text>
          <View style={glassCardStyle}>
            <TouchableOpacity style={settingsRowStyle} className="border-b border-white/5">
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="shield" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Privacy Settings</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>

            <TouchableOpacity style={settingsRowStyle}>
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="download" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Export Data</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>
        </View>

        {/* About Section */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-3">About</Text>
          <View style={glassCardStyle}>
            <TouchableOpacity style={settingsRowStyle} className="border-b border-white/5">
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="info" size={20} color={colors.text.secondary} />
              </View>
              <View className="flex-1">
                <Text className="text-base text-white">Version</Text>
                <Text className="text-sm text-zinc-500">1.0.0</Text>
              </View>
            </TouchableOpacity>

            <TouchableOpacity style={settingsRowStyle} className="border-b border-white/5">
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="help-circle" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Help Center</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>

            <TouchableOpacity style={settingsRowStyle}>
              <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                <Feather name="file-text" size={20} color={colors.text.secondary} />
              </View>
              <Text className="flex-1 text-base text-white">Terms & Privacy</Text>
              <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>
        </View>

        {/* Log Out Button - soft red */}
        <View className="px-4 mb-6">
          <TouchableOpacity
            style={[glassCardStyle, { borderColor: 'rgba(255, 107, 107, 0.3)' }]}
            className="py-4 items-center"
            onPress={handleLogout}
          >
            <Text className="text-base font-semibold" style={{ color: colors.pierre.red }}>Log Out</Text>
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
