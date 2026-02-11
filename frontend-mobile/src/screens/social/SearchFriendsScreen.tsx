// ABOUTME: Search screen for finding and adding new friends
// ABOUTME: Allows users to search by name/email and send friend requests

import React, { useState, useCallback, useRef } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, spacing } from '../../constants/theme';
import { FloatingSearchBar } from '../../components/ui';
import { socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { SearchUserCard } from '../../components/social/FriendCard';
import type { DiscoverableUser } from '../../types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;

export function SearchFriendsScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [query, setQuery] = useState('');
  const [users, setUsers] = useState<DiscoverableUser[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [addingIds, setAddingIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const searchTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  const searchUsers = useCallback(async (searchQuery: string) => {
    if (!isAuthenticated || !searchQuery.trim()) {
      setUsers([]);
      setHasSearched(false);
      return;
    }

    try {
      setIsSearching(true);
      setError(null);
      const response = await socialApi.searchUsers(searchQuery.trim(), 30);
      setUsers(response.users);
      setHasSearched(true);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to search users';
      setError(errorMessage);
      console.error('Failed to search users:', err);
    } finally {
      setIsSearching(false);
    }
  }, [isAuthenticated]);

  const handleSearch = (text: string) => {
    setQuery(text);

    // Debounce search
    if (searchTimeout.current) {
      clearTimeout(searchTimeout.current);
    }

    if (text.trim().length >= 2) {
      searchTimeout.current = setTimeout(() => {
        searchUsers(text);
      }, 300);
    } else {
      setUsers([]);
      setHasSearched(false);
    }
  };

  const handleAddFriend = async (user: DiscoverableUser) => {
    try {
      setAddingIds(prev => new Set(prev).add(user.id));
      await socialApi.sendFriendRequest(user.id);
      // Update local state to show pending
      setUsers(prev =>
        prev.map(u =>
          u.id === user.id
            ? { ...u, has_pending_request: true }
            : u
        )
      );
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to send friend request';
      setError(errorMessage);
      console.error('Failed to send friend request:', err);
    } finally {
      setAddingIds(prev => {
        const next = new Set(prev);
        next.delete(user.id);
        return next;
      });
    }
  };

  const renderUser = ({ item }: { item: DiscoverableUser }) => (
    <SearchUserCard
      user={item}
      onAddFriend={() => handleAddFriend(item)}
      isAdding={addingIds.has(item.id)}
    />
  );

  const renderEmptyState = () => {
    if (!hasSearched) {
      return (
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
            <Feather name="search" size={48} color={colors.pierre.violet} />
          </View>
          <Text className="text-text-primary text-xl font-bold mt-4">Find Friends</Text>
          <Text className="text-text-secondary text-base text-center mt-2">
            Search by name or email to connect with other athletes
          </Text>
        </View>
      );
    }

    return (
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
          <Feather name="user-x" size={48} color={colors.pierre.violet} />
        </View>
        <Text className="text-text-primary text-xl font-bold mt-4">No Users Found</Text>
        <Text className="text-text-secondary text-base text-center mt-2">
          Try a different search term or check the spelling
        </Text>
      </View>
    );
  };

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="search-friends-screen">
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <TouchableOpacity
          className="p-2 mr-2"
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-xl font-bold text-text-primary">Find Friends</Text>
      </View>

      {/* Error Display */}
      {error && (
        <View className="mx-4 mt-2 p-3 bg-error/10 border border-error/30 rounded-lg flex-row items-center justify-between">
          <Text className="flex-1 text-error text-sm mr-3">{error}</Text>
          <TouchableOpacity
            className="px-3 py-1.5 bg-error/20 rounded-md"
            onPress={() => setError(null)}
          >
            <Text className="text-error text-sm font-semibold">Dismiss</Text>
          </TouchableOpacity>
        </View>
      )}

      {/* Search Results */}
      {isSearching ? (
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text className="text-text-secondary mt-4">Searching...</Text>
        </View>
      ) : (
        <FlatList
          testID="search-results-list"
          data={users}
          keyExtractor={item => item.id}
          renderItem={renderUser}
          ListEmptyComponent={renderEmptyState}
          contentContainerStyle={users.length === 0 ? { flexGrow: 1, paddingBottom: 100 } : { paddingVertical: spacing.sm, paddingBottom: 100 }}
          keyboardShouldPersistTaps="handled"
        />
      )}

      {/* Floating Search Bar */}
      <FloatingSearchBar
        value={query}
        onChangeText={handleSearch}
        onSubmit={() => searchUsers(query)}
        placeholder="Search by name or email..."
        isSearching={isSearching}
        testID="user-search-input"
        autoFocus
      />
    </SafeAreaView>
  );
}
