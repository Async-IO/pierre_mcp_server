// ABOUTME: Cross-platform prompt dialog component replacing iOS-only Alert.prompt
// ABOUTME: Works on both iOS and Android with text input in a modal

import React, { useState, useEffect, useRef } from 'react';
import {
  View,
  Text,
  TextInput,
  Modal,
  TouchableOpacity,
  KeyboardAvoidingView,
  Platform,
  TouchableWithoutFeedback,
  Keyboard,
  type ViewStyle,
} from 'react-native';
import { colors } from '../../constants/theme';

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

// Shadow styles need style objects in React Native
const containerShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 8 },
  shadowOpacity: 0.4,
  shadowRadius: 16,
  elevation: 12,
};

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

  const isSubmitDisabled = !inputValue.trim();

  return (
    <Modal
      visible={visible}
      animationType="fade"
      transparent
      onRequestClose={handleCancel}
      testID={testID}
    >
      <TouchableWithoutFeedback onPress={dismissKeyboard}>
        <View className="flex-1 bg-black/50 justify-center items-center">
          <KeyboardAvoidingView
            behavior={Platform.OS === 'ios' ? 'padding' : undefined}
            className="w-full items-center px-6"
          >
            <View
              className="bg-background-secondary rounded-2xl p-6 w-full max-w-[320px]"
              style={containerShadow}
            >
              <Text className="text-lg font-semibold text-text-primary text-center mb-1">
                {title}
              </Text>
              {message && (
                <Text className="text-sm text-text-secondary text-center mb-4">
                  {message}
                </Text>
              )}

              <TextInput
                ref={inputRef}
                className="bg-background-tertiary border border-border-default rounded-lg py-2.5 px-4 text-text-primary text-base mb-6"
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

              <View className="flex-row gap-2">
                <TouchableOpacity
                  className="flex-1 py-2.5 rounded-lg items-center justify-center bg-background-tertiary"
                  onPress={handleCancel}
                  testID={testID ? `${testID}-cancel` : undefined}
                >
                  <Text className="text-base font-medium text-text-secondary">
                    {cancelText}
                  </Text>
                </TouchableOpacity>

                <TouchableOpacity
                  className={`flex-1 py-2.5 rounded-lg items-center justify-center ${
                    isSubmitDisabled ? 'bg-background-tertiary' : 'bg-primary-600'
                  }`}
                  onPress={handleSubmit}
                  disabled={isSubmitDisabled}
                  testID={testID ? `${testID}-submit` : undefined}
                >
                  <Text
                    className={`text-base font-semibold ${
                      isSubmitDisabled ? 'text-text-tertiary' : 'text-text-primary'
                    }`}
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
