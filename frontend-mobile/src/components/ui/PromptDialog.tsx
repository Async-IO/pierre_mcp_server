// ABOUTME: Cross-platform prompt dialog component replacing iOS-only Alert.prompt
// ABOUTME: Works on both iOS and Android with text input in a modal

import React, { useState, useEffect, useRef } from 'react';
import {
  View,
  Text,
  TextInput,
  Modal,
  TouchableOpacity,
  StyleSheet,
  KeyboardAvoidingView,
  Platform,
  TouchableWithoutFeedback,
  Keyboard,
} from 'react-native';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';

interface PromptDialogProps {
  visible: boolean;
  title: string;
  message?: string;
  placeholder?: string;
  defaultValue?: string;
  submitText?: string;
  cancelText?: string;
  onSubmit: (value: string) => void;
  onCancel: () => void;
  testID?: string;
}

export function PromptDialog({
  visible,
  title,
  message,
  placeholder = '',
  defaultValue = '',
  submitText = 'Save',
  cancelText = 'Cancel',
  onSubmit,
  onCancel,
  testID,
}: PromptDialogProps) {
  const [inputValue, setInputValue] = useState(defaultValue);
  const inputRef = useRef<TextInput>(null);

  // Reset input value when dialog opens with new default value
  useEffect(() => {
    if (visible) {
      setInputValue(defaultValue);
      // Focus the input after a brief delay to ensure modal is rendered
      const timer = setTimeout(() => {
        inputRef.current?.focus();
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [visible, defaultValue]);

  const handleSubmit = () => {
    const trimmed = inputValue.trim();
    if (trimmed) {
      onSubmit(trimmed);
      setInputValue('');
    }
  };

  const handleCancel = () => {
    setInputValue('');
    onCancel();
  };

  const dismissKeyboard = () => {
    Keyboard.dismiss();
  };

  return (
    <Modal
      visible={visible}
      animationType="fade"
      transparent
      onRequestClose={handleCancel}
      testID={testID}
    >
      <TouchableWithoutFeedback onPress={dismissKeyboard}>
        <View style={styles.overlay}>
          <KeyboardAvoidingView
            behavior={Platform.OS === 'ios' ? 'padding' : undefined}
            style={styles.keyboardView}
          >
            <View style={styles.container}>
              <Text style={styles.title}>{title}</Text>
              {message && <Text style={styles.message}>{message}</Text>}

              <TextInput
                ref={inputRef}
                style={styles.input}
                value={inputValue}
                onChangeText={setInputValue}
                placeholder={placeholder}
                placeholderTextColor={colors.text.tertiary}
                selectionColor={colors.primary[500]}
                autoCapitalize="sentences"
                autoCorrect
                returnKeyType="done"
                onSubmitEditing={handleSubmit}
                testID={testID ? `${testID}-input` : undefined}
              />

              <View style={styles.buttonContainer}>
                <TouchableOpacity
                  style={[styles.button, styles.cancelButton]}
                  onPress={handleCancel}
                  testID={testID ? `${testID}-cancel` : undefined}
                >
                  <Text style={styles.cancelButtonText}>{cancelText}</Text>
                </TouchableOpacity>

                <TouchableOpacity
                  style={[
                    styles.button,
                    styles.submitButton,
                    !inputValue.trim() && styles.submitButtonDisabled,
                  ]}
                  onPress={handleSubmit}
                  disabled={!inputValue.trim()}
                  testID={testID ? `${testID}-submit` : undefined}
                >
                  <Text
                    style={[
                      styles.submitButtonText,
                      !inputValue.trim() && styles.submitButtonTextDisabled,
                    ]}
                  >
                    {submitText}
                  </Text>
                </TouchableOpacity>
              </View>
            </View>
          </KeyboardAvoidingView>
        </View>
      </TouchableWithoutFeedback>
    </Modal>
  );
}

const styles = StyleSheet.create({
  overlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
    justifyContent: 'center',
    alignItems: 'center',
  },
  keyboardView: {
    width: '100%',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
  },
  container: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.xl,
    padding: spacing.lg,
    width: '100%',
    maxWidth: 320,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 8 },
    shadowOpacity: 0.4,
    shadowRadius: 16,
    elevation: 12,
  },
  title: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
    marginBottom: spacing.xs,
  },
  message: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    textAlign: 'center',
    marginBottom: spacing.md,
  },
  input: {
    backgroundColor: colors.background.tertiary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.sm + 2,
    paddingHorizontal: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
    marginBottom: spacing.lg,
  },
  buttonContainer: {
    flexDirection: 'row',
    gap: spacing.sm,
  },
  button: {
    flex: 1,
    paddingVertical: spacing.sm + 2,
    borderRadius: borderRadius.md,
    alignItems: 'center',
    justifyContent: 'center',
  },
  cancelButton: {
    backgroundColor: colors.background.tertiary,
  },
  cancelButtonText: {
    fontSize: fontSize.md,
    fontWeight: '500',
    color: colors.text.secondary,
  },
  submitButton: {
    backgroundColor: colors.primary[600],
  },
  submitButtonDisabled: {
    backgroundColor: colors.background.tertiary,
  },
  submitButtonText: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
  },
  submitButtonTextDisabled: {
    color: colors.text.tertiary,
  },
});
