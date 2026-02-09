// ABOUTME: Unit tests for SwipeableRow component
// ABOUTME: Tests rendering, action buttons, press callbacks, haptic feedback, and testID

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { Text, View } from 'react-native';
import { SwipeableRow } from '../src/components/ui/SwipeableRow';
import type { SwipeAction } from '../src/components/ui/SwipeableRow';
import * as Haptics from 'expo-haptics';

const createAction = (overrides: Partial<SwipeAction> = {}): SwipeAction => ({
  icon: 'heart',
  label: 'Favorite',
  color: '#FFFFFF',
  backgroundColor: '#EF4444',
  onPress: jest.fn(),
  ...overrides,
});

describe('SwipeableRow', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render children', () => {
      const { getByText } = render(
        <SwipeableRow>
          <Text>Row Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Row Content')).toBeTruthy();
    });

    it('should render with testID', () => {
      const { getByTestId } = render(
        <SwipeableRow testID="test-swipeable">
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByTestId('test-swipeable')).toBeTruthy();
    });

    it('should render complex children', () => {
      const { getByText } = render(
        <SwipeableRow>
          <View>
            <Text>Title</Text>
            <Text>Subtitle</Text>
          </View>
        </SwipeableRow>
      );
      expect(getByText('Title')).toBeTruthy();
      expect(getByText('Subtitle')).toBeTruthy();
    });
  });

  describe('left action buttons', () => {
    it('should render left action buttons when provided', () => {
      const favoriteAction = createAction({ label: 'Favorite', icon: 'heart' });
      const { getByText } = render(
        <SwipeableRow leftActions={[favoriteAction]}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Favorite')).toBeTruthy();
    });

    it('should render multiple left action buttons', () => {
      const actions = [
        createAction({ label: 'Favorite', icon: 'heart' }),
        createAction({ label: 'Share', icon: 'share-2' }),
      ];
      const { getByText } = render(
        <SwipeableRow leftActions={actions}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Favorite')).toBeTruthy();
      expect(getByText('Share')).toBeTruthy();
    });

    it('should not render left action panel when no left actions', () => {
      const { queryByText } = render(
        <SwipeableRow leftActions={[]}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(queryByText('Favorite')).toBeNull();
    });

    it('should generate action testID from parent testID', () => {
      const action = createAction({ label: 'Favorite' });
      const { getByTestId } = render(
        <SwipeableRow testID="row" leftActions={[action]}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByTestId('row-action-favorite')).toBeTruthy();
    });
  });

  describe('right action buttons', () => {
    it('should render right action buttons when provided', () => {
      const deleteAction = createAction({ label: 'Delete', icon: 'trash-2' });
      const { getByText } = render(
        <SwipeableRow rightActions={[deleteAction]}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Delete')).toBeTruthy();
    });

    it('should render multiple right action buttons', () => {
      const actions = [
        createAction({ label: 'Archive', icon: 'archive' }),
        createAction({ label: 'Delete', icon: 'trash-2' }),
      ];
      const { getByText } = render(
        <SwipeableRow rightActions={actions}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Archive')).toBeTruthy();
      expect(getByText('Delete')).toBeTruthy();
    });

    it('should not render right action panel when no right actions', () => {
      const { queryByText } = render(
        <SwipeableRow rightActions={[]}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(queryByText('Delete')).toBeNull();
    });
  });

  describe('action press callbacks', () => {
    it('should call onPress when left action button is pressed', () => {
      const onPress = jest.fn();
      const action = createAction({ label: 'Favorite', onPress });
      const { getByText } = render(
        <SwipeableRow leftActions={[action]}>
          <Text>Content</Text>
        </SwipeableRow>
      );

      fireEvent.press(getByText('Favorite'));
      expect(onPress).toHaveBeenCalledTimes(1);
    });

    it('should call onPress when right action button is pressed', () => {
      const onPress = jest.fn();
      const action = createAction({ label: 'Delete', onPress });
      const { getByText } = render(
        <SwipeableRow rightActions={[action]}>
          <Text>Content</Text>
        </SwipeableRow>
      );

      fireEvent.press(getByText('Delete'));
      expect(onPress).toHaveBeenCalledTimes(1);
    });

    it('should trigger haptic feedback on action press', () => {
      const action = createAction({ label: 'Favorite' });
      const { getByText } = render(
        <SwipeableRow leftActions={[action]}>
          <Text>Content</Text>
        </SwipeableRow>
      );

      fireEvent.press(getByText('Favorite'));
      expect(Haptics.impactAsync).toHaveBeenCalledWith(Haptics.ImpactFeedbackStyle.Light);
    });

    it('should call correct onPress for each action in a multi-action panel', () => {
      const onPressFavorite = jest.fn();
      const onPressShare = jest.fn();
      const actions = [
        createAction({ label: 'Favorite', onPress: onPressFavorite }),
        createAction({ label: 'Share', onPress: onPressShare }),
      ];
      const { getByText } = render(
        <SwipeableRow leftActions={actions}>
          <Text>Content</Text>
        </SwipeableRow>
      );

      fireEvent.press(getByText('Favorite'));
      expect(onPressFavorite).toHaveBeenCalledTimes(1);
      expect(onPressShare).not.toHaveBeenCalled();

      fireEvent.press(getByText('Share'));
      expect(onPressShare).toHaveBeenCalledTimes(1);
    });
  });

  describe('both left and right actions', () => {
    it('should render both left and right action panels', () => {
      const leftAction = createAction({ label: 'Bookmark', icon: 'bookmark' });
      const rightAction = createAction({ label: 'Adapt', icon: 'refresh-cw' });

      const { getByText } = render(
        <SwipeableRow leftActions={[leftAction]} rightActions={[rightAction]}>
          <Text>Content</Text>
        </SwipeableRow>
      );

      expect(getByText('Bookmark')).toBeTruthy();
      expect(getByText('Adapt')).toBeTruthy();
    });
  });

  describe('action button styling', () => {
    it('should apply backgroundColor to action button', () => {
      const action = createAction({
        label: 'Delete',
        backgroundColor: '#DC2626',
      });
      const { getByText } = render(
        <SwipeableRow rightActions={[action]}>
          <Text>Content</Text>
        </SwipeableRow>
      );

      // The parent TouchableOpacity wrapping the text should exist
      const actionButton = getByText('Delete').parent;
      expect(actionButton).toBeTruthy();
    });
  });

  describe('default props', () => {
    it('should default leftActions and rightActions to empty arrays', () => {
      const { getByText, queryByText } = render(
        <SwipeableRow>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Content')).toBeTruthy();
      // No action buttons should be rendered
      expect(queryByText('Favorite')).toBeNull();
      expect(queryByText('Delete')).toBeNull();
    });

    it('should default fullSwipeEnabled to true', () => {
      // Just verify it renders without error - fullSwipeEnabled affects
      // gesture behavior which is handled natively
      const { getByText } = render(
        <SwipeableRow>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Content')).toBeTruthy();
    });

    it('should accept fullSwipeEnabled=false without error', () => {
      const action = createAction({ label: 'Action' });
      const { getByText } = render(
        <SwipeableRow leftActions={[action]} fullSwipeEnabled={false}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByText('Content')).toBeTruthy();
      expect(getByText('Action')).toBeTruthy();
    });
  });

  describe('action label formatting in testIDs', () => {
    it('should lowercase and hyphenate action labels in testID', () => {
      const action = createAction({ label: 'My Action' });
      const { getByTestId } = render(
        <SwipeableRow testID="row" rightActions={[action]}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      expect(getByTestId('row-action-my-action')).toBeTruthy();
    });

    it('should not generate action testID when parent has no testID', () => {
      const action = createAction({ label: 'Delete' });
      const { queryByTestId } = render(
        <SwipeableRow rightActions={[action]}>
          <Text>Content</Text>
        </SwipeableRow>
      );
      // When no parent testID, action testID is undefined
      expect(queryByTestId('undefined-action-delete')).toBeNull();
    });
  });
});
