// ABOUTME: Screen for sharing coach-generated insights with friends
// ABOUTME: Displays suggestions from coach, allows editing, and publishes to social feed

import React, { useState, useEffect, useCallback } from 'react';
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
  RefreshControl,
} from 'react-native';
import { useNavigation, useRoute, type RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { colors } from '../../constants/theme';
import { socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { SuggestionCard } from '../../components/social';
import type { InsightSuggestion, ShareVisibility } from '../../types';
import type { SocialStackParamList } from '../../navigation/MainTabs';

type NavigationProp = NativeStackNavigationProp<SocialStackParamList>;
type ShareInsightRouteProp = RouteProp<SocialStackParamList, 'ShareInsight'>;

// States for the sharing flow
type ShareFlowState = 'loading' | 'suggestions' | 'editing' | 'submitting' | 'error';

export function ShareInsightScreen() {
  const navigation = useNavigation<NavigationProp>();
  const route = useRoute<ShareInsightRouteProp>();
  const { isAuthenticated } = useAuth();

  // Get optional activityId from route params
  const activityId = route.params?.activityId;

  // Flow state
  const [flowState, setFlowState] = useState<ShareFlowState>('loading');
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  // Suggestions from coach
  const [suggestions, setSuggestions] = useState<InsightSuggestion[]>([]);

  // Selected suggestion for editing
  const [selectedSuggestion, setSelectedSuggestion] = useState<InsightSuggestion | null>(null);
  const [editedContent, setEditedContent] = useState('');
  const [visibility, setVisibility] = useState<ShareVisibility>('friends_only');

  // Fetch suggestions on mount (optionally filtered by activityId)
  const fetchSuggestions = useCallback(async () => {
    try {
      setError(null);
      const response = await socialApi.getInsightSuggestions({
        limit: 10,
        activity_id: activityId,
      });
      setSuggestions(response.suggestions);
      setFlowState(response.suggestions.length > 0 ? 'suggestions' : 'error');
      if (response.suggestions.length === 0) {
        setError('No suggestions available. Complete some activities to unlock sharing!');
      }
    } catch (err) {
      console.error('Failed to fetch suggestions:', err);
      setError('Failed to load coach suggestions. Please try again.');
      setFlowState('error');
    }
  }, [activityId]);

  useEffect(() => {
    if (isAuthenticated) {
      fetchSuggestions();
    }
  }, [isAuthenticated, fetchSuggestions]);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    await fetchSuggestions();
    setRefreshing(false);
  }, [fetchSuggestions]);

  // Handle selecting a suggestion to edit
  const handleSelectSuggestion = useCallback((suggestion: InsightSuggestion) => {
    setSelectedSuggestion(suggestion);
    setEditedContent(suggestion.suggested_content);
    setFlowState('editing');
  }, []);

  // Handle going back from editing to suggestions
  const handleBackToSuggestions = useCallback(() => {
    setSelectedSuggestion(null);
    setEditedContent('');
    setFlowState('suggestions');
  }, []);

  // Handle sharing the insight
  const handleShare = async () => {
    if (!selectedSuggestion || !editedContent.trim()) return;

    try {
      setFlowState('submitting');

      await socialApi.shareFromActivity({
        activity_id: selectedSuggestion.source_activity_id,
        insight_type: selectedSuggestion.insight_type,
        content: editedContent.trim(),
        visibility,
      });

      // Navigate back to feed on success
      navigation.navigate('SocialMain');
    } catch (err) {
      console.error('Failed to share insight:', err);
      setError('Failed to share. Please try again.');
      setFlowState('editing');
    }
  };

  const canSubmit = editedContent.trim().length >= 10;

  // Loading state
  if (flowState === 'loading') {
    return (
      <SafeAreaView className="flex-1 bg-background-primary" testID="share-insight-screen">
        <View className="flex-1 items-center justify-center">
          <ActivityIndicator size="large" color={colors.pierre.violet} />
          <Text className="text-text-secondary mt-4">Loading coach suggestions...</Text>
        </View>
      </SafeAreaView>
    );
  }

  // Error state with no suggestions
  if (flowState === 'error' && suggestions.length === 0) {
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
          <Text className="flex-1 text-lg font-bold text-text-primary text-center">
            Share Insight
          </Text>
          <View className="w-10" />
        </View>

        <View className="flex-1 items-center justify-center px-6">
          <Feather name="inbox" size={64} color={colors.text.tertiary} />
          <Text className="text-text-primary text-lg font-semibold mt-4 text-center">
            No Insights Available
          </Text>
          <Text className="text-text-secondary text-center mt-2">
            {error || 'Complete some activities to unlock coach-mediated sharing!'}
          </Text>
          <TouchableOpacity
            className="mt-6 px-6 py-3 rounded-lg"
            style={{ backgroundColor: colors.pierre.violet }}
            onPress={handleRefresh}
          >
            <Text className="text-text-primary font-semibold">Refresh</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  // Editing state - show selected suggestion with edit capability
  if (flowState === 'editing' || flowState === 'submitting') {
    return (
      <SafeAreaView className="flex-1 bg-background-primary" testID="share-insight-screen">
        {/* Header */}
        <View className="flex-row items-center px-4 py-4 border-b border-border-subtle">
          <TouchableOpacity
            className="p-2"
            onPress={handleBackToSuggestions}
            disabled={flowState === 'submitting'}
            testID="back-button"
          >
            <Feather name="arrow-left" size={24} color={colors.text.primary} />
          </TouchableOpacity>
          <Text className="flex-1 text-lg font-bold text-text-primary text-center">
            Edit & Share
          </Text>
          <TouchableOpacity
            className={`px-4 py-2 rounded-md ${canSubmit ? '' : 'opacity-50'}`}
            style={{ backgroundColor: colors.pierre.violet }}
            onPress={handleShare}
            disabled={!canSubmit || flowState === 'submitting'}
            testID="share-button"
          >
            {flowState === 'submitting' ? (
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
            {/* Type indicator */}
            {selectedSuggestion && (
              <View className="mt-4 mb-2">
                <Text className="text-text-tertiary text-sm uppercase tracking-wide">
                  Coach Suggestion: {selectedSuggestion.insight_type.replace('_', ' ')}
                </Text>
              </View>
            )}

            {/* Editable content */}
            <Text className="text-text-secondary text-sm font-semibold mt-4 mb-2 uppercase tracking-wide">
              Content
            </Text>
            <TextInput
              testID="insight-content-input"
              className="bg-background-secondary rounded-md px-4 py-4 text-text-primary text-base min-h-[160px]"
              placeholder="Edit your coach insight... (min 10 characters)"
              placeholderTextColor={colors.text.tertiary}
              value={editedContent}
              onChangeText={setEditedContent}
              multiline
              numberOfLines={8}
              textAlignVertical="top"
              maxLength={500}
              editable={flowState !== 'submitting'}
            />
            <Text className="text-text-tertiary text-xs text-right mt-1">
              {editedContent.length}/500
            </Text>

            {/* Privacy note */}
            <View
              className="flex-row items-start rounded-md p-4 mt-4 gap-2"
              style={{ backgroundColor: colors.pierre.violet + '15' }}
            >
              <Feather name="shield" size={16} color={colors.pierre.violet} />
              <Text className="flex-1 text-text-secondary text-sm leading-5">
                Your insight is automatically sanitized. Private data like GPS coordinates, exact
                pace, and recovery scores are never shared.
              </Text>
            </View>

            {/* Visibility */}
            <Text className="text-text-secondary text-sm font-semibold mt-5 mb-2 uppercase tracking-wide">
              Visibility
            </Text>
            <View className="flex-row gap-4">
              <TouchableOpacity
                className={`flex-1 flex-row items-center justify-center py-4 rounded-md gap-2 ${visibility === 'friends_only' ? '' : 'bg-background-secondary'}`}
                style={
                  visibility === 'friends_only'
                    ? {
                        backgroundColor: colors.pierre.violet + '20',
                        borderWidth: 1,
                        borderColor: colors.pierre.violet,
                      }
                    : undefined
                }
                onPress={() => setVisibility('friends_only')}
                disabled={flowState === 'submitting'}
              >
                <Feather
                  name="users"
                  size={20}
                  color={visibility === 'friends_only' ? colors.pierre.violet : colors.text.tertiary}
                />
                <Text
                  className="text-base font-medium"
                  style={{
                    color:
                      visibility === 'friends_only' ? colors.pierre.violet : colors.text.tertiary,
                  }}
                >
                  Friends Only
                </Text>
              </TouchableOpacity>
              <TouchableOpacity
                className={`flex-1 flex-row items-center justify-center py-4 rounded-md gap-2 ${visibility === 'public' ? '' : 'bg-background-secondary'}`}
                style={
                  visibility === 'public'
                    ? {
                        backgroundColor: colors.pierre.violet + '20',
                        borderWidth: 1,
                        borderColor: colors.pierre.violet,
                      }
                    : undefined
                }
                onPress={() => setVisibility('public')}
                disabled={flowState === 'submitting'}
              >
                <Feather
                  name="globe"
                  size={20}
                  color={visibility === 'public' ? colors.pierre.violet : colors.text.tertiary}
                />
                <Text
                  className="text-base font-medium"
                  style={{
                    color: visibility === 'public' ? colors.pierre.violet : colors.text.tertiary,
                  }}
                >
                  Public
                </Text>
              </TouchableOpacity>
            </View>

            {/* Error message */}
            {error && (
              <View className="mt-4 p-3 rounded-md bg-red-500/10">
                <Text className="text-red-400 text-sm">{error}</Text>
              </View>
            )}

            <View className="h-8" />
          </ScrollView>
        </KeyboardAvoidingView>
      </SafeAreaView>
    );
  }

  // Suggestions state - show list of coach suggestions
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
        <Text className="flex-1 text-lg font-bold text-text-primary text-center">
          Coach Suggestions
        </Text>
        <View className="w-10" />
      </View>

      <ScrollView
        className="flex-1 px-4"
        showsVerticalScrollIndicator={false}
        refreshControl={
          <RefreshControl refreshing={refreshing} onRefresh={handleRefresh} tintColor={colors.pierre.violet} />
        }
      >
        {/* Intro text */}
        <View className="mt-4 mb-4">
          <Text className="text-text-primary text-base font-medium">
            Your coach noticed some achievements!
          </Text>
          <Text className="text-text-secondary text-sm mt-1">
            Select a suggestion to share with friends.
          </Text>
        </View>

        {/* Suggestions list */}
        {suggestions.map((suggestion, index) => (
          <SuggestionCard
            key={`suggestion-${index}-${suggestion.insight_type}`}
            suggestion={suggestion}
            onShare={handleSelectSuggestion}
            testID={`suggestion-card-${index}`}
          />
        ))}

        <View className="h-6" />
      </ScrollView>
    </SafeAreaView>
  );
}
