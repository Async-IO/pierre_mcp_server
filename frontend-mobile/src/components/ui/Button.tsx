// ABOUTME: Reusable button component with variants following Pierre/Stitch design system
// ABOUTME: Supports primary, secondary, ghost, danger, gradient, pill, and pillar variants

import React from 'react';
import {
  TouchableOpacity,
  Text,
  StyleSheet,
  ActivityIndicator,
  type ViewStyle,
  type TextStyle,
} from 'react-native';
import { colors, borderRadius, fontSize, spacing, buttonGlow } from '../../constants/theme';

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'gradient' | 'pill' | 'activity' | 'nutrition' | 'recovery';
type ButtonSize = 'sm' | 'md' | 'lg';

interface ButtonProps {
  title: string;
  onPress: () => void;
  variant?: ButtonVariant;
  size?: ButtonSize;
  disabled?: boolean;
  loading?: boolean;
  fullWidth?: boolean;
  style?: ViewStyle;
  textStyle?: TextStyle;
  testID?: string;
}

export function Button({
  title,
  onPress,
  variant = 'primary',
  size = 'md',
  disabled = false,
  loading = false,
  fullWidth = false,
  style,
  textStyle,
  testID,
}: ButtonProps) {
  const isDisabled = disabled || loading;

  const buttonStyles = [
    styles.base,
    styles[`variant_${variant}`],
    styles[`size_${size}`],
    fullWidth && styles.fullWidth,
    isDisabled && styles.disabled,
    style,
  ];

  const textStyles = [
    styles.text,
    styles[`text_${variant}`],
    styles[`textSize_${size}`],
    isDisabled && styles.textDisabled,
    textStyle,
  ];

  return (
    <TouchableOpacity
      style={buttonStyles}
      onPress={onPress}
      disabled={isDisabled}
      activeOpacity={0.7}
      testID={testID}
    >
      {loading ? (
        <ActivityIndicator
          color={['primary', 'gradient', 'pill', 'danger', 'activity', 'nutrition', 'recovery'].includes(variant) ? colors.text.primary : colors.primary[500]}
          size="small"
        />
      ) : (
        <Text style={textStyles}>{title}</Text>
      )}
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  base: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: borderRadius.lg,
  },

  // Variants
  variant_primary: {
    backgroundColor: colors.pierre.violet,
  },
  variant_secondary: {
    backgroundColor: 'transparent',
    borderWidth: 1,
    borderColor: colors.border.strong,
  },
  variant_ghost: {
    backgroundColor: 'transparent',
  },
  variant_danger: {
    backgroundColor: colors.error,
  },
  // Gradient variant with glow effect - per Stitch design system
  variant_gradient: {
    backgroundColor: colors.pierre.violet,
    ...buttonGlow,
  },
  // Pill-shaped variant with full border radius and glow
  variant_pill: {
    backgroundColor: colors.pierre.violet,
    borderRadius: borderRadius.full,
    ...buttonGlow,
  },
  // Three Pillar Button Variants - per Stitch design system
  variant_activity: {
    backgroundColor: colors.pierre.activity,
    shadowColor: colors.pierre.activity,
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.25,
    shadowRadius: 14,
    elevation: 8,
  },
  variant_nutrition: {
    backgroundColor: colors.pierre.nutrition,
    shadowColor: colors.pierre.nutrition,
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.25,
    shadowRadius: 14,
    elevation: 8,
  },
  variant_recovery: {
    backgroundColor: colors.pierre.recovery,
    shadowColor: colors.pierre.recovery,
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.25,
    shadowRadius: 14,
    elevation: 8,
  },

  // Sizes
  size_sm: {
    paddingVertical: spacing.xs,
    paddingHorizontal: spacing.md,
    minHeight: 36,
  },
  size_md: {
    paddingVertical: spacing.sm,
    paddingHorizontal: spacing.lg,
    minHeight: 44,
  },
  size_lg: {
    paddingVertical: spacing.md,
    paddingHorizontal: spacing.xl,
    minHeight: 52,
  },

  fullWidth: {
    width: '100%',
  },

  disabled: {
    opacity: 0.5,
  },

  // Text styles
  text: {
    fontWeight: '600',
  },
  text_primary: {
    color: colors.text.primary,
  },
  text_secondary: {
    color: colors.text.primary,
  },
  text_ghost: {
    color: colors.primary[500],
  },
  text_danger: {
    color: colors.text.primary,
  },
  text_gradient: {
    color: colors.text.primary,
  },
  text_pill: {
    color: colors.text.primary,
  },
  text_activity: {
    color: colors.text.primary,
  },
  text_nutrition: {
    color: colors.text.primary,
  },
  text_recovery: {
    color: colors.text.primary,
  },

  textSize_sm: {
    fontSize: fontSize.sm,
  },
  textSize_md: {
    fontSize: fontSize.md,
  },
  textSize_lg: {
    fontSize: fontSize.lg,
  },

  textDisabled: {
    opacity: 0.7,
  },
});
