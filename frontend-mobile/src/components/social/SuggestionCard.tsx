// ABOUTME: Card component displaying a coach-generated insight suggestion
// ABOUTME: Shows preview with type badge, content, relevance, and share action

import React from 'react';
import { View, Text, TouchableOpacity } from 'react-native';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';
import { colors } from '../../constants/theme';
import type { InsightSuggestion, InsightType } from '../../types';

type FeatherIconName = ComponentProps<typeof Feather>['name'];

// Icon and color mapping for insight types
const INSIGHT_TYPE_CONFIG: Record<
  InsightType,
  { icon: FeatherIconName; color: string; label: string }
> = {
  achievement: { icon: 'award', color: '#10B981', label: 'Achievement' },
  milestone: { icon: 'flag', color: '#F59E0B', label: 'Milestone' },
  training_tip: { icon: 'zap', color: '#6366F1', label: 'Training Tip' },
  recovery: { icon: 'moon', color: '#8B5CF6', label: 'Recovery' },
  motivation: { icon: 'sun', color: '#F97316', label: 'Motivation' },
  coaching_insight: { icon: 'message-circle', color: '#7C3AED', label: 'Coach Chat' },
};

interface SuggestionCardProps {
  /** The suggestion to display */
  suggestion: InsightSuggestion;
  /** Callback when user wants to share this suggestion */
  onShare: (suggestion: InsightSuggestion) => void;
  /** Whether this card is currently selected */
  isSelected?: boolean;
  /** Test ID for testing */
  testID?: string;
}

/**
 * Displays a coach-generated insight suggestion with share action
 */
export function SuggestionCard({
  suggestion,
  onShare,
  isSelected = false,
  testID,
}: SuggestionCardProps) {
  const config = INSIGHT_TYPE_CONFIG[suggestion.insight_type];
  const relevancePercentage = Math.round(suggestion.relevance_score * 100);

  return (
    <TouchableOpacity
      className={`rounded-xl p-4 mb-3 ${isSelected ? '' : ''}`}
      style={[
        { backgroundColor: colors.background.secondary },
        isSelected && {
          borderWidth: 2,
          borderColor: colors.pierre.violet,
          backgroundColor: colors.pierre.violet + '10',
        },
      ]}
      onPress={() => onShare(suggestion)}
      activeOpacity={0.7}
      testID={testID}
    >
      {/* Header with type badge and relevance */}
      <View className="flex-row items-center justify-between mb-3">
        {/* Type badge */}
        <View
          className="flex-row items-center px-3 py-1.5 rounded-full gap-1.5"
          style={{ backgroundColor: config.color + '20' }}
        >
          <Feather name={config.icon} size={14} color={config.color} />
          <Text className="text-xs font-semibold" style={{ color: config.color }}>
            {config.label}
          </Text>
        </View>

        {/* Relevance indicator */}
        <View className="flex-row items-center gap-1">
          <Text className="text-xs text-text-tertiary">Relevance</Text>
          <View
            className="px-2 py-0.5 rounded"
            style={{
              backgroundColor:
                relevancePercentage >= 70
                  ? '#10B98120'
                  : relevancePercentage >= 40
                    ? '#F59E0B20'
                    : '#64748B20',
            }}
          >
            <Text
              className="text-xs font-semibold"
              style={{
                color:
                  relevancePercentage >= 70
                    ? '#10B981'
                    : relevancePercentage >= 40
                      ? '#F59E0B'
                      : '#64748B',
              }}
            >
              {relevancePercentage}%
            </Text>
          </View>
        </View>
      </View>

      {/* Optional title */}
      {suggestion.suggested_title && (
        <Text className="text-text-primary text-base font-semibold mb-1" numberOfLines={1}>
          {suggestion.suggested_title}
        </Text>
      )}

      {/* Content preview */}
      <Text className="text-text-secondary text-sm leading-5 mb-3" numberOfLines={3}>
        {suggestion.suggested_content}
      </Text>

      {/* Context badges */}
      <View className="flex-row flex-wrap gap-2 mb-3">
        {suggestion.sport_type && (
          <View className="bg-background-tertiary px-2 py-1 rounded">
            <Text className="text-xs text-text-tertiary">{suggestion.sport_type}</Text>
          </View>
        )}
        {suggestion.training_phase && (
          <View className="bg-background-tertiary px-2 py-1 rounded">
            <Text className="text-xs text-text-tertiary capitalize">
              {suggestion.training_phase} phase
            </Text>
          </View>
        )}
      </View>

      {/* Share button */}
      <View className="flex-row justify-end">
        <View
          className="flex-row items-center px-4 py-2 rounded-lg gap-2"
          style={{ backgroundColor: colors.pierre.violet }}
        >
          <Feather name="share-2" size={16} color={colors.text.primary} />
          <Text className="text-text-primary text-sm font-semibold">Share This</Text>
        </View>
      </View>
    </TouchableOpacity>
  );
}
