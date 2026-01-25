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
import { apiService } from '../../services/api';
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
  const searchTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  const searchUsers = useCallback(async (searchQuery: string) => {
    if (!isAuthenticated || !searchQuery.trim()) {
      setUsers([]);
      setHasSearched(false);
      return;
    }

    try {
      setIsSearching(true);
      const response = await apiService.searchUsers(searchQuery.trim(), 30);
      setUsers(response.users);
      setHasSearched(true);
    } catch (error) {
      console.error('Failed to search users:', error);
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
      setAddingIds(prev => new Set(prev).add(user.user_id));
      await apiService.sendFriendRequest(user.user_id);
      // Update local state to show pending
      setUsers(prev =>
        prev.map(u =>
          u.user_id === user.user_id
            ? { ...u, pending_request: true }
            : u
        )
      );
    } catch (error) {
      console.error('Failed to send friend request:', error);
    } finally {
      setAddingIds(prev => {
        const next = new Set(prev);
        next.delete(user.user_id);
        return next;
      });
    }
  };

  const renderUser = ({ item }: { item: DiscoverableUser }) => (
    <SearchUserCard
      user={item}
      onAddFriend={() => handleAddFriend(item)}
      isAdding={addingIds.has(item.user_id)}
    />
  );

  const renderEmptyState = () => {
    if (!hasSearched) {
      return (
        <View className="flex-1 justify-center items-center p-6">
          <Feather name="search" size={64} color={colors.text.tertiary} />
          <Text className="text-text-primary text-xl font-bold mt-5">Find Friends</Text>
          <Text className="text-text-secondary text-base text-center mt-2">
            Search by name or email to connect with other athletes
          </Text>
        </View>
      );
    }

    return (
      <View className="flex-1 justify-center items-center p-6">
        <Feather name="user-x" size={64} color={colors.text.tertiary} />
        <Text className="text-text-primary text-xl font-bold mt-5">No Users Found</Text>
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
          keyExtractor={item => item.user_id}
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
