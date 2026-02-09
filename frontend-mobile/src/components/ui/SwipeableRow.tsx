// ABOUTME: Reusable swipeable row component using react-native-gesture-handler and reanimated
// ABOUTME: Reveals action buttons on left/right swipe with haptic feedback and spring animation

import React, { useCallback } from 'react';
import { View, Text, TouchableOpacity, type ViewStyle } from 'react-native';
import { Gesture, GestureDetector } from 'react-native-gesture-handler';
import Animated, {
  useSharedValue,
  useAnimatedStyle,
  withSpring,
  runOnJS,
  interpolate,
  Extrapolation,
} from 'react-native-reanimated';
import * as Haptics from 'expo-haptics';
import { Feather } from '@expo/vector-icons';
import type { ComponentProps } from 'react';

type FeatherIconName = ComponentProps<typeof Feather>['name'];

export interface SwipeAction {
  icon: FeatherIconName;
  label: string;
  color: string;
  backgroundColor: string;
  onPress: () => void;
}

interface SwipeableRowProps {
  children: React.ReactNode;
  leftActions?: SwipeAction[];
  rightActions?: SwipeAction[];
  /** Width of the action panel revealed on swipe */
  actionWidth?: number;
  /** Whether full swipe triggers the first action */
  fullSwipeEnabled?: boolean;
  testID?: string;
}

const SPRING_CONFIG = {
  damping: 20,
  stiffness: 200,
  mass: 0.5,
};

const ACTION_BUTTON_WIDTH = 72;
const FULL_SWIPE_THRESHOLD_RATIO = 0.6;

export function SwipeableRow({
  children,
  leftActions = [],
  rightActions = [],
  actionWidth,
  fullSwipeEnabled = true,
  testID,
}: SwipeableRowProps) {
  const translateX = useSharedValue(0);
  const contextX = useSharedValue(0);

  const leftPanelWidth = actionWidth ?? leftActions.length * ACTION_BUTTON_WIDTH;
  const rightPanelWidth = actionWidth ?? rightActions.length * ACTION_BUTTON_WIDTH;

  const triggerHaptic = useCallback(() => {
    Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Medium);
  }, []);

  // Callbacks that capture action references on the JS side.
  // runOnJS cannot serialize functions across the worklet bridge,
  // so we invoke the action by index rather than passing the action object.
  const executeFirstLeftAction = useCallback(() => {
    if (leftActions.length > 0) {
      leftActions[0].onPress();
    }
  }, [leftActions]);

  const executeFirstRightAction = useCallback(() => {
    if (rightActions.length > 0) {
      rightActions[0].onPress();
    }
  }, [rightActions]);

  const panGesture = Gesture.Pan()
    .activeOffsetX([-10, 10])
    .failOffsetY([-5, 5])
    .onStart(() => {
      contextX.value = translateX.value;
    })
    .onUpdate((event) => {
      let newTranslateX = contextX.value + event.translationX;

      // Clamp based on available actions
      if (leftActions.length === 0) {
        newTranslateX = Math.min(0, newTranslateX);
      }
      if (rightActions.length === 0) {
        newTranslateX = Math.max(0, newTranslateX);
      }

      // Add resistance beyond action panel width
      const maxLeft = leftPanelWidth * 1.5;
      const maxRight = -rightPanelWidth * 1.5;
      if (newTranslateX > maxLeft) {
        newTranslateX = maxLeft + (newTranslateX - maxLeft) * 0.2;
      } else if (newTranslateX < maxRight) {
        newTranslateX = maxRight + (newTranslateX - maxRight) * 0.2;
      }

      translateX.value = newTranslateX;
    })
    .onEnd((event) => {
      const velocity = event.velocityX;

      // Full swipe trigger
      if (fullSwipeEnabled) {
        if (translateX.value > leftPanelWidth * FULL_SWIPE_THRESHOLD_RATIO && leftActions.length > 0) {
          runOnJS(triggerHaptic)();
          runOnJS(executeFirstLeftAction)();
          translateX.value = withSpring(0, SPRING_CONFIG);
          return;
        }
        if (translateX.value < -rightPanelWidth * FULL_SWIPE_THRESHOLD_RATIO && rightActions.length > 0) {
          runOnJS(triggerHaptic)();
          runOnJS(executeFirstRightAction)();
          translateX.value = withSpring(0, SPRING_CONFIG);
          return;
        }
      }

      // Snap to open or closed
      const shouldOpenLeft = translateX.value > leftPanelWidth * 0.3 || velocity > 500;
      const shouldOpenRight = translateX.value < -rightPanelWidth * 0.3 || velocity < -500;

      if (shouldOpenLeft && leftActions.length > 0) {
        translateX.value = withSpring(leftPanelWidth, SPRING_CONFIG);
      } else if (shouldOpenRight && rightActions.length > 0) {
        translateX.value = withSpring(-rightPanelWidth, SPRING_CONFIG);
      } else {
        translateX.value = withSpring(0, SPRING_CONFIG);
      }
    });

  const contentStyle = useAnimatedStyle(() => ({
    transform: [{ translateX: translateX.value }],
  }));

  const leftPanelStyle = useAnimatedStyle(() => {
    const progress = interpolate(
      translateX.value,
      [0, leftPanelWidth],
      [0, 1],
      Extrapolation.CLAMP
    );
    return {
      opacity: progress,
      transform: [{ scale: interpolate(progress, [0, 1], [0.8, 1], Extrapolation.CLAMP) }],
    };
  });

  const rightPanelStyle = useAnimatedStyle(() => {
    const progress = interpolate(
      translateX.value,
      [-rightPanelWidth, 0],
      [1, 0],
      Extrapolation.CLAMP
    );
    return {
      opacity: progress,
      transform: [{ scale: interpolate(progress, [0, 1], [0.8, 1], Extrapolation.CLAMP) }],
    };
  });

  const handleActionPress = useCallback((action: SwipeAction) => {
    Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    action.onPress();
    translateX.value = withSpring(0, SPRING_CONFIG);
  }, [translateX]);

  const renderActionButton = (action: SwipeAction, index: number) => (
    <TouchableOpacity
      key={`${action.label}-${index}`}
      onPress={() => handleActionPress(action)}
      style={[actionButtonStyle, { backgroundColor: action.backgroundColor }]}
      testID={testID ? `${testID}-action-${action.label.toLowerCase().replace(/\s/g, '-')}` : undefined}
    >
      <Feather name={action.icon} size={20} color={action.color} />
      <Text style={{ color: action.color, fontSize: 11, fontWeight: '600', marginTop: 4 }}>
        {action.label}
      </Text>
    </TouchableOpacity>
  );

  return (
    <View testID={testID}>
      {/* Left actions (revealed on right swipe) */}
      {leftActions.length > 0 && (
        <Animated.View
          style={[leftPanelContainerStyle, { width: leftPanelWidth }, leftPanelStyle]}
        >
          {leftActions.map(renderActionButton)}
        </Animated.View>
      )}

      {/* Right actions (revealed on left swipe) */}
      {rightActions.length > 0 && (
        <Animated.View
          style={[rightPanelContainerStyle, { width: rightPanelWidth }, rightPanelStyle]}
        >
          {rightActions.map(renderActionButton)}
        </Animated.View>
      )}

      {/* Main content */}
      <GestureDetector gesture={panGesture}>
        <Animated.View style={contentStyle}>
          {children}
        </Animated.View>
      </GestureDetector>
    </View>
  );
}

const leftPanelContainerStyle: ViewStyle = {
  position: 'absolute',
  left: 0,
  top: 0,
  bottom: 0,
  flexDirection: 'row',
  alignItems: 'center',
  justifyContent: 'flex-start',
};

const rightPanelContainerStyle: ViewStyle = {
  position: 'absolute',
  right: 0,
  top: 0,
  bottom: 0,
  flexDirection: 'row',
  alignItems: 'center',
  justifyContent: 'flex-end',
};

const actionButtonStyle: ViewStyle = {
  width: ACTION_BUTTON_WIDTH,
  height: '100%',
  justifyContent: 'center',
  alignItems: 'center',
};
