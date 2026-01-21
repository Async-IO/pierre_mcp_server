// ABOUTME: Multi-step wizard for creating/editing AI coaches (ASY-149)
// ABOUTME: Swipeable steps with step indicator, action sheets, and tag chips

import React, { useState, useRef, useCallback } from 'react';
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
  Dimensions,
  Modal,
} from 'react-native';
import { useRoute, type RouteProp } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import {
  useSharedValue,
  withSpring,
} from 'react-native-reanimated';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { CoachVersionHistoryModal } from '../../components/CoachVersionHistoryModal';
import type { CoachCategory, CreateCoachRequest, UpdateCoachRequest } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

const { width: SCREEN_WIDTH } = Dimensions.get('window');
const STEPS = ['Basic Info', 'Category & Tags', 'System Prompt', 'Review'];

interface CoachWizardScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
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
const MAX_SYSTEM_PROMPT_LENGTH = 8000;
const CONTEXT_WINDOW_SIZE = 128000;

export function CoachWizardScreen({ navigation }: CoachWizardScreenProps) {
  const route = useRoute<RouteProp<AppDrawerParamList, 'CoachWizard'>>();
  const coachId = route.params?.coachId;
  const isEditMode = Boolean(coachId);

  // Wizard state
  const [currentStep, setCurrentStep] = useState(0);
  const scrollViewRef = useRef<ScrollView>(null);
  const stepProgress = useSharedValue(0);

  // Form state
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<CoachCategory>('custom');
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
      const coach = await apiService.getCoach(id);
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
        await apiService.updateCoach(coachId, updateData);
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
        await apiService.createCoach(createData);
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

  // Render step indicator
  const renderStepIndicator = () => (
    <View style={styles.stepIndicatorContainer} testID="step-indicator">
      <View style={styles.stepDots}>
        {STEPS.map((step, index) => (
          <TouchableOpacity
            key={step}
            onPress={() => goToStep(index)}
            testID={`step-dot-${index}`}
            style={[
              styles.stepDot,
              index === currentStep && styles.stepDotActive,
              index < currentStep && styles.stepDotCompleted,
            ]}
          >
            {index < currentStep ? (
              <Text style={styles.stepDotCheckmark}>✓</Text>
            ) : (
              <Text style={[styles.stepDotNumber, index === currentStep && styles.stepDotNumberActive]}>
                {index + 1}
              </Text>
            )}
          </TouchableOpacity>
        ))}
      </View>
      <Text style={styles.stepLabel} testID="current-step-label">{STEPS[currentStep]}</Text>
    </View>
  );

  // Render Step 1: Basic Info
  const renderBasicInfoStep = () => (
    <View style={styles.stepContent} testID="step-basic-info">
      <View style={styles.inputGroup}>
        <Text style={styles.label}>Title *</Text>
        <TextInput
          testID="coach-title-input"
          style={[styles.input, errors.title && styles.inputError]}
          value={title}
          onChangeText={setTitle}
          placeholder="Enter coach title"
          placeholderTextColor={colors.text.tertiary}
          maxLength={MAX_TITLE_LENGTH}
        />
        <Text style={styles.charCount}>{title.length}/{MAX_TITLE_LENGTH}</Text>
        {errors.title && <Text style={styles.errorText} testID="title-error">{errors.title}</Text>}
      </View>

      <View style={styles.inputGroup}>
        <View style={styles.labelRow}>
          <Text style={styles.label}>Description</Text>
          <TouchableOpacity onPress={() => setExpandedTextArea('description')} testID="expand-description-button">
            <Text style={styles.expandButton}>Expand ↗</Text>
          </TouchableOpacity>
        </View>
        <TextInput
          testID="coach-description-input"
          style={[styles.textArea, errors.description && styles.inputError]}
          value={description}
          onChangeText={setDescription}
          placeholder="Briefly describe what this coach does"
          placeholderTextColor={colors.text.tertiary}
          multiline
          numberOfLines={3}
          maxLength={MAX_DESCRIPTION_LENGTH}
        />
        <Text style={styles.charCount}>{description.length}/{MAX_DESCRIPTION_LENGTH}</Text>
        {errors.description && <Text style={styles.errorText} testID="description-error">{errors.description}</Text>}
      </View>
    </View>
  );

  // Render Step 2: Category & Tags
  const renderCategoryTagsStep = () => {
    const selectedCategory = CATEGORY_OPTIONS.find(c => c.key === category);

    return (
      <View style={styles.stepContent} testID="step-category-tags">
        <View style={styles.inputGroup}>
          <Text style={styles.label}>Category</Text>
          <TouchableOpacity style={styles.categoryPicker} onPress={showCategoryPicker} testID="category-picker">
            <View style={[styles.categoryBadge, { backgroundColor: selectedCategory?.color }]} testID="selected-category">
              <Text style={styles.categoryBadgeText}>{selectedCategory?.label}</Text>
            </View>
            <Text style={styles.categoryPickerArrow}>▼</Text>
          </TouchableOpacity>
        </View>

        <View style={styles.inputGroup}>
          <Text style={styles.label}>Tags</Text>
          <View style={styles.tagInputRow}>
            <TextInput
              testID="tag-input"
              style={styles.tagInput}
              value={newTag}
              onChangeText={setNewTag}
              placeholder="Add a tag"
              placeholderTextColor={colors.text.tertiary}
              onSubmitEditing={addTag}
              returnKeyType="done"
            />
            <TouchableOpacity style={styles.addTagButton} onPress={addTag} testID="add-tag-button">
              <Text style={styles.addTagButtonText}>+</Text>
            </TouchableOpacity>
          </View>
          <View style={styles.tagsContainer} testID="tags-container">
            {tags.map((tag) => (
              <View key={tag} style={styles.tagChip} testID={`tag-chip-${tag}`}>
                <Text style={styles.tagChipText}>{tag}</Text>
                <TouchableOpacity onPress={() => removeTag(tag)} hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }} testID={`remove-tag-${tag}`}>
                  <Text style={styles.tagChipRemove}>×</Text>
                </TouchableOpacity>
              </View>
            ))}
            {tags.length === 0 && (
              <Text style={styles.noTagsText} testID="no-tags-message">No tags added yet</Text>
            )}
          </View>
        </View>
      </View>
    );
  };

  // Render Step 3: System Prompt
  const renderSystemPromptStep = () => (
    <View style={styles.stepContent} testID="step-system-prompt">
      <View style={styles.inputGroup}>
        <View style={styles.labelRow}>
          <Text style={styles.label}>System Prompt *</Text>
          <TouchableOpacity onPress={() => setExpandedTextArea('systemPrompt')} testID="expand-prompt-button">
            <Text style={styles.expandButton}>Expand ↗</Text>
          </TouchableOpacity>
        </View>
        <TextInput
          testID="system-prompt-input"
          style={[styles.systemPromptInput, errors.systemPrompt && styles.inputError]}
          value={systemPrompt}
          onChangeText={setSystemPrompt}
          placeholder="Define how this coach should behave and respond..."
          placeholderTextColor={colors.text.tertiary}
          multiline
          textAlignVertical="top"
        />
        {errors.systemPrompt && <Text style={styles.errorText} testID="prompt-error">{errors.systemPrompt}</Text>}
      </View>

      <View style={styles.tokenCounter} testID="token-counter">
        <Text style={styles.tokenCountText} testID="token-count-text">
          ~{tokenCount.toLocaleString()} tokens ({contextPercentage}% of context)
        </Text>
        <View style={styles.tokenBar}>
          <View style={[styles.tokenBarFill, { width: `${Math.min(parseFloat(contextPercentage), 100)}%` }]} />
        </View>
      </View>
    </View>
  );

  // Render Step 4: Review
  const renderReviewStep = () => {
    const selectedCategory = CATEGORY_OPTIONS.find(c => c.key === category);

    return (
      <View style={styles.stepContent} testID="step-review">
        <Text style={styles.reviewTitle} testID="review-title">Review Your Coach</Text>

        <View style={styles.reviewSection}>
          <Text style={styles.reviewLabel}>Title</Text>
          <Text style={styles.reviewValue} testID="review-coach-title">{title || '(not set)'}</Text>
        </View>

        <View style={styles.reviewSection}>
          <Text style={styles.reviewLabel}>Category</Text>
          <View style={[styles.categoryBadge, { backgroundColor: selectedCategory?.color, alignSelf: 'flex-start' }]} testID="review-category">
            <Text style={styles.categoryBadgeText}>{selectedCategory?.label}</Text>
          </View>
        </View>

        {description && (
          <View style={styles.reviewSection}>
            <Text style={styles.reviewLabel}>Description</Text>
            <Text style={styles.reviewValue} testID="review-description">{description}</Text>
          </View>
        )}

        {tags.length > 0 && (
          <View style={styles.reviewSection}>
            <Text style={styles.reviewLabel}>Tags</Text>
            <View style={styles.tagsContainer} testID="review-tags">
              {tags.map((tag) => (
                <View key={tag} style={styles.tagChipReview}>
                  <Text style={styles.tagChipText}>{tag}</Text>
                </View>
              ))}
            </View>
          </View>
        )}

        <View style={styles.reviewSection}>
          <Text style={styles.reviewLabel}>System Prompt</Text>
          <Text style={styles.reviewPromptPreview} numberOfLines={5} testID="review-prompt">
            {systemPrompt || '(not set)'}
          </Text>
          <Text style={styles.tokenCountSmall}>~{tokenCount.toLocaleString()} tokens</Text>
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
      <SafeAreaView style={styles.modalContainer} testID="expanded-modal">
        <View style={styles.modalHeader}>
          <TouchableOpacity onPress={() => setExpandedTextArea(null)} testID="modal-done-button">
            <Text style={styles.modalCloseButton}>Done</Text>
          </TouchableOpacity>
          <Text style={styles.modalTitle}>
            {expandedTextArea === 'description' ? 'Description' : 'System Prompt'}
          </Text>
          <View style={{ width: 50 }} />
        </View>
        <TextInput
          testID="modal-text-input"
          style={styles.modalTextArea}
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
          <View style={styles.modalTokenCounter}>
            <Text style={styles.tokenCountText} testID="modal-token-count">
              ~{tokenCount.toLocaleString()} tokens ({contextPercentage}% of context)
            </Text>
          </View>
        )}
      </SafeAreaView>
    </Modal>
  );

  if (isLoading) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.primary[500]} />
          <Text style={styles.loadingText}>Loading coach...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container} testID="coach-wizard-screen">
      <KeyboardAvoidingView
        style={styles.keyboardView}
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      >
        {/* Header */}
        <View style={styles.header}>
          <TouchableOpacity onPress={() => navigation.goBack()} style={styles.headerButton} testID="cancel-button">
            <Text style={styles.headerButtonText}>Cancel</Text>
          </TouchableOpacity>
          <Text style={styles.headerTitle}>
            {isEditMode ? 'Edit Coach' : 'New Coach'}
          </Text>
          {isEditMode ? (
            <TouchableOpacity
              onPress={() => setShowVersionHistory(true)}
              style={styles.headerButton}
              testID="history-button"
            >
              <Text style={styles.headerButtonText}>History</Text>
            </TouchableOpacity>
          ) : (
            <View style={styles.headerButton} />
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
          style={styles.stepsScrollView}
        >
          <View style={styles.stepPage}>{renderBasicInfoStep()}</View>
          <View style={styles.stepPage}>{renderCategoryTagsStep()}</View>
          <View style={styles.stepPage}>{renderSystemPromptStep()}</View>
          <View style={styles.stepPage}>{renderReviewStep()}</View>
        </ScrollView>

        {/* Navigation Buttons */}
        <View style={styles.navigationButtons}>
          {currentStep > 0 && (
            <TouchableOpacity style={styles.backButton} onPress={handleBack} testID="back-button">
              <Text style={styles.backButtonText}>← Back</Text>
            </TouchableOpacity>
          )}
          <View style={styles.navigationSpacer} />
          {currentStep < STEPS.length - 1 ? (
            <TouchableOpacity style={styles.nextButton} onPress={handleNext} testID="next-button">
              <Text style={styles.nextButtonText}>Next →</Text>
            </TouchableOpacity>
          ) : (
            <TouchableOpacity
              style={[styles.saveButton, isSaving && styles.saveButtonDisabled]}
              onPress={handleSave}
              disabled={isSaving}
              testID="save-button"
            >
              {isSaving ? (
                <ActivityIndicator size="small" color="#fff" />
              ) : (
                <Text style={styles.saveButtonText}>
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
    justifyContent: 'center',
    alignItems: 'center',
  },
  loadingText: {
    color: colors.text.secondary,
    marginTop: spacing.md,
    fontSize: fontSize.md,
  },

  // Header
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  headerButton: {
    width: 70,
  },
  headerButtonText: {
    color: colors.primary[500],
    fontSize: fontSize.md,
  },
  headerTitle: {
    color: colors.text.primary,
    fontSize: fontSize.lg,
    fontWeight: '600',
  },

  // Step Indicator
  stepIndicatorContainer: {
    alignItems: 'center',
    paddingVertical: spacing.md,
  },
  stepDots: {
    flexDirection: 'row',
    gap: spacing.md,
  },
  stepDot: {
    width: 32,
    height: 32,
    borderRadius: 16,
    backgroundColor: colors.background.tertiary,
    justifyContent: 'center',
    alignItems: 'center',
  },
  stepDotActive: {
    backgroundColor: colors.primary[500],
  },
  stepDotCompleted: {
    backgroundColor: colors.pierre.activity,
  },
  stepDotNumber: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    fontWeight: '600',
  },
  stepDotNumberActive: {
    color: '#fff',
  },
  stepDotCheckmark: {
    color: '#fff',
    fontSize: fontSize.sm,
    fontWeight: 'bold',
  },
  stepLabel: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
    marginTop: spacing.sm,
  },

  // Steps Content
  stepsScrollView: {
    flex: 1,
  },
  stepPage: {
    width: SCREEN_WIDTH,
    paddingHorizontal: spacing.md,
  },
  stepContent: {
    flex: 1,
    paddingTop: spacing.md,
  },

  // Input Groups
  inputGroup: {
    marginBottom: spacing.lg,
  },
  label: {
    color: colors.text.primary,
    fontSize: fontSize.sm,
    fontWeight: '600',
    marginBottom: spacing.xs,
  },
  labelRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: spacing.xs,
  },
  expandButton: {
    color: colors.primary[500],
    fontSize: fontSize.sm,
  },
  input: {
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
  },
  textArea: {
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
    minHeight: 80,
    textAlignVertical: 'top',
  },
  systemPromptInput: {
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
    minHeight: 200,
    flex: 1,
  },
  inputError: {
    borderColor: colors.error,
  },
  charCount: {
    color: colors.text.tertiary,
    fontSize: fontSize.xs,
    textAlign: 'right',
    marginTop: spacing.xs,
  },
  errorText: {
    color: colors.error,
    fontSize: fontSize.xs,
    marginTop: spacing.xs,
  },

  // Category Picker
  categoryPicker: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.md,
    padding: spacing.md,
  },
  categoryBadge: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
  },
  categoryBadgeText: {
    color: '#fff',
    fontSize: fontSize.sm,
    fontWeight: '600',
  },
  categoryPickerArrow: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
  },

  // Tags
  tagInputRow: {
    flexDirection: 'row',
    gap: spacing.sm,
  },
  tagInput: {
    flex: 1,
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
  },
  addTagButton: {
    backgroundColor: colors.primary[500],
    borderRadius: borderRadius.md,
    width: 44,
    justifyContent: 'center',
    alignItems: 'center',
  },
  addTagButtonText: {
    color: '#fff',
    fontSize: fontSize.xl,
    fontWeight: 'bold',
  },
  tagsContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: spacing.sm,
    marginTop: spacing.sm,
  },
  tagChip: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.background.tertiary,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
    gap: spacing.xs,
  },
  tagChipReview: {
    backgroundColor: colors.background.tertiary,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.full,
  },
  tagChipText: {
    color: colors.text.primary,
    fontSize: fontSize.sm,
  },
  tagChipRemove: {
    color: colors.text.secondary,
    fontSize: fontSize.lg,
    fontWeight: 'bold',
  },
  noTagsText: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
    fontStyle: 'italic',
  },

  // Token Counter
  tokenCounter: {
    marginTop: spacing.md,
  },
  tokenCountText: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    marginBottom: spacing.xs,
  },
  tokenBar: {
    height: 4,
    backgroundColor: colors.background.tertiary,
    borderRadius: 2,
    overflow: 'hidden',
  },
  tokenBarFill: {
    height: '100%',
    backgroundColor: colors.primary[500],
  },
  tokenCountSmall: {
    color: colors.text.tertiary,
    fontSize: fontSize.xs,
    marginTop: spacing.xs,
  },

  // Review
  reviewTitle: {
    color: colors.text.primary,
    fontSize: fontSize.xl,
    fontWeight: 'bold',
    marginBottom: spacing.lg,
  },
  reviewSection: {
    marginBottom: spacing.md,
  },
  reviewLabel: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    marginBottom: spacing.xs,
  },
  reviewValue: {
    color: colors.text.primary,
    fontSize: fontSize.md,
  },
  reviewPromptPreview: {
    color: colors.text.primary,
    fontSize: fontSize.sm,
    backgroundColor: colors.background.secondary,
    padding: spacing.md,
    borderRadius: borderRadius.md,
  },

  // Navigation Buttons
  navigationButtons: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.default,
  },
  navigationSpacer: {
    flex: 1,
  },
  backButton: {
    paddingVertical: spacing.sm,
    paddingHorizontal: spacing.md,
  },
  backButtonText: {
    color: colors.text.secondary,
    fontSize: fontSize.md,
  },
  nextButton: {
    backgroundColor: colors.primary[500],
    paddingVertical: spacing.sm,
    paddingHorizontal: spacing.lg,
    borderRadius: borderRadius.md,
  },
  nextButtonText: {
    color: '#fff',
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  saveButton: {
    backgroundColor: colors.pierre.activity,
    paddingVertical: spacing.sm,
    paddingHorizontal: spacing.lg,
    borderRadius: borderRadius.md,
    minWidth: 120,
    alignItems: 'center',
  },
  saveButtonDisabled: {
    opacity: 0.6,
  },
  saveButtonText: {
    color: '#fff',
    fontSize: fontSize.md,
    fontWeight: '600',
  },

  // Modal
  modalContainer: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  modalHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  modalCloseButton: {
    color: colors.primary[500],
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  modalTitle: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  modalTextArea: {
    flex: 1,
    padding: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
    textAlignVertical: 'top',
  },
  modalTokenCounter: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderTopWidth: 1,
    borderTopColor: colors.border.default,
  },
});
