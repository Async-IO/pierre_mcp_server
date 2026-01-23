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
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius, glassCard } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { PromptDialog } from '../../components/ui';
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
  { key: 'mobility', label: 'Mobility' },
  { key: 'custom', label: 'Custom' },
];

// Source filter options (user-created vs system coaches)
type CoachSource = 'all' | 'user' | 'system';
const SOURCE_FILTERS: Array<{ key: CoachSource; label: string }> = [
  { key: 'all', label: 'All Sources' },
  { key: 'user', label: 'My Coaches' },
  { key: 'system', label: 'System' },
];

// Coach category colors matching web frontend
const COACH_CATEGORY_COLORS: Record<CoachCategory, string> = {
  training: '#10B981',  // Green
  nutrition: '#F59E0B', // Orange
  recovery: '#6366F1',  // Indigo/Blue
  recipes: '#F97316',   // Amber
  mobility: '#EC4899',  // Pink - for stretching/yoga
  custom: '#7C3AED',    // Purple
};

export function CoachLibraryScreen({ navigation }: CoachLibraryScreenProps) {
  const { isAuthenticated } = useAuth();
  const [coaches, setCoaches] = useState<Coach[]>([]);
  const [filteredCoaches, setFilteredCoaches] = useState<Coach[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<CoachCategory | 'all'>('all');
  const [selectedSource, setSelectedSource] = useState<CoachSource>('all');
  const [showFavoritesOnly, setShowFavoritesOnly] = useState(false);
  const [showHidden, setShowHidden] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [selectedCoach, setSelectedCoach] = useState<Coach | null>(null);
  const [renamePromptVisible, setRenamePromptVisible] = useState(false);

  const loadCoaches = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      // Always load all coaches (including hidden) and hidden list in parallel
      // We filter locally based on showHidden state to preserve local changes
      const [coachesResponse, hiddenResponse] = await Promise.all([
        apiService.listCoaches({ include_hidden: true }),
        apiService.getHiddenCoaches(),
      ]);

      // Create a set of hidden coach IDs for quick lookup
      const hiddenIds = new Set((hiddenResponse.coaches || []).map((c) => c.id));

      // Mark coaches as hidden if they're in the hidden list
      const coachesWithHiddenFlag = coachesResponse.coaches.map((coach) => ({
        ...coach,
        is_hidden: hiddenIds.has(coach.id),
      }));

      // Sort: favorites first, then by use_count descending
      const sorted = [...coachesWithHiddenFlag].sort((a, b) => {
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

  // Apply filters whenever coaches, category, source, favorites, or showHidden changes
  React.useEffect(() => {
    let filtered = [...coaches];

    // Filter out hidden coaches unless showHidden is enabled
    if (!showHidden) {
      filtered = filtered.filter((coach) => !coach.is_hidden);
    }

    // Filter by category
    if (selectedCategory !== 'all') {
      filtered = filtered.filter((coach) => coach.category === selectedCategory);
    }

    // Filter by source (user-created vs system)
    if (selectedSource === 'user') {
      filtered = filtered.filter((coach) => !coach.is_system);
    } else if (selectedSource === 'system') {
      filtered = filtered.filter((coach) => coach.is_system);
    }

    // Filter favorites only
    if (showFavoritesOnly) {
      filtered = filtered.filter((coach) => coach.is_favorite);
    }

    setFilteredCoaches(filtered);
  }, [coaches, selectedCategory, selectedSource, showFavoritesOnly, showHidden]);

  useFocusEffect(
    useCallback(() => {
      loadCoaches();
    }, [loadCoaches])
  );

  const handleRefresh = () => {
    loadCoaches(true);
  };

  const handleCoachPress = (coach: Coach) => {
    navigation.navigate('CoachDetail', { coachId: coach.id });
  };

  const handleCoachLongPress = (coach: Coach) => {
    setSelectedCoach(coach);
    setActionMenuVisible(true);
  };

  const handleCreateCoach = () => {
    navigation.navigate('CoachWizard', { coachId: undefined });
  };

  const handleToggleFavorite = async (coach?: Coach) => {
    const targetCoach = coach ?? selectedCoach;
    if (!targetCoach) return;
    setActionMenuVisible(false);

    try {
      const result = await apiService.toggleCoachFavorite(targetCoach.id);
      setCoaches((prev) =>
        prev.map((c) =>
          c.id === targetCoach.id ? { ...c, is_favorite: result.is_favorite } : c
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
    setRenamePromptVisible(true);
  };

  const handleRenameSubmit = async (newTitle: string) => {
    setRenamePromptVisible(false);
    if (!selectedCoach) return;

    try {
      const updated = await apiService.updateCoach(selectedCoach.id, {
        title: newTitle,
      });
      setCoaches((prev) =>
        prev.map((c) => (c.id === selectedCoach.id ? { ...c, title: updated.title } : c))
      );
    } catch (error) {
      console.error('Failed to rename coach:', error);
      Alert.alert('Error', 'Failed to rename coach');
    } finally {
      setSelectedCoach(null);
    }
  };

  const handleRenameCancel = () => {
    setRenamePromptVisible(false);
    setSelectedCoach(null);
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

  const handleHideCoach = async (coach?: Coach) => {
    const targetCoach = coach ?? selectedCoach;
    if (!targetCoach) return;
    setActionMenuVisible(false);

    try {
      await apiService.hideCoach(targetCoach.id);
      // Remove from list if not showing hidden coaches, otherwise update the flag
      if (showHidden) {
        setCoaches((prev) =>
          prev.map((c) => (c.id === targetCoach.id ? { ...c, is_hidden: true } : c))
        );
      } else {
        setCoaches((prev) => prev.filter((c) => c.id !== targetCoach.id));
      }
    } catch (error) {
      console.error('Failed to hide coach:', error);
      Alert.alert('Error', 'Failed to hide coach');
    }
  };

  const handleShowCoach = async (coach?: Coach) => {
    const targetCoach = coach ?? selectedCoach;
    if (!targetCoach) return;
    setActionMenuVisible(false);

    try {
      await apiService.showCoach(targetCoach.id);
      // Update main coaches list - add if not present, update if present
      setCoaches((prev) => {
        const exists = prev.some((c) => c.id === targetCoach.id);
        if (exists) {
          return prev.map((c) => (c.id === targetCoach.id ? { ...c, is_hidden: false } : c));
        }
        // Coach was only loaded via include_hidden, add it to main list
        return [...prev, { ...targetCoach, is_hidden: false }];
      });
    } catch (error) {
      console.error('Failed to show coach:', error);
      Alert.alert('Error', 'Failed to show coach');
    }
  };

  const handleForkCoach = (coach?: Coach) => {
    const targetCoach = coach ?? selectedCoach;
    if (!targetCoach || !targetCoach.is_system) return;
    setActionMenuVisible(false);

    Alert.alert(
      'Fork Coach',
      `Create your own copy of "${targetCoach.title}"? You can customize the forked coach however you like.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Fork',
          onPress: async () => {
            try {
              const result = await apiService.forkCoach(targetCoach.id);
              // Add the new forked coach to the list
              setCoaches((prev) => [result.coach, ...prev]);
              // Navigate to wizard to customize
              navigation.navigate('CoachWizard', { coachId: result.coach.id });
            } catch (error) {
              console.error('Failed to fork coach:', error);
              Alert.alert('Error', 'Failed to fork coach. Please try again.');
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

  const renderCoachCard = ({ item }: { item: Coach }) => {
    const isSystemCoach = item.is_system;
    const isHidden = item.is_hidden;

    return (
      <TouchableOpacity
        style={[
          styles.coachCard,
          { borderLeftColor: COACH_CATEGORY_COLORS[item.category] },
          isHidden && styles.coachCardHidden,
        ]}
        onPress={() => handleCoachPress(item)}
        onLongPress={() => handleCoachLongPress(item)}
        delayLongPress={300}
        activeOpacity={0.7}
        testID={`coach-card-${item.id}`}
      >
        <View style={styles.coachHeader}>
          <View style={styles.coachTitleContainer}>
            <Text style={[styles.coachTitle, isHidden && styles.coachTitleHidden]} numberOfLines={1}>
              {item.title}
            </Text>
            {isSystemCoach && (
              <View style={styles.systemBadge}>
                <Text style={styles.systemBadgeText}>System</Text>
              </View>
            )}
            {isHidden && (
              <Feather name="eye-off" size={14} color={colors.text.tertiary} style={styles.hiddenIcon} />
            )}
          </View>
          <View style={styles.coachActions}>
            {/* Fork button for system coaches */}
            {isSystemCoach && (
              <TouchableOpacity
                style={styles.forkButton}
                onPress={() => handleForkCoach(item)}
                hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
                testID={`fork-button-${item.id}`}
              >
                <Feather name="copy" size={16} color={colors.text.tertiary} />
              </TouchableOpacity>
            )}
            {/* Hide/Show button for system coaches */}
            {isSystemCoach && (
              <TouchableOpacity
                style={styles.hideButton}
                onPress={() => {
                  if (isHidden) {
                    handleShowCoach(item);
                  } else {
                    handleHideCoach(item);
                  }
                }}
                hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
                testID={`hide-button-${item.id}`}
              >
                <Feather
                  name={isHidden ? 'eye' : 'eye-off'}
                  size={16}
                  color={isHidden ? colors.primary[400] : colors.text.tertiary}
                />
              </TouchableOpacity>
            )}
            <TouchableOpacity
              style={styles.favoriteButton}
              onPress={() => handleToggleFavorite(item)}
              hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
              testID={`favorite-button-${item.id}`}
            >
              <Feather
                name="star"
                size={18}
                color={item.is_favorite ? '#F59E0B' : colors.text.tertiary}
                fill={item.is_favorite ? '#F59E0B' : 'none'}
              />
            </TouchableOpacity>
          </View>
        </View>
        <Text style={[styles.coachTokens, isHidden && styles.coachTextHidden]}>
          ~{Math.ceil(item.token_count / 4)} tokens ({((item.token_count / 128000) * 100).toFixed(1)}% context)
        </Text>
        {item.description && (
          <Text style={[styles.coachDescription, isHidden && styles.coachTextHidden]} numberOfLines={2}>
            {item.description}
          </Text>
        )}
        <View style={styles.coachFooter}>
          <Text style={[styles.coachUsage, isHidden && styles.coachTextHidden]}>
            Used {item.use_count} times
          </Text>
        </View>
      </TouchableOpacity>
    );
  };

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
            testID={`category-filter-${filter.key}`}
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
    </View>
  );

  const renderSourceFilter = () => (
    <View style={styles.sourceFilterContainer}>
      {SOURCE_FILTERS.map((filter) => (
        <TouchableOpacity
          key={filter.key}
          style={[
            styles.sourceFilterChip,
            selectedSource === filter.key && styles.sourceFilterChipActive,
          ]}
          onPress={() => setSelectedSource(filter.key)}
          testID={`source-filter-${filter.key}`}
        >
          <Text
            style={[
              styles.sourceFilterChipText,
              selectedSource === filter.key && styles.sourceFilterChipTextActive,
            ]}
          >
            {filter.label}
          </Text>
        </TouchableOpacity>
      ))}
    </View>
  );

  return (
    <SafeAreaView style={styles.container} testID="coach-library-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
          testID="menu-button"
        >
          <Text style={styles.menuIcon}>☰</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>My Coaches</Text>
        <View style={styles.headerActions}>
          <TouchableOpacity
            style={[styles.headerActionButton, showFavoritesOnly && styles.headerActionButtonActive]}
            onPress={() => setShowFavoritesOnly(!showFavoritesOnly)}
            hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
            testID="favorites-toggle"
          >
            <Feather
              name="star"
              size={20}
              color={showFavoritesOnly ? '#F59E0B' : colors.text.tertiary}
            />
          </TouchableOpacity>
          <TouchableOpacity
            style={[styles.headerActionButton, showHidden && styles.headerActionButtonActive]}
            onPress={() => setShowHidden(!showHidden)}
            hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
            testID="show-hidden-toggle"
          >
            <Feather
              name={showHidden ? 'eye' : 'eye-off'}
              size={20}
              color={showHidden ? colors.primary[400] : colors.text.tertiary}
            />
          </TouchableOpacity>
        </View>
      </View>

      {/* Category Filter */}
      {renderCategoryFilter()}

      {/* Source Filter (User vs System) */}
      {renderSourceFilter()}

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
                  : selectedSource === 'user'
                  ? 'No user-created coaches'
                  : selectedSource === 'system'
                  ? 'No system coaches'
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
      <TouchableOpacity style={styles.fab} onPress={handleCreateCoach} testID="create-coach-button">
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
            <TouchableOpacity style={styles.actionMenuItem} onPress={() => handleToggleFavorite()}>
              <View style={styles.actionMenuIconContainer}>
                <Feather
                  name="star"
                  size={18}
                  color={selectedCoach?.is_favorite ? '#F59E0B' : colors.text.primary}
                />
              </View>
              <Text style={styles.actionMenuText}>
                {selectedCoach?.is_favorite ? 'Remove from favorites' : 'Add to favorites'}
              </Text>
            </TouchableOpacity>

            {/* Hide/Show option for system or assigned coaches */}
            {(selectedCoach?.is_system || selectedCoach?.is_assigned) && (
              <TouchableOpacity
                style={styles.actionMenuItem}
                onPress={() => (selectedCoach?.is_hidden ? handleShowCoach() : handleHideCoach())}
              >
                <View style={styles.actionMenuIconContainer}>
                  <Feather
                    name={selectedCoach?.is_hidden ? 'eye' : 'eye-off'}
                    size={18}
                    color={colors.text.primary}
                  />
                </View>
                <Text style={styles.actionMenuText}>
                  {selectedCoach?.is_hidden ? 'Show coach' : 'Hide coach'}
                </Text>
              </TouchableOpacity>
            )}

            {/* Fork option for system coaches */}
            {selectedCoach?.is_system && (
              <TouchableOpacity
                style={styles.actionMenuItem}
                onPress={() => handleForkCoach()}
              >
                <View style={styles.actionMenuIconContainer}>
                  <Feather name="copy" size={18} color={colors.text.primary} />
                </View>
                <Text style={styles.actionMenuText}>Fork (create my copy)</Text>
              </TouchableOpacity>
            )}

            {/* Rename only for user-created coaches */}
            {!selectedCoach?.is_system && (
              <TouchableOpacity style={styles.actionMenuItem} onPress={handleRename}>
                <View style={styles.actionMenuIconContainer}>
                  <Feather name="edit-2" size={18} color={colors.text.primary} />
                </View>
                <Text style={styles.actionMenuText}>Rename</Text>
              </TouchableOpacity>
            )}

            {/* Delete only for user-created coaches */}
            {!selectedCoach?.is_system && (
              <TouchableOpacity style={styles.actionMenuItem} onPress={handleDelete}>
                <View style={styles.actionMenuIconContainer}>
                  <Feather name="trash-2" size={18} color={colors.error} />
                </View>
                <Text style={styles.actionMenuTextDanger}>Delete</Text>
              </TouchableOpacity>
            )}
          </View>
        </TouchableOpacity>
      </Modal>

      {/* Rename Coach Prompt Dialog */}
      <PromptDialog
        visible={renamePromptVisible}
        title="Rename Coach"
        message="Enter a new name for this coach"
        defaultValue={selectedCoach?.title || ''}
        submitText="Save"
        cancelText="Cancel"
        onSubmit={handleRenameSubmit}
        onCancel={handleRenameCancel}
        testID="rename-coach-dialog"
      />
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
  sourceFilterContainer: {
    flexDirection: 'row',
    justifyContent: 'center',
    alignItems: 'center',
    paddingVertical: spacing.xs,
    paddingHorizontal: spacing.md,
    gap: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  sourceFilterChip: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
    backgroundColor: colors.background.tertiary,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  sourceFilterChipActive: {
    backgroundColor: colors.primary[600],
    borderColor: colors.primary[600],
  },
  sourceFilterChipText: {
    fontSize: fontSize.xs,
    color: colors.text.secondary,
  },
  sourceFilterChipTextActive: {
    color: colors.text.primary,
    fontWeight: '600',
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
    backgroundColor: glassCard.background,
    borderRadius: borderRadius.lg,
    padding: spacing.md,
    borderLeftWidth: 4,
    borderWidth: glassCard.borderWidth,
    borderColor: glassCard.borderColor,
    // Glassmorphism shadow
    shadowColor: glassCard.shadowColor,
    shadowOffset: glassCard.shadowOffset,
    shadowOpacity: glassCard.shadowOpacity,
    shadowRadius: glassCard.shadowRadius,
    elevation: glassCard.elevation,
  },
  coachHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: spacing.xs,
  },
  coachTitle: {
    flexShrink: 1,
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
  },
  favoriteButton: {
    padding: spacing.xs,
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
  actionMenuIconContainer: {
    width: 24,
    marginRight: spacing.sm,
    alignItems: 'center',
  },
  actionMenuText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  actionMenuTextDanger: {
    fontSize: fontSize.md,
    color: colors.error,
  },
  // Header action buttons container
  headerActions: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.xs,
  },
  headerActionButton: {
    width: 36,
    height: 36,
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: borderRadius.full,
  },
  headerActionButtonActive: {
    backgroundColor: colors.primary[500] + '20',
  },
  // Coach card title container with badges
  coachTitleContainer: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.xs,
    marginRight: spacing.sm,
  },
  coachActions: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.xs,
  },
  // System coach badge
  systemBadge: {
    backgroundColor: colors.primary[500] + '30',
    paddingHorizontal: spacing.xs,
    paddingVertical: 2,
    borderRadius: borderRadius.sm,
  },
  systemBadgeText: {
    fontSize: fontSize.xs,
    color: colors.primary[500],
    fontWeight: '600',
  },
  // Hidden coach styles
  coachCardHidden: {
    opacity: 0.6,
    borderStyle: 'dashed',
  },
  coachTitleHidden: {
    color: colors.text.tertiary,
  },
  coachTextHidden: {
    color: colors.text.tertiary,
  },
  hiddenIcon: {
    marginLeft: spacing.xs,
  },
  // Fork button on coach card
  forkButton: {
    padding: spacing.xs,
  },
  // Hide button on coach card
  hideButton: {
    padding: spacing.xs,
  },
});
