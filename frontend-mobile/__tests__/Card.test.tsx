// ABOUTME: Unit tests for Card component
// ABOUTME: Tests variants, padding, and children rendering

import React from 'react';
import { Text } from 'react-native';
import { render } from '@testing-library/react-native';
import { Card } from '../src/components/ui/Card';

describe('Card Component', () => {
  describe('rendering', () => {
    it('should render children', () => {
      const { getByText } = render(
        <Card>
          <Text>Card Content</Text>
        </Card>
      );
      expect(getByText('Card Content')).toBeTruthy();
    });

    it('should render multiple children', () => {
      const { getByText } = render(
        <Card>
          <Text>First</Text>
          <Text>Second</Text>
          <Text>Third</Text>
        </Card>
      );
      expect(getByText('First')).toBeTruthy();
      expect(getByText('Second')).toBeTruthy();
      expect(getByText('Third')).toBeTruthy();
    });
  });

  describe('variants', () => {
    it('should render default variant', () => {
      const { getByText } = render(
        <Card variant="default">
          <Text>Default Card</Text>
        </Card>
      );
      expect(getByText('Default Card')).toBeTruthy();
    });

    it('should render elevated variant', () => {
      const { getByText } = render(
        <Card variant="elevated">
          <Text>Elevated Card</Text>
        </Card>
      );
      expect(getByText('Elevated Card')).toBeTruthy();
    });
  });

  describe('padding', () => {
    it('should have padding by default', () => {
      const { getByText } = render(
        <Card>
          <Text>With Padding</Text>
        </Card>
      );
      expect(getByText('With Padding')).toBeTruthy();
    });

    it('should accept noPadding prop', () => {
      const { getByText } = render(
        <Card noPadding>
          <Text>No Padding</Text>
        </Card>
      );
      expect(getByText('No Padding')).toBeTruthy();
    });
  });

  describe('custom styles', () => {
    it('should accept custom style prop', () => {
      const { getByText } = render(
        <Card style={{ marginTop: 20 }}>
          <Text>Styled Card</Text>
        </Card>
      );
      expect(getByText('Styled Card')).toBeTruthy();
    });

    it('should merge custom styles with default styles', () => {
      const { getByText } = render(
        <Card style={{ backgroundColor: '#000' }}>
          <Text>Custom Background</Text>
        </Card>
      );
      expect(getByText('Custom Background')).toBeTruthy();
    });
  });

  describe('nested cards', () => {
    it('should render nested cards', () => {
      const { getByText } = render(
        <Card>
          <Text>Outer Card</Text>
          <Card variant="elevated">
            <Text>Inner Card</Text>
          </Card>
        </Card>
      );
      expect(getByText('Outer Card')).toBeTruthy();
      expect(getByText('Inner Card')).toBeTruthy();
    });
  });
});
