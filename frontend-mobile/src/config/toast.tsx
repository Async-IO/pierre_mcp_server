// ABOUTME: Toast notification configuration for react-native-toast-message
// ABOUTME: Dark theme toast styles matching Pierre design system

import React from 'react';
import { View, Text, TouchableOpacity, type ViewStyle } from 'react-native';
import { colors } from '../constants/theme';
import type { ToastConfig, ToastConfigParams, BaseToastProps } from 'react-native-toast-message';

interface VoiceToastProps extends BaseToastProps {
  onRetry?: () => void;
  onOpenSettings?: () => void;
}

// Shadow style for toast container (React Native shadows cannot use className)
const toastShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 4 },
  shadowOpacity: 0.3,
  shadowRadius: 8,
  elevation: 8,
};

// Custom toast component for voice input errors
function VoiceToast({ text1, text2, onRetry, onOpenSettings }: VoiceToastProps) {
  return (
    <View
      className="w-[90%] bg-background-secondary rounded-lg px-3 py-2 flex-row items-center justify-between border border-border-default"
      style={toastShadow}
    >
      <View className="flex-1 mr-2">
        <Text className="text-base font-semibold text-text-primary">{text1}</Text>
        {text2 && <Text className="text-sm text-text-secondary mt-0.5">{text2}</Text>}
      </View>
      <View className="flex-row gap-2">
        {onOpenSettings && (
          <TouchableOpacity onPress={onOpenSettings} className="bg-background-tertiary px-3 py-1 rounded-lg">
            <Text className="text-sm font-medium text-primary-400">Settings</Text>
          </TouchableOpacity>
        )}
        {onRetry && (
          <TouchableOpacity onPress={onRetry} className="bg-background-tertiary px-3 py-1 rounded-lg">
            <Text className="text-sm font-medium text-primary-400">Retry</Text>
          </TouchableOpacity>
        )}
      </View>
    </View>
  );
}

// Standard error toast
function ErrorToast({ text1, text2 }: BaseToastProps) {
  return (
    <View
      className="w-[90%] bg-background-secondary rounded-lg px-3 py-2 flex-row items-center justify-between border border-error"
      style={toastShadow}
    >
      <View className="flex-1 mr-2">
        <Text className="text-base font-semibold text-text-primary">{text1}</Text>
        {text2 && <Text className="text-sm text-text-secondary mt-0.5">{text2}</Text>}
      </View>
    </View>
  );
}

// Info toast
function InfoToast({ text1, text2 }: BaseToastProps) {
  return (
    <View
      className="w-[90%] bg-background-secondary rounded-lg px-3 py-2 flex-row items-center justify-between"
      style={[{ borderWidth: 1, borderColor: colors.primary[500] }, toastShadow]}
    >
      <View className="flex-1 mr-2">
        <Text className="text-base font-semibold text-text-primary">{text1}</Text>
        {text2 && <Text className="text-sm text-text-secondary mt-0.5">{text2}</Text>}
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
