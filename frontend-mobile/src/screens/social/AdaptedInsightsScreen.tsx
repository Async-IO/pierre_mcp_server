// ABOUTME: Screen listing all insights adapted for the current user
// ABOUTME: Shows history of "Adapt to My Training" results

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  RefreshControl,
  type ViewStyle,
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, glassCard } from '../../constants/theme';
import { socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { AdaptedInsight } from '../../types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;

// Glass card style with shadow (React Native shadows cannot use className)
const cardStyle: ViewStyle = {
  marginHorizontal: spacing.md,
  marginVertical: spacing.sm,
  padding: spacing.md,
  borderRadius: 12,
  ...glassCard,
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

interface AdaptedInsightCardProps {
  insight: AdaptedInsight;
  onPress: () => void;
}

function AdaptedInsightCard({ insight, onPress }: AdaptedInsightCardProps) {
  // Truncate content for preview
  const previewContent = insight.adapted_content.length > 150
    ? insight.adapted_content.substring(0, 150) + '...'
    : insight.adapted_content;

  return (
    <TouchableOpacity style={cardStyle} onPress={onPress}>
      <View className="flex-row items-center mb-4">
        <View
          className="w-8 h-8 rounded-full justify-center items-center mr-2"
          style={{ backgroundColor: colors.pierre.violet + '20' }}
        >
          <Feather name="refresh-cw" size={18} color={colors.pierre.violet} />
        </View>
        <Text className="flex-1 text-text-tertiary text-sm">{formatRelativeTime(insight.created_at)}</Text>
      </View>
      <Text className="text-text-primary text-base leading-6">{previewContent}</Text>
      <View className="flex-row items-center justify-between mt-4 pt-4 border-t border-border-subtle">
        <Text className="text-text-tertiary text-sm">Tap to view full insight</Text>
        <Feather name="chevron-right" size={16} color={colors.text.tertiary} />
      </View>
    </TouchableOpacity>
  );
}

export function AdaptedInsightsScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [insights, setInsights] = useState<AdaptedInsight[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(true);

  const loadInsights = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      const response = await socialApi.getAdaptedInsights({ limit: 20 });
      setInsights(response.insights);
      setNextCursor(response.next_cursor);
      setHasMore(response.has_more);
    } catch (error) {
      console.error('Failed to load adapted insights:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated]);

  const loadMore = useCallback(async () => {
    if (!isAuthenticated || !hasMore || isLoadingMore || !nextCursor) return;

    try {
      setIsLoadingMore(true);
      const response = await socialApi.getAdaptedInsights({
        limit: 20,
        cursor: nextCursor,
      });
      setInsights(prev => [...prev, ...response.insights]);
      setNextCursor(response.next_cursor);
      setHasMore(response.has_more);
    } catch (error) {
      console.error('Failed to load more adapted insights:', error);
    } finally {
      setIsLoadingMore(false);
    }
  }, [isAuthenticated, hasMore, isLoadingMore, nextCursor]);

  useFocusEffect(
    useCallback(() => {
      loadInsights();
    }, [loadInsights])
  );

  const handleInsightPress = (insight: AdaptedInsight) => {
    navigation.navigate('AdaptedInsight', { adaptedInsight: insight });
  };

  const renderInsight = ({ item }: { item: AdaptedInsight }) => (
    <AdaptedInsightCard
      insight={item}
      onPress={() => handleInsightPress(item)}
    />
  );

  const renderEmptyState = () => (
    <View className="flex-1 justify-center items-center p-6">
      <Feather name="refresh-cw" size={64} color={colors.text.tertiary} />
      <Text className="text-text-primary text-xl font-bold mt-5">No Adapted Insights</Text>
      <Text className="text-text-secondary text-base text-center mt-2 mb-6">
        When you tap "Adapt to My Training" on friends' insights, your personalized
        versions will appear here
      </Text>
      <TouchableOpacity
        className="flex-row items-center px-5 py-4 rounded-lg gap-2"
        style={{ backgroundColor: colors.pierre.violet }}
        onPress={() => navigation.navigate('SocialMain')}
      >
        <Feather name="activity" size={18} color={colors.text.primary} />
        <Text className="text-text-primary text-base font-semibold">Browse Feed</Text>
      </TouchableOpacity>
    </View>
  );

  const renderFooter = () => {
    if (!isLoadingMore) return null;
    return (
      <View className="py-5 items-center">
        <ActivityIndicator size="small" color={colors.pierre.violet} />
      </View>
    );
  };

  if (isLoading && insights.length === 0) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text className="text-text-secondary mt-4">Loading insights...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <TouchableOpacity
          className="p-2"
          onPress={() => navigation.goBack()}
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-bold text-text-primary text-center">Adapted Insights</Text>
        <View className="w-10" />
      </View>

      {/* Insights List */}
      <FlatList
        data={insights}
        keyExtractor={item => item.id}
        renderItem={renderInsight}
        ListEmptyComponent={renderEmptyState}
        ListFooterComponent={renderFooter}
        contentContainerStyle={insights.length === 0 ? { flexGrow: 1 } : { paddingVertical: spacing.sm }}
        onEndReached={loadMore}
        onEndReachedThreshold={0.3}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={() => loadInsights(true)}
            tintColor={colors.pierre.violet}
          />
        }
      />
    </SafeAreaView>
  );
}
