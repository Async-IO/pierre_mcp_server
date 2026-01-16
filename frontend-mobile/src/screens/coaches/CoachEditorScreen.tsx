// ABOUTME: Coach editor screen for creating and editing AI coaches
// ABOUTME: Unified form with live token counting and category picker

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
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
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import type { Coach, CoachCategory, CreateCoachRequest, UpdateCoachRequest } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

interface CoachEditorScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
}

// Category options with colors
const CATEGORY_OPTIONS: Array<{ key: CoachCategory; label: string; color: string }> = [
  { key: 'training', label: 'Training', color: '#10B981' },
  { key: 'nutrition', label: 'Nutrition', color: '#F59E0B' },
  { key: 'recovery', label: 'Recovery', color: '#6366F1' },
  { key: 'recipes', label: 'Recipes', color: '#F97316' },
  { key: 'custom', label: 'Custom', color: '#7C3AED' },
];

// Validation constants
const MAX_TITLE_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;
const MAX_SYSTEM_PROMPT_LENGTH = 4000;
const CONTEXT_WINDOW_SIZE = 128000;

export function CoachEditorScreen({ navigation }: CoachEditorScreenProps) {
  const route = useRoute<RouteProp<AppDrawerParamList, 'CoachEditor'>>();
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
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.primary[500]} />
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      <KeyboardAvoidingView
        style={styles.keyboardView}
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      >
        {/* Header */}
        <View style={styles.header}>
          <TouchableOpacity
            style={styles.backButton}
            onPress={() => navigation.goBack()}
          >
            <Text style={styles.backIcon}>←</Text>
          </TouchableOpacity>
          <Text style={styles.headerTitle}>
            {isEditMode ? 'Edit Coach' : 'Create Coach'}
          </Text>
          <TouchableOpacity
            style={[styles.saveButton, isSaving && styles.saveButtonDisabled]}
            onPress={handleSave}
            disabled={isSaving}
          >
            {isSaving ? (
              <ActivityIndicator size="small" color={colors.text.primary} />
            ) : (
              <Text style={styles.saveText}>Save</Text>
            )}
          </TouchableOpacity>
        </View>

        <ScrollView
          style={styles.scrollView}
          contentContainerStyle={styles.scrollContent}
          keyboardShouldPersistTaps="handled"
        >
          {/* Title Field */}
          <View style={styles.fieldContainer}>
            <Text style={styles.fieldLabel}>
              Title <Text style={styles.requiredIndicator}>*</Text>
            </Text>
            <TextInput
              style={[styles.textInput, errors.title && styles.textInputError]}
              value={title}
              onChangeText={setTitle}
              placeholder="Enter coach title"
              placeholderTextColor={colors.text.tertiary}
              maxLength={MAX_TITLE_LENGTH}
            />
            {errors.title && (
              <Text style={styles.errorText}>{errors.title}</Text>
            )}
            <Text style={styles.charCount}>
              {title.length}/{MAX_TITLE_LENGTH}
            </Text>
          </View>

          {/* Category Field */}
          <View style={styles.fieldContainer}>
            <Text style={styles.fieldLabel}>Category</Text>
            <TouchableOpacity
              style={styles.categoryPicker}
              onPress={showCategoryPicker}
            >
              <View style={styles.categoryPickerContent}>
                <View
                  style={[
                    styles.categoryDot,
                    { backgroundColor: currentCategory?.color || colors.text.tertiary },
                  ]}
                />
                <Text style={styles.categoryPickerText}>
                  {currentCategory?.label || 'Select category'}
                </Text>
              </View>
              <Text style={styles.categoryPickerArrow}>▼</Text>
            </TouchableOpacity>
          </View>

          {/* Description Field */}
          <View style={styles.fieldContainer}>
            <Text style={styles.fieldLabel}>Description</Text>
            <TextInput
              style={[
                styles.textInput,
                styles.multilineInput,
                errors.description && styles.textInputError,
              ]}
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
              <Text style={styles.errorText}>{errors.description}</Text>
            )}
            <Text style={styles.charCount}>
              {description.length}/{MAX_DESCRIPTION_LENGTH}
            </Text>
          </View>

          {/* System Prompt Field */}
          <View style={styles.fieldContainer}>
            <Text style={styles.fieldLabel}>
              System Prompt <Text style={styles.requiredIndicator}>*</Text>
            </Text>
            <TextInput
              style={[
                styles.textInput,
                styles.systemPromptInput,
                errors.systemPrompt && styles.textInputError,
              ]}
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
              <Text style={styles.errorText}>{errors.systemPrompt}</Text>
            )}
            <Text style={styles.charCount}>
              {systemPrompt.length}/{MAX_SYSTEM_PROMPT_LENGTH}
            </Text>
          </View>

          {/* Token Count Display */}
          <View style={styles.tokenContainer}>
            <Text style={styles.tokenIcon}>📊</Text>
            <Text style={styles.tokenText}>
              ~{tokenCount} tokens ({contextPercentage}% context)
            </Text>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  keyboardView: {
    flex: 1,
  },
  loadingContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  backButton: {
    width: 40,
    height: 40,
    alignItems: 'center',
    justifyContent: 'center',
  },
  backIcon: {
    fontSize: 24,
    color: colors.text.primary,
  },
  headerTitle: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
  },
  saveButton: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    backgroundColor: colors.primary[500],
    borderRadius: borderRadius.md,
    minWidth: 60,
    alignItems: 'center',
  },
  saveButtonDisabled: {
    opacity: 0.6,
  },
  saveText: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
  },
  scrollView: {
    flex: 1,
  },
  scrollContent: {
    padding: spacing.lg,
    paddingBottom: spacing.xxl,
  },
  fieldContainer: {
    marginBottom: spacing.lg,
  },
  fieldLabel: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.secondary,
    marginBottom: spacing.xs,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  requiredIndicator: {
    color: colors.error,
  },
  textInput: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    fontSize: fontSize.md,
    color: colors.text.primary,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  textInputError: {
    borderColor: colors.error,
  },
  multilineInput: {
    minHeight: 80,
    paddingTop: spacing.md,
  },
  systemPromptInput: {
    minHeight: 200,
    paddingTop: spacing.md,
  },
  errorText: {
    fontSize: fontSize.sm,
    color: colors.error,
    marginTop: spacing.xs,
  },
  charCount: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
    textAlign: 'right',
    marginTop: spacing.xs,
  },
  categoryPicker: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  categoryPickerContent: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  categoryDot: {
    width: 12,
    height: 12,
    borderRadius: 6,
    marginRight: spacing.sm,
  },
  categoryPickerText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  categoryPickerArrow: {
    fontSize: fontSize.sm,
    color: colors.text.tertiary,
  },
  tokenContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
    marginTop: spacing.md,
  },
  tokenIcon: {
    fontSize: 18,
    marginRight: spacing.sm,
  },
  tokenText: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
  },
});
