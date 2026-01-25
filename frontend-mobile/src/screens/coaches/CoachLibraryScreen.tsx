// ABOUTME: Coach library screen for managing user's AI coaches
// ABOUTME: Lists coaches with category filters, favorites toggle, and CRUD actions

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
  Modal,
  RefreshControl,
  type ViewStyle,
} from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, glassCard } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { PromptDialog, ScrollFadeContainer } from '../../components/ui';
import type { Coach, CoachCategory } from '../../types';
import type { CoachesStackParamList } from '../../navigation/MainTabs';

interface CoachLibraryScreenProps {
  navigation: NativeStackNavigationProp<CoachesStackParamList>;
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

// Glass card style with shadow (React Native shadows cannot use className)
const coachCardStyle: ViewStyle = {
  ...glassCard,
  borderLeftWidth: 4,
};

// FAB shadow style
const fabStyle: ViewStyle = {
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
};

// Action menu shadow style
const actionMenuStyle: ViewStyle = {
  backgroundColor: colors.background.primary,
  borderRadius: 12,
  paddingVertical: spacing.xs,
  minWidth: 220,
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 4 },
  shadowOpacity: 0.3,
  shadowRadius: 8,
  elevation: 8,
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
          coachCardStyle,
          { borderLeftColor: COACH_CATEGORY_COLORS[item.category] },
          isHidden && { opacity: 0.6, borderStyle: 'dashed' },
        ]}
        className="rounded-lg p-3"
        onPress={() => handleCoachPress(item)}
        onLongPress={() => handleCoachLongPress(item)}
        delayLongPress={300}
        activeOpacity={0.7}
        testID={`coach-card-${item.id}`}
      >
        <View className="flex-row items-center justify-between mb-1">
          <View className="flex-1 flex-row items-center gap-1 mr-2">
            <Text className={`flex-shrink text-base font-semibold ${isHidden ? 'text-text-tertiary' : 'text-text-primary'}`} numberOfLines={1}>
              {item.title}
            </Text>
            {isSystemCoach && (
              <View className="px-1 py-0.5 rounded" style={{ backgroundColor: colors.primary[500] + '30' }}>
                <Text className="text-xs font-semibold text-primary-500">System</Text>
              </View>
            )}
            {isHidden && (
              <Feather name="eye-off" size={14} color={colors.text.tertiary} style={{ marginLeft: spacing.xs }} />
            )}
          </View>
          <View className="flex-row items-center gap-1">
            {/* Fork button for system coaches */}
            {isSystemCoach && (
              <TouchableOpacity
                className="p-1"
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
                className="p-1"
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
              className="p-1"
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
        <Text className={`text-sm mb-1 ${isHidden ? 'text-text-tertiary' : 'text-text-secondary'}`}>
          ~{Math.ceil(item.token_count / 4)} tokens ({((item.token_count / 128000) * 100).toFixed(1)}% context)
        </Text>
        {item.description && (
          <Text className={`text-sm leading-5 mb-2 ${isHidden ? 'text-text-tertiary' : 'text-text-secondary'}`} numberOfLines={2}>
            {item.description}
          </Text>
        )}
        <View className="border-t border-border-subtle pt-2 mt-1">
          <Text className={`text-sm ${isHidden ? 'text-text-tertiary' : 'text-text-tertiary'}`}>
            Used {item.use_count} times
          </Text>
        </View>
      </TouchableOpacity>
    );
  };

  const renderCategoryFilter = () => (
    <View className="flex-row items-center py-2 border-b border-border-subtle">
      <ScrollFadeContainer
        backgroundColor={colors.background.primary}
        fadeWidth={40}
        contentContainerStyle={{ paddingHorizontal: spacing.md, gap: spacing.xs }}
        testID="category-filter-scroll"
      >
        {CATEGORY_FILTERS.map((filter) => (
          <TouchableOpacity
            key={filter.key}
            className={`px-3 py-1 rounded-full border ${
              selectedCategory === filter.key
                ? 'bg-primary-500 border-primary-500'
                : 'bg-background-secondary border-border-subtle'
            }`}
            onPress={() => setSelectedCategory(filter.key)}
            testID={`category-filter-${filter.key}`}
          >
            <Text
              className={`text-sm ${
                selectedCategory === filter.key
                  ? 'text-text-primary font-semibold'
                  : 'text-text-secondary'
              }`}
            >
              {filter.label}
            </Text>
          </TouchableOpacity>
        ))}
      </ScrollFadeContainer>
    </View>
  );

  const renderSourceFilter = () => (
    <View className="flex-row justify-center items-center py-1 px-3 gap-2 border-b border-border-subtle">
      {SOURCE_FILTERS.map((filter) => (
        <TouchableOpacity
          key={filter.key}
          className={`px-3 py-1 rounded-full border ${
            selectedSource === filter.key
              ? 'bg-primary-600 border-primary-600'
              : 'bg-background-tertiary border-border-subtle'
          }`}
          onPress={() => setSelectedSource(filter.key)}
          testID={`source-filter-${filter.key}`}
        >
          <Text
            className={`text-xs ${
              selectedSource === filter.key
                ? 'text-text-primary font-semibold'
                : 'text-text-secondary'
            }`}
          >
            {filter.label}
          </Text>
        </TouchableOpacity>
      ))}
    </View>
  );

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="coach-library-screen">
      {/* Header */}
      <View className="flex-row items-center px-3 py-2 border-b border-border-subtle">
        <View className="w-10" />
        <Text className="flex-1 text-lg font-semibold text-text-primary text-center">My Coaches</Text>
        <View className="flex-row items-center gap-1">
          <TouchableOpacity
            className={`w-9 h-9 items-center justify-center rounded-full ${showFavoritesOnly ? 'bg-primary-500/20' : ''}`}
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
            className={`w-9 h-9 items-center justify-center rounded-full ${showHidden ? 'bg-primary-500/20' : ''}`}
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
        <View className="flex-1 items-center justify-center">
          <ActivityIndicator size="large" color={colors.primary[500]} />
        </View>
      ) : (
        <FlatList
          data={filteredCoaches}
          renderItem={renderCoachCard}
          keyExtractor={(item) => item.id}
          contentContainerStyle={{ flexGrow: 1, padding: spacing.md, paddingBottom: 100, gap: spacing.md }}
          showsVerticalScrollIndicator={false}
          refreshControl={
            <RefreshControl
              refreshing={isRefreshing}
              onRefresh={handleRefresh}
              tintColor={colors.primary[500]}
            />
          }
          ListEmptyComponent={
            <View className="flex-1 items-center justify-center pt-12 px-5">
              <Text className="text-lg font-semibold text-text-primary mb-2 text-center">
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
              <Text className="text-base text-text-tertiary text-center">
                {coaches.length === 0
                  ? 'Create your first coach to customize how Pierre helps you.'
                  : 'Try adjusting your filters.'}
              </Text>
            </View>
          }
        />
      )}

      {/* Floating Action Button */}
      <TouchableOpacity style={fabStyle} onPress={handleCreateCoach} testID="create-coach-button">
        <Text className="text-3xl text-text-primary -mt-0.5">+</Text>
      </TouchableOpacity>

      {/* Action Menu Modal */}
      <Modal
        visible={actionMenuVisible}
        animationType="fade"
        transparent
        onRequestClose={closeActionMenu}
      >
        <TouchableOpacity
          className="flex-1 bg-black/30 justify-center items-center"
          activeOpacity={1}
          onPress={closeActionMenu}
        >
          <View style={actionMenuStyle}>
            <TouchableOpacity className="flex-row items-center px-3 py-2" onPress={() => handleToggleFavorite()}>
              <View className="w-6 mr-2 items-center">
                <Feather
                  name="star"
                  size={18}
                  color={selectedCoach?.is_favorite ? '#F59E0B' : colors.text.primary}
                />
              </View>
              <Text className="text-base text-text-primary">
                {selectedCoach?.is_favorite ? 'Remove from favorites' : 'Add to favorites'}
              </Text>
            </TouchableOpacity>

            {/* Hide/Show option for system or assigned coaches */}
            {(selectedCoach?.is_system || selectedCoach?.is_assigned) && (
              <TouchableOpacity
                className="flex-row items-center px-3 py-2"
                onPress={() => (selectedCoach?.is_hidden ? handleShowCoach() : handleHideCoach())}
              >
                <View className="w-6 mr-2 items-center">
                  <Feather
                    name={selectedCoach?.is_hidden ? 'eye' : 'eye-off'}
                    size={18}
                    color={colors.text.primary}
                  />
                </View>
                <Text className="text-base text-text-primary">
                  {selectedCoach?.is_hidden ? 'Show coach' : 'Hide coach'}
                </Text>
              </TouchableOpacity>
            )}

            {/* Fork option for system coaches */}
            {selectedCoach?.is_system && (
              <TouchableOpacity
                className="flex-row items-center px-3 py-2"
                onPress={() => handleForkCoach()}
              >
                <View className="w-6 mr-2 items-center">
                  <Feather name="copy" size={18} color={colors.text.primary} />
                </View>
                <Text className="text-base text-text-primary">Fork (create my copy)</Text>
              </TouchableOpacity>
            )}

            {/* Rename only for user-created coaches */}
            {!selectedCoach?.is_system && (
              <TouchableOpacity className="flex-row items-center px-3 py-2" onPress={handleRename}>
                <View className="w-6 mr-2 items-center">
                  <Feather name="edit-2" size={18} color={colors.text.primary} />
                </View>
                <Text className="text-base text-text-primary">Rename</Text>
              </TouchableOpacity>
            )}

            {/* Delete only for user-created coaches */}
            {!selectedCoach?.is_system && (
              <TouchableOpacity className="flex-row items-center px-3 py-2" onPress={handleDelete}>
                <View className="w-6 mr-2 items-center">
                  <Feather name="trash-2" size={18} color={colors.error} />
                </View>
                <Text className="text-base text-error">Delete</Text>
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
