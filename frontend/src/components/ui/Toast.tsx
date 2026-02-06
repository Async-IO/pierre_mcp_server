// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Toast notification system with Pierre design system styling
// ABOUTME: Features auto-dismiss, progress indicator, and semantic pillar colors

import React, { useState, useCallback, useEffect, useRef } from 'react';
import { useToast as useToastLogic } from '@pierre/ui-logic';
import { ToastContext, type Toast, type ToastType } from './ToastContext';

interface ToastProviderProps {
  children: React.ReactNode;
}

export const ToastProvider: React.FC<ToastProviderProps> = ({ children }) => {
  // Delegate queue management and auto-dismiss to @pierre/ui-logic
  const toastLogic = useToastLogic({ maxToasts: 5, defaultDuration: 5000 });

  // Store extended toast data (title + optional description) alongside hook-managed queue.
  // The hook's ToastItem has only `message`, while the frontend uses title + message.
  const toastDataRef = useRef<Map<string, { title: string; message?: string }>>(new Map());

  const addToast = useCallback((toast: Omit<Toast, 'id'>) => {
    const id = toastLogic.addToast({
      message: toast.title,
      type: toast.type,
      duration: toast.duration,
    });
    toastDataRef.current.set(id, { title: toast.title, message: toast.message });
  }, [toastLogic]);

  const removeToast = useCallback((id: string) => {
    toastLogic.removeToast(id);
    toastDataRef.current.delete(id);
  }, [toastLogic]);

  // Map hook-managed queue to frontend Toast type
  const toasts: Toast[] = toastLogic.toasts.map(t => {
    const data = toastDataRef.current.get(t.id);
    return {
      id: t.id,
      type: t.type,
      title: data?.title ?? t.message,
      message: data?.message,
      duration: t.duration,
    };
  });

  return (
    <ToastContext.Provider value={{ toasts, addToast, removeToast }}>
      {children}
      <ToastContainer toasts={toasts} removeToast={removeToast} />
    </ToastContext.Provider>
  );
};

interface ToastContainerProps {
  toasts: Toast[];
  removeToast: (id: string) => void;
}

const ToastContainer: React.FC<ToastContainerProps> = ({ toasts, removeToast }) => {
  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-3 max-w-sm w-full pointer-events-none">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={() => removeToast(toast.id)} />
      ))}
    </div>
  );
};

interface ToastItemProps {
  toast: Toast;
  onDismiss: () => void;
}

const ToastItem: React.FC<ToastItemProps> = ({ toast, onDismiss }) => {
  const duration = toast.duration ?? 5000;
  const [progress, setProgress] = useState(100);

  useEffect(() => {
    if (duration <= 0) return;

    const startTime = Date.now();
    const interval = setInterval(() => {
      const elapsed = Date.now() - startTime;
      const remaining = Math.max(0, 100 - (elapsed / duration) * 100);
      setProgress(remaining);

      if (remaining <= 0) {
        clearInterval(interval);
        onDismiss();
      }
    }, 50);

    return () => clearInterval(interval);
  }, [duration, onDismiss]);

  const typeStyles: Record<ToastType, { bg: string; border: string; icon: string; progressBg: string }> = {
    success: {
      bg: 'bg-white',
      border: 'border-pierre-activity',
      icon: 'text-pierre-activity',
      progressBg: 'bg-pierre-activity',
    },
    error: {
      bg: 'bg-white',
      border: 'border-pierre-red-500',
      icon: 'text-pierre-red-500',
      progressBg: 'bg-pierre-red-500',
    },
    warning: {
      bg: 'bg-white',
      border: 'border-pierre-nutrition',
      icon: 'text-pierre-nutrition',
      progressBg: 'bg-pierre-nutrition',
    },
    info: {
      bg: 'bg-white',
      border: 'border-pierre-recovery',
      icon: 'text-pierre-recovery',
      progressBg: 'bg-pierre-recovery',
    },
  };

  const style = typeStyles[toast.type];

  const icons: Record<ToastType, React.ReactNode> = {
    success: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
      </svg>
    ),
    error: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
      </svg>
    ),
    warning: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
        />
      </svg>
    ),
    info: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
    ),
  };

  return (
    <div
      className={`
        ${style.bg} ${style.border} border-l-4 rounded-lg shadow-lg overflow-hidden
        pointer-events-auto animate-slide-up
      `}
    >
      <div className="p-4">
        <div className="flex items-start gap-3">
          <div className={`flex-shrink-0 ${style.icon}`}>{icons[toast.type]}</div>
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-pierre-gray-900">{toast.title}</p>
            {toast.message && <p className="mt-1 text-sm text-pierre-gray-500">{toast.message}</p>}
          </div>
          <button
            type="button"
            onClick={onDismiss}
            aria-label="Dismiss notification"
            className="flex-shrink-0 p-1 text-pierre-gray-400 hover:text-pierre-gray-600 rounded transition-colors"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {/* Progress bar */}
      {duration > 0 && (
        <div className="h-1 bg-pierre-gray-100">
          <div
            className={`h-full ${style.progressBg} transition-all duration-100`}
            style={{ width: `${progress}%` }}
          />
        </div>
      )}
    </div>
  );
};
