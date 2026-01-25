// ABOUTME: Reusable text input component with dark theme and glass styling per Stitch design
// ABOUTME: Supports labels, error states, password visibility toggle, and glass variant

import React, { useState } from 'react';
import {
  View,
  TextInput,
  Text,
  TouchableOpacity,
  type TextInputProps,
  type ViewStyle,
} from 'react-native';
import { colors } from '../../constants/theme';

interface InputProps extends Omit<TextInputProps, 'style'> {
  label?: string;
  error?: string;
  containerStyle?: ViewStyle;
  showPasswordToggle?: boolean;
  variant?: 'default' | 'glass';
  testID?: string;
}

export function Input({
  label,
  error,
  containerStyle,
  showPasswordToggle = false,
  variant = 'default',
  secureTextEntry,
  testID,
  ...props
}: InputProps) {
  const [isPasswordVisible, setIsPasswordVisible] = useState(false);

  const shouldHidePassword = secureTextEntry && !isPasswordVisible;

  const inputBaseClasses = 'flex-1 py-2.5 px-4 text-text-primary text-base';
  const inputDefaultClasses = 'bg-background-secondary border border-border-default rounded-xl';
  const inputGlassClasses = 'bg-white/[0.03] border border-white/[0.08] rounded-2xl';
  const inputErrorClasses = error ? 'border-error' : '';
  const inputToggleClasses = showPasswordToggle ? 'pr-16' : '';

  const inputClassName = [
    inputBaseClasses,
    variant === 'glass' ? inputGlassClasses : inputDefaultClasses,
    inputErrorClasses,
    inputToggleClasses,
  ].filter(Boolean).join(' ');

  return (
    <View className="mb-4" style={containerStyle}>
      {label && (
        <Text className="text-text-secondary text-sm mb-1 font-medium">
          {label}
        </Text>
      )}
      <View className="relative flex-row items-center">
        <TextInput
          className={inputClassName}
          placeholderTextColor={colors.text.tertiary}
          selectionColor={colors.primary[500]}
          secureTextEntry={shouldHidePassword}
          testID={testID}
          {...props}
        />
        {showPasswordToggle && secureTextEntry !== undefined && (
          <TouchableOpacity
            className="absolute right-4 py-1"
            onPress={() => setIsPasswordVisible(!isPasswordVisible)}
          >
            <Text className="text-primary-500 text-sm font-medium">
              {isPasswordVisible ? 'Hide' : 'Show'}
            </Text>
          </TouchableOpacity>
        )}
      </View>
      {error && (
        <Text className="text-error text-xs mt-1">{error}</Text>
      )}
    </View>
  );
}
