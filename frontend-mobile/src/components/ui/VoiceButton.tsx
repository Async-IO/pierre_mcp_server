// ABOUTME: Voice input button component with recording state indicator
// ABOUTME: Provides visual feedback during speech recognition with pulse animation

import React, { useEffect, useRef } from 'react';
import {
  TouchableOpacity,
  Animated,
  ActivityIndicator,
  type ViewStyle,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { colors } from '../../constants/theme';

interface VoiceButtonProps {
  isListening: boolean;
  isAvailable: boolean;
  onPress: () => void;
  disabled?: boolean;
  size?: 'sm' | 'md' | 'lg';
  testID?: string;
}

const BUTTON_SIZES = {
  sm: 32,
  md: 40,
  lg: 48,
} as const;

const ICON_SIZES = {
  sm: 16,
  md: 20,
  lg: 24,
} as const;

export function VoiceButton({
  isListening,
  isAvailable,
  onPress,
  disabled = false,
  size = 'md',
  testID,
}: VoiceButtonProps) {
  const pulseAnim = useRef(new Animated.Value(1)).current;
  const buttonSize = BUTTON_SIZES[size];
  const iconSize = ICON_SIZES[size];

  useEffect(() => {
    if (isListening) {
      // Pulse animation while listening
      const animation = Animated.loop(
        Animated.sequence([
          Animated.timing(pulseAnim, {
            toValue: 1.15,
            duration: 600,
            useNativeDriver: true,
          }),
          Animated.timing(pulseAnim, {
            toValue: 1,
            duration: 600,
            useNativeDriver: true,
          }),
        ])
      );
      animation.start();
      return () => animation.stop();
    } else {
      pulseAnim.setValue(1);
    }
  }, [isListening, pulseAnim]);

  // Hide button if voice recognition not available
  if (!isAvailable) {
    return null;
  }

  const isDisabled = disabled || !isAvailable;

  // Dynamic button style (size-based, cannot use className)
  const buttonStyle: ViewStyle = {
    width: buttonSize,
    height: buttonSize,
    borderRadius: buttonSize / 2,
    backgroundColor: isListening ? colors.error : colors.background.tertiary,
  };

  return (
    <TouchableOpacity
      className={`items-center justify-center ${isDisabled ? 'opacity-50' : ''}`}
      style={buttonStyle}
      onPress={onPress}
      disabled={isDisabled}
      activeOpacity={0.7}
      testID={testID}
      accessibilityLabel={isListening ? 'Stop voice input' : 'Start voice input'}
      accessibilityRole="button"
      accessibilityState={{ disabled: isDisabled }}
    >
      <Animated.View
        className="items-center justify-center"
        style={{ transform: [{ scale: isListening ? pulseAnim : 1 }] }}
      >
        {isListening ? (
          <ActivityIndicator size="small" color={colors.text.primary} />
        ) : (
          <Ionicons name="mic-outline" size={iconSize} color={colors.text.primary} />
        )}
      </Animated.View>
    </TouchableOpacity>
  );
}
