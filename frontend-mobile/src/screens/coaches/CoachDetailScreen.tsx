// ABOUTME: Coach detail screen showing full coach info with edit/delete actions
// ABOUTME: Read-only view of user's coach with option to edit or use in chat

import React, { useState, useCallback, useEffect } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
} from 'react-native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RouteProp } from '@react-navigation/native';
import { Feather } from '@expo/vector-icons';
import { colors, spacing } from '../../constants/theme';
import { coachesApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { Coach } from '../../types';
import type { CoachesStackParamList } from '../../navigation/MainTabs';

interface CoachDetailScreenProps {
  navigation: NativeStackNavigationProp<CoachesStackParamList>;
  route: RouteProp<CoachesStackParamList, 'CoachDetail'>;
}

// Coach category colors
const COACH_CATEGORY_COLORS: Record<string, string> = {
  training: '#10B981',
  nutrition: '#F59E0B',
  recovery: '#6366F1',
  recipes: '#F97316',
  mobility: '#EC4899',
  custom: '#7C3AED',
};

export function CoachDetailScreen({ navigation, route }: CoachDetailScreenProps) {
  const { coachId } = route.params;
  const { isAuthenticated } = useAuth();
  const [coach, setCoach] = useState<Coach | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isHidden, setIsHidden] = useState(false);
  const [isTogglingHidden, setIsTogglingHidden] = useState(false);

  const loadCoachDetail = useCallback(async () => {
    if (!isAuthenticated || !coachId) return;

    try {
      setIsLoading(true);
      // Load coaches and hidden coaches list in parallel
      const [coachesResponse, hiddenResponse] = await Promise.all([
        coachesApi.listCoaches({ include_hidden: true }),
        coachesApi.getHiddenCoaches(),
      ]);
      const foundCoach = coachesResponse.coaches.find((c: { id: string }) => c.id === coachId);
      setCoach(foundCoach || null);

      // Check if this coach is in the hidden list
      const hiddenIds = new Set((hiddenResponse.coaches || []).map((c: { id: string }) => c.id));
      setIsHidden(hiddenIds.has(coachId));
    } catch (error) {
      console.error('Failed to load coach detail:', error);
      Alert.alert('Error', 'Failed to load coach details');
    } finally {
      setIsLoading(false);
    }
  }, [isAuthenticated, coachId]);

  useEffect(() => {
    loadCoachDetail();
  }, [loadCoachDetail]);

  const handleEdit = () => {
    if (!coach) return;
    navigation.navigate('CoachWizard', { coachId: coach.id });
  };

  const handleDelete = () => {
    if (!coach) return;

    Alert.alert(
      'Delete Coach?',
      `Are you sure you want to delete "${coach.title}"? This action cannot be undone.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete',
          style: 'destructive',
          onPress: async () => {
            try {
              setIsDeleting(true);
              await coachesApi.deleteCoach(coach.id);
              Alert.alert('Deleted', 'Coach has been deleted.');
              navigation.goBack();
            } catch (error) {
              console.error('Failed to delete coach:', error);
              Alert.alert('Error', 'Failed to delete coach. Please try again.');
            } finally {
              setIsDeleting(false);
            }
          },
        },
      ]
    );
  };

  const handleUseInChat = () => {
    if (!coach) return;
    // Navigate to chat tab using parent tab navigator
    const parent = navigation.getParent();
    if (parent) {
      parent.navigate('ChatTab');
    }
  };

  const handleToggleHidden = async () => {
    if (!coach) return;

    try {
      setIsTogglingHidden(true);
      if (isHidden) {
        await coachesApi.show(coach.id);
        setIsHidden(false);
      } else {
        await coachesApi.hide(coach.id);
        setIsHidden(true);
      }
    } catch (error) {
      console.error('Failed to toggle coach visibility:', error);
      Alert.alert('Error', 'Failed to update coach visibility');
    } finally {
      setIsTogglingHidden(false);
    }
  };

  if (isLoading) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.primary[500]} />
          <Text className="mt-3 text-text-secondary text-base">Loading coach details...</Text>
        </View>
      </SafeAreaView>
    );
  }

  if (!coach) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center px-6">
          <Text className="text-lg text-text-secondary mb-3">Coach not found</Text>
          <TouchableOpacity
            className="px-5 py-2 bg-primary-500 rounded-md"
            onPress={() => navigation.goBack()}
          >
            <Text className="text-text-primary text-base font-medium">Go Back</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  const categoryColor = COACH_CATEGORY_COLORS[coach.category];

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="coach-detail-screen">
      {/* Header */}
      <View className="flex-row items-center px-3 py-2 border-b border-border-default">
        <TouchableOpacity
          testID="back-button"
          className="p-2"
          onPress={() => navigation.goBack()}
        >
          <Text className="text-2xl text-text-primary">←</Text>
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-semibold text-text-primary text-center mx-2" numberOfLines={1}>
          {coach.title}
        </Text>
        {!coach.is_system && (
          <TouchableOpacity
            testID="edit-button"
            className="p-2"
            onPress={handleEdit}
          >
            <Feather name="edit-2" size={20} color={colors.primary[500]} />
          </TouchableOpacity>
        )}
        {coach.is_system && <View className="w-10" />}
      </View>

      <ScrollView className="flex-1" showsVerticalScrollIndicator={false}>
        {/* Category & Stats */}
        <View className="flex-row justify-between items-center px-5 pt-5 pb-2">
          <View className="flex-row items-center gap-2">
            <View
              testID="category-badge"
              className="px-3 py-1 rounded-full"
              style={{ backgroundColor: categoryColor + '20' }}
            >
              <Text className="text-sm font-semibold capitalize" style={{ color: categoryColor }}>
                {coach.category}
              </Text>
            </View>
            {coach.is_system && (
              <View className="px-2 py-1 rounded" style={{ backgroundColor: colors.primary[500] + '30' }}>
                <Text className="text-xs font-semibold text-primary-500">System</Text>
              </View>
            )}
            {coach.is_favorite && (
              <Feather name="star" size={16} color="#F59E0B" style={{ marginLeft: spacing.xs }} />
            )}
          </View>
          <Text testID="use-count" className="text-sm text-text-secondary">
            Used {coach.use_count} {coach.use_count === 1 ? 'time' : 'times'}
          </Text>
        </View>

        {/* Title */}
        <Text testID="coach-title" className="text-2xl font-bold text-text-primary px-5 mb-2">{coach.title}</Text>

        {/* Description */}
        {coach.description && (
          <Text className="text-base text-text-secondary px-5 leading-[22px] mb-3">{coach.description}</Text>
        )}

        {/* Tags */}
        {coach.tags.length > 0 && (
          <View className="px-5 py-3">
            <Text className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-2">Tags</Text>
            <View className="flex-row flex-wrap">
              {coach.tags.map((tag, index) => (
                <View key={index} className="bg-background-secondary px-3 py-1 rounded-full mr-2 mb-2 border border-border-default">
                  <Text className="text-sm text-text-primary">{tag}</Text>
                </View>
              ))}
            </View>
          </View>
        )}

        {/* System Prompt */}
        <View className="px-5 py-3">
          <Text className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-2">System Prompt</Text>
          <View className="bg-background-secondary p-3 rounded-md border border-border-default">
            <Text className="text-sm text-text-secondary leading-5 font-mono">
              {coach.system_prompt}
            </Text>
          </View>
        </View>

        {/* Metadata */}
        <View className="px-5 py-3">
          <Text className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-2">Details</Text>
          <View className="bg-background-secondary rounded-md border border-border-default overflow-hidden">
            <View className="flex-row justify-between items-center px-3 py-2 border-b border-border-default">
              <Text className="text-sm text-text-secondary">Token Count</Text>
              <Text className="text-sm text-text-primary font-medium">{coach.token_count}</Text>
            </View>
            <View className="flex-row justify-between items-center px-3 py-2 border-b border-border-default">
              <Text className="text-sm text-text-secondary">Context Usage</Text>
              <Text className="text-sm text-text-primary font-medium">
                {((coach.token_count / 128000) * 100).toFixed(1)}%
              </Text>
            </View>
            {coach.created_at && (
              <View className="flex-row justify-between items-center px-3 py-2 border-b border-border-default">
                <Text className="text-sm text-text-secondary">Created</Text>
                <Text className="text-sm text-text-primary font-medium">
                  {new Date(coach.created_at).toLocaleDateString()}
                </Text>
              </View>
            )}
            {coach.last_used_at && (
              <View className="flex-row justify-between items-center px-3 py-2">
                <Text className="text-sm text-text-secondary">Last Used</Text>
                <Text className="text-sm text-text-primary font-medium">
                  {new Date(coach.last_used_at).toLocaleDateString()}
                </Text>
              </View>
            )}
          </View>
        </View>

        {/* Bottom Spacer for Action Buttons */}
        <View className="h-[120px]" />
      </ScrollView>

      {/* Action Bar - Fixed at bottom */}
      <View className="absolute bottom-0 left-0 right-0 flex-row bg-background-primary border-t border-border-default p-3 pb-5 gap-2">
        <TouchableOpacity
          className="flex-1 flex-row items-center justify-center py-3 rounded-md gap-1 bg-primary-500"
          onPress={handleUseInChat}
          testID="use-in-chat-button"
        >
          <Feather name="message-circle" size={18} color={colors.text.primary} />
          <Text className="text-text-primary text-base font-semibold">Use in Chat</Text>
        </TouchableOpacity>

        {coach.is_system && (
          <TouchableOpacity
            className="flex-1 flex-row items-center justify-center py-3 rounded-md gap-1 bg-background-secondary border border-border-default"
            onPress={handleToggleHidden}
            disabled={isTogglingHidden}
            testID="hide-button"
          >
            {isTogglingHidden ? (
              <ActivityIndicator size="small" color={colors.text.secondary} />
            ) : (
              <>
                <Feather
                  name={isHidden ? 'eye' : 'eye-off'}
                  size={18}
                  color={isHidden ? colors.primary[400] : colors.text.secondary}
                />
                <Text className={`text-base font-medium ${isHidden ? 'text-primary-400' : 'text-text-secondary'}`}>
                  {isHidden ? 'Show' : 'Hide'}
                </Text>
              </>
            )}
          </TouchableOpacity>
        )}

        {!coach.is_system && (
          <TouchableOpacity
            className="flex-1 flex-row items-center justify-center py-3 rounded-md gap-1 bg-background-secondary border border-error"
            onPress={handleDelete}
            disabled={isDeleting}
            testID="delete-button"
          >
            {isDeleting ? (
              <ActivityIndicator size="small" color="#EF4444" />
            ) : (
              <>
                <Feather name="trash-2" size={18} color="#EF4444" />
                <Text className="text-error text-base font-medium">Delete</Text>
              </>
            )}
          </TouchableOpacity>
        )}
      </View>
    </SafeAreaView>
  );
}
