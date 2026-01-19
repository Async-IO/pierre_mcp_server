// ABOUTME: Unit tests for VoiceButton component
// ABOUTME: Tests rendering states, accessibility, and user interactions

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { VoiceButton } from '../src/components/ui/VoiceButton';

describe('VoiceButton Component', () => {
  const defaultProps = {
    isListening: false,
    isAvailable: true,
    onPress: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render when voice is available', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} testID="voice-button" />
      );
      expect(getByTestId('voice-button')).toBeTruthy();
    });

    it('should not render when voice is not available', () => {
      const { queryByTestId } = render(
        <VoiceButton {...defaultProps} isAvailable={false} testID="voice-button" />
      );
      expect(queryByTestId('voice-button')).toBeNull();
    });

    it('should render with testID', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} testID="custom-voice-btn" />
      );
      expect(getByTestId('custom-voice-btn')).toBeTruthy();
    });
  });

  describe('sizes', () => {
    it('should render small size', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} size="sm" testID="voice-button" />
      );
      expect(getByTestId('voice-button')).toBeTruthy();
    });

    it('should render medium size by default', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} testID="voice-button" />
      );
      expect(getByTestId('voice-button')).toBeTruthy();
    });

    it('should render large size', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} size="lg" testID="voice-button" />
      );
      expect(getByTestId('voice-button')).toBeTruthy();
    });
  });

  describe('listening state', () => {
    it('should show microphone icon when not listening', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} isListening={false} testID="voice-button" />
      );
      expect(getByTestId('voice-button')).toBeTruthy();
    });

    it('should show activity indicator when listening', () => {
      const { getByTestId, UNSAFE_getByType } = render(
        <VoiceButton {...defaultProps} isListening={true} testID="voice-button" />
      );
      expect(getByTestId('voice-button')).toBeTruthy();
      const ActivityIndicator = require('react-native').ActivityIndicator;
      expect(UNSAFE_getByType(ActivityIndicator)).toBeTruthy();
    });
  });

  describe('interactions', () => {
    it('should call onPress when pressed', () => {
      const onPressMock = jest.fn();
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} onPress={onPressMock} testID="voice-button" />
      );

      fireEvent.press(getByTestId('voice-button'));
      expect(onPressMock).toHaveBeenCalledTimes(1);
    });

    it('should not call onPress when disabled', () => {
      const onPressMock = jest.fn();
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} onPress={onPressMock} disabled testID="voice-button" />
      );

      fireEvent.press(getByTestId('voice-button'));
      expect(onPressMock).not.toHaveBeenCalled();
    });

    it('should not call onPress when not available', () => {
      const onPressMock = jest.fn();
      const { queryByTestId } = render(
        <VoiceButton {...defaultProps} onPress={onPressMock} isAvailable={false} testID="voice-button" />
      );

      // Button shouldn't even render
      expect(queryByTestId('voice-button')).toBeNull();
    });
  });

  describe('accessibility', () => {
    it('should have correct accessibility label when not listening', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} isListening={false} testID="voice-button" />
      );

      const button = getByTestId('voice-button');
      expect(button.props.accessibilityLabel).toBe('Start voice input');
    });

    it('should have correct accessibility label when listening', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} isListening={true} testID="voice-button" />
      );

      const button = getByTestId('voice-button');
      expect(button.props.accessibilityLabel).toBe('Stop voice input');
    });

    it('should have button accessibility role', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} testID="voice-button" />
      );

      const button = getByTestId('voice-button');
      expect(button.props.accessibilityRole).toBe('button');
    });

    it('should indicate disabled state in accessibility', () => {
      const { getByTestId } = render(
        <VoiceButton {...defaultProps} disabled testID="voice-button" />
      );

      const button = getByTestId('voice-button');
      expect(button.props.accessibilityState.disabled).toBe(true);
    });
  });
});
