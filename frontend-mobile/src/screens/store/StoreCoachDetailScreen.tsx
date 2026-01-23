// ABOUTME: Coach Store detail screen showing full coach info with install/uninstall actions
// ABOUTME: Displays system prompt preview, sample prompts, tags, and install count

import React, { useState, useCallback, useEffect } from 'react';
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
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import type { RouteProp } from '@react-navigation/native';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { StoreCoachDetail, CoachCategory } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

interface StoreCoachDetailScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
  route: RouteProp<AppDrawerParamList, 'StoreCoachDetail'>;
}

// Coach category colors
const COACH_CATEGORY_COLORS: Record<CoachCategory, string> = {
  training: '#10B981',
  nutrition: '#F59E0B',
  recovery: '#6366F1',
  recipes: '#F97316',
  mobility: '#EC4899',
  custom: '#7C3AED',
};

export function StoreCoachDetailScreen({ navigation, route }: StoreCoachDetailScreenProps) {
  const { coachId } = route.params;
  const { isAuthenticated } = useAuth();
  const [coach, setCoach] = useState<StoreCoachDetail | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isInstalling, setIsInstalling] = useState(false);
  const [isInstalled, setIsInstalled] = useState(false);

  const loadCoachDetail = useCallback(async () => {
    if (!isAuthenticated || !coachId) return;

    try {
      setIsLoading(true);
      const response = await apiService.getStoreCoach(coachId);
      setCoach(response);

      // Check if already installed
      const installations = await apiService.getInstalledCoaches();
      const installed = installations.coaches.some(
        (c) => c.id === coachId
      );
      setIsInstalled(installed);
    } catch (error) {
      console.error('Failed to load coach detail:', error);
      Alert.alert('Error', 'Failed to load coach details');
    } finally {
      setIsLoading(false);
    }
  }, [isAuthenticated, coachId]);

  useEffect(() => {
    loadCoachDetail();
  }, [loadCoachDetail]);

  const handleInstall = async () => {
    if (!coach) return;

    try {
      setIsInstalling(true);
      await apiService.installStoreCoach(coach.id);
      setIsInstalled(true);
      Alert.alert(
        'Installed!',
        `"${coach.title}" has been added to your coaches.`,
        [
          { text: 'View My Coaches', onPress: () => navigation.navigate('CoachLibrary') },
          { text: 'Stay Here', style: 'cancel' },
        ]
      );
    } catch (error) {
      console.error('Failed to install coach:', error);
      Alert.alert('Error', 'Failed to install coach. Please try again.');
    } finally {
      setIsInstalling(false);
    }
  };

  const handleUninstall = async () => {
    if (!coach) return;

    Alert.alert(
      'Uninstall Coach?',
      `Remove "${coach.title}" from your coaches? You can always reinstall it later.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Uninstall',
          style: 'destructive',
          onPress: async () => {
            try {
              setIsInstalling(true);
              await apiService.uninstallStoreCoach(coach.id);
              setIsInstalled(false);
              Alert.alert('Uninstalled', 'Coach has been removed from your library.');
            } catch (error) {
              console.error('Failed to uninstall coach:', error);
              Alert.alert('Error', 'Failed to uninstall coach. Please try again.');
            } finally {
              setIsInstalling(false);
            }
          },
        },
      ]
    );
  };

  if (isLoading) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.primary[500]} />
          <Text style={styles.loadingText}>Loading coach details...</Text>
        </View>
      </SafeAreaView>
    );
  }

  if (!coach) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.errorContainer}>
          <Text style={styles.errorText}>Coach not found</Text>
          <TouchableOpacity
            style={styles.backButton}
            onPress={() => navigation.navigate('Store')}
          >
            <Text style={styles.backButtonText}>Go Back</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  const categoryColor = COACH_CATEGORY_COLORS[coach.category];

  return (
    <SafeAreaView style={styles.container} testID="store-coach-detail-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          testID="back-button"
          style={styles.backArrow}
          onPress={() => navigation.navigate('Store')}
        >
          <Text style={styles.backArrowText}>←</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle} numberOfLines={1}>
          {coach.title}
        </Text>
        <View style={styles.headerSpacer} />
      </View>

      <ScrollView style={styles.content} showsVerticalScrollIndicator={false}>
        {/* Category & Stats */}
        <View style={styles.metaSection}>
          <View
            testID="category-badge"
            style={[
              styles.categoryBadge,
              { backgroundColor: categoryColor + '20' },
            ]}
          >
            <Text style={[styles.categoryBadgeText, { color: categoryColor }]}>
              {coach.category}
            </Text>
          </View>
          <Text testID="install-count" style={styles.installCount}>
            {coach.install_count} {coach.install_count === 1 ? 'install' : 'installs'}
          </Text>
        </View>

        {/* Title */}
        <Text testID="coach-title" style={styles.title}>{coach.title}</Text>

        {/* Description */}
        {coach.description && (
          <Text style={styles.description}>{coach.description}</Text>
        )}

        {/* Tags */}
        {coach.tags.length > 0 && (
          <View style={styles.section}>
            <Text style={styles.sectionTitle}>Tags</Text>
            <View style={styles.tagsContainer}>
              {coach.tags.map((tag, index) => (
                <View key={index} style={styles.tag}>
                  <Text style={styles.tagText}>{tag}</Text>
                </View>
              ))}
            </View>
          </View>
        )}

        {/* Sample Prompts */}
        {coach.sample_prompts.length > 0 && (
          <View style={styles.section}>
            <Text style={styles.sectionTitle}>Sample Prompts</Text>
            {coach.sample_prompts.map((prompt, index) => (
              <View key={index} style={styles.promptCard}>
                <Text style={styles.promptText}>{prompt}</Text>
              </View>
            ))}
          </View>
        )}

        {/* System Prompt Preview */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>System Prompt</Text>
          <View style={styles.systemPromptCard}>
            <Text style={styles.systemPromptText} numberOfLines={10}>
              {coach.system_prompt}
            </Text>
            {coach.system_prompt.length > 500 && (
              <Text style={styles.systemPromptMore}>
                ...and more ({coach.token_count} tokens)
              </Text>
            )}
          </View>
        </View>

        {/* Metadata */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Details</Text>
          <View style={styles.detailsCard}>
            <View style={styles.detailRow}>
              <Text style={styles.detailLabel}>Token Count</Text>
              <Text style={styles.detailValue}>{coach.token_count}</Text>
            </View>
            {coach.published_at && (
              <View style={styles.detailRow}>
                <Text style={styles.detailLabel}>Published</Text>
                <Text style={styles.detailValue}>
                  {new Date(coach.published_at).toLocaleDateString()}
                </Text>
              </View>
            )}
          </View>
        </View>

        {/* Bottom Spacer for Install Button */}
        <View style={styles.bottomSpacer} />
      </ScrollView>

      {/* Install/Uninstall Button - Fixed at bottom */}
      <View style={styles.actionBar}>
        {isInstalled ? (
          <TouchableOpacity
            style={[styles.actionButton, styles.uninstallButton]}
            onPress={handleUninstall}
            disabled={isInstalling}
          >
            {isInstalling ? (
              <ActivityIndicator size="small" color={colors.text.primary} />
            ) : (
              <Text style={styles.uninstallButtonText}>Uninstall</Text>
            )}
          </TouchableOpacity>
        ) : (
          <TouchableOpacity
            style={[styles.actionButton, styles.installButton]}
            onPress={handleInstall}
            disabled={isInstalling}
          >
            {isInstalling ? (
              <ActivityIndicator size="small" color={colors.text.primary} />
            ) : (
              <Text style={styles.installButtonText}>Install Coach</Text>
            )}
          </TouchableOpacity>
        )}
      </View>
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
    marginTop: spacing.md,
    color: colors.text.secondary,
    fontSize: fontSize.md,
  },
  errorContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: spacing.xl,
  },
  errorText: {
    fontSize: fontSize.lg,
    color: colors.text.secondary,
    marginBottom: spacing.md,
  },
  backButton: {
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
    backgroundColor: colors.primary[500],
    borderRadius: borderRadius.md,
  },
  backButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '500',
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  backArrow: {
    padding: spacing.sm,
  },
  backArrowText: {
    fontSize: 24,
    color: colors.text.primary,
  },
  headerTitle: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
    marginHorizontal: spacing.sm,
  },
  headerSpacer: {
    width: 40,
  },
  content: {
    flex: 1,
  },
  metaSection: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.lg,
    paddingBottom: spacing.sm,
  },
  categoryBadge: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
  },
  categoryBadgeText: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    textTransform: 'capitalize',
  },
  installCount: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  title: {
    fontSize: fontSize.xxl,
    fontWeight: '700',
    color: colors.text.primary,
    paddingHorizontal: spacing.lg,
    marginBottom: spacing.sm,
  },
  description: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    paddingHorizontal: spacing.lg,
    lineHeight: 22,
    marginBottom: spacing.md,
  },
  section: {
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
  },
  sectionTitle: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.secondary,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: spacing.sm,
  },
  tagsContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
  },
  tag: {
    backgroundColor: colors.background.secondary,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
    marginRight: spacing.sm,
    marginBottom: spacing.sm,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  tagText: {
    fontSize: fontSize.sm,
    color: colors.text.primary,
  },
  promptCard: {
    backgroundColor: colors.background.secondary,
    padding: spacing.md,
    borderRadius: borderRadius.md,
    marginBottom: spacing.sm,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  promptText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
    lineHeight: 20,
  },
  systemPromptCard: {
    backgroundColor: colors.background.secondary,
    padding: spacing.md,
    borderRadius: borderRadius.md,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  systemPromptText: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    lineHeight: 20,
    fontFamily: 'monospace',
  },
  systemPromptMore: {
    fontSize: fontSize.xs,
    color: colors.text.secondary,
    fontStyle: 'italic',
    marginTop: spacing.sm,
  },
  detailsCard: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    borderWidth: 1,
    borderColor: colors.border.default,
    overflow: 'hidden',
  },
  detailRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  detailLabel: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  detailValue: {
    fontSize: fontSize.sm,
    color: colors.text.primary,
    fontWeight: '500',
  },
  bottomSpacer: {
    height: 100,
  },
  actionBar: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    backgroundColor: colors.background.primary,
    borderTopWidth: 1,
    borderTopColor: colors.border.default,
    padding: spacing.md,
    paddingBottom: spacing.lg,
  },
  actionButton: {
    paddingVertical: spacing.md,
    borderRadius: borderRadius.md,
    alignItems: 'center',
    justifyContent: 'center',
  },
  installButton: {
    backgroundColor: colors.primary[500],
  },
  installButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  uninstallButton: {
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  uninstallButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '500',
  },
});
