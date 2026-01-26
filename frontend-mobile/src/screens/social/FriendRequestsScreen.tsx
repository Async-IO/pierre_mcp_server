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
import { colors, spacing } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { RequestCard } from '../../components/social/FriendCard';
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

  const handleAccept = async (request: PendingRequestWithInfo) => {
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

  const handleDecline = async (request: PendingRequestWithInfo) => {
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

  const handleCancel = async (request: PendingRequestWithInfo) => {
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

  const renderRequest = ({ item }: { item: PendingRequestWithInfo }) => (
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
    <View className="flex-1 justify-center items-center p-6">
      <Feather
        name={activeTab === 'incoming' ? 'inbox' : 'send'}
        size={64}
        color={colors.text.tertiary}
      />
      <Text className="text-text-primary text-xl font-bold mt-5">
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

      {/* Tabs */}
      <View className="flex-row px-4 py-4 gap-2">
        <TouchableOpacity
          className={`flex-1 flex-row items-center justify-center py-4 rounded-lg gap-2 ${
            activeTab === 'incoming' ? '' : 'bg-background-secondary'
          }`}
          style={activeTab === 'incoming' ? { backgroundColor: colors.pierre.violet } : undefined}
          onPress={() => setActiveTab('incoming')}
        >
          <Text className={`text-base font-semibold ${activeTab === 'incoming' ? 'text-text-primary' : 'text-text-secondary'}`}>
            Incoming
          </Text>
          {incomingRequests.length > 0 && (
            <View
              className="rounded-full min-w-[20px] h-5 justify-center items-center px-1.5"
              style={{ backgroundColor: activeTab === 'incoming' ? 'rgba(255, 255, 255, 0.2)' : colors.background.tertiary }}
            >
              <Text className="text-text-primary text-xs font-bold">{incomingRequests.length}</Text>
            </View>
          )}
        </TouchableOpacity>
        <TouchableOpacity
          className={`flex-1 flex-row items-center justify-center py-4 rounded-lg gap-2 ${
            activeTab === 'outgoing' ? '' : 'bg-background-secondary'
          }`}
          style={activeTab === 'outgoing' ? { backgroundColor: colors.pierre.violet } : undefined}
          onPress={() => setActiveTab('outgoing')}
        >
          <Text className={`text-base font-semibold ${activeTab === 'outgoing' ? 'text-text-primary' : 'text-text-secondary'}`}>
            Outgoing
          </Text>
          {outgoingRequests.length > 0 && (
            <View
              className="rounded-full min-w-[20px] h-5 justify-center items-center px-1.5"
              style={{ backgroundColor: activeTab === 'outgoing' ? 'rgba(255, 255, 255, 0.2)' : colors.background.tertiary }}
            >
              <Text className="text-text-primary text-xs font-bold">{outgoingRequests.length}</Text>
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
