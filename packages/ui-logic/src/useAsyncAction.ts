// ABOUTME: Headless async action hook for loading/error/success state management
// ABOUTME: Shared logic for async operations across web and mobile

import { useState, useCallback, useRef, useEffect } from 'react';

/**
 * State of an async action
 */
export type AsyncActionState = 'idle' | 'loading' | 'success' | 'error';

/**
 * Props for useAsyncAction hook
 */
export interface UseAsyncActionProps<TResult, TError = Error> {
  /** The async function to execute */
  action: () => Promise<TResult>;
  /** Callback on successful completion */
  onSuccess?: (result: TResult) => void;
  /** Callback on error */
  onError?: (error: TError) => void;
  /** Reset to idle after success (ms). Set to 0 to disable auto-reset */
  successResetDelay?: number;
  /** Reset to idle after error (ms). Set to 0 to disable auto-reset */
  errorResetDelay?: number;
}

/**
 * Return type for useAsyncAction hook
 */
export interface UseAsyncActionReturn<TResult, TError = Error> {
  /** Current state of the action */
  state: AsyncActionState;
  /** Whether action is currently loading */
  isLoading: boolean;
  /** Whether action completed successfully */
  isSuccess: boolean;
  /** Whether action failed with error */
  isError: boolean;
  /** Whether action is idle (not started or reset) */
  isIdle: boolean;
  /** The result from successful execution */
  result: TResult | null;
  /** The error from failed execution */
  error: TError | null;
  /** Execute the async action */
  execute: () => Promise<TResult | null>;
  /** Reset state to idle */
  reset: () => void;
}

/**
 * Headless async action hook
 *
 * Manages the lifecycle of async operations:
 * - Tracks loading/success/error states
 * - Stores result and error
 * - Provides auto-reset after success/error
 * - Prevents duplicate executions while loading
 *
 * @example
 * const { execute, isLoading, isError, error } = useAsyncAction({
 *   action: () => api.saveData(formData),
 *   onSuccess: () => showToast('Saved!'),
 *   onError: (e) => showToast(`Error: ${e.message}`),
 * });
 *
 * return (
 *   <Button loading={isLoading} onClick={execute}>
 *     Save
 *   </Button>
 * );
 */
export function useAsyncAction<TResult, TError = Error>({
  action,
  onSuccess,
  onError,
  successResetDelay = 3000,
  errorResetDelay = 5000,
}: UseAsyncActionProps<TResult, TError>): UseAsyncActionReturn<TResult, TError> {
  const [state, setState] = useState<AsyncActionState>('idle');
  const [result, setResult] = useState<TResult | null>(null);
  const [error, setError] = useState<TError | null>(null);
  const resetTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isLoadingRef = useRef(false);

  // Keep refs in sync with latest callback props to avoid stale closures
  const actionRef = useRef(action);
  const onSuccessRef = useRef(onSuccess);
  const onErrorRef = useRef(onError);
  useEffect(() => { actionRef.current = action; }, [action]);
  useEffect(() => { onSuccessRef.current = onSuccess; }, [onSuccess]);
  useEffect(() => { onErrorRef.current = onError; }, [onError]);

  const clearResetTimeout = useCallback(() => {
    if (resetTimeoutRef.current) {
      clearTimeout(resetTimeoutRef.current);
      resetTimeoutRef.current = null;
    }
  }, []);

  const reset = useCallback(() => {
    clearResetTimeout();
    isLoadingRef.current = false;
    setState('idle');
    setResult(null);
    setError(null);
  }, [clearResetTimeout]);

  const scheduleReset = useCallback((delay: number) => {
    if (delay > 0) {
      clearResetTimeout();
      resetTimeoutRef.current = setTimeout(reset, delay);
    }
  }, [clearResetTimeout, reset]);

  const execute = useCallback(async (): Promise<TResult | null> => {
    // Prevent duplicate executions using ref to avoid stale closure on state
    if (isLoadingRef.current) {
      return null;
    }

    isLoadingRef.current = true;
    clearResetTimeout();
    setState('loading');
    setError(null);

    try {
      const actionResult = await actionRef.current();
      setResult(actionResult);
      setState('success');
      isLoadingRef.current = false;
      onSuccessRef.current?.(actionResult);
      scheduleReset(successResetDelay);
      return actionResult;
    } catch (e) {
      const actionError = e as TError;
      setError(actionError);
      setState('error');
      isLoadingRef.current = false;
      onErrorRef.current?.(actionError);
      scheduleReset(errorResetDelay);
      return null;
    }
  }, [clearResetTimeout, scheduleReset, successResetDelay, errorResetDelay]);

  return {
    state,
    isLoading: state === 'loading',
    isSuccess: state === 'success',
    isError: state === 'error',
    isIdle: state === 'idle',
    result,
    error,
    execute,
    reset,
  };
}
