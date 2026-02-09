// ABOUTME: Visual drag indicator pill for modal sheets
// ABOUTME: Signals to users that the modal can be dismissed by swiping down

import React from 'react';
import { View } from 'react-native';

interface DragIndicatorProps {
  testID?: string;
}

export function DragIndicator({ testID }: DragIndicatorProps) {
  return (
    <View className="items-center pt-2 pb-1" testID={testID}>
      <View
        className="w-9 h-1 rounded-full"
        style={{ backgroundColor: 'rgba(255, 255, 255, 0.3)' }}
      />
    </View>
  );
}
