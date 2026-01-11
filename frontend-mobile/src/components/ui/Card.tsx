// ABOUTME: Reusable card component for content containers
// ABOUTME: Supports elevated and flat variants with consistent padding

import React, { type ReactNode } from 'react';
import { View, StyleSheet, type ViewStyle } from 'react-native';
import { colors, borderRadius, spacing } from '../../constants/theme';

interface CardProps {
  children: ReactNode;
  variant?: 'default' | 'elevated';
  style?: ViewStyle;
  noPadding?: boolean;
}

export function Card({
  children,
  variant = 'default',
  style,
  noPadding = false,
}: CardProps) {
  return (
    <View
      style={[
        styles.base,
        variant === 'elevated' && styles.elevated,
        !noPadding && styles.padding,
        style,
      ]}
    >
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  base: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.lg,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  elevated: {
    backgroundColor: colors.background.elevated,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.25,
    shadowRadius: 4,
    elevation: 4,
  },
  padding: {
    padding: spacing.md,
  },
});
