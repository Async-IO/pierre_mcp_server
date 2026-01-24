// ABOUTME: Friend requests management screen
// ABOUTME: Shows incoming and outgoing friend requests with accept/decline actions

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
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { RequestCard } from '../../components/social/FriendCard';
import type { FriendConnection } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;
type TabType = 'incoming' | 'outgoing';

export function FriendRequestsScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [activeTab, setActiveTab] = useState<TabType>('incoming');
  const [incomingRequests, setIncomingRequests] = useState<FriendConnection[]>([]);
  const [outgoingRequests, setOutgoingRequests] = useState<FriendConnection[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [processingIds, setProcessingIds] = useState<Set<string>>(new Set());

  const loadRequests = useCallback(async (isRefresh = false) => {
    if (!isAuthenticated) return;

    try {
      if (isRefresh) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      const response = await apiService.getPendingRequests();
      setIncomingRequests(response.received);
      setOutgoingRequests(response.sent);
    } catch (error) {
      console.error('Failed to load friend requests:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [isAuthenticated]);

  useFocusEffect(
    useCallback(() => {
      loadRequests();
    }, [loadRequests])
  );

  const handleAccept = async (request: FriendConnection) => {
    try {
      setProcessingIds(prev => new Set(prev).add(request.id));
      await apiService.acceptFriendRequest(request.id);
      setIncomingRequests(prev => prev.filter(r => r.id !== request.id));
    } catch (error) {
      console.error('Failed to accept request:', error);
    } finally {
      setProcessingIds(prev => {
        const next = new Set(prev);
        next.delete(request.id);
        return next;
      });
    }
  };

  const handleDecline = async (request: FriendConnection) => {
    try {
      setProcessingIds(prev => new Set(prev).add(request.id));
      await apiService.declineFriendRequest(request.id);
      setIncomingRequests(prev => prev.filter(r => r.id !== request.id));
    } catch (error) {
      console.error('Failed to decline request:', error);
    } finally {
      setProcessingIds(prev => {
        const next = new Set(prev);
        next.delete(request.id);
        return next;
      });
    }
  };

  const handleCancel = async (request: FriendConnection) => {
    try {
      setProcessingIds(prev => new Set(prev).add(request.id));
      await apiService.removeFriend(request.id);
      setOutgoingRequests(prev => prev.filter(r => r.id !== request.id));
    } catch (error) {
      console.error('Failed to cancel request:', error);
    } finally {
      setProcessingIds(prev => {
        const next = new Set(prev);
        next.delete(request.id);
        return next;
      });
    }
  };

  const renderRequest = ({ item }: { item: FriendConnection }) => (
    <RequestCard
      request={item}
      type={activeTab}
      onAccept={activeTab === 'incoming' ? () => handleAccept(item) : undefined}
      onDecline={activeTab === 'incoming' ? () => handleDecline(item) : undefined}
      onCancel={activeTab === 'outgoing' ? () => handleCancel(item) : undefined}
      isLoading={processingIds.has(item.id)}
    />
  );

  const renderEmptyState = () => (
    <View style={styles.emptyState}>
      <Feather
        name={activeTab === 'incoming' ? 'inbox' : 'send'}
        size={64}
        color={colors.text.tertiary}
      />
      <Text style={styles.emptyTitle}>
        {activeTab === 'incoming' ? 'No Incoming Requests' : 'No Outgoing Requests'}
      </Text>
      <Text style={styles.emptyText}>
        {activeTab === 'incoming'
          ? 'When someone sends you a friend request, it will appear here'
          : 'Friend requests you send will appear here until accepted'}
      </Text>
    </View>
  );

  const currentData = activeTab === 'incoming' ? incomingRequests : outgoingRequests;

  if (isLoading && incomingRequests.length === 0 && outgoingRequests.length === 0) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text style={styles.loadingText}>Loading requests...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.backButton}
          onPress={() => navigation.goBack()}
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.title}>Friend Requests</Text>
      </View>

      {/* Tabs */}
      <View style={styles.tabsContainer}>
        <TouchableOpacity
          style={[styles.tab, activeTab === 'incoming' && styles.activeTab]}
          onPress={() => setActiveTab('incoming')}
        >
          <Text style={[styles.tabText, activeTab === 'incoming' && styles.activeTabText]}>
            Incoming
          </Text>
          {incomingRequests.length > 0 && (
            <View style={[styles.tabBadge, activeTab === 'incoming' && styles.activeTabBadge]}>
              <Text style={styles.tabBadgeText}>{incomingRequests.length}</Text>
            </View>
          )}
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.tab, activeTab === 'outgoing' && styles.activeTab]}
          onPress={() => setActiveTab('outgoing')}
        >
          <Text style={[styles.tabText, activeTab === 'outgoing' && styles.activeTabText]}>
            Outgoing
          </Text>
          {outgoingRequests.length > 0 && (
            <View style={[styles.tabBadge, activeTab === 'outgoing' && styles.activeTabBadge]}>
              <Text style={styles.tabBadgeText}>{outgoingRequests.length}</Text>
            </View>
          )}
        </TouchableOpacity>
      </View>

      {/* Request List */}
      <FlatList
        data={currentData}
        keyExtractor={item => item.id}
        renderItem={renderRequest}
        ListEmptyComponent={renderEmptyState}
        contentContainerStyle={
          currentData.length === 0 ? styles.emptyContainer : styles.listContent
        }
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={() => loadRequests(true)}
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
  tabsContainer: {
    flexDirection: 'row',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    gap: spacing.sm,
  },
  tab: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: spacing.md,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.background.secondary,
    gap: spacing.sm,
  },
  activeTab: {
    backgroundColor: colors.pierre.violet,
  },
  tabText: {
    color: colors.text.secondary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  activeTabText: {
    color: colors.text.primary,
  },
  tabBadge: {
    backgroundColor: colors.background.tertiary,
    borderRadius: 10,
    minWidth: 20,
    height: 20,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 6,
  },
  activeTabBadge: {
    backgroundColor: 'rgba(255, 255, 255, 0.2)',
  },
  tabBadgeText: {
    color: colors.text.primary,
    fontSize: fontSize.xs,
    fontWeight: '700',
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
