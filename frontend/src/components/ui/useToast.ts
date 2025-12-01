// ABOUTME: Toast hooks for showing notifications with Pierre design system styling
// ABOUTME: Separates hooks from components to enable React Fast Refresh

import { useContext } from 'react';
import { ToastContext } from './ToastContext';

export const useToast = () => {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider');
  }
  return context;
};

// Convenience hooks for specific toast types
export const useSuccessToast = () => {
  const { addToast } = useToast();
  return (title: string, message?: string, duration?: number) =>
    addToast({ type: 'success', title, message, duration });
};

export const useErrorToast = () => {
  const { addToast } = useToast();
  return (title: string, message?: string, duration?: number) =>
    addToast({ type: 'error', title, message, duration });
};

export const useWarningToast = () => {
  const { addToast } = useToast();
  return (title: string, message?: string, duration?: number) =>
    addToast({ type: 'warning', title, message, duration });
};

export const useInfoToast = () => {
  const { addToast } = useToast();
  return (title: string, message?: string, duration?: number) =>
    addToast({ type: 'info', title, message, duration });
};
