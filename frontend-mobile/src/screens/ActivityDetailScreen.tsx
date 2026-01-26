// ABOUTME: Activity Detail screen with Stitch UX design
// ABOUTME: Shows activity stats, performance metrics, AI insights, and splits

import React, { useState } from 'react';
import {
  View,
  Text,
  ScrollView,
  TouchableOpacity,
  Share,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { LinearGradient } from 'expo-linear-gradient';
import { Feather, Ionicons } from '@expo/vector-icons';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { colors, spacing, aiGlow } from '../constants/theme';

// Type for navigation
interface ActivityDetailScreenProps {
  navigation: NativeStackNavigationProp<Record<string, undefined>>;
  route: {
    params?: {
      activityId?: string;
    };
  };
}

// Mock activity data type
interface ActivityData {
  id: string;
  type: 'run' | 'bike' | 'swim' | 'strength';
  title: string;
  date: string;
  duration: string;
  distance: string;
  pace: string;
  elevation: string;
  tss: number;
  calories: number;
  avgHeartRate: number;
  maxHeartRate: number;
  weather: {
    temp: number;
    condition: string;
  };
  splits: Array<{
    km: number;
    pace: string;
    heartRate: number;
  }>;
}

// Glassmorphism card style
const glassCardStyle: ViewStyle = {
  backgroundColor: 'rgba(255, 255, 255, 0.05)',
  borderWidth: 1,
  borderColor: 'rgba(255, 255, 255, 0.1)',
  borderRadius: 16,
};

// AI card with glow
const aiCardStyle: ViewStyle = {
  ...glassCardStyle,
  ...aiGlow.ambient,
};

// Activity type icons
const ACTIVITY_ICONS: Record<string, keyof typeof Ionicons.glyphMap> = {
  run: 'walk-outline',
  bike: 'bicycle-outline',
  swim: 'water-outline',
  strength: 'barbell-outline',
};

export function ActivityDetailScreen({ navigation, route }: ActivityDetailScreenProps) {
  const insets = useSafeAreaInsets();

  // Mock activity data - would come from API in production
  const [activity] = useState<ActivityData>({
    id: route.params?.activityId || '1',
    type: 'run',
    title: 'Morning Easy Run',
    date: 'January 26, 2026',
    duration: '45:12',
    distance: '8.2 km',
    pace: '5:31 /km',
    elevation: '124 m',
    tss: 145,
    calories: 512,
    avgHeartRate: 142,
    maxHeartRate: 168,
    weather: {
      temp: 18,
      condition: 'Partly Cloudy',
    },
    splits: [
      { km: 1, pace: '5:45', heartRate: 138 },
      { km: 2, pace: '5:32', heartRate: 142 },
      { km: 3, pace: '5:28', heartRate: 145 },
      { km: 4, pace: '5:25', heartRate: 148 },
      { km: 5, pace: '5:30', heartRate: 146 },
      { km: 6, pace: '5:35', heartRate: 144 },
      { km: 7, pace: '5:28', heartRate: 147 },
      { km: 8, pace: '5:22', heartRate: 152 },
    ],
  });

  // Heart rate zone percentages (mock data)
  const heartRateZones = [
    { zone: 'Z1', percent: 10, color: '#4ADE80' },
    { zone: 'Z2', percent: 35, color: '#22D3EE' },
    { zone: 'Z3', percent: 30, color: '#8B5CF6' },
    { zone: 'Z4', percent: 20, color: '#818CF8' },
    { zone: 'Z5', percent: 5, color: '#EC4899' },
  ];

  const handleShare = async () => {
    try {
      await Share.share({
        message: `${activity.title}\n${activity.distance} in ${activity.duration}\nPace: ${activity.pace}`,
      });
    } catch (error) {
      console.error('Failed to share:', error);
    }
  };

  const handleAskPierre = () => {
    // Navigate to Chat with activity context
    navigation.navigate('ChatTab' as never);
  };

  return (
    <View className="flex-1 bg-pierre-dark">
      <ScrollView
        className="flex-1"
        contentContainerStyle={{
          paddingBottom: 100,
        }}
        showsVerticalScrollIndicator={false}
      >
        {/* Header with back arrow, activity icon, share button */}
        <View
          className="flex-row items-center justify-between px-4 py-3"
          style={{ paddingTop: insets.top + spacing.sm }}
        >
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center"
            onPress={() => navigation.goBack()}
          >
            <Ionicons name="arrow-back" size={24} color={colors.text.primary} />
          </TouchableOpacity>
          <View className="flex-row items-center">
            <View className="w-10 h-10 rounded-xl bg-pierre-slate items-center justify-center mr-2">
              <Ionicons
                name={ACTIVITY_ICONS[activity.type] || 'fitness-outline'}
                size={22}
                color={colors.pierre.cyan}
              />
            </View>
          </View>
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center"
            onPress={handleShare}
          >
            <Feather name="share" size={22} color={colors.text.primary} />
          </TouchableOpacity>
        </View>

        {/* Hero Section with Map Placeholder */}
        <View className="px-4 mb-6">
          <View
            className="h-48 rounded-2xl overflow-hidden mb-4"
            style={{
              backgroundColor: colors.pierre.slate,
              borderWidth: 1,
              borderColor: 'rgba(255, 255, 255, 0.1)',
            }}
          >
            {/* Map placeholder with violet route line indication */}
            <LinearGradient
              colors={['rgba(139, 92, 246, 0.1)', 'rgba(30, 30, 46, 0.9)']}
              className="flex-1 items-center justify-center"
            >
              <View className="items-center">
                <Feather name="map" size={40} color={colors.pierre.violet} />
                <Text className="text-sm text-zinc-500 mt-2">Route Map</Text>
              </View>
            </LinearGradient>
          </View>

          {/* Activity Title and Date */}
          <Text className="text-2xl font-bold text-white mb-1">{activity.title}</Text>
          <Text className="text-base text-zinc-500">{activity.date}</Text>
        </View>

        {/* Stats Grid - 4 cards with cyan accents */}
        <View className="px-4 mb-6">
          <View className="flex-row flex-wrap gap-3">
            {/* Duration */}
            <View style={glassCardStyle} className="flex-1 min-w-[45%] p-4">
              <Text className="text-xs text-zinc-500 mb-1">Duration</Text>
              <Text className="text-xl font-bold text-pierre-cyan">{activity.duration}</Text>
            </View>

            {/* Distance */}
            <View style={glassCardStyle} className="flex-1 min-w-[45%] p-4">
              <Text className="text-xs text-zinc-500 mb-1">Distance</Text>
              <Text className="text-xl font-bold text-pierre-cyan">{activity.distance}</Text>
            </View>

            {/* Pace */}
            <View style={glassCardStyle} className="flex-1 min-w-[45%] p-4">
              <Text className="text-xs text-zinc-500 mb-1">Avg Pace</Text>
              <Text className="text-xl font-bold text-pierre-cyan">{activity.pace}</Text>
            </View>

            {/* Elevation */}
            <View style={glassCardStyle} className="flex-1 min-w-[45%] p-4">
              <Text className="text-xs text-zinc-500 mb-1">Elevation</Text>
              <Text className="text-xl font-bold text-pierre-cyan">{activity.elevation}</Text>
            </View>
          </View>
        </View>

        {/* Performance Section */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-4">Performance</Text>

          <View className="flex-row gap-3 mb-4">
            {/* Circular TSS Badge */}
            <View style={glassCardStyle} className="items-center justify-center p-4 w-24">
              <View className="w-16 h-16 rounded-full border-4 border-pierre-violet items-center justify-center mb-2">
                <Text className="text-xl font-bold text-white">{activity.tss}</Text>
              </View>
              <Text className="text-xs text-zinc-500">TSS</Text>
            </View>

            {/* Heart Rate Card */}
            <View style={glassCardStyle} className="flex-1 p-4">
              <Text className="text-xs text-zinc-500 mb-2">Heart Rate</Text>
              <View className="flex-row justify-between">
                <View>
                  <Text className="text-lg font-bold text-white">{activity.avgHeartRate}</Text>
                  <Text className="text-xs text-zinc-500">avg bpm</Text>
                </View>
                <View>
                  <Text className="text-lg font-bold text-pierre-red">{activity.maxHeartRate}</Text>
                  <Text className="text-xs text-zinc-500">max bpm</Text>
                </View>
              </View>
            </View>
          </View>

          {/* Heart Rate Zones Chart */}
          <View style={glassCardStyle} className="p-4">
            <Text className="text-xs text-zinc-500 mb-3">Heart Rate Zones</Text>
            <View className="flex-row h-20 gap-1">
              {heartRateZones.map((zone) => (
                <View key={zone.zone} className="flex-1 items-center justify-end">
                  <View
                    className="w-full rounded-t"
                    style={{
                      height: `${zone.percent}%`,
                      backgroundColor: zone.color,
                      minHeight: 4,
                    }}
                  />
                  <Text className="text-xs text-zinc-500 mt-1">{zone.zone}</Text>
                </View>
              ))}
            </View>
          </View>
        </View>

        {/* AI Insights Card */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-4">AI Insights</Text>
          <View style={aiCardStyle} className="p-4">
            <View className="flex-row items-start">
              {/* Pierre Avatar */}
              <View className="w-10 h-10 rounded-full bg-pierre-violet/20 items-center justify-center mr-3">
                <Feather name="zap" size={20} color={colors.pierre.violet} />
              </View>
              <View className="flex-1">
                <Text className="text-sm text-white leading-5 mb-3">
                  Great effort today! Your pace was consistent throughout, with a strong finish in the last kilometer.
                  Your heart rate stayed in the aerobic zone for most of the run, which is perfect for building endurance.
                </Text>
                <TouchableOpacity
                  className="self-start px-4 py-2 rounded-full"
                  style={{
                    backgroundColor: colors.pierre.violet,
                    shadowColor: colors.pierre.violet,
                    shadowOffset: { width: 0, height: 0 },
                    shadowOpacity: 0.4,
                    shadowRadius: 8,
                    elevation: 4,
                  }}
                  onPress={handleAskPierre}
                >
                  <Text className="text-sm font-semibold text-white">Ask Pierre</Text>
                </TouchableOpacity>
              </View>
            </View>
          </View>
        </View>

        {/* Splits Table */}
        <View className="px-4 mb-6">
          <Text className="text-lg font-semibold text-white mb-4">Splits</Text>
          <View style={glassCardStyle} className="overflow-hidden">
            {/* Header */}
            <View className="flex-row px-4 py-3 border-b border-white/10">
              <Text className="flex-1 text-xs font-semibold text-zinc-500">KM</Text>
              <Text className="flex-1 text-xs font-semibold text-zinc-500 text-center">PACE</Text>
              <Text className="flex-1 text-xs font-semibold text-zinc-500 text-right">HR</Text>
            </View>
            {/* Rows */}
            {activity.splits.map((split, index) => (
              <View
                key={split.km}
                className={`flex-row px-4 py-3 ${index < activity.splits.length - 1 ? 'border-b border-white/5' : ''}`}
              >
                <Text className="flex-1 text-sm text-white">{split.km}</Text>
                <Text className="flex-1 text-sm text-pierre-cyan text-center">{split.pace}</Text>
                <Text className="flex-1 text-sm text-white text-right">{split.heartRate}</Text>
              </View>
            ))}
          </View>
        </View>

        {/* Weather Summary */}
        <View className="px-4 mb-6">
          <View style={glassCardStyle} className="flex-row items-center p-4">
            <Feather name="cloud" size={24} color={colors.pierre.cyan} />
            <View className="ml-3">
              <Text className="text-lg font-semibold text-white">{activity.weather.temp}°C</Text>
              <Text className="text-sm text-zinc-500">{activity.weather.condition}</Text>
            </View>
          </View>
        </View>
      </ScrollView>
    </View>
  );
}
