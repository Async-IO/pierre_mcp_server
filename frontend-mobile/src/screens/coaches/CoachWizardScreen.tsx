// ABOUTME: Multi-step wizard for creating/editing AI coaches (ASY-149)
// ABOUTME: Swipeable steps with step indicator, action sheets, and tag chips

import React, { useState, useRef, useCallback } from 'react';
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
  Dimensions,
  Modal,
} from 'react-native';
import { useRoute, type RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import {
  useSharedValue,
  withSpring,
} from 'react-native-reanimated';
import { LinearGradient } from 'expo-linear-gradient';
import { colors, spacing, glassCard, gradients, buttonGlow } from '../../constants/theme';
import { coachesApi } from '../../services/api';
import { CoachVersionHistoryModal } from '../../components/CoachVersionHistoryModal';
import type { CreateCoachRequest, UpdateCoachRequest } from '../../types';
import type { CoachesStackParamList } from '../../navigation/MainTabs';

const { width: SCREEN_WIDTH } = Dimensions.get('window');
const STEPS = ['Basic Info', 'Category & Tags', 'System Prompt', 'Review'];

interface CoachWizardScreenProps {
  navigation: NativeStackNavigationProp<CoachesStackParamList>;
}

// Category options with colors matching Stitch UX spec
const CATEGORY_OPTIONS: Array<{ key: string; label: string; color: string }> = [
  { key: 'training', label: 'Training', color: '#4ADE80' },  // Green
  { key: 'nutrition', label: 'Nutrition', color: '#F59E0B' }, // Amber
  { key: 'recovery', label: 'Recovery', color: '#22D3EE' },  // Cyan
  { key: 'recipes', label: 'Recipes', color: '#F59E0B' },    // Amber
  { key: 'mobility', label: 'Mobility', color: '#EC4899' },  // Pink
  { key: 'custom', label: 'Custom', color: '#8B5CF6' },      // Violet
];

// Validation constants
const MAX_TITLE_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;
const MAX_SYSTEM_PROMPT_LENGTH = 8000;
const CONTEXT_WINDOW_SIZE = 128000;

export function CoachWizardScreen({ navigation }: CoachWizardScreenProps) {
  const route = useRoute<RouteProp<CoachesStackParamList, 'CoachWizard'>>();
  const coachId = route.params?.coachId;
  const isEditMode = Boolean(coachId);

  // Wizard state
  const [currentStep, setCurrentStep] = useState(0);
  const scrollViewRef = useRef<ScrollView>(null);
  const stepProgress = useSharedValue(0);

  // Form state
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<string>('custom');
  const [tags, setTags] = useState<string[]>([]);
  const [newTag, setNewTag] = useState('');
  const [systemPrompt, setSystemPrompt] = useState('');

  // UI state
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [expandedTextArea, setExpandedTextArea] = useState<'description' | 'systemPrompt' | null>(null);
  const [showVersionHistory, setShowVersionHistory] = useState(false);

  // Load coach data for edit mode
  const loadCoach = useCallback(async (id: string) => {
    try {
      setIsLoading(true);
      const coach = await coachesApi.get(id);
      setTitle(coach.title);
      setDescription(coach.description || '');
      setCategory(coach.category);
      setTags(coach.tags || []);
      setSystemPrompt(coach.system_prompt);
    } catch (error) {
      console.error('Failed to load coach:', error);
      Alert.alert('Error', 'Failed to load coach data');
      navigation.goBack();
    } finally {
      setIsLoading(false);
    }
  }, [navigation]);

  React.useEffect(() => {
    if (isEditMode && coachId) {
      loadCoach(coachId);
    }
  }, [isEditMode, coachId, loadCoach]);

  // Token count calculation
  const tokenCount = Math.ceil(systemPrompt.length / 4);
  const contextPercentage = ((tokenCount / CONTEXT_WINDOW_SIZE) * 100).toFixed(1);

  // Step validation
  const validateStep = useCallback((step: number): boolean => {
    const newErrors: Record<string, string> = {};

    switch (step) {
      case 0: // Basic Info
        if (!title.trim()) {
          newErrors.title = 'Title is required';
        } else if (title.length > MAX_TITLE_LENGTH) {
          newErrors.title = `Title must be ${MAX_TITLE_LENGTH} characters or less`;
        }
        if (description.length > MAX_DESCRIPTION_LENGTH) {
          newErrors.description = `Description must be ${MAX_DESCRIPTION_LENGTH} characters or less`;
        }
        break;
      case 1: // Category & Tags
        // Category always has a default, tags are optional
        break;
      case 2: // System Prompt
        if (!systemPrompt.trim()) {
          newErrors.systemPrompt = 'System prompt is required';
        } else if (systemPrompt.length > MAX_SYSTEM_PROMPT_LENGTH) {
          newErrors.systemPrompt = `System prompt must be ${MAX_SYSTEM_PROMPT_LENGTH} characters or less`;
        }
        break;
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [title, description, systemPrompt]);

  // Navigation
  const goToStep = useCallback((step: number) => {
    if (step < 0 || step >= STEPS.length) return;

    // Validate current step before advancing
    if (step > currentStep && !validateStep(currentStep)) {
      return;
    }

    setCurrentStep(step);
    stepProgress.value = withSpring(step);
    scrollViewRef.current?.scrollTo({ x: step * SCREEN_WIDTH, animated: true });
  }, [currentStep, validateStep, stepProgress]);

  const handleNext = () => goToStep(currentStep + 1);
  const handleBack = () => goToStep(currentStep - 1);

  // Category picker (action sheet on iOS, alert on Android)
  const showCategoryPicker = () => {
    const options = [...CATEGORY_OPTIONS.map(c => c.label), 'Cancel'];
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
          { text: 'Cancel', style: 'cancel' },
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
    setTags(tags.filter(tag => tag !== tagToRemove));
  };

  // Save coach
  const handleSave = async () => {
    if (!validateStep(currentStep)) return;

    try {
      setIsSaving(true);

      if (isEditMode && coachId) {
        const updateData: UpdateCoachRequest = {
          title,
          description: description || undefined,
          system_prompt: systemPrompt,
          category,
          tags,
        };
        await coachesApi.update(coachId, updateData);
        Alert.alert('Success', 'Coach updated successfully', [
          { text: 'OK', onPress: () => navigation.goBack() },
        ]);
      } else {
        const createData: CreateCoachRequest = {
          title,
          description: description || undefined,
          system_prompt: systemPrompt,
          category,
          tags,
        };
        await coachesApi.create(createData);
        Alert.alert('Success', 'Coach created successfully', [
          { text: 'OK', onPress: () => navigation.goBack() },
        ]);
      }
    } catch (error) {
      console.error('Failed to save coach:', error);
      Alert.alert('Error', 'Failed to save coach. Please try again.');
    } finally {
      setIsSaving(false);
    }
  };

  // Render step indicator with gradient active step
  const renderStepIndicator = () => (
    <View className="items-center py-4" testID="step-indicator">
      <View className="flex-row gap-3">
        {STEPS.map((step, index) => (
          <TouchableOpacity
            key={step}
            onPress={() => goToStep(index)}
            testID={`step-dot-${index}`}
          >
            {index === currentStep ? (
              <LinearGradient
                colors={gradients.violetCyan as [string, string]}
                start={{ x: 0, y: 0 }}
                end={{ x: 1, y: 1 }}
                style={{ width: 32, height: 32, borderRadius: 16, justifyContent: 'center', alignItems: 'center' }}
              >
                <Text className="text-white text-sm font-bold">{index + 1}</Text>
              </LinearGradient>
            ) : index < currentStep ? (
              <View className="w-8 h-8 rounded-full justify-center items-center bg-pierre-activity">
                <Text className="text-white text-sm font-bold">✓</Text>
              </View>
            ) : (
              <View className="w-8 h-8 rounded-full justify-center items-center" style={{ ...glassCard, borderRadius: 16 }}>
                <Text className="text-text-secondary text-sm font-semibold">{index + 1}</Text>
              </View>
            )}
          </TouchableOpacity>
        ))}
      </View>
      <Text className="text-text-primary text-base font-semibold mt-3" testID="current-step-label">{STEPS[currentStep]}</Text>
    </View>
  );

  // Render Step 1: Basic Info
  const renderBasicInfoStep = () => (
    <View className="flex-1 pt-3" testID="step-basic-info">
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
        <Text className="text-text-tertiary text-xs text-right mt-1">{title.length}/{MAX_TITLE_LENGTH}</Text>
        {errors.title && <Text className="text-error text-xs mt-1" testID="title-error">{errors.title}</Text>}
      </View>

      <View className="mb-5">
        <View className="flex-row justify-between items-center mb-2">
          <Text className="text-text-primary text-sm font-semibold">Description</Text>
          <TouchableOpacity onPress={() => setExpandedTextArea('description')} testID="expand-description-button">
            <Text style={{ color: colors.pierre.violet }} className="text-sm">Expand ↗</Text>
          </TouchableOpacity>
        </View>
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
        <Text className="text-text-tertiary text-xs text-right mt-1">{description.length}/{MAX_DESCRIPTION_LENGTH}</Text>
        {errors.description && <Text className="text-error text-xs mt-1" testID="description-error">{errors.description}</Text>}
      </View>
    </View>
  );

  // Render Step 2: Category & Tags
  const renderCategoryTagsStep = () => {
    const selectedCategory = CATEGORY_OPTIONS.find(c => c.key === category);

    return (
      <View className="flex-1 pt-3" testID="step-category-tags">
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
              style={{ backgroundColor: selectedCategory?.color }}
              testID="selected-category"
            >
              <Text className="text-white text-sm font-semibold">{selectedCategory?.label}</Text>
            </View>
            <Text className="text-text-secondary text-sm">▼</Text>
          </TouchableOpacity>
        </View>

        <View className="mb-5">
          <Text className="text-text-primary text-sm font-semibold mb-2">Tags</Text>
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
                <Text style={{ color: colors.pierre.violet }} className="text-sm">{tag}</Text>
                <TouchableOpacity onPress={() => removeTag(tag)} hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }} testID={`remove-tag-${tag}`}>
                  <Text style={{ color: colors.pierre.violet }} className="text-lg font-bold">×</Text>
                </TouchableOpacity>
              </View>
            ))}
            {tags.length === 0 && (
              <Text className="text-text-tertiary text-sm italic" testID="no-tags-message">No tags added yet</Text>
            )}
          </View>
        </View>
      </View>
    );
  };

  // Render Step 3: System Prompt
  const renderSystemPromptStep = () => (
    <View className="flex-1 pt-3" testID="step-system-prompt">
      <View className="mb-5 flex-1">
        <View className="flex-row justify-between items-center mb-2">
          <Text className="text-text-primary text-sm font-semibold">System Prompt *</Text>
          <TouchableOpacity onPress={() => setExpandedTextArea('systemPrompt')} testID="expand-prompt-button">
            <Text style={{ color: colors.pierre.violet }} className="text-sm">Expand ↗</Text>
          </TouchableOpacity>
        </View>
        <TextInput
          testID="system-prompt-input"
          className="p-3.5 text-text-primary text-base min-h-[200px] flex-1"
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
        {errors.systemPrompt && <Text className="text-error text-xs mt-1" testID="prompt-error">{errors.systemPrompt}</Text>}
      </View>

      <View className="mt-3 p-3 rounded-xl" style={{ ...glassCard, borderRadius: 12 }} testID="token-counter">
        <Text className="text-text-secondary text-sm mb-2" testID="token-count-text">
          ~{tokenCount.toLocaleString()} tokens ({contextPercentage}% of context)
        </Text>
        <View className="h-1.5 rounded-full overflow-hidden" style={{ backgroundColor: 'rgba(255, 255, 255, 0.1)' }}>
          <LinearGradient
            colors={gradients.violetCyan as [string, string]}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 0 }}
            style={{ height: '100%', width: `${Math.min(parseFloat(contextPercentage), 100)}%`, borderRadius: 3 }}
          />
        </View>
      </View>
    </View>
  );

  // Render Step 4: Review
  const renderReviewStep = () => {
    const selectedCategory = CATEGORY_OPTIONS.find(c => c.key === category);

    return (
      <View className="flex-1 pt-3" testID="step-review">
        <Text className="text-text-primary text-xl font-bold mb-5" testID="review-title">Review Your Coach</Text>

        <View className="mb-3">
          <Text className="text-text-secondary text-sm mb-1">Title</Text>
          <Text className="text-text-primary text-base" testID="review-coach-title">{title || '(not set)'}</Text>
        </View>

        <View className="mb-3">
          <Text className="text-text-secondary text-sm mb-1">Category</Text>
          <View
            className="px-3 py-1 rounded-full self-start"
            style={{ backgroundColor: selectedCategory?.color }}
            testID="review-category"
          >
            <Text className="text-white text-sm font-semibold">{selectedCategory?.label}</Text>
          </View>
        </View>

        {description && (
          <View className="mb-3">
            <Text className="text-text-secondary text-sm mb-1">Description</Text>
            <Text className="text-text-primary text-base" testID="review-description">{description}</Text>
          </View>
        )}

        {tags.length > 0 && (
          <View className="mb-3">
            <Text className="text-text-secondary text-sm mb-1">Tags</Text>
            <View className="flex-row flex-wrap gap-2" testID="review-tags">
              {tags.map((tag) => (
                <View key={tag} className="bg-background-tertiary px-3 py-1 rounded-full">
                  <Text className="text-text-primary text-sm">{tag}</Text>
                </View>
              ))}
            </View>
          </View>
        )}

        <View className="mb-3">
          <Text className="text-text-secondary text-sm mb-1">System Prompt</Text>
          <Text className="text-text-primary text-sm bg-background-secondary p-3 rounded-md" numberOfLines={5} testID="review-prompt">
            {systemPrompt || '(not set)'}
          </Text>
          <Text className="text-text-tertiary text-xs mt-1">~{tokenCount.toLocaleString()} tokens</Text>
        </View>
      </View>
    );
  };

  // Expanded text area modal
  const renderExpandedModal = () => (
    <Modal
      visible={expandedTextArea !== null}
      animationType="slide"
      presentationStyle="pageSheet"
    >
      <SafeAreaView className="flex-1 bg-background-primary" testID="expanded-modal">
        <View className="flex-row items-center justify-between px-3 py-2 border-b border-border-default">
          <TouchableOpacity onPress={() => setExpandedTextArea(null)} testID="modal-done-button">
            <Text className="text-primary-500 text-base font-semibold">Done</Text>
          </TouchableOpacity>
          <Text className="text-text-primary text-base font-semibold">
            {expandedTextArea === 'description' ? 'Description' : 'System Prompt'}
          </Text>
          <View className="w-[50px]" />
        </View>
        <TextInput
          testID="modal-text-input"
          className="flex-1 p-3 text-text-primary text-base"
          value={expandedTextArea === 'description' ? description : systemPrompt}
          onChangeText={expandedTextArea === 'description' ? setDescription : setSystemPrompt}
          placeholder={expandedTextArea === 'description'
            ? 'Describe what this coach does...'
            : 'Define how this coach should behave...'}
          placeholderTextColor={colors.text.tertiary}
          multiline
          textAlignVertical="top"
          autoFocus
        />
        {expandedTextArea === 'systemPrompt' && (
          <View className="px-3 py-2 border-t border-border-default">
            <Text className="text-text-secondary text-sm" testID="modal-token-count">
              ~{tokenCount.toLocaleString()} tokens ({contextPercentage}% of context)
            </Text>
          </View>
        )}
      </SafeAreaView>
    </Modal>
  );

  if (isLoading) {
    return (
      <SafeAreaView className="flex-1 bg-background-primary">
        <View className="flex-1 justify-center items-center">
          <ActivityIndicator size="large" color={colors.primary[500]} />
          <Text className="text-text-secondary mt-3 text-base">Loading coach...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="coach-wizard-screen">
      <KeyboardAvoidingView
        className="flex-1"
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      >
        {/* Header */}
        <View className="flex-row items-center justify-between px-3 py-2 border-b border-border-default">
          <TouchableOpacity onPress={() => navigation.goBack()} className="w-[70px]" testID="cancel-button">
            <Text className="text-primary-500 text-base">Cancel</Text>
          </TouchableOpacity>
          <Text className="text-text-primary text-lg font-semibold">
            {isEditMode ? 'Edit Coach' : 'New Coach'}
          </Text>
          {isEditMode ? (
            <TouchableOpacity
              onPress={() => setShowVersionHistory(true)}
              className="w-[70px]"
              testID="history-button"
            >
              <Text className="text-primary-500 text-base">History</Text>
            </TouchableOpacity>
          ) : (
            <View className="w-[70px]" />
          )}
        </View>

        {/* Step Indicator */}
        {renderStepIndicator()}

        {/* Steps Content */}
        <ScrollView
          ref={scrollViewRef}
          horizontal
          pagingEnabled
          showsHorizontalScrollIndicator={false}
          scrollEnabled={false}
          className="flex-1"
        >
          <View style={{ width: SCREEN_WIDTH, paddingHorizontal: spacing.md }}>{renderBasicInfoStep()}</View>
          <View style={{ width: SCREEN_WIDTH, paddingHorizontal: spacing.md }}>{renderCategoryTagsStep()}</View>
          <View style={{ width: SCREEN_WIDTH, paddingHorizontal: spacing.md }}>{renderSystemPromptStep()}</View>
          <View style={{ width: SCREEN_WIDTH, paddingHorizontal: spacing.md }}>{renderReviewStep()}</View>
        </ScrollView>

        {/* Navigation Buttons */}
        <View
          className="flex-row items-center px-4 py-4"
          style={{ borderTopWidth: 1, borderTopColor: 'rgba(139, 92, 246, 0.2)' }}
        >
          {currentStep > 0 && (
            <TouchableOpacity
              className="py-2.5 px-4 rounded-xl"
              style={{ ...glassCard, borderRadius: 12 }}
              onPress={handleBack}
              testID="back-button"
            >
              <Text className="text-text-secondary text-base">← Back</Text>
            </TouchableOpacity>
          )}
          <View className="flex-1" />
          {currentStep < STEPS.length - 1 ? (
            <TouchableOpacity
              className="py-2.5 px-6 rounded-xl overflow-hidden"
              style={{
                backgroundColor: colors.pierre.violet,
                ...buttonGlow,
              }}
              onPress={handleNext}
              testID="next-button"
            >
              <Text className="text-white text-base font-semibold">Next →</Text>
            </TouchableOpacity>
          ) : (
            <TouchableOpacity
              className={`py-2.5 px-6 rounded-xl min-w-[140px] items-center ${isSaving ? 'opacity-60' : ''}`}
              style={{
                backgroundColor: colors.pierre.activity,
                shadowColor: colors.pierre.activity,
                shadowOffset: { width: 0, height: 0 },
                shadowOpacity: 0.4,
                shadowRadius: 12,
                elevation: 4,
              }}
              onPress={handleSave}
              disabled={isSaving}
              testID="save-button"
            >
              {isSaving ? (
                <ActivityIndicator size="small" color="#fff" />
              ) : (
                <Text className="text-white text-base font-semibold">
                  {isEditMode ? 'Save Changes' : 'Create Coach'}
                </Text>
              )}
            </TouchableOpacity>
          )}
        </View>

        {/* Expanded Text Area Modal */}
        {renderExpandedModal()}

        {/* Version History Modal */}
        {isEditMode && coachId && (
          <CoachVersionHistoryModal
            visible={showVersionHistory}
            onClose={() => setShowVersionHistory(false)}
            coachId={coachId}
            coachTitle={title || 'Coach'}
            onReverted={() => {
              setShowVersionHistory(false);
              // Reload coach data after revert
              if (coachId) {
                loadCoach(coachId);
              }
            }}
          />
        )}
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
