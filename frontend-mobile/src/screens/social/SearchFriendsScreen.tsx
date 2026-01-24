// ABOUTME: Search screen for finding and adding new friends
// ABOUTME: Allows users to search by name/email and send friend requests

import React, { useState, useCallback, useRef } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  TextInput,
  Keyboard,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { SearchUserCard } from '../../components/social/FriendCard';
import type { DiscoverableUser } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;

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
        <View style={styles.emptyState}>
          <Feather name="search" size={64} color={colors.text.tertiary} />
          <Text style={styles.emptyTitle}>Find Friends</Text>
          <Text style={styles.emptyText}>
            Search by name or email to connect with other athletes
          </Text>
        </View>
      );
    }

    return (
      <View style={styles.emptyState}>
        <Feather name="user-x" size={64} color={colors.text.tertiary} />
        <Text style={styles.emptyTitle}>No Users Found</Text>
        <Text style={styles.emptyText}>
          Try a different search term or check the spelling
        </Text>
      </View>
    );
  };

  return (
    <SafeAreaView style={styles.container} testID="search-friends-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.backButton}
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.title}>Find Friends</Text>
      </View>

      {/* Search Bar */}
      <View style={styles.searchContainer}>
        <Feather name="search" size={18} color={colors.text.tertiary} />
        <TextInput
          testID="user-search-input"
          style={styles.searchInput}
          placeholder="Search by name or email..."
          placeholderTextColor={colors.text.tertiary}
          value={query}
          onChangeText={handleSearch}
          autoCapitalize="none"
          autoCorrect={false}
          returnKeyType="search"
          onSubmitEditing={() => {
            Keyboard.dismiss();
            searchUsers(query);
          }}
        />
        {query.length > 0 && (
          <TouchableOpacity onPress={() => handleSearch('')}>
            <Feather name="x" size={18} color={colors.text.tertiary} />
          </TouchableOpacity>
        )}
      </View>

      {/* Search Results */}
      {isSearching ? (
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text style={styles.loadingText}>Searching...</Text>
        </View>
      ) : (
        <FlatList
          testID="search-results-list"
          data={users}
          keyExtractor={item => item.user_id}
          renderItem={renderUser}
          ListEmptyComponent={renderEmptyState}
          contentContainerStyle={
            users.length === 0 ? styles.emptyContainer : styles.listContent
          }
          keyboardShouldPersistTaps="handled"
        />
      )}
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
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  backButton: {
    padding: spacing.sm,
    marginRight: spacing.sm,
  },
  title: {
    flex: 1,
    fontSize: fontSize.xl,
    fontWeight: '700',
    color: colors.text.primary,
  },
  searchContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    marginHorizontal: spacing.md,
    marginVertical: spacing.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.background.secondary,
  },
  searchInput: {
    flex: 1,
    marginLeft: spacing.sm,
    color: colors.text.primary,
    fontSize: fontSize.md,
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
  },
});
