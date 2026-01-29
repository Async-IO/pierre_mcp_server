// ABOUTME: Home Dashboard screen with coach-first design philosophy
// ABOUTME: Shows greeting, training status, active coach, and quick actions - no raw activity metrics

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  ScrollView,
  TouchableOpacity,
  RefreshControl,
  Image,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useFocusEffect } from '@react-navigation/native';
import { LinearGradient } from 'expo-linear-gradient';
import { Feather } from '@expo/vector-icons';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { colors, spacing, aiGlow } from '../constants/theme';
import { apiService } from '../services/api';
import { useAuth } from '../contexts/AuthContext';
import type { Coach, Conversation } from '../types';

interface HomeScreenProps {
  navigation: NativeStackNavigationProp<Record<string, undefined>>;
}

// Glassmorphism card style
const glassCardStyle: ViewStyle = {
  backgroundColor: 'rgba(255, 255, 255, 0.05)',
  borderWidth: 1,
  borderColor: 'rgba(255, 255, 255, 0.1)',
  borderRadius: 16,
};

// Coach card with AI glow
const coachCardStyle: ViewStyle = {
  ...glassCardStyle,
  ...aiGlow.ambient,
};

export function HomeScreen({ navigation }: HomeScreenProps) {
  const { user } = useAuth();
  const insets = useSafeAreaInsets();
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [activeCoach, setActiveCoach] = useState<Coach | null>(null);
  const [lastConversation, setLastConversation] = useState<Conversation | null>(null);
  const [recentCoaches, setRecentCoaches] = useState<Coach[]>([]);

  // Get greeting based on time of day
  const getGreeting = (): string => {
    const hour = new Date().getHours();
    if (hour < 12) return 'Good morning';
    if (hour < 17) return 'Good afternoon';
    return 'Good evening';
  };

  // Load coaches and find the most recently used one
  const loadData = useCallback(async () => {
    try {
      const [coachesResponse, conversationsResponse] = await Promise.all([
        apiService.listCoaches({ favorites_only: false }),
        apiService.getConversations(1, 0),
      ]);

      // Find most recently used coach
      const sortedByUse = [...coachesResponse.coaches].sort((a, b) => {
        if (!a.last_used_at && !b.last_used_at) return 0;
        if (!a.last_used_at) return 1;
        if (!b.last_used_at) return -1;
        return new Date(b.last_used_at).getTime() - new Date(a.last_used_at).getTime();
      });

      // Get recently used coaches (up to 3)
      const recent = sortedByUse.filter(c => c.last_used_at).slice(0, 3);
      setRecentCoaches(recent);

      if (sortedByUse.length > 0 && sortedByUse[0].last_used_at) {
        setActiveCoach(sortedByUse[0]);
      } else if (coachesResponse.coaches.length > 0) {
        // Fall back to first favorite or first coach
        const favorite = coachesResponse.coaches.find((c: Coach) => c.is_favorite);
        setActiveCoach(favorite || coachesResponse.coaches[0]);
      }

      // Set last conversation
      if (conversationsResponse.conversations.length > 0) {
        setLastConversation(conversationsResponse.conversations[0]);
      }
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
    }
  }, []);

  useFocusEffect(
    useCallback(() => {
      loadData();
    }, [loadData])
  );

  const handleRefresh = async () => {
    setIsRefreshing(true);
    await loadData();
    setIsRefreshing(false);
  };

  const handleChatWithCoach = () => {
    navigation.navigate('ChatTab' as never);
  };

  const displayName = user?.display_name || user?.email?.split('@')[0] || 'Athlete';

  const getCoachEmoji = (category: string): string => {
    switch (category) {
      case 'training': return '🏃';
      case 'nutrition': return '🥗';
      case 'recovery': return '😴';
      case 'recipes': return '👨‍🍳';
      case 'mobility': return '🧘';
      default: return '⚙️';
    }
  };

  return (
    <View className="flex-1 bg-pierre-dark">
      <ScrollView
        className="flex-1"
        contentContainerStyle={{
          paddingTop: insets.top + spacing.sm,
          paddingBottom: 100,
          paddingHorizontal: spacing.md,
        }}
        showsVerticalScrollIndicator={false}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={handleRefresh}
            tintColor={colors.pierre.violet}
          />
        }
      >
        {/* Header */}
        <View className="flex-row items-center justify-between mb-4">
          <Image
            source={require('../../assets/pierre-logo.png')}
            className="w-10 h-10"
            resizeMode="contain"
          />
          <TouchableOpacity className="relative p-2">
            <Feather name="bell" size={24} color={colors.text.primary} />
            <View className="absolute top-1 right-1 w-2.5 h-2.5 rounded-full bg-pierre-violet" />
          </TouchableOpacity>
        </View>

        {/* Greeting Card with Glassmorphism */}
        <LinearGradient
          colors={['rgba(139, 92, 246, 0.15)', 'rgba(34, 211, 238, 0.08)']}
          start={{ x: 0, y: 0 }}
          end={{ x: 1, y: 1 }}
          style={[glassCardStyle, { marginBottom: spacing.lg, padding: spacing.lg }]}
        >
          <Text className="text-2xl font-bold text-white mb-1">
            {getGreeting()}, {displayName}
          </Text>
          <Text className="text-base text-zinc-400">
            Ready for another great day of training?
          </Text>
        </LinearGradient>

        {/* Active Coach Card */}
        {activeCoach && (
          <View className="mb-6">
            <Text className="text-lg font-semibold text-white mb-3">Active Coach</Text>
            <TouchableOpacity
              style={coachCardStyle}
              className="p-4"
              onPress={handleChatWithCoach}
              activeOpacity={0.8}
            >
              <View className="flex-row items-center gap-4">
                <LinearGradient
                  colors={[colors.pierre.violet, colors.pierre.cyan]}
                  start={{ x: 0, y: 0 }}
                  end={{ x: 1, y: 1 }}
                  className="w-12 h-12 rounded-full items-center justify-center"
                >
                  <Text className="text-xl">{getCoachEmoji(activeCoach.category)}</Text>
                </LinearGradient>

                <View className="flex-1 ml-1">
                  <Text className="text-base font-semibold text-white mb-1">
                    {activeCoach.title}
                  </Text>
                  <Text className="text-sm text-zinc-400" numberOfLines={1}>
                    {lastConversation?.title || 'Start a new conversation'}
                  </Text>
                </View>

                <LinearGradient
                  colors={[colors.pierre.violet, '#7C3AED']}
                  start={{ x: 0, y: 0 }}
                  end={{ x: 1, y: 0 }}
                  className="px-4 py-2 rounded-full"
                >
                  <Text className="text-sm font-semibold text-white">Continue Chat</Text>
                </LinearGradient>
              </View>
            </TouchableOpacity>
          </View>
        )}

        {/* Recent Coaches - Coach-First, Not Activity-First */}
        {recentCoaches.length > 1 && (
          <View className="mb-6">
            <Text className="text-lg font-semibold text-white mb-3">Recent Coaches</Text>
            {recentCoaches.slice(1).map((coach) => (
              <TouchableOpacity
                key={coach.id}
                style={glassCardStyle}
                className="p-4 mb-3"
                onPress={handleChatWithCoach}
                activeOpacity={0.8}
              >
                <View className="flex-row items-center">
                  <View className="w-10 h-10 rounded-full bg-pierre-violet/20 items-center justify-center mr-3">
                    <Text className="text-lg">{getCoachEmoji(coach.category)}</Text>
                  </View>
                  <View className="flex-1">
                    <Text className="text-base font-semibold text-white">{coach.title}</Text>
                    <Text className="text-sm text-zinc-500" numberOfLines={1}>
                      {coach.description}
                    </Text>
                  </View>
                  <Feather name="chevron-right" size={20} color={colors.text.tertiary} />
                </View>
              </TouchableOpacity>
            ))}
          </View>
        )}

        {/* Quick Actions - Only Browse Coaches and Friends */}
        <View className="mb-6">
          <Text className="text-lg font-semibold text-white mb-3">Quick Actions</Text>
          <View className="flex-row gap-3">
            <TouchableOpacity
              style={glassCardStyle}
              className="flex-1 p-4 items-center"
              onPress={() => navigation.navigate('CoachesTab' as never)}
            >
              <Feather name="award" size={24} color={colors.pierre.violet} />
              <Text className="text-sm text-white mt-2">Browse Coaches</Text>
            </TouchableOpacity>

            <TouchableOpacity
              style={glassCardStyle}
              className="flex-1 p-4 items-center"
              onPress={() => navigation.navigate('SocialTab' as never)}
            >
              <Feather name="zap" size={24} color={colors.pierre.cyan} />
              <Text className="text-sm text-white mt-2">Insights</Text>
            </TouchableOpacity>
          </View>
        </View>
      </ScrollView>
    </View>
  );
}
