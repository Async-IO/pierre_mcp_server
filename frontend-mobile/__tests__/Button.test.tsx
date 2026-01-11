// ABOUTME: Unit tests for Button component
// ABOUTME: Tests variants, sizes, states, and interactions

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { Button } from '../src/components/ui/Button';

describe('Button Component', () => {
  describe('rendering', () => {
    it('should render with title', () => {
      const { getByText } = render(
        <Button title="Click me" onPress={() => {}} />
      );
      expect(getByText('Click me')).toBeTruthy();
    });

    it('should render with testID', () => {
      const { getByTestId } = render(
        <Button title="Test" onPress={() => {}} testID="test-button" />
      );
      expect(getByTestId('test-button')).toBeTruthy();
    });
  });

  describe('variants', () => {
    it('should render primary variant by default', () => {
      const { getByText } = render(
        <Button title="Primary" onPress={() => {}} />
      );
      expect(getByText('Primary')).toBeTruthy();
    });

    it('should render secondary variant', () => {
      const { getByText } = render(
        <Button title="Secondary" onPress={() => {}} variant="secondary" />
      );
      expect(getByText('Secondary')).toBeTruthy();
    });

    it('should render ghost variant', () => {
      const { getByText } = render(
        <Button title="Ghost" onPress={() => {}} variant="ghost" />
      );
      expect(getByText('Ghost')).toBeTruthy();
    });

    it('should render danger variant', () => {
      const { getByText } = render(
        <Button title="Danger" onPress={() => {}} variant="danger" />
      );
      expect(getByText('Danger')).toBeTruthy();
    });
  });

  describe('sizes', () => {
    it('should render small size', () => {
      const { getByText } = render(
        <Button title="Small" onPress={() => {}} size="sm" />
      );
      expect(getByText('Small')).toBeTruthy();
    });

    it('should render medium size by default', () => {
      const { getByText } = render(
        <Button title="Medium" onPress={() => {}} />
      );
      expect(getByText('Medium')).toBeTruthy();
    });

    it('should render large size', () => {
      const { getByText } = render(
        <Button title="Large" onPress={() => {}} size="lg" />
      );
      expect(getByText('Large')).toBeTruthy();
    });
  });

  describe('interactions', () => {
    it('should call onPress when pressed', () => {
      const onPressMock = jest.fn();
      const { getByText } = render(
        <Button title="Click me" onPress={onPressMock} />
      );

      fireEvent.press(getByText('Click me'));
      expect(onPressMock).toHaveBeenCalledTimes(1);
    });

    it('should not call onPress when disabled', () => {
      const onPressMock = jest.fn();
      const { getByText } = render(
        <Button title="Disabled" onPress={onPressMock} disabled />
      );

      fireEvent.press(getByText('Disabled'));
      expect(onPressMock).not.toHaveBeenCalled();
    });
  });

  describe('loading state', () => {
    it('should show loading indicator when loading', () => {
      const { queryByText, UNSAFE_getByType } = render(
        <Button title="Loading" onPress={() => {}} loading />
      );

      // Text should not be visible when loading
      expect(queryByText('Loading')).toBeNull();
      // ActivityIndicator should be rendered
      const ActivityIndicator = require('react-native').ActivityIndicator;
      expect(UNSAFE_getByType(ActivityIndicator)).toBeTruthy();
    });

    it('should not call onPress when loading', () => {
      const onPressMock = jest.fn();
      const { getByTestId } = render(
        <Button title="Loading" onPress={onPressMock} loading testID="loading-btn" />
      );

      fireEvent.press(getByTestId('loading-btn'));
      expect(onPressMock).not.toHaveBeenCalled();
    });
  });

  describe('fullWidth prop', () => {
    it('should accept fullWidth prop', () => {
      const { getByText } = render(
        <Button title="Full Width" onPress={() => {}} fullWidth />
      );
      expect(getByText('Full Width')).toBeTruthy();
    });
  });

  describe('custom styles', () => {
    it('should accept custom style prop', () => {
      const { getByText } = render(
        <Button
          title="Styled"
          onPress={() => {}}
          style={{ marginTop: 10 }}
        />
      );
      expect(getByText('Styled')).toBeTruthy();
    });

    it('should accept custom textStyle prop', () => {
      const { getByText } = render(
        <Button
          title="Styled Text"
          onPress={() => {}}
          textStyle={{ letterSpacing: 2 }}
        />
      );
      expect(getByText('Styled Text')).toBeTruthy();
    });
  });
});
