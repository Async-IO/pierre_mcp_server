// ABOUTME: Toast notification configuration for react-native-toast-message
// ABOUTME: Dark theme toast styles matching Pierre design system

import React from 'react';
import { View, Text, StyleSheet, TouchableOpacity } from 'react-native';
import { colors, spacing, fontSize, borderRadius } from '../constants/theme';
import type { ToastConfig, ToastConfigParams, BaseToastProps } from 'react-native-toast-message';

interface VoiceToastProps extends BaseToastProps {
  onRetry?: () => void;
  onOpenSettings?: () => void;
}

// Custom toast component for voice input errors
function VoiceToast({ text1, text2, onRetry, onOpenSettings }: VoiceToastProps) {
  return (
    <View style={styles.container}>
      <View style={styles.content}>
        <Text style={styles.title}>{text1}</Text>
        {text2 && <Text style={styles.message}>{text2}</Text>}
      </View>
      <View style={styles.actions}>
        {onOpenSettings && (
          <TouchableOpacity onPress={onOpenSettings} style={styles.actionButton}>
            <Text style={styles.actionText}>Settings</Text>
          </TouchableOpacity>
        )}
        {onRetry && (
          <TouchableOpacity onPress={onRetry} style={styles.actionButton}>
            <Text style={styles.actionText}>Retry</Text>
          </TouchableOpacity>
        )}
      </View>
    </View>
  );
}

// Standard error toast
function ErrorToast({ text1, text2 }: BaseToastProps) {
  return (
    <View style={[styles.container, styles.errorContainer]}>
      <View style={styles.content}>
        <Text style={styles.title}>{text1}</Text>
        {text2 && <Text style={styles.message}>{text2}</Text>}
      </View>
    </View>
  );
}

// Info toast
function InfoToast({ text1, text2 }: BaseToastProps) {
  return (
    <View style={[styles.container, styles.infoContainer]}>
      <View style={styles.content}>
        <Text style={styles.title}>{text1}</Text>
        {text2 && <Text style={styles.message}>{text2}</Text>}
      </View>
    </View>
  );
}

export const toastConfig: ToastConfig = {
  error: (props: ToastConfigParams<BaseToastProps>) => <ErrorToast {...props} />,
  info: (props: ToastConfigParams<BaseToastProps>) => <InfoToast {...props} />,
  voiceError: (props: ToastConfigParams<VoiceToastProps>) => (
    <VoiceToast
      {...props}
      onRetry={props.props?.onRetry}
      onOpenSettings={props.props?.onOpenSettings}
    />
  ),
};

const styles = StyleSheet.create({
  container: {
    width: '90%',
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.lg,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    borderWidth: 1,
    borderColor: colors.border.default,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 8,
  },
  errorContainer: {
    borderColor: colors.error,
  },
  infoContainer: {
    borderColor: colors.primary[500],
  },
  content: {
    flex: 1,
    marginRight: spacing.sm,
  },
  title: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
  },
  message: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    marginTop: 2,
  },
  actions: {
    flexDirection: 'row',
    gap: spacing.sm,
  },
  actionButton: {
    backgroundColor: colors.background.tertiary,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.md,
  },
  actionText: {
    fontSize: fontSize.sm,
    fontWeight: '500',
    color: colors.primary[400],
  },
});
