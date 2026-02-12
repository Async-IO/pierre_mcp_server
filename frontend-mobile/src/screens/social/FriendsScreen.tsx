// ABOUTME: Friends list screen for managing social connections
// ABOUTME: Shows friends list, pending requests badge, and navigation to search

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  RefreshControl,
  TextInput,
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, glassCard, buttonGlow } from '../../constants/theme';
import { socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { FriendCard } from '../../components/social/FriendCard';
import type { FriendWithInfo } from '../../types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;

export function FriendsScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [friends, setFriends] = useState<FriendWithInfo[]>([]);
  const [filteredFriends, setFilteredFriends] = useState<FriendWithInfo[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [pendingCount, setPendingCount] = useState(0);
  const [removingIds, setRemovingIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);

  const loadFriends = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }
      setError(null);

      const [friendsResponse, pendingResponse] = await Promise.all([
        socialApi.listFriends(),
        socialApi.getPendingRequests(),
      ]);

      setFriends(friendsResponse.friends);
      setFilteredFriends(friendsResponse.friends);
      setPendingCount(pendingResponse.received.length);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load friends';
      setError(errorMessage);
      console.error('Failed to load friends:', err);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated]);

  useFocusEffect(
    useCallback(() => {
      loadFriends();
    }, [loadFriends])
  );

  const handleSearch = (text: string) => {
    setSearchQuery(text);
    if (!text.trim()) {
      setFilteredFriends(friends);
      return;
    }
    const query = text.toLowerCase();
    const filtered = friends.filter(f =>
      (f.friend_display_name?.toLowerCase().includes(query)) ||
      f.friend_email.toLowerCase().includes(query)
    );
    setFilteredFriends(filtered);
  };

  const handleRemoveFriend = async (friend: FriendWithInfo) => {
    try {
      setRemovingIds(prev => new Set(prev).add(friend.id));
      await socialApi.removeFriend(friend.id);
      setFriends(prev => prev.filter(f => f.id !== friend.id));
      setFilteredFriends(prev => prev.filter(f => f.id !== friend.id));
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to remove friend';
      setError(errorMessage);
      console.error('Failed to remove friend:', err);
    } finally {
      setRemovingIds(prev => {
        const next = new Set(prev);
        next.delete(friend.id);
        return next;
      });
    }
  };

  const renderFriend = ({ item }: { item: FriendWithInfo }) => (
    <FriendCard
      friend={item}
      onRemove={() => handleRemoveFriend(item)}
      isRemoving={removingIds.has(item.id)}
    />
  );

  const renderEmptyState = () => (
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
        <Feather name="users" size={48} color={colors.pierre.violet} />
      </View>
      <Text className="text-text-primary text-xl font-bold mt-4">No Friends Yet</Text>
      <Text className="text-text-secondary text-base text-center mt-2 mb-6">
        Find and connect with other athletes to share coach insights
      </Text>
      <TouchableOpacity
        className="flex-row items-center px-6 py-4 rounded-xl gap-2"
        style={{
          backgroundColor: colors.pierre.violet,
          ...buttonGlow,
        }}
        onPress={() => navigation.navigate('SearchFriends')}
      >
        <Feather name="search" size={18} color="#FFFFFF" />
        <Text className="text-white text-base font-semibold">Find Friends</Text>
      </TouchableOpacity>
    </View>
  );

  if (isLoading && friends.length === 0) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text className="text-text-secondary mt-4">Loading friends...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="friends-screen">
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <View className="w-10" />
        <Text className="flex-1 text-xl font-bold text-text-primary text-center">Friends</Text>
        <View className="flex-row gap-2">
          <TouchableOpacity
            className="p-2 relative"
            onPress={() => navigation.navigate('FriendRequests')}
            testID="friend-requests-button"
          >
            <Feather name="inbox" size={22} color={colors.text.primary} />
            {pendingCount > 0 && (
              <View
                className="absolute top-0 right-0 rounded-full min-w-[18px] h-[18px] justify-center items-center px-1"
                style={{ backgroundColor: colors.pierre.violet }}
              >
                <Text className="text-text-primary text-[10px] font-bold">
                  {pendingCount > 9 ? '9+' : pendingCount}
                </Text>
              </View>
            )}
          </TouchableOpacity>
          <TouchableOpacity
            className="p-2"
            onPress={() => navigation.navigate('SearchFriends')}
            testID="search-friends-button"
          >
            <Feather name="user-plus" size={22} color={colors.text.primary} />
          </TouchableOpacity>
        </View>
      </View>

      {/* Error Display */}
      {error && (
        <View className="mx-4 mt-2 p-3 bg-error/10 border border-error/30 rounded-lg flex-row items-center justify-between">
          <Text className="flex-1 text-error text-sm mr-3">{error}</Text>
          <TouchableOpacity
            className="px-3 py-1.5 bg-error/20 rounded-md"
            onPress={() => {
              setError(null);
              loadFriends();
            }}
          >
            <Text className="text-error text-sm font-semibold">Retry</Text>
          </TouchableOpacity>
        </View>
      )}

      {/* Search Bar with glassmorphism */}
      {friends.length > 0 && (
        <View
          className="flex-row items-center mx-4 my-4 px-4 py-3 rounded-xl"
          style={{
            ...glassCard,
            borderRadius: 12,
            borderColor: 'rgba(139, 92, 246, 0.2)',
          }}
        >
          <Feather name="search" size={18} color={colors.pierre.violet} />
          <TextInput
            className="flex-1 ml-3 text-text-primary text-base"
            placeholder="Search friends..."
            placeholderTextColor={colors.text.tertiary}
            value={searchQuery}
            onChangeText={handleSearch}
          />
          {searchQuery.length > 0 && (
            <TouchableOpacity onPress={() => handleSearch('')}>
              <Feather name="x" size={18} color={colors.text.secondary} />
            </TouchableOpacity>
          )}
        </View>
      )}

      {/* Friends List */}
      <FlatList
        data={filteredFriends}
        keyExtractor={item => item.id}
        renderItem={renderFriend}
        ListEmptyComponent={renderEmptyState}
        contentContainerStyle={filteredFriends.length === 0 ? { flexGrow: 1 } : { paddingVertical: spacing.sm }}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={() => loadFriends(true)}
            tintColor={colors.pierre.violet}
          />
        }
      />
    </SafeAreaView>
  );
}
