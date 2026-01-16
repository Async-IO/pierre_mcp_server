// ABOUTME: Coach library screen for managing user's AI coaches
// ABOUTME: Lists coaches with category filters, favorites toggle, and CRUD actions

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
  Modal,
  RefreshControl,
  ScrollView,
} from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { Coach, CoachCategory } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

interface CoachLibraryScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
}

// Category filter options
const CATEGORY_FILTERS: Array<{ key: CoachCategory | 'all'; label: string }> = [
  { key: 'all', label: 'All' },
  { key: 'training', label: 'Training' },
  { key: 'nutrition', label: 'Nutrition' },
  { key: 'recovery', label: 'Recovery' },
  { key: 'recipes', label: 'Recipes' },
  { key: 'custom', label: 'Custom' },
];

// Coach category colors matching web frontend
const COACH_CATEGORY_COLORS: Record<CoachCategory, string> = {
  training: '#10B981',  // Green
  nutrition: '#F59E0B', // Orange
  recovery: '#6366F1',  // Indigo/Blue
  recipes: '#F97316',   // Amber
  custom: '#7C3AED',    // Purple
};

export function CoachLibraryScreen({ navigation }: CoachLibraryScreenProps) {
  const { isAuthenticated } = useAuth();
  const [coaches, setCoaches] = useState<Coach[]>([]);
  const [filteredCoaches, setFilteredCoaches] = useState<Coach[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<CoachCategory | 'all'>('all');
  const [showFavoritesOnly, setShowFavoritesOnly] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [selectedCoach, setSelectedCoach] = useState<Coach | null>(null);

  const loadCoaches = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }
      const response = await apiService.listCoaches();
      // Sort: favorites first, then by use_count descending
      const sorted = [...response.coaches].sort((a, b) => {
        if (a.is_favorite !== b.is_favorite) {
          return a.is_favorite ? -1 : 1;
        }
        return b.use_count - a.use_count;
      });
      setCoaches(sorted);
    } catch (error) {
      console.error('Failed to load coaches:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated]);

  // Apply filters whenever coaches, category, or favorites filter changes
  React.useEffect(() => {
    let filtered = [...coaches];

    // Filter by category
    if (selectedCategory !== 'all') {
      filtered = filtered.filter((coach) => coach.category === selectedCategory);
    }

    // Filter favorites only
    if (showFavoritesOnly) {
      filtered = filtered.filter((coach) => coach.is_favorite);
    }

    setFilteredCoaches(filtered);
  }, [coaches, selectedCategory, showFavoritesOnly]);

  useFocusEffect(
    useCallback(() => {
      loadCoaches();
    }, [loadCoaches])
  );

  const handleRefresh = () => {
    loadCoaches(true);
  };

  const handleCoachPress = (coach: Coach) => {
    navigation.navigate('CoachEditor', { coachId: coach.id });
  };

  const handleCoachLongPress = (coach: Coach) => {
    setSelectedCoach(coach);
    setActionMenuVisible(true);
  };

  const handleCreateCoach = () => {
    navigation.navigate('CoachEditor', { coachId: undefined });
  };

  const handleToggleFavorite = async () => {
    if (!selectedCoach) return;
    setActionMenuVisible(false);

    try {
      const result = await apiService.toggleCoachFavorite(selectedCoach.id);
      setCoaches((prev) =>
        prev.map((c) =>
          c.id === selectedCoach.id ? { ...c, is_favorite: result.is_favorite } : c
        )
      );
    } catch (error) {
      console.error('Failed to toggle favorite:', error);
      Alert.alert('Error', 'Failed to update favorite status');
    }
  };

  const handleRename = () => {
    if (!selectedCoach) return;
    setActionMenuVisible(false);

    Alert.prompt(
      'Rename Coach',
      'Enter a new name for this coach',
      async (newTitle: string | undefined) => {
        if (!newTitle?.trim() || !selectedCoach) return;
        try {
          const updated = await apiService.updateCoach(selectedCoach.id, {
            title: newTitle.trim(),
          });
          setCoaches((prev) =>
            prev.map((c) => (c.id === selectedCoach.id ? { ...c, title: updated.title } : c))
          );
        } catch (error) {
          console.error('Failed to rename coach:', error);
          Alert.alert('Error', 'Failed to rename coach');
        }
      },
      'plain-text',
      selectedCoach.title
    );
  };

  const handleDelete = () => {
    if (!selectedCoach) return;
    setActionMenuVisible(false);

    Alert.alert(
      'Delete Coach',
      `Are you sure you want to delete "${selectedCoach.title}"? This cannot be undone.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete',
          style: 'destructive',
          onPress: async () => {
            try {
              await apiService.deleteCoach(selectedCoach.id);
              setCoaches((prev) => prev.filter((c) => c.id !== selectedCoach.id));
            } catch (error) {
              console.error('Failed to delete coach:', error);
              Alert.alert('Error', 'Failed to delete coach');
            }
          },
        },
      ]
    );
  };

  const closeActionMenu = () => {
    setActionMenuVisible(false);
    setSelectedCoach(null);
  };

  const renderCoachCard = ({ item }: { item: Coach }) => (
    <TouchableOpacity
      style={[
        styles.coachCard,
        { borderLeftColor: COACH_CATEGORY_COLORS[item.category] },
      ]}
      onPress={() => handleCoachPress(item)}
      onLongPress={() => handleCoachLongPress(item)}
      delayLongPress={300}
      activeOpacity={0.7}
    >
      <View style={styles.coachHeader}>
        <Text style={styles.coachTitle} numberOfLines={1}>
          {item.title}
        </Text>
        <TouchableOpacity
          style={styles.favoriteButton}
          onPress={() => {
            setSelectedCoach(item);
            handleToggleFavorite();
          }}
          hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
        >
          <Text style={[styles.favoriteIcon, item.is_favorite && styles.favoriteIconActive]}>
            {item.is_favorite ? '★' : '☆'}
          </Text>
        </TouchableOpacity>
      </View>
      <Text style={styles.coachTokens}>
        ~{Math.ceil(item.token_count / 4)} tokens ({((item.token_count / 128000) * 100).toFixed(1)}% context)
      </Text>
      {item.description && (
        <Text style={styles.coachDescription} numberOfLines={2}>
          {item.description}
        </Text>
      )}
      <View style={styles.coachFooter}>
        <Text style={styles.coachUsage}>Used {item.use_count} times</Text>
      </View>
    </TouchableOpacity>
  );

  const renderCategoryFilter = () => (
    <View style={styles.filterContainer}>
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        contentContainerStyle={styles.filterScrollContent}
      >
        {CATEGORY_FILTERS.map((filter) => (
          <TouchableOpacity
            key={filter.key}
            style={[
              styles.filterChip,
              selectedCategory === filter.key && styles.filterChipActive,
            ]}
            onPress={() => setSelectedCategory(filter.key)}
          >
            <Text
              style={[
                styles.filterChipText,
                selectedCategory === filter.key && styles.filterChipTextActive,
              ]}
            >
              {filter.label}
            </Text>
          </TouchableOpacity>
        ))}
      </ScrollView>
      <TouchableOpacity
        style={[styles.favoritesToggle, showFavoritesOnly && styles.favoritesToggleActive]}
        onPress={() => setShowFavoritesOnly(!showFavoritesOnly)}
      >
        <Text style={[styles.favoritesToggleIcon, showFavoritesOnly && styles.favoritesToggleIconActive]}>
          ★
        </Text>
      </TouchableOpacity>
    </View>
  );

  return (
    <SafeAreaView style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
        >
          <Text style={styles.menuIcon}>☰</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>My Coaches</Text>
        <View style={styles.headerSpacer} />
      </View>

      {/* Category Filter */}
      {renderCategoryFilter()}

      {/* Coaches List */}
      {isLoading ? (
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.primary[500]} />
        </View>
      ) : (
        <FlatList
          data={filteredCoaches}
          renderItem={renderCoachCard}
          keyExtractor={(item) => item.id}
          contentContainerStyle={styles.listContent}
          showsVerticalScrollIndicator={false}
          refreshControl={
            <RefreshControl
              refreshing={isRefreshing}
              onRefresh={handleRefresh}
              tintColor={colors.primary[500]}
            />
          }
          ListEmptyComponent={
            <View style={styles.emptyContainer}>
              <Text style={styles.emptyTitle}>
                {showFavoritesOnly
                  ? 'No favorite coaches'
                  : selectedCategory !== 'all'
                  ? `No ${selectedCategory} coaches`
                  : 'No coaches yet'}
              </Text>
              <Text style={styles.emptySubtitle}>
                {coaches.length === 0
                  ? 'Create your first coach to customize how Pierre helps you.'
                  : 'Try adjusting your filters.'}
              </Text>
            </View>
          }
        />
      )}

      {/* Floating Action Button */}
      <TouchableOpacity style={styles.fab} onPress={handleCreateCoach}>
        <Text style={styles.fabIcon}>+</Text>
      </TouchableOpacity>

      {/* Action Menu Modal */}
      <Modal
        visible={actionMenuVisible}
        animationType="fade"
        transparent
        onRequestClose={closeActionMenu}
      >
        <TouchableOpacity
          style={styles.modalOverlay}
          activeOpacity={1}
          onPress={closeActionMenu}
        >
          <View style={styles.actionMenuContainer}>
            <TouchableOpacity style={styles.actionMenuItem} onPress={handleToggleFavorite}>
              <Text style={styles.actionMenuIcon}>
                {selectedCoach?.is_favorite ? '☆' : '★'}
              </Text>
              <Text style={styles.actionMenuText}>
                {selectedCoach?.is_favorite ? 'Remove from favorites' : 'Add to favorites'}
              </Text>
            </TouchableOpacity>

            <TouchableOpacity style={styles.actionMenuItem} onPress={handleRename}>
              <Text style={styles.actionMenuIcon}>✎</Text>
              <Text style={styles.actionMenuText}>Rename</Text>
            </TouchableOpacity>

            <TouchableOpacity style={styles.actionMenuItem} onPress={handleDelete}>
              <Text style={styles.actionMenuIconDanger}>🗑</Text>
              <Text style={styles.actionMenuTextDanger}>Delete</Text>
            </TouchableOpacity>
          </View>
        </TouchableOpacity>
      </Modal>
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
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  menuButton: {
    width: 40,
    height: 40,
    alignItems: 'center',
    justifyContent: 'center',
  },
  menuIcon: {
    fontSize: 20,
    color: colors.text.primary,
  },
  headerTitle: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
  },
  headerSpacer: {
    width: 40,
  },
  filterContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  filterScrollContent: {
    paddingHorizontal: spacing.md,
    gap: spacing.xs,
  },
  filterChip: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.subtle,
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
    fontWeight: '600',
  },
  favoritesToggle: {
    width: 36,
    height: 36,
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.md,
    borderRadius: borderRadius.full,
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  favoritesToggleActive: {
    backgroundColor: '#F59E0B',
    borderColor: '#F59E0B',
  },
  favoritesToggleIcon: {
    fontSize: 18,
    color: colors.text.tertiary,
  },
  favoritesToggleIconActive: {
    color: colors.text.primary,
  },
  loadingContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  listContent: {
    flexGrow: 1,
    padding: spacing.md,
    paddingBottom: 100, // Space for FAB
    gap: spacing.md,
  },
  coachCard: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    borderLeftWidth: 4,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  coachHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: spacing.xs,
  },
  coachTitle: {
    flex: 1,
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
    marginRight: spacing.sm,
  },
  favoriteButton: {
    padding: spacing.xs,
  },
  favoriteIcon: {
    fontSize: 20,
    color: colors.text.tertiary,
  },
  favoriteIconActive: {
    color: '#F59E0B',
  },
  coachTokens: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    marginBottom: spacing.xs,
  },
  coachDescription: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    marginBottom: spacing.sm,
    lineHeight: 20,
  },
  coachFooter: {
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
    paddingTop: spacing.sm,
    marginTop: spacing.xs,
  },
  coachUsage: {
    fontSize: fontSize.sm,
    color: colors.text.tertiary,
  },
  emptyContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingTop: spacing.xxl,
    paddingHorizontal: spacing.lg,
  },
  emptyTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.sm,
    textAlign: 'center',
  },
  emptySubtitle: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
    textAlign: 'center',
  },
  fab: {
    position: 'absolute',
    bottom: spacing.xl,
    right: spacing.lg,
    width: 56,
    height: 56,
    borderRadius: 28,
    backgroundColor: colors.primary[500],
    alignItems: 'center',
    justifyContent: 'center',
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 6,
    elevation: 6,
  },
  fabIcon: {
    fontSize: 32,
    color: colors.text.primary,
    marginTop: -2,
  },
  modalOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.3)',
    justifyContent: 'center',
    alignItems: 'center',
  },
  actionMenuContainer: {
    backgroundColor: colors.background.primary,
    borderRadius: borderRadius.lg,
    paddingVertical: spacing.xs,
    minWidth: 220,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 8,
  },
  actionMenuItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
  },
  actionMenuIcon: {
    fontSize: 18,
    marginRight: spacing.sm,
    width: 24,
  },
  actionMenuIconDanger: {
    fontSize: 18,
    marginRight: spacing.sm,
    width: 24,
  },
  actionMenuText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  actionMenuTextDanger: {
    fontSize: fontSize.md,
    color: colors.error,
  },
});
