// ABOUTME: Screen for sharing coach insights with friends
// ABOUTME: Allows composing and publishing sanitized insights to the social feed

import React, { useState } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
  TextInput,
  ActivityIndicator,
  KeyboardAvoidingView,
  Platform,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';
import { colors } from '../../constants/theme';

type FeatherIconName = ComponentProps<typeof Feather>['name'];
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { InsightType, ShareVisibility, TrainingPhase } from '../../types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;

// Insight type options
const INSIGHT_TYPES: Array<{ key: InsightType; label: string; icon: FeatherIconName; color: string }> = [
  { key: 'achievement', label: 'Achievement', icon: 'award', color: '#10B981' },
  { key: 'milestone', label: 'Milestone', icon: 'flag', color: '#F59E0B' },
  { key: 'training_tip', label: 'Training Tip', icon: 'zap', color: '#6366F1' },
  { key: 'recovery', label: 'Recovery', icon: 'moon', color: '#8B5CF6' },
  { key: 'motivation', label: 'Motivation', icon: 'sun', color: '#F97316' },
];

// Sport type options
const SPORT_TYPES = ['Running', 'Cycling', 'Swimming', 'Triathlon', 'Strength', 'Other'];

// Training phase options
const TRAINING_PHASES: Array<{ key: TrainingPhase; label: string }> = [
  { key: 'base', label: 'Base Building' },
  { key: 'build', label: 'Build Phase' },
  { key: 'peak', label: 'Peak/Race' },
  { key: 'recovery', label: 'Recovery' },
];

export function ShareInsightScreen() {
  const navigation = useNavigation<NavigationProp>();
  const { isAuthenticated } = useAuth();

  const [insightType, setInsightType] = useState<InsightType>('achievement');
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [sportType, setSportType] = useState<string | null>(null);
  const [trainingPhase, setTrainingPhase] = useState<TrainingPhase | null>(null);
  const [visibility, setVisibility] = useState<ShareVisibility>('friends_only');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const canSubmit = content.trim().length >= 10;

  const handleSubmit = async () => {
    if (!canSubmit || !isAuthenticated) return;

    try {
      setIsSubmitting(true);

      await apiService.shareInsight({
        insight_type: insightType,
        title: title.trim() || undefined,
        content: content.trim(),
        sport_type: sportType || undefined,
        training_phase: trainingPhase || undefined,
        visibility,
      });

      // Navigate back to feed
      navigation.navigate('SocialMain');
    } catch (error) {
      console.error('Failed to share insight:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="share-insight-screen">
      {/* Header */}
      <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
        <TouchableOpacity
          className="p-2"
          onPress={() => navigation.goBack()}
          testID="close-button"
        >
          <Feather name="x" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-bold text-text-primary text-center">Share Insight</Text>
        <TouchableOpacity
          className={`px-4 py-2 rounded-md ${canSubmit ? '' : 'opacity-50'}`}
          style={{ backgroundColor: colors.pierre.violet }}
          onPress={handleSubmit}
          disabled={!canSubmit || isSubmitting}
          testID="share-button"
        >
          {isSubmitting ? (
            <ActivityIndicator size="small" color={colors.text.primary} />
          ) : (
            <Text className="text-text-primary text-base font-semibold">Share</Text>
          )}
        </TouchableOpacity>
      </View>

      <KeyboardAvoidingView
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
        className="flex-1"
      >
        <ScrollView className="flex-1 px-4" showsVerticalScrollIndicator={false}>
          {/* Insight Type Selection */}
          <Text className="text-text-secondary text-sm font-semibold mt-5 mb-2 uppercase tracking-wide">Type</Text>
          <View className="flex-row flex-wrap gap-2" testID="insight-type-picker">
            {INSIGHT_TYPES.map((type) => (
              <TouchableOpacity
                key={type.key}
                testID={`insight-type-${type.key}`}
                className="flex-row items-center px-4 py-2 rounded-md gap-1"
                style={
                  insightType === type.key
                    ? { backgroundColor: type.color + '20', borderWidth: 1, borderColor: type.color }
                    : { backgroundColor: colors.background.secondary, borderWidth: 1, borderColor: 'transparent' }
                }
                onPress={() => setInsightType(type.key)}
              >
                <Feather
                  name={type.icon}
                  size={20}
                  color={insightType === type.key ? type.color : colors.text.tertiary}
                />
                <Text
                  className="text-sm font-medium"
                  style={{ color: insightType === type.key ? type.color : colors.text.tertiary }}
                >
                  {type.label}
                </Text>
              </TouchableOpacity>
            ))}
          </View>

          {/* Title (optional) */}
          <Text className="text-text-secondary text-sm font-semibold mt-5 mb-2 uppercase tracking-wide">Title (optional)</Text>
          <TextInput
            testID="insight-title-input"
            className="bg-background-secondary rounded-md px-4 py-4 text-text-primary text-base"
            placeholder="Give your insight a catchy title..."
            placeholderTextColor={colors.text.tertiary}
            value={title}
            onChangeText={setTitle}
            maxLength={100}
          />

          {/* Content */}
          <Text className="text-text-secondary text-sm font-semibold mt-5 mb-2 uppercase tracking-wide">Content *</Text>
          <TextInput
            testID="insight-content-input"
            className="bg-background-secondary rounded-md px-4 py-4 text-text-primary text-base min-h-[120px]"
            placeholder="Share your coach insight... (min 10 characters)"
            placeholderTextColor={colors.text.tertiary}
            value={content}
            onChangeText={setContent}
            multiline
            numberOfLines={6}
            textAlignVertical="top"
            maxLength={500}
          />
          <Text className="text-text-tertiary text-xs text-right mt-1">{content.length}/500</Text>

          {/* Privacy note */}
          <View
            className="flex-row items-start rounded-md p-4 mt-4 gap-2"
            style={{ backgroundColor: colors.pierre.violet + '15' }}
          >
            <Feather name="shield" size={16} color={colors.pierre.violet} />
            <Text className="flex-1 text-text-secondary text-sm leading-5">
              Your insight is automatically sanitized. Private data like GPS coordinates,
              exact pace, and recovery scores are never shared.
            </Text>
          </View>

          {/* Sport Type */}
          <Text className="text-text-secondary text-sm font-semibold mt-5 mb-2 uppercase tracking-wide">Sport (optional)</Text>
          <ScrollView horizontal showsHorizontalScrollIndicator={false} className="flex-row">
            {SPORT_TYPES.map((sport) => (
              <TouchableOpacity
                key={sport}
                className={`px-4 py-2 rounded-full mr-2 ${sportType === sport ? '' : 'bg-background-secondary'}`}
                style={sportType === sport ? { backgroundColor: colors.pierre.violet } : undefined}
                onPress={() => setSportType(sportType === sport ? null : sport)}
              >
                <Text
                  className={`text-sm font-medium ${sportType === sport ? 'text-text-primary' : 'text-text-tertiary'}`}
                >
                  {sport}
                </Text>
              </TouchableOpacity>
            ))}
          </ScrollView>

          {/* Training Phase */}
          <Text className="text-text-secondary text-sm font-semibold mt-5 mb-2 uppercase tracking-wide">Training Phase (optional)</Text>
          <ScrollView horizontal showsHorizontalScrollIndicator={false} className="flex-row">
            {TRAINING_PHASES.map((phase) => (
              <TouchableOpacity
                key={phase.key}
                className={`px-4 py-2 rounded-full mr-2 ${trainingPhase === phase.key ? '' : 'bg-background-secondary'}`}
                style={trainingPhase === phase.key ? { backgroundColor: colors.pierre.violet } : undefined}
                onPress={() => setTrainingPhase(trainingPhase === phase.key ? null : phase.key)}
              >
                <Text
                  className={`text-sm font-medium ${trainingPhase === phase.key ? 'text-text-primary' : 'text-text-tertiary'}`}
                >
                  {phase.label}
                </Text>
              </TouchableOpacity>
            ))}
          </ScrollView>

          {/* Visibility */}
          <Text className="text-text-secondary text-sm font-semibold mt-5 mb-2 uppercase tracking-wide">Visibility</Text>
          <View className="flex-row gap-4">
            <TouchableOpacity
              className={`flex-1 flex-row items-center justify-center py-4 rounded-md gap-2 ${visibility === 'friends_only' ? '' : 'bg-background-secondary'}`}
              style={
                visibility === 'friends_only'
                  ? { backgroundColor: colors.pierre.violet + '20', borderWidth: 1, borderColor: colors.pierre.violet }
                  : undefined
              }
              onPress={() => setVisibility('friends_only')}
            >
              <Feather
                name="users"
                size={20}
                color={visibility === 'friends_only' ? colors.pierre.violet : colors.text.tertiary}
              />
              <Text
                className="text-base font-medium"
                style={{ color: visibility === 'friends_only' ? colors.pierre.violet : colors.text.tertiary }}
              >
                Friends Only
              </Text>
            </TouchableOpacity>
            <TouchableOpacity
              className={`flex-1 flex-row items-center justify-center py-4 rounded-md gap-2 ${visibility === 'public' ? '' : 'bg-background-secondary'}`}
              style={
                visibility === 'public'
                  ? { backgroundColor: colors.pierre.violet + '20', borderWidth: 1, borderColor: colors.pierre.violet }
                  : undefined
              }
              onPress={() => setVisibility('public')}
            >
              <Feather
                name="globe"
                size={20}
                color={visibility === 'public' ? colors.pierre.violet : colors.text.tertiary}
              />
              <Text
                className="text-base font-medium"
                style={{ color: visibility === 'public' ? colors.pierre.violet : colors.text.tertiary }}
              >
                Public
              </Text>
            </TouchableOpacity>
          </View>

          <View className="h-6" />
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
