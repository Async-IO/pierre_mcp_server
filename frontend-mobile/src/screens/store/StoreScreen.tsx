// ABOUTME: Discover screen for browsing and installing coaches
// ABOUTME: Lists published coaches with category filters, search, and install actions

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
  TextInput,
} from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { colors, spacing, fontSize, borderRadius, glassCard } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { StoreCoach, CoachCategory } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

interface StoreScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
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
const COACH_CATEGORY_COLORS: Record<CoachCategory, string> = {
  training: '#10B981',
  nutrition: '#F59E0B',
  recovery: '#6366F1',
  recipes: '#F97316',
  mobility: '#EC4899',
  custom: '#7C3AED',
};

export function StoreScreen({ navigation }: StoreScreenProps) {
  const { isAuthenticated } = useAuth();
  const [coaches, setCoaches] = useState<StoreCoach[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<CoachCategory | 'all'>('all');
  const [selectedSort, setSelectedSort] = useState<SortOption>('popular');
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isSearching, setIsSearching] = useState(false);

  const loadCoaches = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      const response = await apiService.browseStoreCoaches({
        category: selectedCategory === 'all' ? undefined : selectedCategory,
        sort_by: selectedSort,
        limit: 50,
      });
      setCoaches(response.coaches);
    } catch (error) {
      console.error('Failed to load store coaches:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated, selectedCategory, selectedSort]);

  const searchCoaches = useCallback(async (query: string) => {
    if (!isAuthenticated || !query.trim()) {
      loadCoaches();
      return;
    }

    try {
      setIsSearching(true);
      const response = await apiService.searchStoreCoaches(query.trim(), 50);
      setCoaches(response.coaches);
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
      style={[
        styles.filterChip,
        selectedCategory === key && styles.filterChipActive,
      ]}
      onPress={() => setSelectedCategory(key)}
    >
      <Text
        style={[
          styles.filterChipText,
          selectedCategory === key && styles.filterChipTextActive,
        ]}
      >
        {label}
      </Text>
    </TouchableOpacity>
  );

  const renderSortChip = ({ key, label }: { key: SortOption; label: string }) => (
    <TouchableOpacity
      key={key}
      style={[
        styles.sortChip,
        selectedSort === key && styles.sortChipActive,
      ]}
      onPress={() => setSelectedSort(key)}
    >
      <Text
        style={[
          styles.sortChipText,
          selectedSort === key && styles.sortChipTextActive,
        ]}
      >
        {label}
      </Text>
    </TouchableOpacity>
  );

  const renderCoachCard = ({ item, index }: { item: StoreCoach; index: number }) => (
    <TouchableOpacity
      testID={`coach-card-${index}`}
      style={styles.coachCard}
      onPress={() => navigateToCoachDetail(item)}
    >
      <View style={styles.cardHeader}>
        <View
          style={[
            styles.categoryBadge,
            { backgroundColor: COACH_CATEGORY_COLORS[item.category] + '20' },
          ]}
        >
          <Text
            style={[
              styles.categoryBadgeText,
              { color: COACH_CATEGORY_COLORS[item.category] },
            ]}
          >
            {item.category}
          </Text>
        </View>
        <Text style={styles.installCount}>
          {item.install_count} {item.install_count === 1 ? 'install' : 'installs'}
        </Text>
      </View>

      <Text style={styles.coachTitle} numberOfLines={1}>
        {item.title}
      </Text>

      {item.description && (
        <Text style={styles.coachDescription} numberOfLines={2}>
          {item.description}
        </Text>
      )}

      {item.tags.length > 0 && (
        <View style={styles.tagsContainer}>
          {item.tags.slice(0, 3).map((tag, index) => (
            <View key={index} style={styles.tag}>
              <Text style={styles.tagText}>{tag}</Text>
            </View>
          ))}
          {item.tags.length > 3 && (
            <Text style={styles.moreTagsText}>+{item.tags.length - 3}</Text>
          )}
        </View>
      )}
    </TouchableOpacity>
  );

  const renderEmptyState = () => (
    <View style={styles.emptyState}>
      <Text style={styles.emptyStateTitle}>
        {searchQuery ? 'No coaches found' : 'No coaches available'}
      </Text>
      <Text style={styles.emptyStateSubtitle}>
        {searchQuery
          ? `No coaches match "${searchQuery}"`
          : 'No published coaches available yet'}
      </Text>
    </View>
  );

  if (isLoading && coaches.length === 0) {
    return (
      <SafeAreaView style={styles.container} testID="store-screen">
        <View style={styles.loadingContainer} testID="loading-indicator">
          <ActivityIndicator size="large" color={colors.primary[500]} />
          <Text style={styles.loadingText}>Loading coaches...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container} testID="store-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
          testID="menu-button"
        >
          <Text style={styles.menuIcon}>☰</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Discover</Text>
        <View style={styles.headerSpacer} />
      </View>

      {/* Search Bar */}
      <View style={styles.searchContainer}>
        <TextInput
          style={styles.searchInput}
          placeholder="Search coaches..."
          placeholderTextColor={colors.text.secondary}
          value={searchQuery}
          onChangeText={handleSearch}
          testID="search-input"
        />
        {isSearching && (
          <ActivityIndicator
            size="small"
            color={colors.primary[500]}
            style={styles.searchSpinner}
          />
        )}
      </View>

      {/* Category Filters */}
      <View style={styles.filtersContainer}>
        <FlatList
          horizontal
          showsHorizontalScrollIndicator={false}
          data={CATEGORY_FILTERS}
          keyExtractor={(item) => item.key}
          renderItem={({ item }) => renderCategoryChip(item)}
          contentContainerStyle={styles.filtersList}
        />
      </View>

      {/* Sort Options */}
      <View style={styles.sortContainer}>
        <Text style={styles.sortLabel}>Sort by:</Text>
        {SORT_OPTIONS.map((option) => renderSortChip(option))}
      </View>

      {/* Coach List */}
      <FlatList
        testID="coach-list"
        data={coaches}
        keyExtractor={(item) => item.id}
        renderItem={({ item, index }) => renderCoachCard({ item, index })}
        contentContainerStyle={styles.listContent}
        ListEmptyComponent={renderEmptyState}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={() => loadCoaches(true)}
            tintColor={colors.primary[500]}
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
    marginTop: spacing.md,
    color: colors.text.secondary,
    fontSize: fontSize.md,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  menuButton: {
    padding: spacing.sm,
  },
  menuIcon: {
    fontSize: 24,
    color: colors.text.primary,
  },
  headerTitle: {
    flex: 1,
    fontSize: fontSize.xl,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
  },
  headerSpacer: {
    width: 40,
  },
  searchContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
  },
  searchInput: {
    flex: 1,
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    fontSize: fontSize.md,
    color: colors.text.primary,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  searchSpinner: {
    position: 'absolute',
    right: spacing.lg,
  },
  filtersContainer: {
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  filtersList: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    gap: spacing.xs,
  },
  filterChip: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
    backgroundColor: colors.background.secondary,
    marginRight: spacing.xs,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  filterChipActive: {
    backgroundColor: colors.primary[500],
    borderColor: colors.primary[500],
  },
  filterChipText: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  filterChipTextActive: {
    color: colors.text.primary,
    fontWeight: '500',
  },
  sortContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    backgroundColor: colors.background.secondary,
  },
  sortLabel: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    marginRight: spacing.sm,
  },
  sortChip: {
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.sm,
    marginRight: spacing.xs,
  },
  sortChipActive: {
    backgroundColor: colors.primary[500] + '20',
  },
  sortChipText: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  sortChipTextActive: {
    color: colors.primary[500],
    fontWeight: '500',
  },
  listContent: {
    padding: spacing.md,
    paddingBottom: spacing.xl,
  },
  coachCard: {
    backgroundColor: glassCard.background,
    borderRadius: borderRadius.lg,
    padding: spacing.md,
    marginBottom: spacing.md,
    borderWidth: glassCard.borderWidth,
    borderColor: glassCard.borderColor,
    // Glassmorphism shadow
    shadowColor: glassCard.shadowColor,
    shadowOffset: glassCard.shadowOffset,
    shadowOpacity: glassCard.shadowOpacity,
    shadowRadius: glassCard.shadowRadius,
    elevation: glassCard.elevation,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: spacing.xs,
  },
  categoryBadge: {
    paddingHorizontal: spacing.sm,
    paddingVertical: 2,
    borderRadius: borderRadius.sm,
  },
  categoryBadgeText: {
    fontSize: fontSize.xs,
    fontWeight: '500',
    textTransform: 'capitalize',
  },
  installCount: {
    fontSize: fontSize.xs,
    color: colors.text.secondary,
  },
  coachTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.xs,
  },
  coachDescription: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    marginBottom: spacing.sm,
    lineHeight: 20,
  },
  tagsContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    alignItems: 'center',
  },
  tag: {
    backgroundColor: colors.background.primary,
    paddingHorizontal: spacing.sm,
    paddingVertical: 2,
    borderRadius: borderRadius.sm,
    marginRight: spacing.xs,
    marginBottom: spacing.xs,
  },
  tagText: {
    fontSize: fontSize.xs,
    color: colors.text.secondary,
  },
  moreTagsText: {
    fontSize: fontSize.xs,
    color: colors.text.secondary,
    marginLeft: spacing.xs,
  },
  emptyState: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingVertical: spacing.xl * 2,
  },
  emptyStateTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.xs,
  },
  emptyStateSubtitle: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    textAlign: 'center',
  },
});
