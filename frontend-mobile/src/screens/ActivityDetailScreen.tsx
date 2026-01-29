// ABOUTME: Activity Detail screen with coach-insight-first approach
// ABOUTME: Shows activity context and AI insights, not raw metrics - matches web philosophy

import React from 'react';
import {
  View,
  Text,
  ScrollView,
  TouchableOpacity,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import { LinearGradient } from 'expo-linear-gradient';
import { Feather, Ionicons } from '@expo/vector-icons';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { colors, spacing, aiGlow } from '../constants/theme';
import type { SocialStackParamList } from '../navigation/MainTabs';

type SocialNavigationProp = NativeStackNavigationProp<SocialStackParamList>;

interface ActivityDetailScreenProps {
  navigation: NativeStackNavigationProp<SocialStackParamList, 'ActivityDetail'>;
  route: {
    params: {
      activityId: string;
      activityTitle?: string;
      activityType?: string;
      activityDate?: string;
      insightContent?: string;
    };
  };
}

// Activity type icons
const ACTIVITY_ICONS: Record<string, keyof typeof Ionicons.glyphMap> = {
  run: 'walk-outline',
  bike: 'bicycle-outline',
  swim: 'water-outline',
  strength: 'barbell-outline',
  default: 'fitness-outline',
};

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

export function ActivityDetailScreen({ navigation, route }: ActivityDetailScreenProps) {
  const insets = useSafeAreaInsets();
  const socialNavigation = useNavigation<SocialNavigationProp>();

  const {
    activityId,
    activityTitle = 'Activity',
    activityType = 'run',
    activityDate,
    insightContent,
  } = route.params;

  const iconName = ACTIVITY_ICONS[activityType.toLowerCase()] || ACTIVITY_ICONS.default;

  const handleAskPierre = () => {
    navigation.navigate('ChatTab' as never);
  };

  const handleShareWithFriends = () => {
    socialNavigation.navigate('ShareInsight', { activityId });
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
        {/* Header with back button */}
        <View
          className="flex-row items-center px-4 py-3"
          style={{ paddingTop: insets.top + spacing.sm }}
        >
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center"
            onPress={() => navigation.goBack()}
            testID="back-button"
          >
            <Ionicons name="arrow-back" size={24} color={colors.text.primary} />
          </TouchableOpacity>
          <Text className="flex-1 text-lg font-semibold text-white text-center">
            Activity Details
          </Text>
          <View className="w-10" />
        </View>

        {/* Activity Header Card */}
        <View className="px-4 mb-6">
          <View style={glassCardStyle} className="p-4">
            <View className="flex-row items-start">
              {/* Activity Icon */}
              <View className="w-12 h-12 rounded-xl bg-pierre-violet/20 items-center justify-center mr-4">
                <Ionicons
                  name={iconName}
                  size={24}
                  color={colors.pierre.violet}
                />
              </View>
              {/* Activity Info */}
              <View className="flex-1">
                <Text className="text-lg font-semibold text-white mb-1">
                  {activityTitle}
                </Text>
                {activityDate && (
                  <Text className="text-sm text-zinc-400 mb-2">{activityDate}</Text>
                )}
                <View className="self-start px-2 py-1 rounded-full bg-pierre-cyan/20">
                  <Text className="text-xs font-medium text-pierre-cyan capitalize">
                    {activityType}
                  </Text>
                </View>
              </View>
            </View>
          </View>
        </View>

        {/* AI Insight Card - Primary Content */}
        {insightContent && (
          <View className="px-4 mb-6">
            <View style={aiCardStyle} className="p-4">
              <View className="flex-row items-start mb-4">
                {/* Pierre Avatar */}
                <View className="w-10 h-10 rounded-full bg-pierre-violet/20 items-center justify-center mr-3">
                  <Feather name="zap" size={20} color={colors.pierre.violet} />
                </View>
                <View className="flex-1">
                  <Text className="text-xs font-medium text-pierre-violet mb-1">
                    AI Insight
                  </Text>
                  <Text className="text-sm text-white leading-5">
                    {insightContent}
                  </Text>
                </View>
              </View>

              {/* Ask Pierre Button */}
              <TouchableOpacity
                className="flex-row items-center justify-center px-4 py-3 rounded-xl"
                style={{
                  backgroundColor: colors.pierre.violet,
                  shadowColor: colors.pierre.violet,
                  shadowOffset: { width: 0, height: 0 },
                  shadowOpacity: 0.4,
                  shadowRadius: 8,
                  elevation: 4,
                }}
                onPress={handleAskPierre}
                testID="ask-pierre-button"
              >
                <Feather name="message-circle" size={18} color="white" />
                <Text className="text-sm font-semibold text-white ml-2">
                  Ask Pierre for More
                </Text>
              </TouchableOpacity>
            </View>
          </View>
        )}

        {/* Share CTA Section */}
        <View className="px-4 mb-6">
          <LinearGradient
            colors={['rgba(139, 92, 246, 0.15)', 'rgba(34, 211, 238, 0.1)']}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 0 }}
            style={[glassCardStyle, { overflow: 'hidden' }]}
          >
            <View className="p-5 items-center">
              <Text className="text-base font-medium text-white mb-2 text-center">
                Share this activity with friends
              </Text>
              <Text className="text-sm text-zinc-400 mb-4 text-center leading-5">
                Let Pierre create a coach-generated insight to share with your training partners.
                Your private data stays private - only the insight is shared.
              </Text>
              <TouchableOpacity
                className="flex-row items-center px-6 py-3 rounded-full"
                style={{
                  backgroundColor: colors.pierre.violet,
                  shadowColor: colors.pierre.violet,
                  shadowOffset: { width: 0, height: 0 },
                  shadowOpacity: 0.4,
                  shadowRadius: 8,
                  elevation: 4,
                }}
                onPress={handleShareWithFriends}
                testID="share-with-friends-button"
              >
                <Feather name="users" size={18} color="white" />
                <Text className="text-sm font-semibold text-white ml-2">
                  Share with Friends
                </Text>
              </TouchableOpacity>
            </View>
          </LinearGradient>
        </View>

        {/* No insight fallback */}
        {!insightContent && (
          <View className="px-4 mb-6">
            <View style={glassCardStyle} className="p-5 items-center">
              <View className="w-12 h-12 rounded-full bg-pierre-violet/20 items-center justify-center mb-3">
                <Feather name="zap" size={24} color={colors.pierre.violet} />
              </View>
              <Text className="text-base font-medium text-white mb-2 text-center">
                Get AI Insights
              </Text>
              <Text className="text-sm text-zinc-400 mb-4 text-center leading-5">
                Ask Pierre to analyze this activity and provide personalized coaching insights.
              </Text>
              <TouchableOpacity
                className="flex-row items-center px-6 py-3 rounded-full"
                style={{
                  backgroundColor: colors.pierre.violet,
                  shadowColor: colors.pierre.violet,
                  shadowOffset: { width: 0, height: 0 },
                  shadowOpacity: 0.4,
                  shadowRadius: 8,
                  elevation: 4,
                }}
                onPress={handleAskPierre}
                testID="get-insights-button"
              >
                <Feather name="message-circle" size={18} color="white" />
                <Text className="text-sm font-semibold text-white ml-2">
                  Ask Pierre
                </Text>
              </TouchableOpacity>
            </View>
          </View>
        )}
      </ScrollView>
    </View>
  );
}
