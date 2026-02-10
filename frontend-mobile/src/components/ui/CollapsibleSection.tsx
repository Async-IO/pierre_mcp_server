// ABOUTME: Reusable collapsible accordion section with animated chevron
// ABOUTME: Glass card styling with smooth expand/collapse transitions

import React, { useState, useCallback } from 'react';
import { View, Text, TouchableOpacity, LayoutAnimation, Platform, UIManager } from 'react-native';
import { Feather } from '@expo/vector-icons';
import { colors, glassCard } from '../../constants/theme';

// Enable LayoutAnimation on Android
if (Platform.OS === 'android' && UIManager.setLayoutAnimationEnabledExperimental) {
  UIManager.setLayoutAnimationEnabledExperimental(true);
}

interface CollapsibleSectionProps {
  title: string;
  defaultExpanded?: boolean;
  children: React.ReactNode;
  testID?: string;
}

export function CollapsibleSection({
  title,
  defaultExpanded = false,
  children,
  testID,
}: CollapsibleSectionProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);

  const toggle = useCallback(() => {
    LayoutAnimation.configureNext(LayoutAnimation.Presets.easeInEaseOut);
    setExpanded((prev) => !prev);
  }, []);

  return (
    <View
      className="mb-5 overflow-hidden"
      style={{
        ...glassCard,
        borderRadius: 12,
      }}
      testID={testID}
    >
      <TouchableOpacity
        className="flex-row items-center justify-between p-3.5"
        onPress={toggle}
        activeOpacity={0.7}
        testID={testID ? `${testID}-toggle` : undefined}
      >
        <Text className="text-text-primary text-sm font-semibold">{title}</Text>
        <Feather
          name={expanded ? 'chevron-up' : 'chevron-down'}
          size={18}
          color={colors.text.secondary}
        />
      </TouchableOpacity>

      {expanded && (
        <View className="px-3.5 pb-3.5" testID={testID ? `${testID}-content` : undefined}>
          {children}
        </View>
      )}
    </View>
  );
}
