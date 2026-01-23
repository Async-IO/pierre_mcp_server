// ABOUTME: Horizontal scroll container with edge-fading gradients per Stitch UX recommendations
// ABOUTME: Indicates scrollability with gradient fades, improving discoverability of hidden content

import React, { useState, useCallback, type ReactNode } from 'react';
import {
  View,
  ScrollView,
  StyleSheet,
  type ViewStyle,
  type NativeSyntheticEvent,
  type NativeScrollEvent,
  type LayoutChangeEvent,
} from 'react-native';
import { LinearGradient } from 'expo-linear-gradient';
import { colors } from '../../constants/theme';

interface ScrollFadeContainerProps {
  children: ReactNode;
  /** Background color for the gradient fade (should match container background) */
  backgroundColor?: string;
  /** Width of the fade gradient in pixels */
  fadeWidth?: number;
  /** Style for the outer container */
  containerStyle?: ViewStyle;
  /** Style for the scroll view content container */
  contentContainerStyle?: ViewStyle;
  /** Test ID for the component */
  testID?: string;
}

/**
 * A horizontal scroll container with edge-fading gradients.
 *
 * Per Stitch UX recommendations, this component:
 * - Shows a gradient fade on the right edge when content extends beyond view
 * - Shows a gradient fade on the left edge when scrolled right
 * - Hides fades when at scroll boundaries
 * - Provides visual affordance that indicates "there is more content here"
 */
export function ScrollFadeContainer({
  children,
  backgroundColor = colors.background.primary,
  fadeWidth = 32,
  containerStyle,
  contentContainerStyle,
  testID,
}: ScrollFadeContainerProps) {
  const [showLeftFade, setShowLeftFade] = useState(false);
  const [showRightFade, setShowRightFade] = useState(false);
  const [contentWidth, setContentWidth] = useState(0);
  const [containerWidth, setContainerWidth] = useState(0);

  // Check if content is scrollable (content wider than container)
  const isScrollable = contentWidth > containerWidth;

  const handleScroll = useCallback(
    (event: NativeSyntheticEvent<NativeScrollEvent>) => {
      const { contentOffset, contentSize, layoutMeasurement } = event.nativeEvent;
      const scrollX = contentOffset.x;
      const maxScrollX = contentSize.width - layoutMeasurement.width;

      // Show left fade when scrolled past threshold (not at start)
      setShowLeftFade(scrollX > 10);

      // Show right fade when not at end (with small threshold for float precision)
      setShowRightFade(scrollX < maxScrollX - 10);
    },
    []
  );

  const handleContentSizeChange = useCallback((width: number) => {
    setContentWidth(width);
  }, []);

  const handleContainerLayout = useCallback((event: LayoutChangeEvent) => {
    const { width } = event.nativeEvent.layout;
    setContainerWidth(width);
  }, []);

  // Initialize right fade visibility when layout is measured
  React.useEffect(() => {
    if (contentWidth > 0 && containerWidth > 0) {
      setShowRightFade(contentWidth > containerWidth);
    }
  }, [contentWidth, containerWidth]);

  // Transparent to background color (for right edge - content fades into background)
  const rightGradientColors: [string, string] = [
    'transparent',
    backgroundColor,
  ];

  // Background color to transparent (for left edge - background fades into content)
  const leftGradientColors: [string, string] = [
    backgroundColor,
    'transparent',
  ];

  return (
    <View
      style={[styles.container, containerStyle]}
      onLayout={handleContainerLayout}
      testID={testID}
    >
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        onScroll={handleScroll}
        onContentSizeChange={handleContentSizeChange}
        scrollEventThrottle={16}
        contentContainerStyle={contentContainerStyle}
        testID={testID ? `${testID}-scroll` : undefined}
      >
        {children}
      </ScrollView>

      {/* Left fade gradient - shows when scrolled right */}
      {isScrollable && showLeftFade && (
        <View
          style={[styles.fadeOverlay, styles.leftFade, { width: fadeWidth }]}
          pointerEvents="none"
          testID={testID ? `${testID}-left-fade` : undefined}
        >
          <LinearGradient
            colors={leftGradientColors}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 0 }}
            style={styles.gradient}
          />
        </View>
      )}

      {/* Right fade gradient - shows when more content available */}
      {isScrollable && showRightFade && (
        <View
          style={[styles.fadeOverlay, styles.rightFade, { width: fadeWidth }]}
          pointerEvents="none"
          testID={testID ? `${testID}-right-fade` : undefined}
        >
          <LinearGradient
            colors={rightGradientColors}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 0 }}
            style={styles.gradient}
          />
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    position: 'relative',
    overflow: 'hidden',
  },
  fadeOverlay: {
    position: 'absolute',
    top: 0,
    bottom: 0,
    zIndex: 1,
  },
  leftFade: {
    left: 0,
  },
  rightFade: {
    right: 0,
  },
  gradient: {
    flex: 1,
  },
});
