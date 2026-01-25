// ABOUTME: Coach editor screen for creating and editing AI coaches
// ABOUTME: Unified form with live token counting and category picker

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  ScrollView,
  TextInput,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
  ActionSheetIOS,
  Platform,
  KeyboardAvoidingView,
} from 'react-native';
import { useRoute, type RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { colors, spacing } from '../../constants/theme';
import { apiService } from '../../services/api';
import type { CoachCategory, CreateCoachRequest, UpdateCoachRequest } from '../../types';
import type { CoachesStackParamList } from '../../navigation/MainTabs';

interface CoachEditorScreenProps {
  navigation: NativeStackNavigationProp<CoachesStackParamList>;
}

// Category options with colors
const CATEGORY_OPTIONS: Array<{ key: CoachCategory; label: string; color: string }> = [
  { key: 'training', label: 'Training', color: '#10B981' },
  { key: 'nutrition', label: 'Nutrition', color: '#F59E0B' },
  { key: 'recovery', label: 'Recovery', color: '#6366F1' },
  { key: 'recipes', label: 'Recipes', color: '#F97316' },
  { key: 'mobility', label: 'Mobility', color: '#EC4899' },
  { key: 'custom', label: 'Custom', color: '#7C3AED' },
];

// Validation constants
const MAX_TITLE_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;
const MAX_SYSTEM_PROMPT_LENGTH = 4000;
const CONTEXT_WINDOW_SIZE = 128000;

export function CoachEditorScreen({ navigation }: CoachEditorScreenProps) {
  const route = useRoute<RouteProp<CoachesStackParamList, 'CoachEditor'>>();
  const coachId = route.params?.coachId;
  const isEditMode = Boolean(coachId);

  // Form state
  const [title, setTitle] = useState('');
  const [category, setCategory] = useState<CoachCategory>('custom');
  const [description, setDescription] = useState('');
  const [systemPrompt, setSystemPrompt] = useState('');

  // UI state
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Load coach data for edit mode
  useEffect(() => {
    if (isEditMode && coachId) {
      loadCoach(coachId);
    }
    // loadCoach intentionally omitted - including it would cause infinite loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isEditMode, coachId]);

  const loadCoach = async (id: string) => {
    try {
      setIsLoading(true);
      const coach = await apiService.getCoach(id);
      setTitle(coach.title);
      setCategory(coach.category);
      setDescription(coach.description || '');
      setSystemPrompt(coach.system_prompt);
    } catch (error) {
      console.error('Failed to load coach:', error);
      Alert.alert('Error', 'Failed to load coach data');
      navigation.goBack();
    } finally {
      setIsLoading(false);
    }
  };

  // Calculate token count (same formula as web)
  const tokenCount = Math.ceil(systemPrompt.length / 4);
  const contextPercentage = ((tokenCount / CONTEXT_WINDOW_SIZE) * 100).toFixed(1);

  // Validation
  const validate = useCallback((): boolean => {
    const newErrors: Record<string, string> = {};

    if (!title.trim()) {
      newErrors.title = 'Title is required';
    } else if (title.length > MAX_TITLE_LENGTH) {
      newErrors.title = `Title must be ${MAX_TITLE_LENGTH} characters or less`;
    }

    if (description.length > MAX_DESCRIPTION_LENGTH) {
      newErrors.description = `Description must be ${MAX_DESCRIPTION_LENGTH} characters or less`;
    }

    if (!systemPrompt.trim()) {
      newErrors.systemPrompt = 'System prompt is required';
    } else if (systemPrompt.length > MAX_SYSTEM_PROMPT_LENGTH) {
      newErrors.systemPrompt = `System prompt must be ${MAX_SYSTEM_PROMPT_LENGTH} characters or less`;
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [title, description, systemPrompt]);

  // Show category picker
  const showCategoryPicker = () => {
    if (Platform.OS === 'ios') {
      const options = [...CATEGORY_OPTIONS.map((c) => c.label), 'Cancel'];
      const cancelButtonIndex = options.length - 1;

      ActionSheetIOS.showActionSheetWithOptions(
        {
          options,
          cancelButtonIndex,
          title: 'Select Category',
        },
        (buttonIndex) => {
          if (buttonIndex !== cancelButtonIndex) {
            setCategory(CATEGORY_OPTIONS[buttonIndex].key);
          }
        }
      );
    } else {
      // Android fallback - simple alert with buttons
      Alert.alert(
        'Select Category',
        undefined,
        CATEGORY_OPTIONS.map((option) => ({
          text: option.label,
          onPress: () => setCategory(option.key),
        }))
      );
    }
  };

  // Save handler
  const handleSave = async () => {
    if (!validate()) return;

    try {
      setIsSaving(true);

      if (isEditMode && coachId) {
        const updateData: UpdateCoachRequest = {
          title: title.trim(),
          category,
          description: description.trim() || undefined,
          system_prompt: systemPrompt.trim(),
        };
        await apiService.updateCoach(coachId, updateData);
      } else {
        const createData: CreateCoachRequest = {
          title: title.trim(),
          category,
          description: description.trim() || undefined,
          system_prompt: systemPrompt.trim(),
        };
        await apiService.createCoach(createData);
      }

      navigation.goBack();
    } catch (error) {
      console.error('Failed to save coach:', error);
      Alert.alert('Error', `Failed to ${isEditMode ? 'update' : 'create'} coach`);
    } finally {
      setIsSaving(false);
    }
  };

  // Get current category info
  const currentCategory = CATEGORY_OPTIONS.find((c) => c.key === category);

  if (isLoading) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 items-center justify-center">
          <ActivityIndicator size="large" color={colors.primary[500]} />
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      <KeyboardAvoidingView
        className="flex-1"
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      >
        {/* Header */}
        <View className="flex-row items-center px-3 py-2 border-b border-border-subtle">
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center"
            onPress={() => navigation.goBack()}
          >
            <Text className="text-2xl text-text-primary">←</Text>
          </TouchableOpacity>
          <Text className="flex-1 text-lg font-semibold text-text-primary text-center">
            {isEditMode ? 'Edit Coach' : 'Create Coach'}
          </Text>
          <TouchableOpacity
            className={`px-3 py-1 bg-primary-500 rounded-md min-w-[60px] items-center ${isSaving ? 'opacity-60' : ''}`}
            onPress={handleSave}
            disabled={isSaving}
          >
            {isSaving ? (
              <ActivityIndicator size="small" color={colors.text.primary} />
            ) : (
              <Text className="text-base font-semibold text-text-primary">Save</Text>
            )}
          </TouchableOpacity>
        </View>

        <ScrollView
          className="flex-1"
          contentContainerStyle={{ padding: spacing.lg, paddingBottom: spacing.xxl }}
          keyboardShouldPersistTaps="handled"
        >
          {/* Title Field */}
          <View className="mb-5">
            <Text className="text-sm font-semibold text-text-secondary mb-1 uppercase tracking-wide">
              Title <Text className="text-error">*</Text>
            </Text>
            <TextInput
              className={`bg-background-secondary rounded-md p-3 text-base text-text-primary border ${errors.title ? 'border-error' : 'border-border-subtle'}`}
              value={title}
              onChangeText={setTitle}
              placeholder="Enter coach title"
              placeholderTextColor={colors.text.tertiary}
              maxLength={MAX_TITLE_LENGTH}
            />
            {errors.title && (
              <Text className="text-sm text-error mt-1">{errors.title}</Text>
            )}
            <Text className="text-xs text-text-tertiary text-right mt-1">
              {title.length}/{MAX_TITLE_LENGTH}
            </Text>
          </View>

          {/* Category Field */}
          <View className="mb-5">
            <Text className="text-sm font-semibold text-text-secondary mb-1 uppercase tracking-wide">Category</Text>
            <TouchableOpacity
              className="flex-row items-center justify-between bg-background-secondary rounded-md p-3 border border-border-subtle"
              onPress={showCategoryPicker}
            >
              <View className="flex-row items-center">
                <View
                  className="w-3 h-3 rounded-full mr-2"
                  style={{ backgroundColor: currentCategory?.color || colors.text.tertiary }}
                />
                <Text className="text-base text-text-primary">
                  {currentCategory?.label || 'Select category'}
                </Text>
              </View>
              <Text className="text-sm text-text-tertiary">▼</Text>
            </TouchableOpacity>
          </View>

          {/* Description Field */}
          <View className="mb-5">
            <Text className="text-sm font-semibold text-text-secondary mb-1 uppercase tracking-wide">Description</Text>
            <TextInput
              className={`bg-background-secondary rounded-md p-3 text-base text-text-primary border min-h-[80px] pt-3 ${errors.description ? 'border-error' : 'border-border-subtle'}`}
              value={description}
              onChangeText={setDescription}
              placeholder="Brief description of what this coach does"
              placeholderTextColor={colors.text.tertiary}
              multiline
              numberOfLines={3}
              maxLength={MAX_DESCRIPTION_LENGTH}
              textAlignVertical="top"
            />
            {errors.description && (
              <Text className="text-sm text-error mt-1">{errors.description}</Text>
            )}
            <Text className="text-xs text-text-tertiary text-right mt-1">
              {description.length}/{MAX_DESCRIPTION_LENGTH}
            </Text>
          </View>

          {/* System Prompt Field */}
          <View className="mb-5">
            <Text className="text-sm font-semibold text-text-secondary mb-1 uppercase tracking-wide">
              System Prompt <Text className="text-error">*</Text>
            </Text>
            <TextInput
              className={`bg-background-secondary rounded-md p-3 text-base text-text-primary border min-h-[200px] pt-3 ${errors.systemPrompt ? 'border-error' : 'border-border-subtle'}`}
              value={systemPrompt}
              onChangeText={setSystemPrompt}
              placeholder="You are Pierre, an expert coach who..."
              placeholderTextColor={colors.text.tertiary}
              multiline
              numberOfLines={10}
              maxLength={MAX_SYSTEM_PROMPT_LENGTH}
              textAlignVertical="top"
            />
            {errors.systemPrompt && (
              <Text className="text-sm text-error mt-1">{errors.systemPrompt}</Text>
            )}
            <Text className="text-xs text-text-tertiary text-right mt-1">
              {systemPrompt.length}/{MAX_SYSTEM_PROMPT_LENGTH}
            </Text>
          </View>

          {/* Token Count Display */}
          <View className="flex-row items-center justify-center py-3 border-t border-border-subtle mt-3">
            <Text className="text-lg mr-2">📊</Text>
            <Text className="text-base text-text-secondary">
              ~{tokenCount} tokens ({contextPercentage}% context)
            </Text>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
