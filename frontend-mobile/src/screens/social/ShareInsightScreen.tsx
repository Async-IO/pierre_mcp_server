// ABOUTME: Screen for sharing coach insights with friends
// ABOUTME: Allows composing and publishing sanitized insights to the social feed

import React, { useState } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  TextInput,
  ActivityIndicator,
  KeyboardAvoidingView,
  Platform,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';

type FeatherIconName = ComponentProps<typeof Feather>['name'];
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { InsightType, ShareVisibility, TrainingPhase } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;

// Insight type options
const INSIGHT_TYPES: Array<{ key: InsightType; label: string; icon: FeatherIconName; color: string }> = [
  { key: 'achievement', label: 'Achievement', icon: 'award', color: '#10B981' },
  { key: 'milestone', label: 'Milestone', icon: 'flag', color: '#F59E0B' },
  { key: 'training_tip', label: 'Training Tip', icon: 'zap', color: '#6366F1' },
  { key: 'recovery', label: 'Recovery', icon: 'moon', color: '#8B5CF6' },
  { key: 'motivation', label: 'Motivation', icon: 'sun', color: '#F97316' },
];

// Sport type options
const SPORT_TYPES = ['Running', 'Cycling', 'Swimming', 'Triathlon', 'Strength', 'Other'];

// Training phase options
const TRAINING_PHASES: Array<{ key: TrainingPhase; label: string }> = [
  { key: 'base', label: 'Base Building' },
  { key: 'build', label: 'Build Phase' },
  { key: 'peak', label: 'Peak/Race' },
  { key: 'recovery', label: 'Recovery' },
];

export function ShareInsightScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();

  const [insightType, setInsightType] = useState<InsightType>('achievement');
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [sportType, setSportType] = useState<string | null>(null);
  const [trainingPhase, setTrainingPhase] = useState<TrainingPhase | null>(null);
  const [visibility, setVisibility] = useState<ShareVisibility>('friends_only');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const canSubmit = content.trim().length >= 10;

  const handleSubmit = async () => {
    if (!canSubmit || !isAuthenticated) return;

    try {
      setIsSubmitting(true);

      await apiService.shareInsight({
        insight_type: insightType,
        title: title.trim() || undefined,
        content: content.trim(),
        sport_type: sportType || undefined,
        training_phase: trainingPhase || undefined,
        visibility,
      });

      // Navigate back to feed
      navigation.navigate('SocialFeed');
    } catch (error) {
      console.error('Failed to share insight:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <SafeAreaView style={styles.container} testID="share-insight-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.backButton}
          onPress={() => navigation.goBack()}
          testID="close-button"
        >
          <Feather name="x" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Share Insight</Text>
        <TouchableOpacity
          style={[styles.submitButton, !canSubmit && styles.submitButtonDisabled]}
          onPress={handleSubmit}
          disabled={!canSubmit || isSubmitting}
          testID="share-button"
        >
          {isSubmitting ? (
            <ActivityIndicator size="small" color={colors.text.primary} />
          ) : (
            <Text style={styles.submitText}>Share</Text>
          )}
        </TouchableOpacity>
      </View>

      <KeyboardAvoidingView
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
        style={styles.keyboardView}
      >
        <ScrollView style={styles.scrollView} showsVerticalScrollIndicator={false}>
          {/* Insight Type Selection */}
          <Text style={styles.sectionLabel}>Type</Text>
          <View style={styles.typeGrid} testID="insight-type-picker">
            {INSIGHT_TYPES.map((type) => (
              <TouchableOpacity
                key={type.key}
                testID={`insight-type-${type.key}`}
                style={[
                  styles.typeButton,
                  insightType === type.key && {
                    backgroundColor: type.color + '20',
                    borderColor: type.color,
                  },
                ]}
                onPress={() => setInsightType(type.key)}
              >
                <Feather
                  name={type.icon}
                  size={20}
                  color={insightType === type.key ? type.color : colors.text.tertiary}
                />
                <Text
                  style={[
                    styles.typeButtonText,
                    insightType === type.key && { color: type.color },
                  ]}
                >
                  {type.label}
                </Text>
              </TouchableOpacity>
            ))}
          </View>

          {/* Title (optional) */}
          <Text style={styles.sectionLabel}>Title (optional)</Text>
          <TextInput
            testID="insight-title-input"
            style={styles.titleInput}
            placeholder="Give your insight a catchy title..."
            placeholderTextColor={colors.text.tertiary}
            value={title}
            onChangeText={setTitle}
            maxLength={100}
          />

          {/* Content */}
          <Text style={styles.sectionLabel}>Content *</Text>
          <TextInput
            testID="insight-content-input"
            style={styles.contentInput}
            placeholder="Share your coach insight... (min 10 characters)"
            placeholderTextColor={colors.text.tertiary}
            value={content}
            onChangeText={setContent}
            multiline
            numberOfLines={6}
            textAlignVertical="top"
            maxLength={500}
          />
          <Text style={styles.charCount}>{content.length}/500</Text>

          {/* Privacy note */}
          <View style={styles.privacyNote}>
            <Feather name="shield" size={16} color={colors.pierre.violet} />
            <Text style={styles.privacyNoteText}>
              Your insight is automatically sanitized. Private data like GPS coordinates,
              exact pace, and recovery scores are never shared.
            </Text>
          </View>

          {/* Sport Type */}
          <Text style={styles.sectionLabel}>Sport (optional)</Text>
          <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.chipScroll}>
            {SPORT_TYPES.map((sport) => (
              <TouchableOpacity
                key={sport}
                style={[
                  styles.chip,
                  sportType === sport && styles.chipActive,
                ]}
                onPress={() => setSportType(sportType === sport ? null : sport)}
              >
                <Text
                  style={[
                    styles.chipText,
                    sportType === sport && styles.chipTextActive,
                  ]}
                >
                  {sport}
                </Text>
              </TouchableOpacity>
            ))}
          </ScrollView>

          {/* Training Phase */}
          <Text style={styles.sectionLabel}>Training Phase (optional)</Text>
          <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.chipScroll}>
            {TRAINING_PHASES.map((phase) => (
              <TouchableOpacity
                key={phase.key}
                style={[
                  styles.chip,
                  trainingPhase === phase.key && styles.chipActive,
                ]}
                onPress={() => setTrainingPhase(trainingPhase === phase.key ? null : phase.key)}
              >
                <Text
                  style={[
                    styles.chipText,
                    trainingPhase === phase.key && styles.chipTextActive,
                  ]}
                >
                  {phase.label}
                </Text>
              </TouchableOpacity>
            ))}
          </ScrollView>

          {/* Visibility */}
          <Text style={styles.sectionLabel}>Visibility</Text>
          <View style={styles.visibilityRow}>
            <TouchableOpacity
              style={[
                styles.visibilityOption,
                visibility === 'friends_only' && styles.visibilityOptionActive,
              ]}
              onPress={() => setVisibility('friends_only')}
            >
              <Feather
                name="users"
                size={20}
                color={visibility === 'friends_only' ? colors.pierre.violet : colors.text.tertiary}
              />
              <Text
                style={[
                  styles.visibilityText,
                  visibility === 'friends_only' && styles.visibilityTextActive,
                ]}
              >
                Friends Only
              </Text>
            </TouchableOpacity>
            <TouchableOpacity
              style={[
                styles.visibilityOption,
                visibility === 'public' && styles.visibilityOptionActive,
              ]}
              onPress={() => setVisibility('public')}
            >
              <Feather
                name="globe"
                size={20}
                color={visibility === 'public' ? colors.pierre.violet : colors.text.tertiary}
              />
              <Text
                style={[
                  styles.visibilityText,
                  visibility === 'public' && styles.visibilityTextActive,
                ]}
              >
                Public
              </Text>
            </TouchableOpacity>
          </View>

          <View style={styles.bottomSpacer} />
        </ScrollView>
      </KeyboardAvoidingView>
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
  submitButton: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    backgroundColor: colors.pierre.violet,
  },
  submitButtonDisabled: {
    opacity: 0.5,
  },
  submitText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  keyboardView: {
    flex: 1,
  },
  scrollView: {
    flex: 1,
    paddingHorizontal: spacing.md,
  },
  sectionLabel: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    fontWeight: '600',
    marginTop: spacing.lg,
    marginBottom: spacing.sm,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  typeGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: spacing.sm,
  },
  typeButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: 'transparent',
    gap: spacing.xs,
  },
  typeButtonText: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
    fontWeight: '500',
  },
  titleInput: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
  },
  contentInput: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
    minHeight: 120,
  },
  charCount: {
    color: colors.text.tertiary,
    fontSize: fontSize.xs,
    textAlign: 'right',
    marginTop: spacing.xs,
  },
  privacyNote: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    backgroundColor: colors.pierre.violet + '15',
    borderRadius: borderRadius.md,
    padding: spacing.md,
    marginTop: spacing.md,
    gap: spacing.sm,
  },
  privacyNoteText: {
    flex: 1,
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    lineHeight: 20,
  },
  chipScroll: {
    flexDirection: 'row',
  },
  chip: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.full,
    backgroundColor: colors.background.secondary,
    marginRight: spacing.sm,
  },
  chipActive: {
    backgroundColor: colors.pierre.violet,
  },
  chipText: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
    fontWeight: '500',
  },
  chipTextActive: {
    color: colors.text.primary,
  },
  visibilityRow: {
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
  bottomSpacer: {
    height: spacing.xl,
  },
});
