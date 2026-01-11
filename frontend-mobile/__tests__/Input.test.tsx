// ABOUTME: Unit tests for Input component
// ABOUTME: Tests labels, errors, password toggle, and input behavior

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { Input } from '../src/components/ui/Input';

describe('Input Component', () => {
  describe('rendering', () => {
    it('should render basic input', () => {
      const { getByTestId } = render(
        <Input testID="basic-input" placeholder="Enter text" />
      );
      expect(getByTestId('basic-input')).toBeTruthy();
    });

    it('should render with placeholder', () => {
      const { getByPlaceholderText } = render(
        <Input placeholder="Enter your name" />
      );
      expect(getByPlaceholderText('Enter your name')).toBeTruthy();
    });
  });

  describe('label', () => {
    it('should render label when provided', () => {
      const { getByText } = render(
        <Input label="Email Address" placeholder="Enter email" />
      );
      expect(getByText('Email Address')).toBeTruthy();
    });

    it('should not render label when not provided', () => {
      const { queryByText } = render(
        <Input placeholder="No label" />
      );
      expect(queryByText('Email Address')).toBeNull();
    });
  });

  describe('error state', () => {
    it('should render error message when provided', () => {
      const { getByText } = render(
        <Input placeholder="Enter email" error="Invalid email format" />
      );
      expect(getByText('Invalid email format')).toBeTruthy();
    });

    it('should not render error when not provided', () => {
      const { queryByText } = render(
        <Input placeholder="No error" />
      );
      expect(queryByText('Invalid email format')).toBeNull();
    });

    it('should show both label and error', () => {
      const { getByText } = render(
        <Input
          label="Email"
          placeholder="Enter email"
          error="This field is required"
        />
      );
      expect(getByText('Email')).toBeTruthy();
      expect(getByText('This field is required')).toBeTruthy();
    });
  });

  describe('text input behavior', () => {
    it('should accept text input', () => {
      const onChangeMock = jest.fn();
      const { getByTestId } = render(
        <Input
          testID="text-input"
          onChangeText={onChangeMock}
          placeholder="Type here"
        />
      );

      fireEvent.changeText(getByTestId('text-input'), 'Hello World');
      expect(onChangeMock).toHaveBeenCalledWith('Hello World');
    });

    it('should handle value prop', () => {
      const { getByDisplayValue } = render(
        <Input value="Initial Value" placeholder="Enter text" />
      );
      expect(getByDisplayValue('Initial Value')).toBeTruthy();
    });
  });

  describe('password toggle', () => {
    it('should show toggle when showPasswordToggle is true and secureTextEntry is set', () => {
      const { getByText } = render(
        <Input
          placeholder="Password"
          secureTextEntry={true}
          showPasswordToggle={true}
        />
      );
      expect(getByText('Show')).toBeTruthy();
    });

    it('should toggle password visibility when pressed', () => {
      const { getByText } = render(
        <Input
          placeholder="Password"
          secureTextEntry={true}
          showPasswordToggle={true}
        />
      );

      // Initially shows "Show"
      const toggleButton = getByText('Show');
      expect(toggleButton).toBeTruthy();

      // Press to show password
      fireEvent.press(toggleButton);
      expect(getByText('Hide')).toBeTruthy();

      // Press again to hide password
      fireEvent.press(getByText('Hide'));
      expect(getByText('Show')).toBeTruthy();
    });

    it('should not show toggle when showPasswordToggle is false', () => {
      const { queryByText } = render(
        <Input
          placeholder="Password"
          secureTextEntry={true}
          showPasswordToggle={false}
        />
      );
      expect(queryByText('Show')).toBeNull();
      expect(queryByText('Hide')).toBeNull();
    });
  });

  describe('custom styles', () => {
    it('should accept containerStyle prop', () => {
      const { getByTestId } = render(
        <Input
          testID="styled-input"
          containerStyle={{ marginTop: 20 }}
          placeholder="Styled"
        />
      );
      expect(getByTestId('styled-input')).toBeTruthy();
    });
  });

  describe('input props passthrough', () => {
    it('should pass through keyboardType', () => {
      const { getByTestId } = render(
        <Input
          testID="email-input"
          keyboardType="email-address"
          placeholder="Email"
        />
      );
      expect(getByTestId('email-input').props.keyboardType).toBe('email-address');
    });

    it('should pass through autoCapitalize', () => {
      const { getByTestId } = render(
        <Input
          testID="name-input"
          autoCapitalize="words"
          placeholder="Name"
        />
      );
      expect(getByTestId('name-input').props.autoCapitalize).toBe('words');
    });

    it('should pass through maxLength', () => {
      const { getByTestId } = render(
        <Input
          testID="limited-input"
          maxLength={100}
          placeholder="Limited"
        />
      );
      expect(getByTestId('limited-input').props.maxLength).toBe(100);
    });
  });
});
