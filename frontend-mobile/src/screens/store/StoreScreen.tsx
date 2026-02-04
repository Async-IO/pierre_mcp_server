// ABOUTME: Discover screen for browsing and installing coaches
// ABOUTME: Lists published coaches with category filters, search, and install actions

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
import { FlashList } from '@shopify/flash-list';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, glassCard } from '../../constants/theme';
import { FloatingSearchBar } from '../../components/ui';

// Shadow styles for coach cards (React Native shadows cannot use className)
const coachCardShadow: ViewStyle = {
  shadowColor: glassCard.shadowColor,
  shadowOffset: glassCard.shadowOffset,
  shadowOpacity: glassCard.shadowOpacity,
  shadowRadius: glassCard.shadowRadius,
  elevation: glassCard.elevation,
};
import { storeApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { StoreCoach, CoachCategory } from '../../types';
import type { DiscoverStackParamList } from '../../navigation/MainTabs';

interface StoreScreenProps {
  navigation: NativeStackNavigationProp<DiscoverStackParamList>;
}

// Category filter options
const CATEGORY_FILTERS: Array<{ key: CoachCategory | 'all'; label: string }> = [
  { key: 'all', label: 'All' },
  { key: 'training', label: 'Training' },
  { key: 'nutrition', label: 'Nutrition' },
  { key: 'recovery', label: 'Recovery' },
  { key: 'recipes', label: 'Recipes' },
  { key: 'mobility', label: 'Mobility' },
  { key: 'custom', label: 'Custom' },
];

// Sort options
type SortOption = 'newest' | 'popular' | 'title';
const SORT_OPTIONS: Array<{ key: SortOption; label: string }> = [
  { key: 'popular', label: 'Popular' },
  { key: 'newest', label: 'Newest' },
  { key: 'title', label: 'A-Z' },
];

// Coach category colors
const COACH_CATEGORY_COLORS: Record<string, string> = {
  training: '#10B981',
  nutrition: '#F59E0B',
  recovery: '#6366F1',
  recipes: '#F97316',
  mobility: '#EC4899',
  custom: '#7C3AED',
};

export function StoreScreen({ navigation }: StoreScreenProps) {
  const { isAuthenticated } = useAuth();
  const stackNavigation = useNavigation();
  const canGoBack = stackNavigation.canGoBack();
  const [coaches, setCoaches] = useState<StoreCoach[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<CoachCategory | 'all'>('all');
  const [selectedSort, setSelectedSort] = useState<SortOption>('popular');
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(true);

  const loadCoaches = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      const response = await storeApi.browse({
        category: selectedCategory === 'all' ? undefined : selectedCategory,
        sort_by: selectedSort,
        limit: 20,
      });
      setCoaches(response.coaches);
      setNextCursor(response.next_cursor ?? null);
      setHasMore(response.has_more ?? false);
    } catch (error) {
      console.error('Failed to load store coaches:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated, selectedCategory, selectedSort]);

  const loadMoreCoaches = useCallback(async () => {
    if (!isAuthenticated || !hasMore || isLoadingMore || !nextCursor) return;

    try {
      setIsLoadingMore(true);
      const response = await storeApi.browse({
        category: selectedCategory === 'all' ? undefined : selectedCategory,
        sort_by: selectedSort,
        limit: 20,
        cursor: nextCursor,
      });
      setCoaches(prev => [...prev, ...response.coaches]);
      setNextCursor(response.next_cursor ?? null);
      setHasMore(response.has_more ?? false);
    } catch (error) {
      console.error('Failed to load more coaches:', error);
    } finally {
      setIsLoadingMore(false);
    }
  }, [isAuthenticated, hasMore, isLoadingMore, nextCursor, selectedCategory, selectedSort]);

  const searchCoaches = useCallback(async (query: string) => {
    if (!isAuthenticated || !query.trim()) {
      loadCoaches();
      return;
    }

    try {
      setIsSearching(true);
      const response = await storeApi.search(query.trim(), 50);
      setCoaches(response.coaches);
      setNextCursor(null);
      setHasMore(false);
    } catch (error) {
      console.error('Failed to search coaches:', error);
    } finally {
      setIsSearching(false);
    }
  }, [isAuthenticated, loadCoaches]);

  // Reload when screen focuses or filters change
  useFocusEffect(
    useCallback(() => {
      if (searchQuery.trim()) {
        searchCoaches(searchQuery);
      } else {
        loadCoaches();
      }
    }, [loadCoaches, searchCoaches, searchQuery])
  );

  const handleSearch = (text: string) => {
    setSearchQuery(text);
    // Debounce search
    if (text.trim()) {
      setTimeout(() => searchCoaches(text), 300);
    } else {
      loadCoaches();
    }
  };

  const navigateToCoachDetail = (coach: StoreCoach) => {
    navigation.navigate('StoreCoachDetail', { coachId: coach.id });
  };

  const renderCategoryChip = ({ key, label }: { key: CoachCategory | 'all'; label: string }) => (
    <TouchableOpacity
      key={key}
      className={`px-3 py-1 rounded-full mr-1 border ${
        selectedCategory === key
          ? 'bg-primary-500 border-primary-500'
          : 'bg-background-secondary border-border-default'
      }`}
      onPress={() => setSelectedCategory(key)}
    >
      <Text
        className={`text-sm ${
          selectedCategory === key
            ? 'text-text-primary font-medium'
            : 'text-text-secondary'
        }`}
      >
        {label}
      </Text>
    </TouchableOpacity>
  );

  const renderSortChip = ({ key, label }: { key: SortOption; label: string }) => (
    <TouchableOpacity
      key={key}
      className={`px-2 py-1 rounded mr-1 ${
        selectedSort === key ? 'bg-primary-500/20' : ''
      }`}
      onPress={() => setSelectedSort(key)}
    >
      <Text
        className={`text-sm ${
          selectedSort === key
            ? 'text-primary-500 font-medium'
            : 'text-text-secondary'
        }`}
      >
        {label}
      </Text>
    </TouchableOpacity>
  );

  const renderCoachCard = ({ item, index }: { item: StoreCoach; index: number }) => (
    <TouchableOpacity
      testID={`coach-card-${index}`}
      className="bg-white/[0.03] rounded-lg p-3 mb-3 border border-white/[0.08]"
      style={coachCardShadow}
      onPress={() => navigateToCoachDetail(item)}
    >
      <View className="flex-row justify-between items-center mb-1">
        <View
          className="px-2 py-0.5 rounded"
          style={{ backgroundColor: COACH_CATEGORY_COLORS[item.category] + '20' }}
        >
          <Text
            className="text-xs font-medium capitalize"
            style={{ color: COACH_CATEGORY_COLORS[item.category] }}
          >
            {item.category}
          </Text>
        </View>
        <Text className="text-xs text-text-secondary">
          {item.install_count} {item.install_count === 1 ? 'install' : 'installs'}
        </Text>
      </View>

      <Text className="text-lg font-semibold text-text-primary mb-1" numberOfLines={1}>
        {item.title}
      </Text>

      {item.description && (
        <Text className="text-sm text-text-secondary mb-2 leading-5" numberOfLines={2}>
          {item.description}
        </Text>
      )}

      {item.tags.length > 0 && (
        <View className="flex-row flex-wrap items-center">
          {item.tags.slice(0, 3).map((tag, tagIndex) => (
            <View key={tagIndex} className="bg-background-primary px-2 py-0.5 rounded mr-1 mb-1">
              <Text className="text-xs text-text-secondary">{tag}</Text>
            </View>
          ))}
          {item.tags.length > 3 && (
            <Text className="text-xs text-text-secondary ml-1">+{item.tags.length - 3}</Text>
          )}
        </View>
      )}
    </TouchableOpacity>
  );

  const renderEmptyState = () => (
    <View className="flex-1 justify-center items-center py-16">
      <Text className="text-lg font-semibold text-text-primary mb-1">
        {searchQuery ? 'No coaches found' : 'No coaches available'}
      </Text>
      <Text className="text-base text-text-secondary text-center">
        {searchQuery
          ? `No coaches match "${searchQuery}"`
          : 'No published coaches available yet'}
      </Text>
    </View>
  );

  if (isLoading && coaches.length === 0) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary" testID="store-screen">
        <View className="flex-1 justify-center items-center" testID="loading-indicator">
          <ActivityIndicator size="large" color={colors.primary[500]} />
          <Text className="mt-3 text-text-secondary text-base">Loading coaches...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="store-screen">
      {/* Header */}
      <View className="flex-row items-center px-3 py-2 border-b border-border-default">
        {canGoBack ? (
          <TouchableOpacity
            className="p-2"
            onPress={() => stackNavigation.goBack()}
            testID="back-button"
          >
            <Feather name="arrow-left" size={24} color={colors.text.primary} />
          </TouchableOpacity>
        ) : (
          <View className="w-10" />
        )}
        <Text className="flex-1 text-xl font-semibold text-text-primary text-center">Discover</Text>
        <View className="w-10" />
      </View>

      {/* Category Filters */}
      <View className="border-b border-border-default">
        <FlatList
          horizontal
          showsHorizontalScrollIndicator={false}
          data={CATEGORY_FILTERS}
          keyExtractor={(item) => item.key}
          renderItem={({ item }) => renderCategoryChip(item)}
          contentContainerStyle={{ paddingHorizontal: spacing.md, paddingVertical: spacing.sm }}
        />
      </View>

      {/* Sort Options */}
      <View className="flex-row items-center px-3 py-1 bg-background-secondary">
        <Text className="text-sm text-text-secondary mr-2">Sort by:</Text>
        {SORT_OPTIONS.map((option) => renderSortChip(option))}
      </View>

      {/* Coach List with FlashList for optimized performance */}
      <FlashList
        testID="coach-list"
        data={coaches}
        keyExtractor={(item) => item.id}
        renderItem={({ item, index }) => renderCoachCard({ item, index })}
        contentContainerStyle={{ padding: spacing.md, paddingBottom: 100 }}
        ListEmptyComponent={renderEmptyState}
        onEndReached={loadMoreCoaches}
        onEndReachedThreshold={0.5}
        ListFooterComponent={
          isLoadingMore ? (
            <View className="flex-row items-center justify-center py-4 gap-2">
              <ActivityIndicator size="small" color={colors.primary[500]} />
              <Text className="text-sm text-text-secondary">Loading more coaches...</Text>
            </View>
          ) : null
        }
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={() => loadCoaches(true)}
            tintColor={colors.primary[500]}
          />
        }
      />

      {/* Floating Search Bar */}
      <FloatingSearchBar
        value={searchQuery}
        onChangeText={handleSearch}
        onSubmit={() => searchCoaches(searchQuery)}
        placeholder="Search coaches..."
        isSearching={isSearching}
        testID="search-input"
      />
    </SafeAreaView>
  );
}

