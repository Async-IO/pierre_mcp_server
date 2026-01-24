// ABOUTME: Insight card component for displaying shared coach insights in the feed
// ABOUTME: Shows author, content, reactions, and adapt-to-my-training action

import React from 'react';
import { View, Text, StyleSheet, TouchableOpacity, ActivityIndicator } from 'react-native';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';
import { colors, spacing, fontSize, borderRadius, glassCard } from '../../constants/theme';
import type { FeedItem, ReactionType } from '../../types';

type FeatherIconName = ComponentProps<typeof Feather>['name'];

// Reaction emoji mapping
const REACTION_ICONS: Record<ReactionType, { icon: FeatherIconName; color: string }> = {
  like: { icon: 'thumbs-up', color: '#3B82F6' },
  celebrate: { icon: 'award', color: '#F59E0B' },
  inspire: { icon: 'star', color: '#8B5CF6' },
  support: { icon: 'heart', color: '#EF4444' },
};

// Insight type styling
const INSIGHT_TYPE_STYLES: Record<string, { icon: FeatherIconName; color: string; label: string }> = {
  achievement: { icon: 'award', color: '#10B981', label: 'Achievement' },
  milestone: { icon: 'flag', color: '#F59E0B', label: 'Milestone' },
  training_tip: { icon: 'zap', color: '#6366F1', label: 'Training Tip' },
  recovery: { icon: 'moon', color: '#8B5CF6', label: 'Recovery' },
  motivation: { icon: 'sun', color: '#F97316', label: 'Motivation' },
};

// Generate avatar initials from name or email
const getInitials = (name: string | null, email: string): string => {
  if (name) {
    const parts = name.split(' ');
    if (parts.length >= 2) {
      return (parts[0][0] + parts[1][0]).toUpperCase();
    }
    return name.substring(0, 2).toUpperCase();
  }
  return email.substring(0, 2).toUpperCase();
};

// Generate consistent color from string
const getAvatarColor = (str: string): string => {
  const hash = str.split('').reduce((acc, char) => {
    return char.charCodeAt(0) + ((acc << 5) - acc);
  }, 0);
  const hue = Math.abs(hash % 360);
  return `hsl(${hue}, 70%, 50%)`;
};

// Format relative time
const formatRelativeTime = (dateStr: string): string => {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / (1000 * 60));
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
};

interface InsightCardProps {
  item: FeedItem;
  onReaction: (type: ReactionType) => void;
  onAdapt: () => void;
  isReacting?: boolean;
  isAdapting?: boolean;
}

export function InsightCard({
  item,
  onReaction,
  onAdapt,
  isReacting,
  isAdapting,
}: InsightCardProps) {
  const { insight, author, reactions, user_reaction, user_has_adapted } = item;
  const displayName = author.display_name || author.email.split('@')[0];
  const initials = getInitials(author.display_name, author.email);
  const avatarColor = getAvatarColor(author.email);
  const typeStyle = INSIGHT_TYPE_STYLES[insight.insight_type] || INSIGHT_TYPE_STYLES.motivation;

  return (
    <View style={styles.card}>
      {/* Header with author info and type badge */}
      <View style={styles.header}>
        <View style={[styles.avatar, { backgroundColor: avatarColor }]}>
          <Text style={styles.avatarText}>{initials}</Text>
        </View>
        <View style={styles.authorInfo}>
          <Text style={styles.authorName}>{displayName}</Text>
          <Text style={styles.timestamp}>{formatRelativeTime(insight.created_at)}</Text>
        </View>
        <View style={[styles.typeBadge, { backgroundColor: typeStyle.color + '20' }]}>
          <Feather name={typeStyle.icon} size={12} color={typeStyle.color} />
          <Text style={[styles.typeLabel, { color: typeStyle.color }]}>{typeStyle.label}</Text>
        </View>
      </View>

      {/* Insight content */}
      {insight.title && <Text style={styles.title}>{insight.title}</Text>}
      <Text style={styles.content}>{insight.content}</Text>

      {/* Sport type and training phase context */}
      {(insight.sport_type || insight.training_phase) && (
        <View style={styles.contextRow}>
          {insight.sport_type && (
            <View style={styles.contextBadge}>
              <Feather name="activity" size={12} color={colors.text.tertiary} />
              <Text style={styles.contextText}>{insight.sport_type}</Text>
            </View>
          )}
          {insight.training_phase && (
            <View style={styles.contextBadge}>
              <Feather name="trending-up" size={12} color={colors.text.tertiary} />
              <Text style={styles.contextText}>{insight.training_phase} phase</Text>
            </View>
          )}
        </View>
      )}

      {/* Reaction bar */}
      <View style={styles.reactionBar}>
        {(Object.keys(REACTION_ICONS) as ReactionType[]).map((type) => {
          const reactionStyle = REACTION_ICONS[type];
          const count = reactions[type];
          const isActive = user_reaction === type;

          return (
            <TouchableOpacity
              key={type}
              style={[styles.reactionButton, isActive && styles.reactionButtonActive]}
              onPress={() => onReaction(type)}
              disabled={isReacting}
            >
              <Feather
                name={reactionStyle.icon}
                size={16}
                color={isActive ? reactionStyle.color : colors.text.tertiary}
              />
              {count > 0 && (
                <Text
                  style={[
                    styles.reactionCount,
                    isActive && { color: reactionStyle.color },
                  ]}
                >
                  {count}
                </Text>
              )}
            </TouchableOpacity>
          );
        })}
      </View>

      {/* Adapt to My Training button */}
      <TouchableOpacity
        style={[
          styles.adaptButton,
          user_has_adapted && styles.adaptButtonUsed,
        ]}
        onPress={onAdapt}
        disabled={isAdapting || user_has_adapted}
      >
        {isAdapting ? (
          <ActivityIndicator size="small" color={colors.text.primary} />
        ) : (
          <>
            <Feather
              name={user_has_adapted ? 'check-circle' : 'refresh-cw'}
              size={16}
              color={user_has_adapted ? colors.pierre.activity : colors.text.primary}
            />
            <Text
              style={[
                styles.adaptButtonText,
                user_has_adapted && styles.adaptButtonTextUsed,
              ]}
            >
              {user_has_adapted ? 'Adapted' : 'Adapt to My Training'}
            </Text>
          </>
        )}
      </TouchableOpacity>
    </View>
  );
}

const styles = StyleSheet.create({
  card: {
    marginHorizontal: spacing.md,
    marginVertical: spacing.sm,
    padding: spacing.md,
    borderRadius: borderRadius.lg,
    ...glassCard,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: spacing.md,
  },
  avatar: {
    width: 40,
    height: 40,
    borderRadius: 20,
    justifyContent: 'center',
    alignItems: 'center',
  },
  avatarText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  authorInfo: {
    flex: 1,
    marginLeft: spacing.sm,
  },
  authorName: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  timestamp: {
    color: colors.text.tertiary,
    fontSize: fontSize.xs,
    marginTop: 2,
  },
  typeBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.sm,
    paddingVertical: 4,
    borderRadius: borderRadius.sm,
    gap: 4,
  },
  typeLabel: {
    fontSize: fontSize.xs,
    fontWeight: '500',
  },
  title: {
    color: colors.text.primary,
    fontSize: fontSize.lg,
    fontWeight: '700',
    marginBottom: spacing.sm,
  },
  content: {
    color: colors.text.secondary,
    fontSize: fontSize.md,
    lineHeight: 22,
  },
  contextRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    marginTop: spacing.sm,
    gap: spacing.sm,
  },
  contextBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 4,
    paddingHorizontal: spacing.sm,
    paddingVertical: 4,
    backgroundColor: colors.background.tertiary,
    borderRadius: borderRadius.sm,
  },
  contextText: {
    color: colors.text.tertiary,
    fontSize: fontSize.xs,
    textTransform: 'capitalize',
  },
  reactionBar: {
    flexDirection: 'row',
    justifyContent: 'space-around',
    marginTop: spacing.md,
    paddingTop: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  reactionButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    gap: spacing.xs,
  },
  reactionButtonActive: {
    backgroundColor: colors.background.secondary,
  },
  reactionCount: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
  },
  adaptButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    marginTop: spacing.md,
    paddingVertical: spacing.md,
    borderRadius: borderRadius.md,
    backgroundColor: colors.pierre.violet,
    gap: spacing.sm,
  },
  adaptButtonUsed: {
    backgroundColor: colors.background.secondary,
  },
  adaptButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  adaptButtonTextUsed: {
    color: colors.pierre.activity,
  },
});
