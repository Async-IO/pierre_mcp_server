// ABOUTME: Social feed screen showing friends' shared coach insights
// ABOUTME: Displays timeline with reactions, adapt-to-my-training, and infinite scroll

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  RefreshControl,
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { InsightCard } from '../../components/social/InsightCard';
import type { FeedItem, ReactionType } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;

export function SocialFeedScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [feedItems, setFeedItems] = useState<FeedItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [reactingIds, setReactingIds] = useState<Set<string>>(new Set());
  const [adaptingIds, setAdaptingIds] = useState<Set<string>>(new Set());

  const loadFeed = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      const response = await apiService.getSocialFeed({ limit: 20 });
      setFeedItems(response.items);
      setNextCursor(response.next_cursor);
      setHasMore(response.has_more);
    } catch (error) {
      console.error('Failed to load feed:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated]);

  const loadMoreFeed = useCallback(async () => {
    if (!isAuthenticated || !hasMore || isLoadingMore || !nextCursor) return;

    try {
      setIsLoadingMore(true);
      const response = await apiService.getSocialFeed({
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
        await apiService.removeReaction(insightId);
      } else {
        await apiService.addReaction(insightId, reactionType);
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

      const response = await apiService.adaptInsight(insightId);

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

  const renderFeedItem = ({ item }: { item: FeedItem }) => (
    <InsightCard
      item={item}
      onReaction={(type) => handleReaction(item.insight.id, type)}
      onAdapt={() => handleAdapt(item.insight.id)}
      isReacting={reactingIds.has(item.insight.id)}
      isAdapting={adaptingIds.has(item.insight.id)}
    />
  );

  const renderEmptyState = () => (
    <View style={styles.emptyState}>
      <Feather name="users" size={64} color={colors.text.tertiary} />
      <Text style={styles.emptyTitle}>No Insights Yet</Text>
      <Text style={styles.emptyText}>
        When your friends share coach insights, they'll appear here. Find friends to get started!
      </Text>
      <TouchableOpacity
        style={styles.findFriendsButton}
        onPress={() => navigation.navigate('Friends')}
      >
        <Feather name="user-plus" size={18} color={colors.text.primary} />
        <Text style={styles.findFriendsText}>Find Friends</Text>
      </TouchableOpacity>
    </View>
  );

  const renderFooter = () => {
    if (!isLoadingMore) return null;
    return (
      <View style={styles.loadingMore}>
        <ActivityIndicator size="small" color={colors.pierre.violet} />
      </View>
    );
  };

  if (isLoading && feedItems.length === 0) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text style={styles.loadingText}>Loading feed...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container} testID="social-feed-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
          testID="drawer-toggle"
        >
          <Feather name="menu" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.title}>Feed</Text>
        <TouchableOpacity
          style={styles.iconButton}
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
        ListEmptyComponent={renderEmptyState}
        ListFooterComponent={renderFooter}
        contentContainerStyle={
          feedItems.length === 0 ? styles.emptyContainer : styles.listContent
        }
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
  menuButton: {
    padding: spacing.sm,
    marginRight: spacing.sm,
  },
  title: {
    flex: 1,
    fontSize: fontSize.xl,
    fontWeight: '700',
    color: colors.text.primary,
  },
  iconButton: {
    padding: spacing.sm,
  },
  listContent: {
    paddingVertical: spacing.sm,
  },
  emptyContainer: {
    flexGrow: 1,
  },
  emptyState: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: spacing.xl,
  },
  emptyTitle: {
    color: colors.text.primary,
    fontSize: fontSize.xl,
    fontWeight: '700',
    marginTop: spacing.lg,
  },
  emptyText: {
    color: colors.text.secondary,
    fontSize: fontSize.md,
    textAlign: 'center',
    marginTop: spacing.sm,
    marginBottom: spacing.xl,
  },
  findFriendsButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.pierre.violet,
    gap: spacing.sm,
  },
  findFriendsText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  loadingMore: {
    paddingVertical: spacing.lg,
    alignItems: 'center',
  },
});
