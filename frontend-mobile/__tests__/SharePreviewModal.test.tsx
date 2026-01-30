// ABOUTME: Unit tests for SharePreviewModal component
// ABOUTME: Tests rendering, visibility options, and user interactions

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { SharePreviewModal } from '../src/components/social/SharePreviewModal';

describe('SharePreviewModal Component', () => {
  const defaultProps = {
    visible: true,
    content: 'Great workout today! Hit a new personal best.',
    visibility: 'friends_only' as const,
    isSharing: false,
    onVisibilityChange: jest.fn(),
    onShare: jest.fn(),
    onEdit: jest.fn(),
    onClose: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render the modal when visible', () => {
      const { getByText } = render(<SharePreviewModal {...defaultProps} />);
      expect(getByText('Share to Feed')).toBeTruthy();
    });

    it('should display the content preview', () => {
      const { getByText } = render(<SharePreviewModal {...defaultProps} />);
      expect(getByText('Great workout today! Hit a new personal best.')).toBeTruthy();
    });

    it('should show Coach Insight label', () => {
      const { getByText } = render(<SharePreviewModal {...defaultProps} />);
      expect(getByText('Coach Insight')).toBeTruthy();
    });

    it('should display visibility options', () => {
      const { getByText } = render(<SharePreviewModal {...defaultProps} />);
      expect(getByText('Friends Only')).toBeTruthy();
      expect(getByText('Public')).toBeTruthy();
    });

    it('should display privacy notice', () => {
      const { getByText } = render(<SharePreviewModal {...defaultProps} />);
      expect(getByText(/Your privacy matters/)).toBeTruthy();
    });
  });

  describe('visibility options', () => {
    it('should call onVisibilityChange when Friends Only is selected', () => {
      const onVisibilityChange = jest.fn();
      const { getByTestId } = render(
        <SharePreviewModal {...defaultProps} onVisibilityChange={onVisibilityChange} />
      );

      fireEvent.press(getByTestId('visibility-friends-only'));
      expect(onVisibilityChange).toHaveBeenCalledWith('friends_only');
    });

    it('should call onVisibilityChange when Public is selected', () => {
      const onVisibilityChange = jest.fn();
      const { getByTestId } = render(
        <SharePreviewModal {...defaultProps} onVisibilityChange={onVisibilityChange} />
      );

      fireEvent.press(getByTestId('visibility-public'));
      expect(onVisibilityChange).toHaveBeenCalledWith('public');
    });
  });

  describe('interactions', () => {
    it('should call onShare when Share Now button is pressed', () => {
      const onShare = jest.fn();
      const { getByTestId } = render(
        <SharePreviewModal {...defaultProps} onShare={onShare} />
      );

      fireEvent.press(getByTestId('share-now-button'));
      expect(onShare).toHaveBeenCalledTimes(1);
    });

    it('should call onEdit when edit button is pressed', () => {
      const onEdit = jest.fn();
      const { getByTestId } = render(
        <SharePreviewModal {...defaultProps} onEdit={onEdit} />
      );

      fireEvent.press(getByTestId('edit-share-content'));
      expect(onEdit).toHaveBeenCalledTimes(1);
    });

    it('should call onClose when close button is pressed', () => {
      const onClose = jest.fn();
      const { getByTestId } = render(
        <SharePreviewModal {...defaultProps} onClose={onClose} />
      );

      fireEvent.press(getByTestId('close-share-preview'));
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it('should call onClose when cancel button is pressed', () => {
      const onClose = jest.fn();
      const { getByTestId } = render(
        <SharePreviewModal {...defaultProps} onClose={onClose} />
      );

      fireEvent.press(getByTestId('cancel-share-button'));
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  describe('loading state', () => {
    it('should show loading indicator when isSharing is true', () => {
      const { UNSAFE_getByType } = render(
        <SharePreviewModal {...defaultProps} isSharing={true} />
      );

      const ActivityIndicator = require('react-native').ActivityIndicator;
      expect(UNSAFE_getByType(ActivityIndicator)).toBeTruthy();
    });

    it('should disable Share Now button when isSharing is true', () => {
      const onShare = jest.fn();
      const { getByTestId } = render(
        <SharePreviewModal {...defaultProps} isSharing={true} onShare={onShare} />
      );

      fireEvent.press(getByTestId('share-now-button'));
      expect(onShare).not.toHaveBeenCalled();
    });
  });

  describe('long content handling', () => {
    it('should render long content within constrained preview area', () => {
      const longContent = 'This is a very long insight. '.repeat(20);
      const { getByText } = render(
        <SharePreviewModal {...defaultProps} content={longContent} />
      );

      // Content should still be visible (rendered in scrollable area)
      expect(getByText(longContent)).toBeTruthy();
      // Visibility options should also be accessible
      expect(getByText('Friends Only')).toBeTruthy();
      expect(getByText('Public')).toBeTruthy();
    });
  });
});
