// ABOUTME: Unit tests for DragIndicator component
// ABOUTME: Tests rendering and testID propagation for the modal drag pill

import React from 'react';
import { render } from '@testing-library/react-native';
import { DragIndicator } from '../src/components/ui/DragIndicator';

describe('DragIndicator', () => {
  describe('rendering', () => {
    it('should render without crashing', () => {
      const { toJSON } = render(<DragIndicator />);
      expect(toJSON()).toBeTruthy();
    });

    it('should render a container view with the indicator pill', () => {
      const tree = render(<DragIndicator />);
      const json = tree.toJSON();

      // Root is a View container with the pill View inside
      expect(json).toBeTruthy();
      if (json && !Array.isArray(json)) {
        expect(json.type).toBe('View');
        expect(json.children).toBeTruthy();
        expect(json.children).toHaveLength(1);
      }
    });
  });

  describe('testID', () => {
    it('should pass testID to container view', () => {
      const { getByTestId } = render(<DragIndicator testID="drag-indicator" />);
      expect(getByTestId('drag-indicator')).toBeTruthy();
    });

    it('should render without testID when not provided', () => {
      const { queryByTestId } = render(<DragIndicator />);
      expect(queryByTestId('drag-indicator')).toBeNull();
    });
  });

  describe('visual structure', () => {
    it('should render the indicator pill with rgba background', () => {
      const tree = render(<DragIndicator />);
      const json = tree.toJSON();

      // The pill is the child View with the inline backgroundColor style
      if (json && !Array.isArray(json) && json.children) {
        const pill = json.children[0];
        if (pill && typeof pill === 'object' && 'props' in pill) {
          expect(pill.props.style).toBeDefined();
          // Check that the pill has the rgba background color
          const styles = Array.isArray(pill.props.style) ? pill.props.style : [pill.props.style];
          const hasRgbaBackground = styles.some(
            (s: Record<string, string>) => s && s.backgroundColor === 'rgba(255, 255, 255, 0.3)'
          );
          expect(hasRgbaBackground).toBe(true);
        }
      }
    });
  });
});
