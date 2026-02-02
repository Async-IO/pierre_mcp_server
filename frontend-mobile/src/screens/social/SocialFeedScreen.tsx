// ABOUTME: Social feed screen showing friends' shared coach insights
// ABOUTME: Displays timeline with reactions, adapt-to-my-training, and infinite scroll

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  RefreshControl,
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { LinearGradient } from 'expo-linear-gradient';
import { colors, spacing, glassCard, gradients, buttonGlow } from '../../constants/theme';
import { socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { InsightCard } from '../../components/social/InsightCard';
import type { FeedItem, ReactionType, InsightSuggestion } from '../../types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;

export function SocialFeedScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [feedItems, setFeedItems] = useState<FeedItem[]>([]);
  const [suggestions, setSuggestions] = useState<InsightSuggestion[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [reactingIds, setReactingIds] = useState<Set<string>>(new Set());
  const [adaptingIds, setAdaptingIds] = useState<Set<string>>(new Set());
  const [showSuggestionsBanner, setShowSuggestionsBanner] = useState(true);

  // Load suggestions for the banner
  const loadSuggestions = useCallback(async () => {
    if (!isAuthenticated) return;
    try {
      const response = await socialApi.getInsightSuggestions({ limit: 3 });
      setSuggestions(response.suggestions);
    } catch (error) {
      // Silently fail - suggestions are optional enhancement
      console.debug('Failed to load suggestions:', error);
    }
  }, [isAuthenticated]);

  const loadFeed = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      // Load feed and suggestions in parallel
      const [feedResponse] = await Promise.all([
        socialApi.getSocialFeed({ limit: 20 }),
        loadSuggestions(),
      ]);
      setFeedItems(feedResponse.items);
      setNextCursor(feedResponse.next_cursor);
      setHasMore(feedResponse.has_more);
    } catch (error) {
      console.error('Failed to load feed:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated, loadSuggestions]);

  const loadMoreFeed = useCallback(async () => {
    if (!isAuthenticated || !hasMore || isLoadingMore || !nextCursor) return;

    try {
      setIsLoadingMore(true);
      const response = await socialApi.getSocialFeed({
        limit: 20,
        cursor: nextCursor,
      });
      setFeedItems(prev => [...prev, ...response.items]);
      setNextCursor(response.next_cursor);
      setHasMore(response.has_more);
    } catch (error) {
      console.error('Failed to load more feed:', error);
    } finally {
      setIsLoadingMore(false);
    }
  }, [isAuthenticated, hasMore, isLoadingMore, nextCursor]);

  useFocusEffect(
    useCallback(() => {
      loadFeed();
    }, [loadFeed])
  );

  const handleReaction = async (insightId: string, reactionType: ReactionType) => {
    try {
      setReactingIds(prev => new Set(prev).add(insightId));

      const currentItem = feedItems.find(item => item.insight.id === insightId);
      const isRemoving = currentItem?.user_reaction === reactionType;

      if (isRemoving) {
        await socialApi.removeReaction(insightId, reactionType);
      } else {
        await socialApi.addReaction(insightId, reactionType);
      }

      // Update local state optimistically
      setFeedItems(prev =>
        prev.map(item => {
          if (item.insight.id !== insightId) return item;

          const newReactions = { ...item.reactions };

          // Remove previous reaction count if switching
          if (item.user_reaction && item.user_reaction !== reactionType) {
            newReactions[item.user_reaction] = Math.max(0, newReactions[item.user_reaction] - 1);
            newReactions.total = Math.max(0, newReactions.total - 1);
          }

          if (isRemoving) {
            // Removing reaction
            newReactions[reactionType] = Math.max(0, newReactions[reactionType] - 1);
            newReactions.total = Math.max(0, newReactions.total - 1);
            return { ...item, reactions: newReactions, user_reaction: null };
          } else {
            // Adding reaction
            if (!item.user_reaction) {
              newReactions[reactionType] = newReactions[reactionType] + 1;
              newReactions.total = newReactions.total + 1;
            } else {
              newReactions[reactionType] = newReactions[reactionType] + 1;
            }
            return { ...item, reactions: newReactions, user_reaction: reactionType };
          }
        })
      );
    } catch (error) {
      console.error('Failed to update reaction:', error);
    } finally {
      setReactingIds(prev => {
        const next = new Set(prev);
        next.delete(insightId);
        return next;
      });
    }
  };

  const handleAdapt = async (insightId: string) => {
    try {
      setAdaptingIds(prev => new Set(prev).add(insightId));

      const response = await socialApi.adaptInsight(insightId);

      // Update local state to show adapted
      setFeedItems(prev =>
        prev.map(item => {
          if (item.insight.id !== insightId) return item;
          return {
            ...item,
            user_has_adapted: true,
            insight: {
              ...item.insight,
              adapt_count: item.insight.adapt_count + 1,
            },
          };
        })
      );

      // Navigate to adapted insight view
      navigation.navigate('AdaptedInsight', { adaptedInsight: response.adapted });
    } catch (error) {
      console.error('Failed to adapt insight:', error);
    } finally {
      setAdaptingIds(prev => {
        const next = new Set(prev);
        next.delete(insightId);
        return next;
      });
    }
  };

  const handleShare = (activityId: string) => {
    navigation.navigate('ShareInsight', { activityId });
  };

  const renderFeedItem = ({ item }: { item: FeedItem }) => (
    <InsightCard
      item={item}
      onReaction={(type) => handleReaction(item.insight.id, type)}
      onAdapt={() => handleAdapt(item.insight.id)}
      onShare={handleShare}
      isReacting={reactingIds.has(item.insight.id)}
      isAdapting={adaptingIds.has(item.insight.id)}
    />
  );

  const renderEmptyState = () => (
    <View className="flex-1 justify-center items-center p-6">
      {/* Icon with subtle glow */}
      <View
        className="w-24 h-24 rounded-full items-center justify-center mb-2"
        style={{
          backgroundColor: 'rgba(139, 92, 246, 0.1)',
          shadowColor: colors.pierre.violet,
          shadowOffset: { width: 0, height: 0 },
          shadowOpacity: 0.3,
          shadowRadius: 20,
        }}
      >
        <Feather name="users" size={48} color={colors.pierre.violet} />
      </View>
      <Text className="text-text-primary text-xl font-bold mt-4">No Insights Yet</Text>
      <Text className="text-text-secondary text-base text-center mt-2 mb-6">
        When your friends share coach insights, they'll appear here. Find friends to get started!
      </Text>
      <TouchableOpacity
        className="flex-row items-center px-6 py-4 rounded-xl gap-2"
        style={{
          backgroundColor: colors.pierre.violet,
          ...buttonGlow,
        }}
        onPress={() => navigation.navigate('Friends')}
      >
        <Feather name="user-plus" size={18} color="#FFFFFF" />
        <Text className="text-white text-base font-semibold">Find Friends</Text>
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

  // Suggestions banner at top of feed with glassmorphism
  const renderSuggestionsBanner = () => {
    if (suggestions.length === 0 || !showSuggestionsBanner) return null;

    return (
      <View
        className="mx-4 mt-4 mb-2 rounded-xl overflow-hidden"
        style={{ ...glassCard, borderRadius: 16 }}
        testID="suggestions-banner"
      >
        {/* Gradient accent bar */}
        <LinearGradient
          colors={gradients.violetCyan as [string, string]}
          start={{ x: 0, y: 0 }}
          end={{ x: 1, y: 0 }}
          style={{ height: 3, width: '100%' }}
        />
        <View className="p-4">
          {/* Header with dismiss */}
          <View className="flex-row items-center justify-between mb-3">
            <View className="flex-row items-center gap-2">
              <Feather name="zap" size={18} color={colors.pierre.violet} />
              <Text className="text-text-primary font-semibold">Coach noticed something!</Text>
            </View>
            <TouchableOpacity
              onPress={() => setShowSuggestionsBanner(false)}
              className="p-1"
              testID="dismiss-suggestions"
            >
              <Feather name="x" size={18} color={colors.text.tertiary} />
            </TouchableOpacity>
          </View>

          {/* Preview of top suggestion */}
          <Text className="text-text-secondary text-sm mb-4" numberOfLines={2}>
            {suggestions[0].suggested_content}
          </Text>

          {/* Share button with glow */}
          <TouchableOpacity
            className="flex-row items-center justify-center py-3.5 rounded-xl gap-2"
            style={{
              backgroundColor: colors.pierre.violet,
              ...buttonGlow,
            }}
            onPress={() => navigation.navigate('ShareInsight')}
            testID="share-suggestion-button"
          >
            <Feather name="share-2" size={16} color="#FFFFFF" />
            <Text className="text-white font-semibold">
              Share with Friends ({suggestions.length} available)
            </Text>
          </TouchableOpacity>
        </View>
      </View>
    );
  };

  if (isLoading && feedItems.length === 0) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text className="text-text-secondary mt-4">Loading feed...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="social-feed-screen">
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <View className="w-10" />
        <Text className="flex-1 text-xl font-bold text-text-primary text-center">Feed</Text>
        <TouchableOpacity
          className="p-2"
          onPress={() => navigation.navigate('ShareInsight')}
          testID="share-insight-button"
        >
          <Feather name="plus-circle" size={24} color={colors.pierre.violet} />
        </TouchableOpacity>
      </View>

      {/* Feed List */}
      <FlatList
        testID="feed-list"
        data={feedItems}
        keyExtractor={item => item.insight.id}
        renderItem={renderFeedItem}
        ListHeaderComponent={renderSuggestionsBanner}
        ListEmptyComponent={renderEmptyState}
        ListFooterComponent={renderFooter}
        contentContainerStyle={feedItems.length === 0 ? { flexGrow: 1 } : { paddingVertical: spacing.sm }}
        onEndReached={loadMoreFeed}
        onEndReachedThreshold={0.3}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={() => loadFeed(true)}
            tintColor={colors.pierre.violet}
          />
        }
      />
    </SafeAreaView>
  );
}
