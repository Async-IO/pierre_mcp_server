// ABOUTME: Screen displaying the result of "Adapt to My Training" feature
// ABOUTME: Shows personalized coach insight adapted from friend's shared content

import React from 'react';
import {
  View,
  Text,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  type ViewStyle,
} from 'react-native';
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors, glassCard } from '../../constants/theme';
import { DragIndicator } from '../../components/ui';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;
type RouteProps = RouteProp<SocialStackParamList, 'AdaptedInsight'>;

// Glass card style with shadow (React Native shadows cannot use className)
const contentCardStyle: ViewStyle = {
  ...glassCard,
};

// Format date for display
const formatDate = (dateStr: string): string => {
  const date = new Date(dateStr);
  return date.toLocaleDateString('en-US', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
};

// Format adaptation context from JSON to readable text
const formatAdaptationContext = (contextJson: string): string => {
  try {
    const context = JSON.parse(contextJson);
    const lines: string[] = [];

    if (context.fitness_level && context.fitness_level !== 'Unknown') {
      lines.push(`Fitness Level: ${context.fitness_level}`);
    }
    if (context.training_phase) {
      lines.push(`Training Phase: ${context.training_phase}`);
    }
    if (context.primary_sport) {
      lines.push(`Sport: ${context.primary_sport}`);
    }
    if (context.weekly_volume_hours) {
      lines.push(`Weekly Volume: ${context.weekly_volume_hours} hours`);
    }
    if (context.additional_context) {
      lines.push(`Context: ${context.additional_context}`);
    }

    if (lines.length === 0) {
      return 'Adapted based on general training principles';
    }
    return lines.join('\n');
  } catch {
    // If not valid JSON, return as-is (might be plain text)
    return contextJson;
  }
};

export function AdaptedInsightScreen() {
  const navigation = useNavigation<NavigationProp>();
  const route = useRoute<RouteProps>();
  const { adaptedInsight } = route.params;

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="adapt-insight-screen">
      <DragIndicator testID="adapted-insight-drag-indicator" />
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <TouchableOpacity
          className="p-2"
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-bold text-text-primary text-center">Adapted for You</Text>
        <View className="w-10" />
      </View>

      <ScrollView className="flex-1 px-4" showsVerticalScrollIndicator={false}>
        {/* Success indicator */}
        <View
          className="flex-row items-center rounded-lg p-4 mt-4 gap-4"
          style={{ backgroundColor: colors.pierre.activity + '20' }}
        >
          <View
            className="w-10 h-10 rounded-full justify-center items-center"
            style={{ backgroundColor: colors.pierre.activity }}
          >
            <Feather name="check" size={24} color={colors.text.primary} />
          </View>
          <Text className="flex-1 text-text-primary text-base font-medium">
            Pierre has personalized this insight for your training
          </Text>
        </View>

        {/* Adapted content card */}
        <View className="mt-5 rounded-lg p-5" style={contentCardStyle}>
          <View className="flex-row items-center mb-4 gap-2">
            <Feather name="refresh-cw" size={20} color={colors.pierre.violet} />
            <Text className="text-base font-semibold" style={{ color: colors.pierre.violet }}>
              Your Personalized Version
            </Text>
          </View>
          <Text className="text-text-primary text-lg leading-7">{adaptedInsight.adapted_content}</Text>
          {adaptedInsight.adaptation_context && (
            <View className="mt-5 pt-4 border-t border-border-subtle">
              <Text className="text-text-tertiary text-sm font-medium mb-2">How this was adapted:</Text>
              <Text className="text-text-secondary text-base leading-6">
                {formatAdaptationContext(adaptedInsight.adaptation_context)}
              </Text>
            </View>
          )}
        </View>

        {/* Meta info */}
        <View className="mt-4 p-4 rounded-md bg-background-secondary">
          <View className="flex-row items-center gap-2">
            <Feather name="calendar" size={16} color={colors.text.tertiary} />
            <Text className="text-text-tertiary text-sm">
              Created {formatDate(adaptedInsight.created_at)}
            </Text>
          </View>
        </View>

        {/* How it works section */}
        <View className="mt-6 p-5 rounded-lg bg-background-secondary">
          <Text className="text-text-primary text-base font-bold mb-5">How "Adapt to My Training" Works</Text>
          <View className="flex-row items-start mb-4 gap-4">
            <View
              className="w-6 h-6 rounded-full justify-center items-center"
              style={{ backgroundColor: colors.pierre.violet + '30' }}
            >
              <Text className="text-sm font-bold" style={{ color: colors.pierre.violet }}>1</Text>
            </View>
            <Text className="flex-1 text-text-secondary text-base leading-6">
              Your friend shared a coach insight from their training
            </Text>
          </View>
          <View className="flex-row items-start mb-4 gap-4">
            <View
              className="w-6 h-6 rounded-full justify-center items-center"
              style={{ backgroundColor: colors.pierre.violet + '30' }}
            >
              <Text className="text-sm font-bold" style={{ color: colors.pierre.violet }}>2</Text>
            </View>
            <Text className="flex-1 text-text-secondary text-base leading-6">
              Pierre analyzed your recent activities and fitness profile
            </Text>
          </View>
          <View className="flex-row items-start gap-4">
            <View
              className="w-6 h-6 rounded-full justify-center items-center"
              style={{ backgroundColor: colors.pierre.violet + '30' }}
            >
              <Text className="text-sm font-bold" style={{ color: colors.pierre.violet }}>3</Text>
            </View>
            <Text className="flex-1 text-text-secondary text-base leading-6">
              The insight was personalized to match your current training phase
            </Text>
          </View>
        </View>

        {/* Actions */}
        <View className="mt-6 gap-4">
          <TouchableOpacity
            className="flex-row items-center justify-center py-4 rounded-lg gap-2"
            style={{ backgroundColor: colors.pierre.violet }}
            onPress={() => navigation.navigate('SocialMain')}
          >
            <Feather name="home" size={18} color={colors.text.primary} />
            <Text className="text-text-primary text-base font-semibold">Back to Feed</Text>
          </TouchableOpacity>
          <TouchableOpacity
            className="flex-row items-center justify-center py-4 rounded-lg bg-background-secondary gap-2"
            onPress={() => navigation.navigate('AdaptedInsights')}
          >
            <Feather name="list" size={18} color={colors.pierre.violet} />
            <Text className="text-base font-medium" style={{ color: colors.pierre.violet }}>
              View All Adapted Insights
            </Text>
          </TouchableOpacity>
        </View>

        <View className="h-6" />
      </ScrollView>
    </SafeAreaView>
  );
}
