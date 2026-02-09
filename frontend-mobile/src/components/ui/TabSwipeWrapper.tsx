// ABOUTME: Wrapper component enabling horizontal swipe between bottom tab screens
// ABOUTME: Uses PanGesture to detect left/right swipes and programmatically switches tabs

import React, { useCallback } from 'react';
import { Dimensions } from 'react-native';
import { Gesture, GestureDetector } from 'react-native-gesture-handler';
import Animated, { runOnJS } from 'react-native-reanimated';
import * as Haptics from 'expo-haptics';
import { useNavigation } from '@react-navigation/native';
import type { BottomTabNavigationProp } from '@react-navigation/bottom-tabs';
import type { MainTabsParamList } from '../../navigation/MainTabs';

const { width: SCREEN_WIDTH } = Dimensions.get('window');
const SWIPE_THRESHOLD = SCREEN_WIDTH * 0.25;
const VELOCITY_THRESHOLD = 800;

// Ordered tab route names for swipe navigation
const TAB_ORDER: (keyof MainTabsParamList)[] = [
  'ChatTab',
  'CoachesTab',
  'DiscoverTab',
  'SocialTab',
  'SettingsTab',
];

interface TabSwipeWrapperProps {
  children: React.ReactNode;
  tabName: keyof MainTabsParamList;
  /** Disable swipe (e.g., when in a nested detail screen) */
  enabled?: boolean;
}

export function TabSwipeWrapper({ children, tabName, enabled = true }: TabSwipeWrapperProps) {
  const navigation = useNavigation<BottomTabNavigationProp<MainTabsParamList>>();

  const currentIndex = TAB_ORDER.indexOf(tabName);

  const navigateToTab = useCallback((direction: 'left' | 'right') => {
    const targetIndex = direction === 'left' ? currentIndex + 1 : currentIndex - 1;
    if (targetIndex >= 0 && targetIndex < TAB_ORDER.length) {
      Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
      navigation.navigate(TAB_ORDER[targetIndex]);
    }
  }, [currentIndex, navigation]);

  const panGesture = Gesture.Pan()
    .enabled(enabled)
    .activeOffsetX([-20, 20])
    .failOffsetY([-10, 10])
    .onEnd((event) => {
      const { translationX, velocityX } = event;

      // Swipe left (go to next tab)
      if (translationX < -SWIPE_THRESHOLD || velocityX < -VELOCITY_THRESHOLD) {
        if (currentIndex < TAB_ORDER.length - 1) {
          runOnJS(navigateToTab)('left');
        }
        return;
      }

      // Swipe right (go to previous tab)
      if (translationX > SWIPE_THRESHOLD || velocityX > VELOCITY_THRESHOLD) {
        if (currentIndex > 0) {
          runOnJS(navigateToTab)('right');
        }
      }
    });

  return (
    <GestureDetector gesture={panGesture}>
      <Animated.View style={{ flex: 1 }}>
        {children}
      </Animated.View>
    </GestureDetector>
  );
}

