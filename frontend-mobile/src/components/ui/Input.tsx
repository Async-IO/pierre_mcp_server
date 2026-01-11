// ABOUTME: Reusable text input component with dark theme styling
// ABOUTME: Supports labels, error states, and password visibility toggle

import React, { useState } from 'react';
import {
  View,
  TextInput,
  Text,
  StyleSheet,
  TouchableOpacity,
  type TextInputProps,
  type ViewStyle,
} from 'react-native';
import { colors, borderRadius, fontSize, spacing } from '../../constants/theme';

interface InputProps extends Omit<TextInputProps, 'style'> {
  label?: string;
  error?: string;
  containerStyle?: ViewStyle;
  showPasswordToggle?: boolean;
  testID?: string;
}

export function Input({
  label,
  error,
  containerStyle,
  showPasswordToggle = false,
  secureTextEntry,
  testID,
  ...props
}: InputProps) {
  const [isPasswordVisible, setIsPasswordVisible] = useState(false);

  const shouldHidePassword = secureTextEntry && !isPasswordVisible;

  return (
    <View style={[styles.container, containerStyle]}>
      {label && <Text style={styles.label}>{label}</Text>}
      <View style={styles.inputContainer}>
        <TextInput
          style={[
            styles.input,
            error && styles.inputError,
            showPasswordToggle && styles.inputWithToggle,
          ]}
          placeholderTextColor={colors.text.tertiary}
          selectionColor={colors.primary[500]}
          secureTextEntry={shouldHidePassword}
          testID={testID}
          {...props}
        />
        {showPasswordToggle && secureTextEntry !== undefined && (
          <TouchableOpacity
            style={styles.toggleButton}
            onPress={() => setIsPasswordVisible(!isPasswordVisible)}
          >
            <Text style={styles.toggleText}>
              {isPasswordVisible ? 'Hide' : 'Show'}
            </Text>
          </TouchableOpacity>
        )}
      </View>
      {error && <Text style={styles.errorText}>{error}</Text>}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    marginBottom: spacing.md,
  },
  label: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    marginBottom: spacing.xs,
    fontWeight: '500',
  },
  inputContainer: {
    position: 'relative',
    flexDirection: 'row',
    alignItems: 'center',
  },
  input: {
    flex: 1,
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.lg,
    paddingVertical: spacing.sm + 2,
    paddingHorizontal: spacing.md,
    color: colors.text.primary,
    fontSize: fontSize.md,
  },
  inputWithToggle: {
    paddingRight: 60,
  },
  inputError: {
    borderColor: colors.error,
  },
  toggleButton: {
    position: 'absolute',
    right: spacing.md,
    paddingVertical: spacing.xs,
  },
  toggleText: {
    color: colors.primary[500],
    fontSize: fontSize.sm,
    fontWeight: '500',
  },
  errorText: {
    color: colors.error,
    fontSize: fontSize.xs,
    marginTop: spacing.xs,
  },
});
