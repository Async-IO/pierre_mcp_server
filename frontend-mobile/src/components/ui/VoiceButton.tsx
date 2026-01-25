// ABOUTME: Voice input button component with recording state indicator
// ABOUTME: Provides visual feedback during speech recognition with pulse animation

import React, { useEffect, useRef } from 'react';
import {
  TouchableOpacity,
  Animated,
  View,
  ActivityIndicator,
  type ViewStyle,
} from 'react-native';
import { colors, borderRadius } from '../../constants/theme';

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

const ICON_SCALES = {
  sm: 0.7,
  md: 0.85,
  lg: 1,
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
  const iconScale = ICON_SCALES[size];

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

  // Microphone icon styles (pixel-specific, need style objects)
  const micHeadStyle: ViewStyle = {
    width: 12,
    height: 16,
    backgroundColor: colors.text.primary,
    borderTopLeftRadius: borderRadius.md,
    borderTopRightRadius: borderRadius.md,
  };

  const micBodyStyle: ViewStyle = {
    width: 18,
    height: 6,
    borderBottomLeftRadius: 9,
    borderBottomRightRadius: 9,
    borderWidth: 2,
    borderColor: colors.text.primary,
    borderTopWidth: 0,
    marginTop: -2,
  };

  const micStandStyle: ViewStyle = {
    width: 2,
    height: 5,
    backgroundColor: colors.text.primary,
    marginTop: 1,
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
          <View
            className="items-center"
            style={{ transform: [{ scale: iconScale }] }}
          >
            <View style={micHeadStyle} />
            <View style={micBodyStyle} />
            <View style={micStandStyle} />
          </View>
        )}
      </Animated.View>
    </TouchableOpacity>
  );
}
