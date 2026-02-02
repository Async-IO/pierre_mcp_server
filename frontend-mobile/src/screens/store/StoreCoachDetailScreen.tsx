// ABOUTME: Coach Store detail screen showing full coach info with install/uninstall actions
// ABOUTME: Displays system prompt preview, sample prompts, tags, and install count

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
import { LinearGradient } from 'expo-linear-gradient';
import { colors, glassCard, buttonGlow } from '../../constants/theme';
import { Feather } from '@expo/vector-icons';
import { storeApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { StoreCoachDetail } from '../../types';
import type { CoachesStackParamList } from '../../navigation/MainTabs';

interface StoreCoachDetailScreenProps {
  navigation: NativeStackNavigationProp<CoachesStackParamList>;
  route: RouteProp<CoachesStackParamList, 'StoreCoachDetail'>;
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

export function StoreCoachDetailScreen({ navigation, route }: StoreCoachDetailScreenProps) {
  const { coachId } = route.params;
  const { isAuthenticated } = useAuth();
  const [coach, setCoach] = useState<StoreCoachDetail | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isInstalling, setIsInstalling] = useState(false);
  const [isInstalled, setIsInstalled] = useState(false);

  const loadCoachDetail = useCallback(async () => {
    if (!isAuthenticated || !coachId) return;

    try {
      setIsLoading(true);
      const response = await storeApi.getStoreCoach(coachId);
      setCoach(response);

      // Check if already installed
      const installations = await storeApi.getInstalledCoaches();
      const installed = installations.coaches.some(
        (c: { id: string }) => c.id === coachId
      );
      setIsInstalled(installed);
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

  const handleInstall = async () => {
    if (!coach) return;

    try {
      setIsInstalling(true);
      await storeApi.installStoreCoach(coach.id);
      setIsInstalled(true);
      Alert.alert(
        'Installed!',
        `"${coach.title}" has been added to your coaches.`,
        [
          { text: 'View My Coaches', onPress: () => navigation.navigate('CoachesMain') },
          { text: 'Stay Here', style: 'cancel' },
        ]
      );
    } catch (error) {
      console.error('Failed to install coach:', error);
      Alert.alert('Error', 'Failed to install coach. Please try again.');
    } finally {
      setIsInstalling(false);
    }
  };

  const handleUninstall = async () => {
    if (!coach) return;

    Alert.alert(
      'Uninstall Coach?',
      `Remove "${coach.title}" from your coaches? You can always reinstall it later.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Uninstall',
          style: 'destructive',
          onPress: async () => {
            try {
              setIsInstalling(true);
              await storeApi.uninstallStoreCoach(coach.id);
              setIsInstalled(false);
              Alert.alert('Uninstalled', 'Coach has been removed from your library.');
            } catch (error) {
              console.error('Failed to uninstall coach:', error);
              Alert.alert('Error', 'Failed to uninstall coach. Please try again.');
            } finally {
              setIsInstalling(false);
            }
          },
        },
      ]
    );
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
        <View className="flex-1 justify-center items-center p-6">
          <Text className="text-lg text-text-secondary mb-3">Coach not found</Text>
          <TouchableOpacity
            className="px-5 py-2 bg-primary-500 rounded-lg"
            onPress={() => navigation.navigate('Store')}
          >
            <Text className="text-text-primary text-base font-medium">Go Back</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  const categoryColor = COACH_CATEGORY_COLORS[coach.category];

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="store-coach-detail-screen">
      {/* Header */}
      <View className="flex-row items-center px-3 py-2 border-b border-border-subtle">
        <TouchableOpacity
          testID="back-button"
          className="w-10 h-10 items-center justify-center"
          onPress={() => navigation.navigate('Store')}
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-semibold text-text-primary text-center mx-2" numberOfLines={1}>
          {coach.title}
        </Text>
        <View className="w-10" />
      </View>

      <ScrollView className="flex-1" showsVerticalScrollIndicator={false}>
        {/* Category & Stats */}
        <View className="flex-row justify-between items-center px-4 pt-4 pb-2">
          <View
            testID="category-badge"
            className="px-3 py-1 rounded-full"
            style={{ backgroundColor: categoryColor + '20' }}
          >
            <Text className="text-sm font-semibold capitalize" style={{ color: categoryColor }}>
              {coach.category}
            </Text>
          </View>
          <Text testID="install-count" className="text-sm text-text-secondary">
            {coach.install_count} {coach.install_count === 1 ? 'install' : 'installs'}
          </Text>
        </View>

        {/* Title */}
        <Text testID="coach-title" className="text-2xl font-bold text-text-primary px-4 mb-2">{coach.title}</Text>

        {/* Description */}
        {coach.description && (
          <Text className="text-base text-text-secondary px-4 leading-[22px] mb-3">{coach.description}</Text>
        )}

        {/* Tags */}
        {coach.tags.length > 0 && (
          <View className="px-4 py-3">
            <Text className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-2">Tags</Text>
            <View className="flex-row flex-wrap">
              {coach.tags.map((tag, tagIndex) => (
                <View
                  key={tagIndex}
                  className="px-3 py-1.5 rounded-full mr-2 mb-2"
                  style={{
                    backgroundColor: `${categoryColor}15`,
                    borderWidth: 1,
                    borderColor: `${categoryColor}30`,
                  }}
                >
                  <Text className="text-sm font-medium" style={{ color: categoryColor }}>{tag}</Text>
                </View>
              ))}
            </View>
          </View>
        )}

        {/* Sample Prompts */}
        {coach.sample_prompts.length > 0 && (
          <View className="px-4 py-3">
            <Text className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-2">Sample Prompts</Text>
            {coach.sample_prompts.map((prompt, promptIndex) => (
              <View
                key={promptIndex}
                className="p-3 rounded-xl mb-2 overflow-hidden"
                style={{
                  ...glassCard,
                  borderRadius: 12,
                  borderColor: 'rgba(139, 92, 246, 0.15)',
                }}
              >
                <Text className="text-base text-text-primary leading-5">{prompt}</Text>
              </View>
            ))}
          </View>
        )}

        {/* System Prompt Preview */}
        <View className="px-4 py-3">
          <Text className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-2">System Prompt</Text>
          <View
            className="rounded-xl overflow-hidden"
            style={{
              ...glassCard,
              borderRadius: 12,
              borderColor: `${categoryColor}30`,
            }}
          >
            <LinearGradient
              colors={[categoryColor, `${categoryColor}40`] as [string, string]}
              start={{ x: 0, y: 0 }}
              end={{ x: 1, y: 0 }}
              style={{ height: 2, width: '100%' }}
            />
            <View className="p-3">
              <Text className="text-sm text-text-secondary leading-5 font-mono" numberOfLines={10}>
                {coach.system_prompt}
              </Text>
              {coach.system_prompt.length > 500 && (
                <Text className="text-xs text-text-tertiary italic mt-2">
                  ...and more ({coach.token_count} tokens)
                </Text>
              )}
            </View>
          </View>
        </View>

        {/* Metadata */}
        <View className="px-4 py-3">
          <Text className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-2">Details</Text>
          <View
            className="rounded-xl overflow-hidden"
            style={{
              ...glassCard,
              borderRadius: 12,
              borderColor: 'rgba(139, 92, 246, 0.15)',
            }}
          >
            <View className="flex-row justify-between items-center px-4 py-3 border-b border-border-subtle">
              <Text className="text-sm text-text-secondary">Token Count</Text>
              <Text className="text-sm text-text-primary font-medium">{coach.token_count}</Text>
            </View>
            {coach.published_at && (
              <View className="flex-row justify-between items-center px-4 py-3">
                <Text className="text-sm text-text-secondary">Published</Text>
                <Text className="text-sm text-text-primary font-medium">
                  {new Date(coach.published_at).toLocaleDateString()}
                </Text>
              </View>
            )}
          </View>
        </View>

        {/* Bottom Spacer for Install Button */}
        <View style={{ height: 100 }} />
      </ScrollView>

      {/* Install/Uninstall Button - Fixed at bottom */}
      <View className="absolute bottom-0 left-0 right-0 bg-background-primary border-t border-border-subtle p-3 pb-5">
        {isInstalled ? (
          <TouchableOpacity
            className="flex-row items-center justify-center py-3.5 rounded-xl"
            style={{
              ...glassCard,
              borderRadius: 12,
              borderColor: 'rgba(139, 92, 246, 0.2)',
            }}
            onPress={handleUninstall}
            disabled={isInstalling}
          >
            {isInstalling ? (
              <ActivityIndicator size="small" color={colors.text.primary} />
            ) : (
              <>
                <Feather name="check" size={18} color={colors.pierre.activity} />
                <Text className="text-text-primary text-base font-medium ml-2">Installed</Text>
              </>
            )}
          </TouchableOpacity>
        ) : (
          <TouchableOpacity
            className="flex-row items-center justify-center py-3.5 rounded-xl"
            style={{
              backgroundColor: colors.pierre.violet,
              ...buttonGlow,
            }}
            onPress={handleInstall}
            disabled={isInstalling}
          >
            {isInstalling ? (
              <ActivityIndicator size="small" color="#FFFFFF" />
            ) : (
              <>
                <Feather name="download" size={18} color="#FFFFFF" />
                <Text className="text-white text-base font-semibold ml-2">Install Coach</Text>
              </>
            )}
          </TouchableOpacity>
        )}
      </View>
    </SafeAreaView>
  );
}

