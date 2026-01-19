// ABOUTME: Unit tests for safe area handling in the app
// ABOUTME: Verifies SafeAreaProvider is set up and useSafeAreaInsets is used correctly

import React from 'react';
import { render } from '@testing-library/react-native';
import { View, Text } from 'react-native';
import { SafeAreaProvider, useSafeAreaInsets } from 'react-native-safe-area-context';

describe('Safe Area Configuration', () => {
  describe('SafeAreaProvider mock', () => {
    it('should provide safe area insets', () => {
      const TestComponent = () => {
        const insets = useSafeAreaInsets();
        return (
          <View testID="test-view">
            <Text testID="top-inset">{insets.top}</Text>
            <Text testID="bottom-inset">{insets.bottom}</Text>
          </View>
        );
      };

      const { getByTestId } = render(
        <SafeAreaProvider>
          <TestComponent />
        </SafeAreaProvider>
      );

      expect(getByTestId('test-view')).toBeTruthy();
      expect(getByTestId('top-inset').props.children).toBe(44);
      expect(getByTestId('bottom-inset').props.children).toBe(34);
    });

    it('should render SafeAreaProvider with children', () => {
      const { getByTestId } = render(
        <SafeAreaProvider>
          <View testID="child-view" />
        </SafeAreaProvider>
      );

      expect(getByTestId('safe-area-provider')).toBeTruthy();
      expect(getByTestId('child-view')).toBeTruthy();
    });
  });

  describe('useSafeAreaInsets hook', () => {
    it('should return all inset values', () => {
      const TestComponent = () => {
        const insets = useSafeAreaInsets();
        return (
          <View testID="insets-test">
            <Text testID="inset-top">{insets.top}</Text>
            <Text testID="inset-bottom">{insets.bottom}</Text>
            <Text testID="inset-left">{insets.left}</Text>
            <Text testID="inset-right">{insets.right}</Text>
          </View>
        );
      };

      const { getByTestId } = render(<TestComponent />);

      expect(getByTestId('inset-top').props.children).toBe(44);
      expect(getByTestId('inset-bottom').props.children).toBe(34);
      expect(getByTestId('inset-left').props.children).toBe(0);
      expect(getByTestId('inset-right').props.children).toBe(0);
    });

    it('should be usable for dynamic padding calculation', () => {
      const TestComponent = () => {
        const insets = useSafeAreaInsets();
        const headerPadding = insets.top + 8; // Common pattern for header padding
        return (
          <View testID="header" style={{ paddingTop: headerPadding }}>
            <Text>Header</Text>
          </View>
        );
      };

      const { getByTestId } = render(<TestComponent />);
      const header = getByTestId('header');

      // paddingTop should be insets.top (44) + 8 = 52
      expect(header.props.style.paddingTop).toBe(52);
    });
  });

  describe('Header with safe area insets', () => {
    it('should apply top inset to header for status bar clearance', () => {
      const HeaderComponent = () => {
        const insets = useSafeAreaInsets();
        return (
          <View
            testID="app-header"
            style={{
              paddingTop: insets.top + 8,
              paddingBottom: 8,
            }}
          >
            <Text>Header Content</Text>
          </View>
        );
      };

      const { getByTestId } = render(<HeaderComponent />);
      const header = getByTestId('app-header');

      // Verify the header has proper padding for status bar
      expect(header.props.style.paddingTop).toBeGreaterThan(0);
      expect(header.props.style.paddingTop).toBe(52); // 44 + 8
    });
  });
});
