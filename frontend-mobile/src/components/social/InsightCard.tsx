// ABOUTME: Insight card component for displaying shared coach insights in the feed
// ABOUTME: Shows author, content, reactions, and adapt-to-my-training action

import React from 'react';
import { View, Text, TouchableOpacity, ActivityIndicator, type ViewStyle } from 'react-native';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';
import { colors, glassCard } from '../../constants/theme';
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

// Glass card style with shadow (React Native shadows cannot use className)
const cardStyle: ViewStyle = {
  ...glassCard,
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
    <View
      className="mx-3 my-2 p-3 rounded-lg"
      style={cardStyle}
    >
      {/* Header with author info and type badge */}
      <View className="flex-row items-center mb-3">
        <View
          className="w-10 h-10 rounded-full justify-center items-center"
          style={{ backgroundColor: avatarColor }}
        >
          <Text className="text-base font-semibold text-text-primary">{initials}</Text>
        </View>
        <View className="flex-1 ml-2">
          <Text className="text-base font-semibold text-text-primary">{displayName}</Text>
          <Text className="text-xs text-text-tertiary mt-0.5">{formatRelativeTime(insight.created_at)}</Text>
        </View>
        <View
          className="flex-row items-center px-2 py-1 rounded gap-1"
          style={{ backgroundColor: typeStyle.color + '20' }}
        >
          <Feather name={typeStyle.icon} size={12} color={typeStyle.color} />
          <Text className="text-xs font-medium" style={{ color: typeStyle.color }}>{typeStyle.label}</Text>
        </View>
      </View>

      {/* Insight content */}
      {insight.title && (
        <Text className="text-lg font-bold text-text-primary mb-2">{insight.title}</Text>
      )}
      <Text className="text-base text-text-secondary leading-[22px]">{insight.content}</Text>

      {/* Sport type and training phase context */}
      {(insight.sport_type || insight.training_phase) && (
        <View className="flex-row flex-wrap mt-2 gap-2">
          {insight.sport_type && (
            <View className="flex-row items-center gap-1 px-2 py-1 bg-background-tertiary rounded">
              <Feather name="activity" size={12} color={colors.text.tertiary} />
              <Text className="text-xs text-text-tertiary capitalize">{insight.sport_type}</Text>
            </View>
          )}
          {insight.training_phase && (
            <View className="flex-row items-center gap-1 px-2 py-1 bg-background-tertiary rounded">
              <Feather name="trending-up" size={12} color={colors.text.tertiary} />
              <Text className="text-xs text-text-tertiary capitalize">{insight.training_phase} phase</Text>
            </View>
          )}
        </View>
      )}

      {/* Reaction bar */}
      <View className="flex-row justify-around mt-3 pt-3 border-t border-border-subtle">
        {(Object.keys(REACTION_ICONS) as ReactionType[]).map((type) => {
          const reactionStyle = REACTION_ICONS[type];
          const count = reactions[type];
          const isActive = user_reaction === type;

          return (
            <TouchableOpacity
              key={type}
              className={`flex-row items-center px-3 py-2 rounded-lg gap-1 ${
                isActive ? 'bg-background-secondary' : ''
              }`}
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
                  className="text-sm"
                  style={{ color: isActive ? reactionStyle.color : colors.text.tertiary }}
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
        className={`flex-row items-center justify-center mt-3 py-3 rounded-lg gap-2 ${
          user_has_adapted ? 'bg-background-secondary' : ''
        }`}
        style={!user_has_adapted ? { backgroundColor: colors.pierre.violet } : undefined}
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
              className={`text-base font-semibold ${
                user_has_adapted ? '' : 'text-text-primary'
              }`}
              style={user_has_adapted ? { color: colors.pierre.activity } : undefined}
            >
              {user_has_adapted ? 'Adapted' : 'Adapt to My Training'}
            </Text>
          </>
        )}
      </TouchableOpacity>
    </View>
  );
}
