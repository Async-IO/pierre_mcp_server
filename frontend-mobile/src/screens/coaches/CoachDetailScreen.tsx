// ABOUTME: Coach detail screen showing full coach info with edit/delete actions
// ABOUTME: Read-only view of user's coach with option to edit or use in chat

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
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { Coach, CoachCategory } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

interface CoachDetailScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
  route: RouteProp<AppDrawerParamList, 'CoachDetail'>;
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

export function CoachDetailScreen({ navigation, route }: CoachDetailScreenProps) {
  const { coachId } = route.params;
  const { isAuthenticated } = useAuth();
  const [coach, setCoach] = useState<Coach | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isHidden, setIsHidden] = useState(false);
  const [isTogglingHidden, setIsTogglingHidden] = useState(false);

  const loadCoachDetail = useCallback(async () => {
    if (!isAuthenticated || !coachId) return;

    try {
      setIsLoading(true);
      // Load coaches and hidden coaches list in parallel
      const [coachesResponse, hiddenResponse] = await Promise.all([
        apiService.listCoaches({ include_hidden: true }),
        apiService.getHiddenCoaches(),
      ]);
      const foundCoach = coachesResponse.coaches.find((c) => c.id === coachId);
      setCoach(foundCoach || null);

      // Check if this coach is in the hidden list
      const hiddenIds = new Set((hiddenResponse.coaches || []).map((c) => c.id));
      setIsHidden(hiddenIds.has(coachId));
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

  const handleEdit = () => {
    if (!coach) return;
    navigation.navigate('CoachWizard', { coachId: coach.id });
  };

  const handleDelete = () => {
    if (!coach) return;

    Alert.alert(
      'Delete Coach?',
      `Are you sure you want to delete "${coach.title}"? This action cannot be undone.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete',
          style: 'destructive',
          onPress: async () => {
            try {
              setIsDeleting(true);
              await apiService.deleteCoach(coach.id);
              Alert.alert('Deleted', 'Coach has been deleted.');
              navigation.navigate('CoachLibrary');
            } catch (error) {
              console.error('Failed to delete coach:', error);
              Alert.alert('Error', 'Failed to delete coach. Please try again.');
            } finally {
              setIsDeleting(false);
            }
          },
        },
      ]
    );
  };

  const handleUseInChat = () => {
    if (!coach) return;
    // Navigate to chat with this coach selected
    navigation.navigate('Chat');
  };

  const handleToggleHidden = async () => {
    if (!coach) return;

    try {
      setIsTogglingHidden(true);
      if (isHidden) {
        await apiService.showCoach(coach.id);
        setIsHidden(false);
      } else {
        await apiService.hideCoach(coach.id);
        setIsHidden(true);
      }
    } catch (error) {
      console.error('Failed to toggle coach visibility:', error);
      Alert.alert('Error', 'Failed to update coach visibility');
    } finally {
      setIsTogglingHidden(false);
    }
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
            onPress={() => navigation.navigate('CoachLibrary')}
          >
            <Text style={styles.backButtonText}>Go Back</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  const categoryColor = COACH_CATEGORY_COLORS[coach.category];

  return (
    <SafeAreaView style={styles.container} testID="coach-detail-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          testID="back-button"
          style={styles.backArrow}
          onPress={() => navigation.navigate('CoachLibrary')}
        >
          <Text style={styles.backArrowText}>←</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle} numberOfLines={1}>
          {coach.title}
        </Text>
        {!coach.is_system && (
          <TouchableOpacity
            testID="edit-button"
            style={styles.editButton}
            onPress={handleEdit}
          >
            <Feather name="edit-2" size={20} color={colors.primary[500]} />
          </TouchableOpacity>
        )}
        {coach.is_system && <View style={styles.headerSpacer} />}
      </View>

      <ScrollView style={styles.content} showsVerticalScrollIndicator={false}>
        {/* Category & Stats */}
        <View style={styles.metaSection}>
          <View style={styles.badgeRow}>
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
            {coach.is_system && (
              <View style={styles.systemBadge}>
                <Text style={styles.systemBadgeText}>System</Text>
              </View>
            )}
            {coach.is_favorite && (
              <Feather name="star" size={16} color="#F59E0B" style={styles.starIcon} />
            )}
          </View>
          <Text testID="use-count" style={styles.useCount}>
            Used {coach.use_count} {coach.use_count === 1 ? 'time' : 'times'}
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

        {/* System Prompt */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>System Prompt</Text>
          <View style={styles.systemPromptCard}>
            <Text style={styles.systemPromptText}>
              {coach.system_prompt}
            </Text>
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
            <View style={styles.detailRow}>
              <Text style={styles.detailLabel}>Context Usage</Text>
              <Text style={styles.detailValue}>
                {((coach.token_count / 128000) * 100).toFixed(1)}%
              </Text>
            </View>
            {coach.created_at && (
              <View style={styles.detailRow}>
                <Text style={styles.detailLabel}>Created</Text>
                <Text style={styles.detailValue}>
                  {new Date(coach.created_at).toLocaleDateString()}
                </Text>
              </View>
            )}
            {coach.last_used_at && (
              <View style={[styles.detailRow, styles.detailRowLast]}>
                <Text style={styles.detailLabel}>Last Used</Text>
                <Text style={styles.detailValue}>
                  {new Date(coach.last_used_at).toLocaleDateString()}
                </Text>
              </View>
            )}
          </View>
        </View>

        {/* Bottom Spacer for Action Buttons */}
        <View style={styles.bottomSpacer} />
      </ScrollView>

      {/* Action Bar - Fixed at bottom */}
      <View style={styles.actionBar}>
        <TouchableOpacity
          style={[styles.actionButton, styles.useButton]}
          onPress={handleUseInChat}
          testID="use-in-chat-button"
        >
          <Feather name="message-circle" size={18} color={colors.text.primary} />
          <Text style={styles.useButtonText}>Use in Chat</Text>
        </TouchableOpacity>

        {coach.is_system && (
          <TouchableOpacity
            style={[styles.actionButton, styles.hideButton]}
            onPress={handleToggleHidden}
            disabled={isTogglingHidden}
            testID="hide-button"
          >
            {isTogglingHidden ? (
              <ActivityIndicator size="small" color={colors.text.secondary} />
            ) : (
              <>
                <Feather
                  name={isHidden ? 'eye' : 'eye-off'}
                  size={18}
                  color={isHidden ? colors.primary[400] : colors.text.secondary}
                />
                <Text style={[styles.hideButtonText, isHidden && styles.hideButtonTextActive]}>
                  {isHidden ? 'Show' : 'Hide'}
                </Text>
              </>
            )}
          </TouchableOpacity>
        )}

        {!coach.is_system && (
          <TouchableOpacity
            style={[styles.actionButton, styles.deleteButton]}
            onPress={handleDelete}
            disabled={isDeleting}
            testID="delete-button"
          >
            {isDeleting ? (
              <ActivityIndicator size="small" color="#EF4444" />
            ) : (
              <>
                <Feather name="trash-2" size={18} color="#EF4444" />
                <Text style={styles.deleteButtonText}>Delete</Text>
              </>
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
  editButton: {
    padding: spacing.sm,
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
  badgeRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
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
  systemBadge: {
    backgroundColor: colors.primary[500] + '30',
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.sm,
  },
  systemBadgeText: {
    fontSize: fontSize.xs,
    color: colors.primary[500],
    fontWeight: '600',
  },
  starIcon: {
    marginLeft: spacing.xs,
  },
  useCount: {
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
  detailRowLast: {
    borderBottomWidth: 0,
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
    height: 120,
  },
  actionBar: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    flexDirection: 'row',
    backgroundColor: colors.background.primary,
    borderTopWidth: 1,
    borderTopColor: colors.border.default,
    padding: spacing.md,
    paddingBottom: spacing.lg,
    gap: spacing.sm,
  },
  actionButton: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: spacing.md,
    borderRadius: borderRadius.md,
    gap: spacing.xs,
  },
  useButton: {
    backgroundColor: colors.primary[500],
  },
  useButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  deleteButton: {
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: '#EF4444',
  },
  deleteButtonText: {
    color: '#EF4444',
    fontSize: fontSize.md,
    fontWeight: '500',
  },
  hideButton: {
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  hideButtonText: {
    color: colors.text.secondary,
    fontSize: fontSize.md,
    fontWeight: '500',
  },
  hideButtonTextActive: {
    color: colors.primary[400],
  },
});
