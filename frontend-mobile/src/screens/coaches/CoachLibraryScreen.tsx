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
  TextInput,
  type ViewStyle,
} from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, spacing } from '../../constants/theme';
import { coachesApi } from '../../services/api';
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

// Coach category colors matching Stitch UX spec
const COACH_CATEGORY_COLORS: Record<string, string> = {
  training: '#4ADE80',  // Green per Stitch spec
  nutrition: '#F59E0B', // Amber per Stitch spec
  recovery: '#22D3EE',  // Cyan per Stitch spec
  recipes: '#F59E0B',   // Amber
  mobility: '#EC4899',  // Pink - for stretching/yoga
  custom: '#8B5CF6',    // Violet per Stitch spec
};

// Search container shadow style
const searchContainerShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 2 },
  shadowOpacity: 0.25,
  shadowRadius: 4,
  elevation: 4,
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
  const [searchQuery, setSearchQuery] = useState('');
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
        coachesApi.list({ include_hidden: true }),
        coachesApi.getHidden(),
      ]);

      // Create a set of hidden coach IDs for quick lookup
      const hiddenIds = new Set((hiddenResponse.coaches || []).map((c: { id: string }) => c.id));

      // Mark coaches as hidden if they're in the hidden list
      const coachesWithHiddenFlag = coachesResponse.coaches.map((coach: Coach) => ({
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

  // Apply filters whenever coaches, category, source, favorites, showHidden, or searchQuery changes
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

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter((coach) =>
        coach.title.toLowerCase().includes(query) ||
        (coach.description || '').toLowerCase().includes(query)
      );
    }

    setFilteredCoaches(filtered);
  }, [coaches, selectedCategory, selectedSource, showFavoritesOnly, showHidden, searchQuery]);

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
      const result = await coachesApi.toggleFavorite(targetCoach.id);
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
      const updated = await coachesApi.update(selectedCoach.id, {
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
              await coachesApi.delete(selectedCoach.id);
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
      await coachesApi.hide(targetCoach.id);
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
      await coachesApi.show(targetCoach.id);
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
              const result = await coachesApi.fork(targetCoach.id);
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
    const isHidden = item.is_hidden;
    const categoryColor = COACH_CATEGORY_COLORS[item.category];

    return (
      <TouchableOpacity
        style={[
          {
            backgroundColor: 'rgba(255, 255, 255, 0.05)',
            borderWidth: 1,
            borderColor: 'rgba(255, 255, 255, 0.1)',
            borderRadius: 16,
          },
          isHidden && { opacity: 0.6 },
        ]}
        className="p-4"
        onPress={() => handleCoachPress(item)}
        onLongPress={() => handleCoachLongPress(item)}
        delayLongPress={300}
        activeOpacity={0.7}
        testID={`coach-card-${item.id}`}
      >
        <View className="flex-row items-start">
          {/* Coach Avatar/Icon */}
          <View
            className="w-12 h-12 rounded-xl items-center justify-center mr-3"
            style={{ backgroundColor: `${categoryColor}20` }}
          >
            <Text className="text-xl">
              {item.category === 'training' ? '🏃' :
               item.category === 'nutrition' ? '🥗' :
               item.category === 'recovery' ? '😴' :
               item.category === 'recipes' ? '👨‍🍳' :
               item.category === 'mobility' ? '🧘' : '⚙️'}
            </Text>
          </View>

          <View className="flex-1">
            {/* Title and badges row */}
            <View className="flex-row items-center gap-2 mb-1">
              <Text className={`text-base font-semibold ${isHidden ? 'text-zinc-500' : 'text-white'}`} numberOfLines={1}>
                {item.title}
              </Text>
              {/* Category badge with color per Stitch spec */}
              <View
                className="px-2 py-0.5 rounded-full"
                style={{ backgroundColor: `${categoryColor}20` }}
              >
                <Text className="text-xs font-medium" style={{ color: categoryColor }}>
                  {item.category.charAt(0).toUpperCase() + item.category.slice(1)}
                </Text>
              </View>
            </View>

            {/* Star rating (use count as proxy) and favorite button */}
            <View className="flex-row items-center gap-1 mb-1">
              {[1, 2, 3, 4, 5].map((star) => (
                <Feather
                  key={star}
                  name="star"
                  size={12}
                  color={item.use_count >= star * 2 ? '#F59E0B' : colors.text.tertiary}
                />
              ))}
              <TouchableOpacity
                className="ml-2 p-0.5"
                onPress={() => handleToggleFavorite(item)}
                hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
                testID={`favorite-button-${item.id}`}
              >
                <Feather
                  name="heart"
                  size={14}
                  color={item.is_favorite ? colors.pierre.violet : colors.text.tertiary}
                />
              </TouchableOpacity>
            </View>

            {/* Description */}
            {item.description && (
              <Text className={`text-sm leading-5 ${isHidden ? 'text-zinc-600' : 'text-zinc-400'}`} numberOfLines={2}>
                {item.description}
              </Text>
            )}
          </View>

          {/* Chat button with violet glow per Stitch spec */}
          <TouchableOpacity
            className="px-4 py-2 rounded-full ml-2"
            style={{
              backgroundColor: colors.pierre.violet,
              shadowColor: colors.pierre.violet,
              shadowOffset: { width: 0, height: 0 },
              shadowOpacity: 0.4,
              shadowRadius: 8,
              elevation: 4,
            }}
            onPress={() => handleCoachPress(item)}
            testID={`chat-button-${item.id}`}
          >
            <Text className="text-sm font-semibold text-white">Chat</Text>
          </TouchableOpacity>
        </View>

        {/* Action row for system coaches and hidden coaches */}
        {(item.is_system || isHidden) && (
          <View className="flex-row items-center justify-end mt-3 pt-2 border-t border-white/5 gap-2">
            {/* Fork button for system coaches */}
            {item.is_system && (
              <TouchableOpacity
                className="flex-row items-center px-2 py-1 rounded"
                style={{ backgroundColor: 'rgba(255, 255, 255, 0.05)' }}
                onPress={() => handleForkCoach(item)}
                hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
                testID={`fork-button-${item.id}`}
              >
                <Feather name="copy" size={14} color={colors.text.tertiary} />
                <Text className="text-xs text-zinc-500 ml-1">Fork</Text>
              </TouchableOpacity>
            )}
            {/* Hide/Show button */}
            {item.is_system && (
              <TouchableOpacity
                className="flex-row items-center px-2 py-1 rounded"
                style={{ backgroundColor: 'rgba(255, 255, 255, 0.05)' }}
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
                  size={14}
                  color={isHidden ? colors.pierre.violet : colors.text.tertiary}
                />
                <Text className="text-xs text-zinc-500 ml-1">{isHidden ? 'Show' : 'Hide'}</Text>
              </TouchableOpacity>
            )}
            {/* Hidden indicator */}
            {isHidden && !item.is_system && (
              <View className="flex-row items-center">
                <Feather name="eye-off" size={14} color={colors.text.tertiary} />
                <Text className="text-xs text-zinc-500 ml-1">Hidden</Text>
              </View>
            )}
          </View>
        )}
      </TouchableOpacity>
    );
  };

  const renderCategoryFilter = () => (
    <View className="flex-row items-center py-3 border-b border-border-subtle">
      <ScrollFadeContainer
        backgroundColor={colors.background.primary}
        fadeWidth={40}
        contentContainerStyle={{ paddingHorizontal: spacing.md, gap: spacing.sm }}
        testID="category-filter-scroll"
      >
        {CATEGORY_FILTERS.map((filter) => (
          <TouchableOpacity
            key={filter.key}
            className="px-4 py-2 rounded-full"
            style={{
              backgroundColor: selectedCategory === filter.key
                ? colors.pierre.violet
                : 'rgba(255, 255, 255, 0.05)',
              borderWidth: 1,
              borderColor: selectedCategory === filter.key
                ? colors.pierre.violet
                : 'rgba(255, 255, 255, 0.1)',
            }}
            onPress={() => setSelectedCategory(filter.key)}
            testID={`category-filter-${filter.key}`}
          >
            <Text
              className={`text-sm ${
                selectedCategory === filter.key
                  ? 'text-white font-semibold'
                  : 'text-zinc-400'
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
    <View className="flex-row justify-center items-center py-2 px-4 gap-3">
      {SOURCE_FILTERS.map((filter) => (
        <TouchableOpacity
          key={filter.key}
          className="px-3 py-1.5 rounded-full"
          style={{
            backgroundColor: selectedSource === filter.key
              ? 'rgba(139, 92, 246, 0.2)'
              : 'transparent',
            borderWidth: 1,
            borderColor: selectedSource === filter.key
              ? colors.pierre.violet
              : 'transparent',
          }}
          onPress={() => setSelectedSource(filter.key)}
          testID={`source-filter-${filter.key}`}
        >
          <Text
            className={`text-xs ${
              selectedSource === filter.key
                ? 'font-semibold'
                : ''
            }`}
            style={{
              color: selectedSource === filter.key
                ? colors.pierre.violet
                : colors.text.secondary,
            }}
          >
            {filter.label}
          </Text>
        </TouchableOpacity>
      ))}
    </View>
  );

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="coach-library-screen">
      {/* Header with bold title and action buttons - + button in top right like Chat tab */}
      <View className="flex-row items-center px-4 py-3 border-b border-border-subtle">
        <Text className="flex-1 text-xl font-bold text-white">Coaches</Text>
        <View className="flex-row items-center gap-2">
          <TouchableOpacity
            className={`w-10 h-10 items-center justify-center rounded-full ${showFavoritesOnly ? 'bg-pierre-violet/20' : ''}`}
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
            className={`w-10 h-10 items-center justify-center rounded-full ${showHidden ? 'bg-pierre-violet/20' : ''}`}
            onPress={() => setShowHidden(!showHidden)}
            hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
            testID="show-hidden-toggle"
          >
            <Feather
              name={showHidden ? 'eye' : 'eye-off'}
              size={20}
              color={showHidden ? colors.pierre.violet : colors.text.tertiary}
            />
          </TouchableOpacity>
          {/* Create coach button - matches Chat tab style */}
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center bg-background-tertiary rounded-lg"
            onPress={handleCreateCoach}
            testID="create-coach-button"
          >
            <Text className="text-2xl text-text-primary font-light">+</Text>
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

      {/* Floating Bottom Search Bar - liquid style above tab bar */}
      <View
        className="absolute left-4 right-4 flex-row items-center"
        style={{ bottom: 8 }}
      >
        <View
          className="flex-1 flex-row items-center rounded-full px-4"
          style={[
            {
              height: 36,
              backgroundColor: 'rgba(30, 27, 45, 0.95)',
              borderWidth: 1,
              borderColor: 'rgba(139, 92, 246, 0.4)',
            },
            searchContainerShadow,
          ]}
        >
          <Feather name="search" size={18} color={colors.pierre.violet} style={{ marginRight: 8 }} />
          <TextInput
            className="flex-1 text-base text-text-primary"
            placeholder="Search coaches..."
            placeholderTextColor={colors.text.secondary}
            value={searchQuery}
            onChangeText={setSearchQuery}
            testID="coach-search-input"
          />
          {searchQuery.length > 0 && (
            <TouchableOpacity onPress={() => setSearchQuery('')}>
              <Feather name="x" size={18} color={colors.text.secondary} />
            </TouchableOpacity>
          )}
        </View>
      </View>

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
