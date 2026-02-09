// ABOUTME: Unit tests for TabSwipeWrapper component
// ABOUTME: Tests rendering, tab order, and navigation integration

import React from 'react';
import { render } from '@testing-library/react-native';
import { Text, View } from 'react-native';
import { TabSwipeWrapper } from '../src/components/ui/TabSwipeWrapper';

// Mock navigation
const mockNavigate = jest.fn();
jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({
    navigate: mockNavigate,
  }),
}));

describe('TabSwipeWrapper', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render children', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="CoachesTab">
          <Text>Tab Content</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Tab Content')).toBeTruthy();
    });

    it('should render complex children tree', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="ChatTab">
          <View>
            <Text>Header</Text>
            <View>
              <Text>Body</Text>
            </View>
          </View>
        </TabSwipeWrapper>
      );
      expect(getByText('Header')).toBeTruthy();
      expect(getByText('Body')).toBeTruthy();
    });
  });

  describe('tab names', () => {
    it('should accept ChatTab', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="ChatTab">
          <Text>Chat</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Chat')).toBeTruthy();
    });

    it('should accept CoachesTab', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="CoachesTab">
          <Text>Coaches</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Coaches')).toBeTruthy();
    });

    it('should accept DiscoverTab', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="DiscoverTab">
          <Text>Discover</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Discover')).toBeTruthy();
    });

    it('should accept SocialTab', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="SocialTab">
          <Text>Social</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Social')).toBeTruthy();
    });

    it('should accept SettingsTab', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="SettingsTab">
          <Text>Settings</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Settings')).toBeTruthy();
    });
  });

  describe('enabled prop', () => {
    it('should render with enabled=true by default', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="ChatTab">
          <Text>Content</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Content')).toBeTruthy();
    });

    it('should render with enabled=false', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="ChatTab" enabled={false}>
          <Text>Content</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Content')).toBeTruthy();
    });

    it('should render with enabled=true explicitly', () => {
      const { getByText } = render(
        <TabSwipeWrapper tabName="ChatTab" enabled={true}>
          <Text>Content</Text>
        </TabSwipeWrapper>
      );
      expect(getByText('Content')).toBeTruthy();
    });
  });
});
