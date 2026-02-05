// ABOUTME: Headless toast hook for notification queue management
// ABOUTME: Platform-agnostic toast state and auto-dismiss logic

import { useState, useCallback, useRef, useEffect } from 'react';

/**
 * Toast severity/type
 */
export type ToastType = 'info' | 'success' | 'warning' | 'error';

/**
 * Toast item in the queue
 */
export interface ToastItem {
  id: string;
  message: string;
  type: ToastType;
  /** Duration in ms before auto-dismiss. 0 = no auto-dismiss */
  duration: number;
  /** Timestamp when toast was added */
  createdAt: number;
}

/**
 * Props for adding a toast
 */
export interface AddToastProps {
  message: string;
  type?: ToastType;
  /** Duration in ms. Defaults based on type: error=5000, others=3000 */
  duration?: number;
}

/**
 * Props for useToast hook
 */
export interface UseToastProps {
  /** Maximum number of toasts to show at once */
  maxToasts?: number;
  /** Default duration for toasts (ms) */
  defaultDuration?: number;
}

/**
 * Return type for useToast hook
 */
export interface UseToastReturn {
  /** Current toast queue */
  toasts: ToastItem[];
  /** Add a new toast */
  addToast: (props: AddToastProps) => string;
  /** Remove a specific toast by id */
  removeToast: (id: string) => void;
  /** Clear all toasts */
  clearAll: () => void;
  /** Convenience methods */
  showInfo: (message: string, duration?: number) => string;
  showSuccess: (message: string, duration?: number) => string;
  showWarning: (message: string, duration?: number) => string;
  showError: (message: string, duration?: number) => string;
}

/**
 * Generate unique toast ID
 */
function generateId(): string {
  return `toast-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Get default duration based on toast type
 */
function getDefaultDuration(type: ToastType, defaultDuration: number): number {
  // Errors stay longer by default
  return type === 'error' ? Math.max(defaultDuration, 5000) : defaultDuration;
}

/**
 * Headless toast hook
 *
 * Manages a queue of toast notifications:
 * - Auto-dismiss with configurable duration
 * - Maximum toast limit
 * - FIFO queue management
 *
 * @example
 * const { toasts, showSuccess, showError, removeToast } = useToast();
 *
 * // Show toasts
 * showSuccess('Profile saved!');
 * showError('Connection failed');
 *
 * // Render (platform-specific)
 * return toasts.map(toast => (
 *   <Toast key={toast.id} {...toast} onDismiss={() => removeToast(toast.id)} />
 * ));
 */
export function useToast({
  maxToasts = 5,
  defaultDuration = 3000,
}: UseToastProps = {}): UseToastReturn {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const timersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      timersRef.current.forEach(timer => clearTimeout(timer));
      timersRef.current.clear();
    };
  }, []);

  const removeToast = useCallback((id: string) => {
    // Clear the timer for this toast
    const timer = timersRef.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timersRef.current.delete(id);
    }

    setToasts(prev => prev.filter(t => t.id !== id));
  }, []);

  const addToast = useCallback(({ message, type = 'info', duration }: AddToastProps): string => {
    const id = generateId();
    const actualDuration = duration ?? getDefaultDuration(type, defaultDuration);

    const newToast: ToastItem = {
      id,
      message,
      type,
      duration: actualDuration,
      createdAt: Date.now(),
    };

    setToasts(prev => {
      // Remove oldest if at max capacity
      const updated = prev.length >= maxToasts ? prev.slice(1) : prev;
      return [...updated, newToast];
    });

    // Set auto-dismiss timer if duration > 0
    if (actualDuration > 0) {
      const timer = setTimeout(() => {
        removeToast(id);
      }, actualDuration);
      timersRef.current.set(id, timer);
    }

    return id;
  }, [defaultDuration, maxToasts, removeToast]);

  const clearAll = useCallback(() => {
    timersRef.current.forEach(timer => clearTimeout(timer));
    timersRef.current.clear();
    setToasts([]);
  }, []);

  // Convenience methods
  const showInfo = useCallback((message: string, duration?: number) =>
    addToast({ message, type: 'info', duration }), [addToast]);

  const showSuccess = useCallback((message: string, duration?: number) =>
    addToast({ message, type: 'success', duration }), [addToast]);

  const showWarning = useCallback((message: string, duration?: number) =>
    addToast({ message, type: 'warning', duration }), [addToast]);

  const showError = useCallback((message: string, duration?: number) =>
    addToast({ message, type: 'error', duration }), [addToast]);

  return {
    toasts,
    addToast,
    removeToast,
    clearAll,
    showInfo,
    showSuccess,
    showWarning,
    showError,
  };
}
