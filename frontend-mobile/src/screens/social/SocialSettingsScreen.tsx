// ABOUTME: Social and privacy settings screen for managing discoverability and sharing
// ABOUTME: Controls visibility, notifications, and default share preferences

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  Switch,
  ActivityIndicator,
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';
import { colors, spacing, fontSize, borderRadius, glassCard } from '../../constants/theme';

type FeatherIconName = ComponentProps<typeof Feather>['name'];
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { UserSocialSettings } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;

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
    <View style={styles.settingRow}>
      <View style={styles.settingIcon}>
        <Feather name={icon} size={20} color={colors.pierre.violet} />
      </View>
      <View style={styles.settingInfo}>
        <Text style={styles.settingTitle}>{title}</Text>
        <Text style={styles.settingDescription}>{description}</Text>
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

  const loadSettings = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoading(true);
      const response = await apiService.getSocialSettings();
      setSettings(response.settings);
    } catch (error) {
      console.error('Failed to load social settings:', error);
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
      await apiService.updateSocialSettings({
        discoverable: settings.discoverable,
        default_visibility: settings.default_visibility,
        notifications: settings.notifications,
      });
      setHasChanges(false);
    } catch (error) {
      console.error('Failed to save settings:', error);
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading || !settings) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text style={styles.loadingText}>Loading settings...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container} testID="social-settings-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.backButton}
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Social Settings</Text>
        <TouchableOpacity
          style={[styles.saveButton, !hasChanges && styles.saveButtonDisabled]}
          onPress={handleSave}
          disabled={!hasChanges || isSaving}
          testID="save-button"
        >
          {isSaving ? (
            <ActivityIndicator size="small" color={colors.text.primary} />
          ) : (
            <Text style={[styles.saveText, !hasChanges && styles.saveTextDisabled]}>
              Save
            </Text>
          )}
        </TouchableOpacity>
      </View>

      <ScrollView style={styles.scrollView} showsVerticalScrollIndicator={false}>
        {/* Privacy Section */}
        <Text style={styles.sectionTitle}>Privacy</Text>
        <View style={styles.sectionCard}>
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
        <Text style={styles.sectionTitle}>Default Sharing</Text>
        <View style={styles.sectionCard}>
          <Text style={styles.cardLabel}>Default visibility for new insights</Text>
          <View style={styles.visibilityOptions}>
            <TouchableOpacity
              style={[
                styles.visibilityOption,
                settings.default_visibility === 'friends_only' && styles.visibilityOptionActive,
              ]}
              onPress={() => {
                updateSetting('default_visibility', 'friends_only');
              }}
            >
              <Feather
                name="users"
                size={20}
                color={settings.default_visibility === 'friends_only'
                  ? colors.pierre.violet
                  : colors.text.tertiary}
              />
              <Text
                style={[
                  styles.visibilityText,
                  settings.default_visibility === 'friends_only' && styles.visibilityTextActive,
                ]}
              >
                Friends Only
              </Text>
            </TouchableOpacity>
            <TouchableOpacity
              style={[
                styles.visibilityOption,
                settings.default_visibility === 'public' && styles.visibilityOptionActive,
              ]}
              onPress={() => {
                updateSetting('default_visibility', 'public');
              }}
            >
              <Feather
                name="globe"
                size={20}
                color={settings.default_visibility === 'public'
                  ? colors.pierre.violet
                  : colors.text.tertiary}
              />
              <Text
                style={[
                  styles.visibilityText,
                  settings.default_visibility === 'public' && styles.visibilityTextActive,
                ]}
              >
                Public
              </Text>
            </TouchableOpacity>
          </View>
        </View>

        {/* Notifications Section */}
        <Text style={styles.sectionTitle}>Notifications</Text>
        <View style={styles.sectionCard}>
          <SettingRow
            icon="user-plus"
            title="Friend Requests"
            description="Get notified when someone sends you a friend request"
            value={settings.notifications.friend_requests}
            onValueChange={(value) => updateNotification('friend_requests', value)}
          />
          <View style={styles.settingDivider} />
          <SettingRow
            icon="heart"
            title="Reactions"
            description="Get notified when someone reacts to your insights"
            value={settings.notifications.insight_reactions}
            onValueChange={(value) => updateNotification('insight_reactions', value)}
          />
          <View style={styles.settingDivider} />
          <SettingRow
            icon="refresh-cw"
            title="Adapted Insights"
            description="Get notified when someone adapts your shared insight"
            value={settings.notifications.adapted_insights}
            onValueChange={(value) => updateNotification('adapted_insights', value)}
          />
        </View>

        {/* Privacy Info */}
        <View style={styles.privacyInfo}>
          <Feather name="shield" size={24} color={colors.pierre.violet} />
          <Text style={styles.privacyInfoTitle}>Your Privacy is Protected</Text>
          <Text style={styles.privacyInfoText}>
            When you share insights, your private data is automatically sanitized.
            GPS coordinates, exact pace, recovery scores, and other sensitive
            information is never shared with friends.
          </Text>
        </View>

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  loadingContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  loadingText: {
    color: colors.text.secondary,
    marginTop: spacing.md,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  backButton: {
    padding: spacing.sm,
  },
  headerTitle: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '700',
    color: colors.text.primary,
    textAlign: 'center',
  },
  saveButton: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    backgroundColor: colors.pierre.violet,
  },
  saveButtonDisabled: {
    opacity: 0.5,
  },
  saveText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  saveTextDisabled: {
    opacity: 0.5,
  },
  scrollView: {
    flex: 1,
    paddingHorizontal: spacing.md,
  },
  sectionTitle: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    fontWeight: '600',
    marginTop: spacing.xl,
    marginBottom: spacing.sm,
    marginLeft: spacing.sm,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  sectionCard: {
    borderRadius: borderRadius.lg,
    padding: spacing.md,
    ...glassCard,
  },
  cardLabel: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    marginBottom: spacing.md,
  },
  settingRow: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: spacing.sm,
  },
  settingIcon: {
    width: 40,
    height: 40,
    borderRadius: 20,
    backgroundColor: colors.pierre.violet + '20',
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: spacing.md,
  },
  settingInfo: {
    flex: 1,
    marginRight: spacing.md,
  },
  settingTitle: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  settingDescription: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
    marginTop: 2,
  },
  settingDivider: {
    height: 1,
    backgroundColor: colors.border.subtle,
    marginVertical: spacing.sm,
  },
  visibilityOptions: {
    flexDirection: 'row',
    gap: spacing.md,
  },
  visibilityOption: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: spacing.md,
    borderRadius: borderRadius.md,
    backgroundColor: colors.background.secondary,
    gap: spacing.sm,
  },
  visibilityOptionActive: {
    backgroundColor: colors.pierre.violet + '20',
    borderWidth: 1,
    borderColor: colors.pierre.violet,
  },
  visibilityText: {
    color: colors.text.tertiary,
    fontSize: fontSize.md,
    fontWeight: '500',
  },
  visibilityTextActive: {
    color: colors.pierre.violet,
  },
  privacyInfo: {
    alignItems: 'center',
    padding: spacing.lg,
    marginTop: spacing.xl,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.pierre.violet + '10',
  },
  privacyInfoTitle: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '700',
    marginTop: spacing.md,
  },
  privacyInfoText: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    textAlign: 'center',
    marginTop: spacing.sm,
    lineHeight: 20,
  },
  bottomSpacer: {
    height: spacing.xl,
  },
});
