// ABOUTME: Screen listing all insights adapted for the current user
// ABOUTME: Shows history of "Adapt to My Training" results

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
import { colors, spacing, fontSize, borderRadius, glassCard } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { AdaptedInsight } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;

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
    <TouchableOpacity style={styles.card} onPress={onPress}>
      <View style={styles.cardHeader}>
        <View style={styles.iconContainer}>
          <Feather name="refresh-cw" size={18} color={colors.pierre.violet} />
        </View>
        <Text style={styles.timestamp}>{formatRelativeTime(insight.created_at)}</Text>
      </View>
      <Text style={styles.contentPreview}>{previewContent}</Text>
      <View style={styles.cardFooter}>
        <Text style={styles.tapToView}>Tap to view full insight</Text>
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

      const response = await apiService.getAdaptedInsights({ limit: 20 });
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
      const response = await apiService.getAdaptedInsights({
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
    <View style={styles.emptyState}>
      <Feather name="refresh-cw" size={64} color={colors.text.tertiary} />
      <Text style={styles.emptyTitle}>No Adapted Insights</Text>
      <Text style={styles.emptyText}>
        When you tap "Adapt to My Training" on friends' insights, your personalized
        versions will appear here
      </Text>
      <TouchableOpacity
        style={styles.feedButton}
        onPress={() => navigation.navigate('SocialFeed')}
      >
        <Feather name="activity" size={18} color={colors.text.primary} />
        <Text style={styles.feedButtonText}>Browse Feed</Text>
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

  if (isLoading && insights.length === 0) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text style={styles.loadingText}>Loading insights...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.backButton}
          onPress={() => navigation.goBack()}
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Adapted Insights</Text>
        <View style={styles.headerSpacer} />
      </View>

      {/* Insights List */}
      <FlatList
        data={insights}
        keyExtractor={item => item.id}
        renderItem={renderInsight}
        ListEmptyComponent={renderEmptyState}
        ListFooterComponent={renderFooter}
        contentContainerStyle={
          insights.length === 0 ? styles.emptyContainer : styles.listContent
        }
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
  headerSpacer: {
    width: 40,
  },
  listContent: {
    paddingVertical: spacing.sm,
  },
  emptyContainer: {
    flexGrow: 1,
  },
  card: {
    marginHorizontal: spacing.md,
    marginVertical: spacing.sm,
    padding: spacing.md,
    borderRadius: borderRadius.lg,
    ...glassCard,
  },
  cardHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: spacing.md,
  },
  iconContainer: {
    width: 32,
    height: 32,
    borderRadius: 16,
    backgroundColor: colors.pierre.violet + '20',
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: spacing.sm,
  },
  timestamp: {
    flex: 1,
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
  },
  contentPreview: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    lineHeight: 22,
  },
  cardFooter: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginTop: spacing.md,
    paddingTop: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  tapToView: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
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
  feedButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.pierre.violet,
    gap: spacing.sm,
  },
  feedButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  loadingMore: {
    paddingVertical: spacing.lg,
    alignItems: 'center',
  },
});
