// ABOUTME: Friend requests management screen
// ABOUTME: Shows incoming and outgoing friend requests with accept/decline actions

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  RefreshControl,
} from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, glassCard } from '../../constants/theme';
import { socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { RequestCard } from '../../components/social/FriendCard';
import { SwipeableRow, type SwipeAction } from '../../components/ui';
import type { PendingRequestWithInfo } from '@pierre/shared-types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;
type TabType = 'incoming' | 'outgoing';

export function FriendRequestsScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();
  const [activeTab, setActiveTab] = useState<TabType>('incoming');
  const [incomingRequests, setIncomingRequests] = useState<PendingRequestWithInfo[]>([]);
  const [outgoingRequests, setOutgoingRequests] = useState<PendingRequestWithInfo[]>([]);
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

      const response = await socialApi.getPendingRequests();
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

  const handleAccept = async (request: PendingRequestWithInfo) => {
    try {
      setProcessingIds(prev => new Set(prev).add(request.id));
      await socialApi.acceptFriendRequest(request.id);
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

  const handleDecline = async (request: PendingRequestWithInfo) => {
    try {
      setProcessingIds(prev => new Set(prev).add(request.id));
      await socialApi.declineFriendRequest(request.id);
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

  const handleCancel = async (request: PendingRequestWithInfo) => {
    try {
      setProcessingIds(prev => new Set(prev).add(request.id));
      await socialApi.removeFriend(request.id);
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

  const renderRequest = ({ item }: { item: PendingRequestWithInfo }) => {
    const leftActions: SwipeAction[] = activeTab === 'incoming' ? [
      {
        icon: 'check',
        label: 'Accept',
        color: '#FFFFFF',
        backgroundColor: '#10B981',
        onPress: () => handleAccept(item),
      },
    ] : [];

    const rightActions: SwipeAction[] = activeTab === 'incoming' ? [
      {
        icon: 'x',
        label: 'Decline',
        color: '#FFFFFF',
        backgroundColor: '#EF4444',
        onPress: () => handleDecline(item),
      },
    ] : [
      {
        icon: 'x-circle',
        label: 'Cancel',
        color: '#FFFFFF',
        backgroundColor: '#EF4444',
        onPress: () => handleCancel(item),
      },
    ];

    return (
      <SwipeableRow
        leftActions={leftActions}
        rightActions={rightActions}
        testID={`swipeable-request-${item.id}`}
      >
        <RequestCard
          request={item}
          type={activeTab}
          onAccept={activeTab === 'incoming' ? () => handleAccept(item) : undefined}
          onDecline={activeTab === 'incoming' ? () => handleDecline(item) : undefined}
          onCancel={activeTab === 'outgoing' ? () => handleCancel(item) : undefined}
          isLoading={processingIds.has(item.id)}
        />
      </SwipeableRow>
    );
  };

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
        <Feather
          name={activeTab === 'incoming' ? 'inbox' : 'send'}
          size={48}
          color={colors.pierre.violet}
        />
      </View>
      <Text className="text-text-primary text-xl font-bold mt-4">
        {activeTab === 'incoming' ? 'No Incoming Requests' : 'No Outgoing Requests'}
      </Text>
      <Text className="text-text-secondary text-base text-center mt-2">
        {activeTab === 'incoming'
          ? 'When someone sends you a friend request, it will appear here'
          : 'Friend requests you send will appear here until accepted'}
      </Text>
    </View>
  );

  const currentData = activeTab === 'incoming' ? incomingRequests : outgoingRequests;

  if (isLoading && incomingRequests.length === 0 && outgoingRequests.length === 0) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text className="text-text-secondary mt-4">Loading requests...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <TouchableOpacity
          className="p-2 mr-2"
          onPress={() => navigation.goBack()}
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-xl font-bold text-text-primary">Friend Requests</Text>
      </View>

      {/* Tabs with glassmorphism */}
      <View className="flex-row px-4 py-4 gap-3">
        <TouchableOpacity
          className="flex-1 flex-row items-center justify-center py-4 rounded-xl gap-2"
          style={activeTab === 'incoming' ? {
            backgroundColor: colors.pierre.violet,
            shadowColor: colors.pierre.violet,
            shadowOffset: { width: 0, height: 0 },
            shadowOpacity: 0.3,
            shadowRadius: 8,
            elevation: 4,
          } : {
            ...glassCard,
            borderRadius: 12,
          }}
          onPress={() => setActiveTab('incoming')}
        >
          <Text className={`text-base font-semibold ${activeTab === 'incoming' ? 'text-white' : 'text-text-secondary'}`}>
            Incoming
          </Text>
          {incomingRequests.length > 0 && (
            <View
              className="rounded-full min-w-[20px] h-5 justify-center items-center px-1.5"
              style={{ backgroundColor: activeTab === 'incoming' ? 'rgba(255, 255, 255, 0.25)' : 'rgba(139, 92, 246, 0.2)' }}
            >
              <Text className={`text-xs font-bold ${activeTab === 'incoming' ? 'text-white' : ''}`} style={activeTab !== 'incoming' ? { color: colors.pierre.violet } : undefined}>
                {incomingRequests.length}
              </Text>
            </View>
          )}
        </TouchableOpacity>
        <TouchableOpacity
          className="flex-1 flex-row items-center justify-center py-4 rounded-xl gap-2"
          style={activeTab === 'outgoing' ? {
            backgroundColor: colors.pierre.violet,
            shadowColor: colors.pierre.violet,
            shadowOffset: { width: 0, height: 0 },
            shadowOpacity: 0.3,
            shadowRadius: 8,
            elevation: 4,
          } : {
            ...glassCard,
            borderRadius: 12,
          }}
          onPress={() => setActiveTab('outgoing')}
        >
          <Text className={`text-base font-semibold ${activeTab === 'outgoing' ? 'text-white' : 'text-text-secondary'}`}>
            Outgoing
          </Text>
          {outgoingRequests.length > 0 && (
            <View
              className="rounded-full min-w-[20px] h-5 justify-center items-center px-1.5"
              style={{ backgroundColor: activeTab === 'outgoing' ? 'rgba(255, 255, 255, 0.25)' : 'rgba(139, 92, 246, 0.2)' }}
            >
              <Text className={`text-xs font-bold ${activeTab === 'outgoing' ? 'text-white' : ''}`} style={activeTab !== 'outgoing' ? { color: colors.pierre.violet } : undefined}>
                {outgoingRequests.length}
              </Text>
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
        contentContainerStyle={currentData.length === 0 ? { flexGrow: 1 } : { paddingVertical: spacing.sm }}
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
