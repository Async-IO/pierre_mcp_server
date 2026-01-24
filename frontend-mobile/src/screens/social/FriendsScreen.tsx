// ABOUTME: Friends list screen for managing social connections
// ABOUTME: Shows friends list, pending requests badge, and navigation to search

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
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { FriendCard } from '../../components/social/FriendCard';
import type { FriendWithInfo } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;

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

  const loadFriends = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      const [friendsResponse, pendingResponse] = await Promise.all([
        apiService.listFriends(),
        apiService.getPendingRequests(),
      ]);

      setFriends(friendsResponse.friends);
      setFilteredFriends(friendsResponse.friends);
      setPendingCount(pendingResponse.received.length);
    } catch (error) {
      console.error('Failed to load friends:', error);
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
      await apiService.removeFriend(friend.id);
      setFriends(prev => prev.filter(f => f.id !== friend.id));
      setFilteredFriends(prev => prev.filter(f => f.id !== friend.id));
    } catch (error) {
      console.error('Failed to remove friend:', error);
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
    <View style={styles.emptyState}>
      <Feather name="users" size={64} color={colors.text.tertiary} />
      <Text style={styles.emptyTitle}>No Friends Yet</Text>
      <Text style={styles.emptyText}>
        Find and connect with other athletes to share coach insights
      </Text>
      <TouchableOpacity
        style={styles.findFriendsButton}
        onPress={() => navigation.navigate('SearchFriends')}
      >
        <Feather name="search" size={18} color={colors.text.primary} />
        <Text style={styles.findFriendsText}>Find Friends</Text>
      </TouchableOpacity>
    </View>
  );

  if (isLoading && friends.length === 0) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text style={styles.loadingText}>Loading friends...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container} testID="friends-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
          testID="drawer-toggle"
        >
          <Feather name="menu" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.title}>Friends</Text>
        <View style={styles.headerActions}>
          <TouchableOpacity
            style={styles.iconButton}
            onPress={() => navigation.navigate('FriendRequests')}
            testID="friend-requests-button"
          >
            <Feather name="inbox" size={22} color={colors.text.primary} />
            {pendingCount > 0 && (
              <View style={styles.badge}>
                <Text style={styles.badgeText}>
                  {pendingCount > 9 ? '9+' : pendingCount}
                </Text>
              </View>
            )}
          </TouchableOpacity>
          <TouchableOpacity
            style={styles.iconButton}
            onPress={() => navigation.navigate('SearchFriends')}
            testID="search-friends-button"
          >
            <Feather name="user-plus" size={22} color={colors.text.primary} />
          </TouchableOpacity>
        </View>
      </View>

      {/* Search Bar */}
      {friends.length > 0 && (
        <View style={styles.searchContainer}>
          <Feather name="search" size={18} color={colors.text.tertiary} />
          <TextInput
            style={styles.searchInput}
            placeholder="Search friends..."
            placeholderTextColor={colors.text.tertiary}
            value={searchQuery}
            onChangeText={handleSearch}
          />
          {searchQuery.length > 0 && (
            <TouchableOpacity onPress={() => handleSearch('')}>
              <Feather name="x" size={18} color={colors.text.tertiary} />
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
        contentContainerStyle={
          filteredFriends.length === 0 ? styles.emptyContainer : styles.listContent
        }
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
  menuButton: {
    padding: spacing.sm,
    marginRight: spacing.sm,
  },
  title: {
    flex: 1,
    fontSize: fontSize.xl,
    fontWeight: '700',
    color: colors.text.primary,
  },
  headerActions: {
    flexDirection: 'row',
    gap: spacing.sm,
  },
  iconButton: {
    padding: spacing.sm,
    position: 'relative',
  },
  badge: {
    position: 'absolute',
    top: 0,
    right: 0,
    backgroundColor: colors.pierre.violet,
    borderRadius: 10,
    minWidth: 18,
    height: 18,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 4,
  },
  badgeText: {
    color: colors.text.primary,
    fontSize: 10,
    fontWeight: '700',
  },
  searchContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    marginHorizontal: spacing.md,
    marginVertical: spacing.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.background.secondary,
  },
  searchInput: {
    flex: 1,
    marginLeft: spacing.sm,
    color: colors.text.primary,
    fontSize: fontSize.md,
  },
  listContent: {
    paddingVertical: spacing.sm,
  },
  emptyContainer: {
    flexGrow: 1,
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
  findFriendsButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.pierre.violet,
    gap: spacing.sm,
  },
  findFriendsText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
});
