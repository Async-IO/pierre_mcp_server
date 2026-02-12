// ABOUTME: Social and privacy settings screen for managing discoverability and sharing
// ABOUTME: Controls visibility, notifications, and default share preferences

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  Switch,
  ActivityIndicator,
  type ViewStyle,
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';
import { colors, spacing, glassCard } from '../../constants/theme';

type FeatherIconName = ComponentProps<typeof Feather>['name'];
import { socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { UserSocialSettings } from '../../types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;

// Glass card style with shadow (React Native shadows cannot use className)
const sectionCardStyle: ViewStyle = {
  borderRadius: 12,
  padding: spacing.md,
  ...glassCard,
};

interface SettingRowProps {
  icon: FeatherIconName;
  title: string;
  description: string;
  value: boolean;
  onValueChange: (value: boolean) => void;
  disabled?: boolean;
  testID?: string;
}

function SettingRow({ icon, title, description, value, onValueChange, disabled, testID }: SettingRowProps) {
  return (
    <View className="flex-row items-center py-2">
      <View
        className="w-10 h-10 rounded-full justify-center items-center mr-4"
        style={{ backgroundColor: colors.pierre.violet + '20' }}
      >
        <Feather name={icon} size={20} color={colors.pierre.violet} />
      </View>
      <View className="flex-1 mr-4">
        <Text className="text-text-primary text-base font-semibold">{title}</Text>
        <Text className="text-text-tertiary text-sm mt-0.5">{description}</Text>
      </View>
      <Switch
        testID={testID}
        value={value}
        onValueChange={onValueChange}
        trackColor={{ false: colors.background.tertiary, true: colors.pierre.violet + '60' }}
        thumbColor={value ? colors.pierre.violet : colors.text.tertiary}
        disabled={disabled}
      />
    </View>
  );
}

export function SocialSettingsScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [settings, setSettings] = useState<UserSocialSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadSettings = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoading(true);
      setError(null);
      const response = await socialApi.getSocialSettings();
      setSettings(response.settings);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load settings';
      setError(errorMessage);
      console.error('Failed to load social settings:', err);
    } finally {
      setIsLoading(false);
    }
  }, [isAuthenticated]);

  useFocusEffect(
    useCallback(() => {
      loadSettings();
    }, [loadSettings])
  );

  const updateSetting = <K extends keyof UserSocialSettings>(
    key: K,
    value: UserSocialSettings[K]
  ) => {
    if (!settings) return;
    setSettings({ ...settings, [key]: value });
    setHasChanges(true);
  };

  const updateNotification = (
    key: keyof UserSocialSettings['notifications'],
    value: boolean
  ) => {
    if (!settings) return;
    setSettings({
      ...settings,
      notifications: { ...settings.notifications, [key]: value },
    });
    setHasChanges(true);
  };

  const handleSave = async () => {
    if (!settings || !hasChanges) return;

    try {
      setIsSaving(true);
      setError(null);
      await socialApi.updateSocialSettings({
        discoverable: settings.discoverable,
        default_visibility: settings.default_visibility,
        notifications: settings.notifications,
      });
      setHasChanges(false);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to save settings';
      setError(errorMessage);
      console.error('Failed to save settings:', err);
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading || !settings) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text className="text-text-secondary mt-4">Loading settings...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="social-settings-screen">
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <TouchableOpacity
          className="p-2"
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-bold text-text-primary text-center">Social Settings</Text>
        <TouchableOpacity
          className={`px-4 py-2 rounded-md ${hasChanges ? '' : 'opacity-50'}`}
          style={{ backgroundColor: colors.pierre.violet }}
          onPress={handleSave}
          disabled={!hasChanges || isSaving}
          testID="save-button"
        >
          {isSaving ? (
            <ActivityIndicator size="small" color={colors.text.primary} />
          ) : (
            <Text className={`text-text-primary text-base font-semibold ${!hasChanges ? 'opacity-50' : ''}`}>
              Save
            </Text>
          )}
        </TouchableOpacity>
      </View>

      <ScrollView className="flex-1 px-4" showsVerticalScrollIndicator={false}>
        {/* Error Display */}
        {error && (
          <View className="mt-4 p-3 bg-error/10 border border-error/30 rounded-lg flex-row items-center justify-between">
            <Text className="flex-1 text-error text-sm mr-3">{error}</Text>
            <TouchableOpacity
              className="px-3 py-1.5 bg-error/20 rounded-md"
              onPress={() => {
                setError(null);
                loadSettings();
              }}
            >
              <Text className="text-error text-sm font-semibold">Retry</Text>
            </TouchableOpacity>
          </View>
        )}

        {/* Privacy Section */}
        <Text className="text-text-secondary text-sm font-semibold mt-6 mb-2 ml-2 uppercase tracking-wide">Privacy</Text>
        <View style={sectionCardStyle}>
          <SettingRow
            icon="eye"
            title="Discoverable"
            description="Allow others to find you when searching for friends"
            value={settings.discoverable}
            onValueChange={(value) => updateSetting('discoverable', value)}
            testID="discoverable-switch"
          />
        </View>

        {/* Default Sharing Section */}
        <Text className="text-text-secondary text-sm font-semibold mt-6 mb-2 ml-2 uppercase tracking-wide">Default Sharing</Text>
        <View style={sectionCardStyle}>
          <Text className="text-text-secondary text-sm mb-4">Default visibility for new insights</Text>
          <View className="flex-row gap-4">
            <TouchableOpacity
              className={`flex-1 flex-row items-center justify-center py-4 rounded-md gap-2 ${
                settings.default_visibility === 'friends_only' ? '' : 'bg-background-secondary'
              }`}
              style={
                settings.default_visibility === 'friends_only'
                  ? { backgroundColor: colors.pierre.violet + '20', borderWidth: 1, borderColor: colors.pierre.violet }
                  : undefined
              }
              onPress={() => updateSetting('default_visibility', 'friends_only')}
            >
              <Feather
                name="users"
                size={20}
                color={settings.default_visibility === 'friends_only' ? colors.pierre.violet : colors.text.tertiary}
              />
              <Text
                className="text-base font-medium"
                style={{ color: settings.default_visibility === 'friends_only' ? colors.pierre.violet : colors.text.tertiary }}
              >
                Friends Only
              </Text>
            </TouchableOpacity>
            <TouchableOpacity
              className={`flex-1 flex-row items-center justify-center py-4 rounded-md gap-2 ${
                settings.default_visibility === 'public' ? '' : 'bg-background-secondary'
              }`}
              style={
                settings.default_visibility === 'public'
                  ? { backgroundColor: colors.pierre.violet + '20', borderWidth: 1, borderColor: colors.pierre.violet }
                  : undefined
              }
              onPress={() => updateSetting('default_visibility', 'public')}
            >
              <Feather
                name="globe"
                size={20}
                color={settings.default_visibility === 'public' ? colors.pierre.violet : colors.text.tertiary}
              />
              <Text
                className="text-base font-medium"
                style={{ color: settings.default_visibility === 'public' ? colors.pierre.violet : colors.text.tertiary }}
              >
                Public
              </Text>
            </TouchableOpacity>
          </View>
        </View>

        {/* Notifications Section */}
        <Text className="text-text-secondary text-sm font-semibold mt-6 mb-2 ml-2 uppercase tracking-wide">Notifications</Text>
        <View style={sectionCardStyle}>
          <SettingRow
            icon="user-plus"
            title="Friend Requests"
            description="Get notified when someone sends you a friend request"
            value={settings.notifications.friend_requests}
            onValueChange={(value) => updateNotification('friend_requests', value)}
          />
          <View className="h-px bg-border-subtle my-2" />
          <SettingRow
            icon="heart"
            title="Reactions"
            description="Get notified when someone reacts to your insights"
            value={settings.notifications.insight_reactions}
            onValueChange={(value) => updateNotification('insight_reactions', value)}
          />
          <View className="h-px bg-border-subtle my-2" />
          <SettingRow
            icon="refresh-cw"
            title="Adapted Insights"
            description="Get notified when someone adapts your shared insight"
            value={settings.notifications.adapted_insights}
            onValueChange={(value) => updateNotification('adapted_insights', value)}
          />
        </View>

        {/* Privacy Info */}
        <View
          className="items-center p-5 mt-6 rounded-lg"
          style={{ backgroundColor: colors.pierre.violet + '10' }}
        >
          <Feather name="shield" size={24} color={colors.pierre.violet} />
          <Text className="text-text-primary text-base font-bold mt-4">Your Privacy is Protected</Text>
          <Text className="text-text-secondary text-sm text-center mt-2 leading-5">
            When you share insights, your private data is automatically sanitized.
            GPS coordinates, exact pace, recovery scores, and other sensitive
            information is never shared with friends.
          </Text>
        </View>

        <View className="h-6" />
      </ScrollView>
    </SafeAreaView>
  );
}
