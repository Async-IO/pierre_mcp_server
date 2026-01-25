// ABOUTME: Floating search bar component positioned at bottom of screen
// ABOUTME: Keyboard-aware with glass effect per iOS design guidelines

import React, { useRef, useEffect, useState } from 'react';
import {
  View,
  TextInput,
  TouchableOpacity,
  ActivityIndicator,
  Keyboard,
  Animated,
  Platform,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { Feather } from '@expo/vector-icons';
import { colors, spacing } from '../../constants/theme';

// Glass effect container with shadow
const containerStyle: ViewStyle = {
  backgroundColor: colors.background.secondary + 'F5', // 96% opacity
  borderTopWidth: 1,
  borderTopColor: colors.border.default + '40',
  shadowColor: '#000',
  shadowOffset: { width: 0, height: -4 },
  shadowOpacity: 0.15,
  shadowRadius: 8,
  elevation: 8,
};

interface FloatingSearchBarProps {
  value: string;
  onChangeText: (text: string) => void;
  onSubmit?: () => void;
  placeholder?: string;
  isSearching?: boolean;
  testID?: string;
  autoFocus?: boolean;
}

export function FloatingSearchBar({
  value,
  onChangeText,
  onSubmit,
  placeholder = 'Search...',
  isSearching = false,
  testID,
  autoFocus = false,
}: FloatingSearchBarProps) {
  const insets = useSafeAreaInsets();
  const inputRef = useRef<TextInput>(null);
  const [keyboardHeight, setKeyboardHeight] = useState(0);
  const translateY = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    const showEvent = Platform.OS === 'ios' ? 'keyboardWillShow' : 'keyboardDidShow';
    const hideEvent = Platform.OS === 'ios' ? 'keyboardWillHide' : 'keyboardDidHide';

    const showSubscription = Keyboard.addListener(showEvent, (e) => {
      const height = e.endCoordinates.height;
      setKeyboardHeight(height);
      Animated.spring(translateY, {
        toValue: -height + insets.bottom,
        useNativeDriver: true,
        damping: 20,
        stiffness: 300,
      }).start();
    });

    const hideSubscription = Keyboard.addListener(hideEvent, () => {
      setKeyboardHeight(0);
      Animated.spring(translateY, {
        toValue: 0,
        useNativeDriver: true,
        damping: 20,
        stiffness: 300,
      }).start();
    });

    return () => {
      showSubscription.remove();
      hideSubscription.remove();
    };
  }, [translateY, insets.bottom]);

  const handleClear = () => {
    onChangeText('');
    inputRef.current?.focus();
  };

  const handleSubmit = () => {
    Keyboard.dismiss();
    onSubmit?.();
  };

  return (
    <Animated.View
      style={[
        containerStyle,
        {
          position: 'absolute',
          bottom: 0,
          left: 0,
          right: 0,
          paddingBottom: keyboardHeight > 0 ? spacing.sm : insets.bottom + spacing.sm,
          paddingTop: spacing.sm,
          paddingHorizontal: spacing.md,
          transform: [{ translateY }],
        },
      ]}
      testID={testID ? `${testID}-container` : undefined}
    >
      <View className="flex-row items-center bg-background-tertiary rounded-lg px-4 py-3">
        <Feather name="search" size={18} color={colors.text.tertiary} />
        <TextInput
          ref={inputRef}
          testID={testID}
          className="flex-1 ml-2 text-text-primary text-base"
          placeholder={placeholder}
          placeholderTextColor={colors.text.tertiary}
          value={value}
          onChangeText={onChangeText}
          autoCapitalize="none"
          autoCorrect={false}
          returnKeyType="search"
          onSubmitEditing={handleSubmit}
          autoFocus={autoFocus}
        />
        {isSearching ? (
          <ActivityIndicator size="small" color={colors.primary[500]} />
        ) : value.length > 0 ? (
          <TouchableOpacity
            onPress={handleClear}
            hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
          >
            <Feather name="x" size={18} color={colors.text.tertiary} />
          </TouchableOpacity>
        ) : null}
      </View>
    </Animated.View>
  );
}
