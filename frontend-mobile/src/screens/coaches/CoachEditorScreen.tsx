// ABOUTME: Coach editor screen for creating and editing AI coaches
// ABOUTME: Single scrollable page with collapsible accordion sections for system prompt and tags

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
  Modal,
} from 'react-native';
import { useRoute, type RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Ionicons, Feather } from '@expo/vector-icons';
import * as Haptics from 'expo-haptics';
import { LinearGradient } from 'expo-linear-gradient';
import { colors, spacing, glassCard, gradients, buttonGlow } from '../../constants/theme';
import { coachesApi } from '../../services/api';
import { CollapsibleSection } from '../../components/ui';
import { CoachVersionHistory } from '../../components/coaches/CoachVersionHistory';
import type { CreateCoachRequest, UpdateCoachRequest } from '../../types';
import type { CoachesStackParamList } from '../../navigation/MainTabs';

interface CoachEditorScreenProps {
  navigation: NativeStackNavigationProp<CoachesStackParamList>;
}

// Category options with colors matching Stitch UX spec
const CATEGORY_OPTIONS: Array<{ key: string; label: string; color: string }> = [
  { key: 'training', label: 'Training', color: '#4ADE80' },
  { key: 'nutrition', label: 'Nutrition', color: '#F59E0B' },
  { key: 'recovery', label: 'Recovery', color: '#22D3EE' },
  { key: 'recipes', label: 'Recipes', color: '#F59E0B' },
  { key: 'mobility', label: 'Mobility', color: '#EC4899' },
  { key: 'custom', label: 'Custom', color: '#8B5CF6' },
];

// Validation constants
const MAX_TITLE_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;
const MAX_SYSTEM_PROMPT_LENGTH = 8000;
const CONTEXT_WINDOW_SIZE = 128000;

export function CoachEditorScreen({ navigation }: CoachEditorScreenProps) {
  const route = useRoute<RouteProp<CoachesStackParamList, 'CoachEditor'>>();
  const coachId = route.params?.coachId;
  const isEditMode = Boolean(coachId);

  // Form state
  const [title, setTitle] = useState('');
  const [category, setCategory] = useState<string>('custom');
  const [description, setDescription] = useState('');
  const [systemPrompt, setSystemPrompt] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [newTag, setNewTag] = useState('');
  const [forkedFrom, setForkedFrom] = useState<string | null>(null);

  // UI state
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [showVersionHistory, setShowVersionHistory] = useState(false);
  const [expandedTextArea, setExpandedTextArea] = useState(false);

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
      const coach = await coachesApi.get(id);
      setTitle(coach.title);
      setCategory(coach.category);
      setDescription(coach.description || '');
      setSystemPrompt(coach.system_prompt);
      setTags(coach.tags || []);
      setForkedFrom(coach.forked_from || null);
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
    const options = [...CATEGORY_OPTIONS.map((c) => c.label), 'Cancel'];
    const cancelButtonIndex = options.length - 1;

    if (Platform.OS === 'ios') {
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
      Alert.alert(
        'Select Category',
        undefined,
        [
          ...CATEGORY_OPTIONS.map((cat) => ({
            text: cat.label,
            onPress: () => setCategory(cat.key),
          })),
          { text: 'Cancel', style: 'cancel' as const },
        ]
      );
    }
  };

  // Tag management
  const addTag = () => {
    const trimmedTag = newTag.trim().toLowerCase();
    if (trimmedTag && !tags.includes(trimmedTag) && tags.length < 10) {
      setTags([...tags, trimmedTag]);
      setNewTag('');
    }
  };

  const removeTag = (tagToRemove: string) => {
    setTags(tags.filter((tag) => tag !== tagToRemove));
  };

  // Save handler
  const handleSave = async () => {
    if (!validate()) return;

    try {
      setIsSaving(true);
      Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Medium);

      if (isEditMode && coachId) {
        const updateData: UpdateCoachRequest = {
          title: title.trim(),
          category,
          description: description.trim() || undefined,
          system_prompt: systemPrompt.trim(),
          tags,
        };
        await coachesApi.update(coachId, updateData);
      } else {
        const createData: CreateCoachRequest = {
          title: title.trim(),
          category,
          description: description.trim() || undefined,
          system_prompt: systemPrompt.trim(),
          tags,
        };
        await coachesApi.create(createData);
      }

      Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
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
          <Text className="text-text-secondary mt-3 text-base">Loading coach...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="coach-editor-screen">
      <KeyboardAvoidingView
        className="flex-1"
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      >
        {/* Header */}
        <View className="flex-row items-center px-3 py-2 border-b border-border-subtle">
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center"
            onPress={() => navigation.goBack()}
            testID="back-button"
          >
            <Text className="text-2xl text-text-primary">{'\u2190'}</Text>
          </TouchableOpacity>
          <Text className="flex-1 text-lg font-semibold text-text-primary text-center">
            {isEditMode ? 'Edit Coach' : 'Create Coach'}
          </Text>
          {/* Version History Button (edit mode only) */}
          {isEditMode && (
            <TouchableOpacity
              className="w-10 h-10 items-center justify-center mr-1"
              onPress={() => setShowVersionHistory(true)}
              testID="version-history-button"
            >
              <Ionicons name="git-branch-outline" size={22} color={colors.text.primary} />
            </TouchableOpacity>
          )}
          <TouchableOpacity
            className={`px-4 py-1.5 rounded-xl min-w-[60px] items-center ${isSaving ? 'opacity-60' : ''}`}
            style={{
              backgroundColor: colors.pierre.violet,
              ...buttonGlow,
            }}
            onPress={handleSave}
            disabled={isSaving}
            testID="save-button"
          >
            {isSaving ? (
              <ActivityIndicator size="small" color="#fff" />
            ) : (
              <Text className="text-base font-semibold text-white">Save</Text>
            )}
          </TouchableOpacity>
        </View>

        <ScrollView
          className="flex-1"
          contentContainerStyle={{ padding: spacing.lg, paddingBottom: spacing.xxl }}
          keyboardShouldPersistTaps="handled"
        >
          {/* Forked From Banner */}
          {forkedFrom && (
            <View
              className="flex-row items-center mb-5 p-3 rounded-xl"
              style={{
                backgroundColor: 'rgba(139, 92, 246, 0.1)',
                borderWidth: 1,
                borderColor: 'rgba(139, 92, 246, 0.2)',
                borderRadius: 12,
              }}
              testID="forked-from-banner"
            >
              <Feather name="git-branch" size={16} color={colors.pierre.violet} />
              <Text className="text-sm ml-2" style={{ color: colors.pierre.violet }}>
                Forked from a system coach
              </Text>
            </View>
          )}

          {/* Title Field */}
          <View className="mb-5">
            <Text className="text-text-primary text-sm font-semibold mb-2">Title *</Text>
            <TextInput
              testID="coach-title-input"
              className="p-3.5 text-text-primary text-base"
              style={{
                ...glassCard,
                borderRadius: 12,
                borderColor: errors.title ? colors.error : 'rgba(139, 92, 246, 0.2)',
              }}
              value={title}
              onChangeText={setTitle}
              placeholder="Enter coach title"
              placeholderTextColor={colors.text.tertiary}
              maxLength={MAX_TITLE_LENGTH}
            />
            <Text className="text-text-tertiary text-xs text-right mt-1">
              {title.length}/{MAX_TITLE_LENGTH}
            </Text>
            {errors.title && (
              <Text className="text-error text-xs mt-1" testID="title-error">
                {errors.title}
              </Text>
            )}
          </View>

          {/* Category Field */}
          <View className="mb-5">
            <Text className="text-text-primary text-sm font-semibold mb-2">Category</Text>
            <TouchableOpacity
              className="flex-row items-center justify-between p-3.5"
              style={{
                ...glassCard,
                borderRadius: 12,
              }}
              onPress={showCategoryPicker}
              testID="category-picker"
            >
              <View
                className="px-3 py-1.5 rounded-full"
                style={{ backgroundColor: currentCategory?.color }}
                testID="selected-category"
              >
                <Text className="text-white text-sm font-semibold">
                  {currentCategory?.label}
                </Text>
              </View>
              <Text className="text-text-secondary text-sm">{'\u25BC'}</Text>
            </TouchableOpacity>
          </View>

          {/* Description Field */}
          <View className="mb-5">
            <Text className="text-text-primary text-sm font-semibold mb-2">Description</Text>
            <TextInput
              testID="coach-description-input"
              className="p-3.5 text-text-primary text-base min-h-[100px]"
              style={{
                ...glassCard,
                borderRadius: 12,
                borderColor: errors.description ? colors.error : 'rgba(139, 92, 246, 0.2)',
              }}
              value={description}
              onChangeText={setDescription}
              placeholder="Briefly describe what this coach does"
              placeholderTextColor={colors.text.tertiary}
              multiline
              numberOfLines={3}
              maxLength={MAX_DESCRIPTION_LENGTH}
              textAlignVertical="top"
            />
            <Text className="text-text-tertiary text-xs text-right mt-1">
              {description.length}/{MAX_DESCRIPTION_LENGTH}
            </Text>
            {errors.description && (
              <Text className="text-error text-xs mt-1" testID="description-error">
                {errors.description}
              </Text>
            )}
          </View>

          {/* System Prompt Section (collapsible, expanded by default) */}
          <CollapsibleSection
            title="System Prompt *"
            defaultExpanded
            testID="system-prompt-section"
          >
            <View className="flex-row justify-end mb-2">
              <TouchableOpacity
                onPress={() => setExpandedTextArea(true)}
                testID="expand-prompt-button"
              >
                <Text style={{ color: colors.pierre.violet }} className="text-sm">
                  Expand {'\u2197'}
                </Text>
              </TouchableOpacity>
            </View>
            <TextInput
              testID="system-prompt-input"
              className="p-3.5 text-text-primary text-base min-h-[200px]"
              style={{
                ...glassCard,
                borderRadius: 12,
                borderColor: errors.systemPrompt ? colors.error : 'rgba(139, 92, 246, 0.2)',
              }}
              value={systemPrompt}
              onChangeText={setSystemPrompt}
              placeholder="Define how this coach should behave and respond..."
              placeholderTextColor={colors.text.tertiary}
              multiline
              textAlignVertical="top"
            />
            {errors.systemPrompt && (
              <Text className="text-error text-xs mt-1" testID="prompt-error">
                {errors.systemPrompt}
              </Text>
            )}

            {/* Token counter with gradient progress bar */}
            <View
              className="mt-3 p-3 rounded-xl"
              style={{ ...glassCard, borderRadius: 12 }}
              testID="token-counter"
            >
              <Text className="text-text-secondary text-sm mb-2" testID="token-count-text">
                ~{tokenCount.toLocaleString()} tokens ({contextPercentage}% of context)
              </Text>
              <View
                className="h-1.5 rounded-full overflow-hidden"
                style={{ backgroundColor: 'rgba(255, 255, 255, 0.1)' }}
              >
                <LinearGradient
                  colors={gradients.violetCyan as [string, string]}
                  start={{ x: 0, y: 0 }}
                  end={{ x: 1, y: 0 }}
                  style={{
                    height: '100%',
                    width: `${Math.min(parseFloat(contextPercentage), 100)}%`,
                    borderRadius: 3,
                  }}
                />
              </View>
            </View>
          </CollapsibleSection>

          {/* Tags Section (collapsible, collapsed by default) */}
          <CollapsibleSection title="Tags" defaultExpanded={false} testID="tags-section">
            <View className="flex-row gap-2">
              <TextInput
                testID="tag-input"
                className="flex-1 p-3.5 text-text-primary text-base"
                style={{
                  ...glassCard,
                  borderRadius: 12,
                }}
                value={newTag}
                onChangeText={setNewTag}
                placeholder="Add a tag"
                placeholderTextColor={colors.text.tertiary}
                onSubmitEditing={addTag}
                returnKeyType="done"
              />
              <TouchableOpacity
                className="w-12 justify-center items-center rounded-xl"
                style={{
                  backgroundColor: colors.pierre.violet,
                  ...buttonGlow,
                }}
                onPress={addTag}
                testID="add-tag-button"
              >
                <Text className="text-white text-xl font-bold">+</Text>
              </TouchableOpacity>
            </View>
            <View className="flex-row flex-wrap gap-2 mt-3" testID="tags-container">
              {tags.map((tag) => (
                <View
                  key={tag}
                  className="flex-row items-center px-3 py-1.5 rounded-full gap-1"
                  style={{
                    backgroundColor: 'rgba(139, 92, 246, 0.15)',
                    borderWidth: 1,
                    borderColor: 'rgba(139, 92, 246, 0.3)',
                  }}
                  testID={`tag-chip-${tag}`}
                >
                  <Text style={{ color: colors.pierre.violet }} className="text-sm">
                    {tag}
                  </Text>
                  <TouchableOpacity
                    onPress={() => removeTag(tag)}
                    hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
                    testID={`remove-tag-${tag}`}
                  >
                    <Text style={{ color: colors.pierre.violet }} className="text-lg font-bold">
                      {'\u00D7'}
                    </Text>
                  </TouchableOpacity>
                </View>
              ))}
              {tags.length === 0 && (
                <Text className="text-text-tertiary text-sm italic" testID="no-tags-message">
                  No tags added yet
                </Text>
              )}
            </View>
          </CollapsibleSection>
        </ScrollView>
      </KeyboardAvoidingView>

      {/* Fullscreen System Prompt Modal */}
      <Modal
        visible={expandedTextArea}
        animationType="slide"
        presentationStyle="pageSheet"
      >
        <SafeAreaView className="flex-1 bg-background-primary" testID="expanded-modal">
          <View className="flex-row items-center justify-between px-3 py-2 border-b border-border-default">
            <TouchableOpacity
              onPress={() => setExpandedTextArea(false)}
              testID="modal-done-button"
            >
              <Text className="text-primary-500 text-base font-semibold">Done</Text>
            </TouchableOpacity>
            <Text className="text-text-primary text-base font-semibold">System Prompt</Text>
            <View className="w-[50px]" />
          </View>
          <TextInput
            testID="modal-text-input"
            className="flex-1 p-3 text-text-primary text-base"
            value={systemPrompt}
            onChangeText={setSystemPrompt}
            placeholder="Define how this coach should behave..."
            placeholderTextColor={colors.text.tertiary}
            multiline
            textAlignVertical="top"
            autoFocus
          />
          <View className="px-3 py-2 border-t border-border-default">
            <Text className="text-text-secondary text-sm" testID="modal-token-count">
              ~{tokenCount.toLocaleString()} tokens ({contextPercentage}% of context)
            </Text>
          </View>
        </SafeAreaView>
      </Modal>

      {/* Version History Modal */}
      {isEditMode && coachId && (
        <CoachVersionHistory
          coachId={coachId}
          coachTitle={title || 'Coach'}
          isOpen={showVersionHistory}
          onClose={() => setShowVersionHistory(false)}
          onReverted={() => {
            // Reload the coach data after revert
            loadCoach(coachId);
          }}
        />
      )}
    </SafeAreaView>
  );
}
