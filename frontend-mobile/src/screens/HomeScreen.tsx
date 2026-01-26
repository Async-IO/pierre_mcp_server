// ABOUTME: Home Dashboard screen with Stitch UX design
// ABOUTME: Shows greeting, quick stats, active coach, and recent activities

import React, { useState, useEffect, useCallback } from 'react';
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
import { Feather, Ionicons } from '@expo/vector-icons';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { colors, spacing, aiGlow } from '../constants/theme';
import { apiService } from '../services/api';
import { useAuth } from '../contexts/AuthContext';
import type { Coach, Conversation } from '../types';

// Type for navigation - we'll navigate to different tabs
interface HomeScreenProps {
  navigation: NativeStackNavigationProp<Record<string, undefined>>;
}

// Mock activity data type (would come from provider in production)
interface RecentActivity {
  id: string;
  type: 'run' | 'bike' | 'swim' | 'strength' | 'yoga';
  title: string;
  date: string;
  duration: string;
  distance?: string;
  tss?: number;
}

// Activity icons mapping
const ACTIVITY_ICONS: Record<string, keyof typeof Ionicons.glyphMap> = {
  run: 'walk-outline',
  bike: 'bicycle-outline',
  swim: 'water-outline',
  strength: 'barbell-outline',
  yoga: 'body-outline',
};

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
  const [coaches, setCoaches] = useState<Coach[]>([]);

  // Mock stats data - in production, these would come from provider APIs
  const [stats] = useState({
    weeklyTSS: 342,
    trainingLoad: { value: 'Optimal', trend: 'up' as const },
    recoveryScore: 78,
  });

  // Mock recent activities - in production, these would come from provider APIs
  const [recentActivities] = useState<RecentActivity[]>([
    {
      id: '1',
      type: 'run',
      title: 'Morning Easy Run',
      date: 'Today',
      duration: '45:12',
      distance: '8.2 km',
      tss: 48,
    },
    {
      id: '2',
      type: 'bike',
      title: 'Interval Training',
      date: 'Yesterday',
      duration: '1:15:30',
      distance: '35.4 km',
      tss: 89,
    },
    {
      id: '3',
      type: 'strength',
      title: 'Core & Stability',
      date: '2 days ago',
      duration: '30:00',
      tss: 25,
    },
  ]);

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

      setCoaches(coachesResponse.coaches);

      // Find most recently used coach
      const sortedByUse = [...coachesResponse.coaches].sort((a, b) => {
        if (!a.last_used_at && !b.last_used_at) return 0;
        if (!a.last_used_at) return 1;
        if (!b.last_used_at) return -1;
        return new Date(b.last_used_at).getTime() - new Date(a.last_used_at).getTime();
      });

      if (sortedByUse.length > 0 && sortedByUse[0].last_used_at) {
        setActiveCoach(sortedByUse[0]);
      } else if (coachesResponse.coaches.length > 0) {
        // Fall back to first favorite or first coach
        const favorite = coachesResponse.coaches.find(c => c.is_favorite);
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
    // Navigate to Chat tab
    navigation.navigate('ChatTab' as never);
  };

  const handleViewActivity = (activityId: string) => {
    // Navigate to Activity detail (would need to add this route)
    console.log('View activity:', activityId);
  };

  const displayName = user?.display_name || user?.email?.split('@')[0] || 'Athlete';

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
            {/* Notification dot */}
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

        {/* Quick Stats Row */}
        <View className="flex-row gap-3 mb-6">
          {/* Weekly TSS */}
          <View style={glassCardStyle} className="flex-1 p-4">
            <Text className="text-xs text-zinc-500 mb-1">Weekly TSS</Text>
            <Text className="text-2xl font-bold text-pierre-cyan">{stats.weeklyTSS}</Text>
          </View>

          {/* Training Load */}
          <View style={glassCardStyle} className="flex-1 p-4">
            <Text className="text-xs text-zinc-500 mb-1">Training Load</Text>
            <View className="flex-row items-center">
              <Text className="text-lg font-semibold text-white mr-1">{stats.trainingLoad.value}</Text>
              <Feather
                name={stats.trainingLoad.trend === 'up' ? 'arrow-up' : 'arrow-down'}
                size={16}
                color={colors.pierre.activity}
              />
            </View>
          </View>

          {/* Recovery Score */}
          <View style={glassCardStyle} className="flex-1 p-4 items-center">
            <Text className="text-xs text-zinc-500 mb-1">Recovery</Text>
            <View className="relative w-12 h-12 items-center justify-center">
              {/* Circular indicator background */}
              <View
                className="absolute w-12 h-12 rounded-full border-4 border-zinc-700"
              />
              {/* Circular progress indicator */}
              <View
                className="absolute w-12 h-12 rounded-full border-4 border-pierre-cyan"
                style={{
                  borderTopColor: 'transparent',
                  borderRightColor: 'transparent',
                  transform: [{ rotate: `${(stats.recoveryScore / 100) * 360 - 90}deg` }],
                }}
              />
              <Text className="text-sm font-bold text-pierre-cyan">{stats.recoveryScore}</Text>
            </View>
          </View>
        </View>

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
              <View className="flex-row items-center">
                {/* Coach Avatar */}
                <LinearGradient
                  colors={[colors.pierre.violet, colors.pierre.cyan]}
                  start={{ x: 0, y: 0 }}
                  end={{ x: 1, y: 1 }}
                  className="w-14 h-14 rounded-full items-center justify-center mr-4"
                >
                  <Text className="text-2xl">
                    {activeCoach.category === 'training' ? '🏃' :
                     activeCoach.category === 'nutrition' ? '🥗' :
                     activeCoach.category === 'recovery' ? '😴' :
                     activeCoach.category === 'recipes' ? '👨‍🍳' :
                     activeCoach.category === 'mobility' ? '🧘' : '⚙️'}
                  </Text>
                </LinearGradient>

                <View className="flex-1">
                  <Text className="text-base font-semibold text-white mb-1">
                    {activeCoach.title}
                  </Text>
                  <Text className="text-sm text-zinc-400" numberOfLines={1}>
                    {lastConversation?.title || 'Start a new conversation'}
                  </Text>
                </View>

                {/* Continue Chat Button */}
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

        {/* Recent Activities */}
        <View className="mb-6">
          <View className="flex-row items-center justify-between mb-3">
            <Text className="text-lg font-semibold text-white">Recent Activities</Text>
            <TouchableOpacity>
              <Text className="text-sm text-pierre-violet">See All</Text>
            </TouchableOpacity>
          </View>

          {recentActivities.map((activity) => (
            <TouchableOpacity
              key={activity.id}
              style={glassCardStyle}
              className="p-4 mb-3"
              onPress={() => handleViewActivity(activity.id)}
              activeOpacity={0.8}
            >
              <View className="flex-row items-center">
                {/* Activity Icon */}
                <View className="w-12 h-12 rounded-xl bg-pierre-slate items-center justify-center mr-3">
                  <Ionicons
                    name={ACTIVITY_ICONS[activity.type] || 'fitness-outline'}
                    size={24}
                    color={colors.pierre.cyan}
                  />
                </View>

                {/* Activity Info */}
                <View className="flex-1">
                  <Text className="text-base font-semibold text-white mb-0.5">
                    {activity.title}
                  </Text>
                  <Text className="text-sm text-zinc-500">{activity.date}</Text>
                </View>

                {/* Activity Metrics */}
                <View className="items-end">
                  <Text className="text-sm font-medium text-white">{activity.duration}</Text>
                  {activity.distance && (
                    <Text className="text-xs text-zinc-500">{activity.distance}</Text>
                  )}
                  {activity.tss && (
                    <Text className="text-xs text-pierre-cyan">TSS {activity.tss}</Text>
                  )}
                </View>
              </View>
            </TouchableOpacity>
          ))}
        </View>

        {/* Quick Actions */}
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
              <Feather name="users" size={24} color={colors.pierre.cyan} />
              <Text className="text-sm text-white mt-2">Social Feed</Text>
            </TouchableOpacity>

            <TouchableOpacity
              style={glassCardStyle}
              className="flex-1 p-4 items-center"
              onPress={() => navigation.navigate('SettingsTab' as never)}
            >
              <Feather name="settings" size={24} color={colors.text.secondary} />
              <Text className="text-sm text-white mt-2">Settings</Text>
            </TouchableOpacity>
          </View>
        </View>
      </ScrollView>
    </View>
  );
}
