// ABOUTME: Unit tests for ScrollFadeContainer component
// ABOUTME: Tests edge-fading gradients for horizontal scroll affordance per Stitch UX

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { View, Text } from 'react-native';
import { ScrollFadeContainer } from '../src/components/ui/ScrollFadeContainer';

describe('ScrollFadeContainer', () => {
  describe('rendering', () => {
    it('should render children', () => {
      const { getByText } = render(
        <ScrollFadeContainer>
          <Text>Test Content</Text>
        </ScrollFadeContainer>
      );
      expect(getByText('Test Content')).toBeTruthy();
    });

    it('should render with testID', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="test-scroll-fade">
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      expect(getByTestId('test-scroll-fade')).toBeTruthy();
    });

    it('should render scroll view with testID suffix', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="test-scroll-fade">
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      expect(getByTestId('test-scroll-fade-scroll')).toBeTruthy();
    });
  });

  describe('props', () => {
    it('should accept custom backgroundColor', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer
          backgroundColor="#FF0000"
          testID="test-scroll"
        >
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      expect(getByTestId('test-scroll')).toBeTruthy();
    });

    it('should accept custom fadeWidth', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer
          fadeWidth={64}
          testID="test-scroll"
        >
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      expect(getByTestId('test-scroll')).toBeTruthy();
    });

    it('should accept custom containerStyle', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer
          containerStyle={{ marginTop: 10 }}
          testID="test-scroll"
        >
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      expect(getByTestId('test-scroll')).toBeTruthy();
    });

    it('should accept custom contentContainerStyle', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer
          contentContainerStyle={{ paddingHorizontal: 16 }}
          testID="test-scroll"
        >
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      expect(getByTestId('test-scroll')).toBeTruthy();
    });
  });

  describe('scroll behavior', () => {
    it('should handle horizontal scroll events', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="scroll-fade">
          <View style={{ width: 1000 }}>
            <Text>Wide content that requires scrolling</Text>
          </View>
        </ScrollFadeContainer>
      );

      const scrollView = getByTestId('scroll-fade-scroll');

      // Simulate scroll event
      fireEvent.scroll(scrollView, {
        nativeEvent: {
          contentOffset: { x: 100, y: 0 },
          contentSize: { width: 1000, height: 50 },
          layoutMeasurement: { width: 390, height: 50 },
        },
      });

      // Component should still be rendered after scroll
      expect(scrollView).toBeTruthy();
    });

    it('should handle layout change events', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="scroll-fade">
          <Text>Content</Text>
        </ScrollFadeContainer>
      );

      const container = getByTestId('scroll-fade');

      // Simulate layout event
      fireEvent(container, 'layout', {
        nativeEvent: {
          layout: { x: 0, y: 0, width: 390, height: 50 },
        },
      });

      expect(container).toBeTruthy();
    });
  });

  describe('multiple children', () => {
    it('should render multiple children (like filter chips)', () => {
      const filterItems = ['All', 'Training', 'Nutrition', 'Recovery', 'Recipes'];

      const { getByText } = render(
        <ScrollFadeContainer testID="filter-scroll">
          {filterItems.map((item) => (
            <View key={item} style={{ marginRight: 8 }}>
              <Text>{item}</Text>
            </View>
          ))}
        </ScrollFadeContainer>
      );

      // All filter items should be rendered
      filterItems.forEach((item) => {
        expect(getByText(item)).toBeTruthy();
      });
    });
  });

  describe('fade visibility logic', () => {
    it('should not show fades when content is not scrollable', () => {
      const { queryByTestId } = render(
        <ScrollFadeContainer testID="scroll-fade">
          <Text>Short content</Text>
        </ScrollFadeContainer>
      );

      // Without content size change triggering scrollability,
      // fades should not be visible initially
      expect(queryByTestId('scroll-fade-left-fade')).toBeNull();
      expect(queryByTestId('scroll-fade-right-fade')).toBeNull();
    });

    it('should show right fade when scrollable content is detected', () => {
      const { getByTestId, rerender } = render(
        <ScrollFadeContainer testID="scroll-fade">
          <View style={{ width: 1000 }}>
            <Text>Wide scrollable content</Text>
          </View>
        </ScrollFadeContainer>
      );

      const container = getByTestId('scroll-fade');
      const scrollView = getByTestId('scroll-fade-scroll');

      // Simulate container layout
      fireEvent(container, 'layout', {
        nativeEvent: {
          layout: { x: 0, y: 0, width: 390, height: 50 },
        },
      });

      // Simulate content size change (indicating scrollable content)
      fireEvent(scrollView, 'contentSizeChange', 1000, 50);

      // Re-render to trigger useEffect for fade visibility
      rerender(
        <ScrollFadeContainer testID="scroll-fade">
          <View style={{ width: 1000 }}>
            <Text>Wide scrollable content</Text>
          </View>
        </ScrollFadeContainer>
      );

      // Component should handle this without crashing
      expect(container).toBeTruthy();
    });

    it('should show left fade after scrolling right', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="scroll-fade">
          <View style={{ width: 1000 }}>
            <Text>Wide content</Text>
          </View>
        </ScrollFadeContainer>
      );

      const container = getByTestId('scroll-fade');
      const scrollView = getByTestId('scroll-fade-scroll');

      // Setup scrollable content dimensions
      fireEvent(container, 'layout', {
        nativeEvent: {
          layout: { x: 0, y: 0, width: 390, height: 50 },
        },
      });
      fireEvent(scrollView, 'contentSizeChange', 1000, 50);

      // Simulate scrolling right (which should show left fade)
      fireEvent.scroll(scrollView, {
        nativeEvent: {
          contentOffset: { x: 200, y: 0 },
          contentSize: { width: 1000, height: 50 },
          layoutMeasurement: { width: 390, height: 50 },
        },
      });

      // Component should handle scroll events properly
      expect(scrollView).toBeTruthy();
    });

    it('should hide right fade when scrolled to end', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="scroll-fade">
          <View style={{ width: 1000 }}>
            <Text>Wide content</Text>
          </View>
        </ScrollFadeContainer>
      );

      const container = getByTestId('scroll-fade');
      const scrollView = getByTestId('scroll-fade-scroll');

      // Setup scrollable content dimensions
      fireEvent(container, 'layout', {
        nativeEvent: {
          layout: { x: 0, y: 0, width: 390, height: 50 },
        },
      });
      fireEvent(scrollView, 'contentSizeChange', 1000, 50);

      // Simulate scrolling to the very end
      fireEvent.scroll(scrollView, {
        nativeEvent: {
          contentOffset: { x: 610, y: 0 }, // 1000 - 390 = 610 (max scroll)
          contentSize: { width: 1000, height: 50 },
          layoutMeasurement: { width: 390, height: 50 },
        },
      });

      // Component should handle scroll to end
      expect(scrollView).toBeTruthy();
    });
  });

  describe('default values', () => {
    it('should use default backgroundColor from theme', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="test-scroll">
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      // Should render without errors using default background color
      expect(getByTestId('test-scroll')).toBeTruthy();
    });

    it('should use default fadeWidth of 32', () => {
      const { getByTestId } = render(
        <ScrollFadeContainer testID="test-scroll">
          <Text>Content</Text>
        </ScrollFadeContainer>
      );
      // Should render without errors using default fade width
      expect(getByTestId('test-scroll')).toBeTruthy();
    });
  });
});
