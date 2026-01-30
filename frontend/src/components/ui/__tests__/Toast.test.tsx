// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Tests for Toast component and hooks
// ABOUTME: Verifies toast notifications display correctly

import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ToastProvider } from '../Toast';
import { useSuccessToast, useInfoToast, useErrorToast, useWarningToast, useToast } from '../useToast';

// Test component that uses the toast hooks
function TestComponent({ type }: { type: 'success' | 'info' | 'error' | 'warning' }) {
  const showSuccessToast = useSuccessToast();
  const showInfoToast = useInfoToast();
  const showErrorToast = useErrorToast();
  const showWarningToast = useWarningToast();

  const handleClick = () => {
    // Use a very long duration so toast doesn't auto-dismiss during tests
    const duration = 60000;
    switch (type) {
      case 'success':
        showSuccessToast('Success Title', 'Success message', duration);
        break;
      case 'info':
        showInfoToast('Info Title', 'Info message', duration);
        break;
      case 'error':
        showErrorToast('Error Title', 'Error message', duration);
        break;
      case 'warning':
        showWarningToast('Warning Title', 'Warning message', duration);
        break;
    }
  };

  return <button onClick={handleClick}>Show Toast</button>;
}

// Test component for testing toast context directly
function DirectToastComponent() {
  const { addToast, toasts } = useToast();

  return (
    <div>
      <button onClick={() => addToast({ type: 'success', title: 'Direct Toast', duration: 60000 })}>
        Add Toast
      </button>
      <div data-testid="toast-count">{toasts.length}</div>
    </div>
  );
}

describe('Toast System', () => {
  describe('ToastProvider', () => {
    it('should render children', () => {
      render(
        <ToastProvider>
          <div>Child Content</div>
        </ToastProvider>
      );

      expect(screen.getByText('Child Content')).toBeInTheDocument();
    });
  });

  describe('useSuccessToast', () => {
    it('should show success toast when called', async () => {
      const user = userEvent.setup();

      render(
        <ToastProvider>
          <TestComponent type="success" />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));

      await waitFor(() => {
        expect(screen.getByText('Success Title')).toBeInTheDocument();
      });
      expect(screen.getByText('Success message')).toBeInTheDocument();
    });
  });

  describe('useInfoToast', () => {
    it('should show info toast when called', async () => {
      const user = userEvent.setup();

      render(
        <ToastProvider>
          <TestComponent type="info" />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));

      await waitFor(() => {
        expect(screen.getByText('Info Title')).toBeInTheDocument();
      });
      expect(screen.getByText('Info message')).toBeInTheDocument();
    });
  });

  describe('useErrorToast', () => {
    it('should show error toast when called', async () => {
      const user = userEvent.setup();

      render(
        <ToastProvider>
          <TestComponent type="error" />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));

      await waitFor(() => {
        expect(screen.getByText('Error Title')).toBeInTheDocument();
      });
      expect(screen.getByText('Error message')).toBeInTheDocument();
    });
  });

  describe('useWarningToast', () => {
    it('should show warning toast when called', async () => {
      const user = userEvent.setup();

      render(
        <ToastProvider>
          <TestComponent type="warning" />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));

      await waitFor(() => {
        expect(screen.getByText('Warning Title')).toBeInTheDocument();
      });
      expect(screen.getByText('Warning message')).toBeInTheDocument();
    });
  });

  describe('toast dismissal', () => {
    it('should dismiss toast when clicking dismiss button', async () => {
      const user = userEvent.setup();

      render(
        <ToastProvider>
          <TestComponent type="success" />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));

      await waitFor(() => {
        expect(screen.getByText('Success Title')).toBeInTheDocument();
      });

      const dismissButton = screen.getByLabelText('Dismiss notification');
      await user.click(dismissButton);

      await waitFor(() => {
        expect(screen.queryByText('Success Title')).not.toBeInTheDocument();
      });
    });
  });

  describe('useToast hook', () => {
    it('should throw error when used outside ToastProvider', () => {
      // Suppress console.error for this test
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        render(<DirectToastComponent />);
      }).toThrow('useToast must be used within a ToastProvider');

      consoleSpy.mockRestore();
    });

    it('should provide addToast and toasts from context', async () => {
      const user = userEvent.setup();

      render(
        <ToastProvider>
          <DirectToastComponent />
        </ToastProvider>
      );

      expect(screen.getByTestId('toast-count')).toHaveTextContent('0');

      await user.click(screen.getByText('Add Toast'));

      await waitFor(() => {
        expect(screen.getByTestId('toast-count')).toHaveTextContent('1');
      });
      expect(screen.getByText('Direct Toast')).toBeInTheDocument();
    });
  });

  describe('multiple toasts', () => {
    it('should display multiple toasts', async () => {
      const user = userEvent.setup();

      render(
        <ToastProvider>
          <TestComponent type="success" />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));
      await user.click(screen.getByText('Show Toast'));

      await waitFor(() => {
        const titles = screen.getAllByText('Success Title');
        expect(titles).toHaveLength(2);
      });
    });
  });
});
