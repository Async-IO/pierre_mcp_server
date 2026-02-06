// ABOUTME: Unit tests for headless UI hooks (useButton, useAsyncAction, useModal, useFormField, useToast)
// ABOUTME: Tests hook state management, callbacks, and accessibility props

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useButton } from '../src/useButton';
import { useAsyncAction } from '../src/useAsyncAction';
import { useModal, useConfirmDialog } from '../src/useModal';
import { useFormField, required, email } from '../src/useFormField';
import { useToast } from '../src/useToast';

describe('useButton', () => {
  it('returns disabled=false and loading=false by default', () => {
    const { result } = renderHook(() => useButton({}));
    expect(result.current.isDisabled).toBe(false);
    expect(result.current.isLoading).toBe(false);
  });

  it('combines loading and disabled into isDisabled', () => {
    const { result } = renderHook(() => useButton({ loading: true }));
    expect(result.current.isDisabled).toBe(true);
    expect(result.current.isLoading).toBe(true);
  });

  it('calls onClick when pressed and not disabled', () => {
    const onClick = vi.fn();
    const { result } = renderHook(() => useButton({ onClick }));
    act(() => result.current.handlePress());
    expect(onClick).toHaveBeenCalledOnce();
  });

  it('does not call onClick when disabled', () => {
    const onClick = vi.fn();
    const { result } = renderHook(() => useButton({ onClick, disabled: true }));
    act(() => result.current.handlePress());
    expect(onClick).not.toHaveBeenCalled();
  });

  it('does not call onClick when loading', () => {
    const onClick = vi.fn();
    const { result } = renderHook(() => useButton({ onClick, loading: true }));
    act(() => result.current.handlePress());
    expect(onClick).not.toHaveBeenCalled();
  });

  it('provides accessibility state', () => {
    const { result } = renderHook(() => useButton({ loading: true }));
    expect(result.current.accessibilityState).toEqual({ disabled: true, busy: true });
    expect(result.current.ariaProps).toEqual({ 'aria-disabled': true, 'aria-busy': true });
  });

  it('provides style hints from variant and size', () => {
    const { result } = renderHook(() => useButton({ variant: 'danger', size: 'lg' }));
    expect(result.current.styleHints).toEqual({ variant: 'danger', size: 'lg' });
  });
});

describe('useAsyncAction', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts in idle state', () => {
    const { result } = renderHook(() =>
      useAsyncAction({ action: async () => 'result' })
    );
    expect(result.current.isIdle).toBe(true);
    expect(result.current.isLoading).toBe(false);
  });

  it('transitions through loading to success', async () => {
    let resolve: (value: string) => void;
    const action = () => new Promise<string>((r) => { resolve = r; });

    const { result } = renderHook(() =>
      useAsyncAction({ action })
    );

    let executePromise: Promise<string | null>;
    act(() => {
      executePromise = result.current.execute();
    });

    expect(result.current.isLoading).toBe(true);

    await act(async () => {
      resolve!('done');
      await executePromise;
    });

    expect(result.current.isSuccess).toBe(true);
    expect(result.current.result).toBe('done');
  });

  it('transitions to error state on failure', async () => {
    const action = async () => { throw new Error('fail'); };
    const onError = vi.fn();

    const { result } = renderHook(() =>
      useAsyncAction({ action, onError })
    );

    await act(async () => {
      await result.current.execute();
    });

    expect(result.current.isError).toBe(true);
    expect(result.current.error).toBeInstanceOf(Error);
    expect(onError).toHaveBeenCalled();
  });

  it('prevents duplicate execution while loading', async () => {
    let callCount = 0;
    const action = async () => { callCount++; return 'ok'; };

    const { result } = renderHook(() =>
      useAsyncAction({ action })
    );

    await act(async () => {
      // Fire two executions in quick succession
      const p1 = result.current.execute();
      const p2 = result.current.execute();
      await Promise.all([p1, p2]);
    });

    expect(callCount).toBe(1);
  });

  it('calls onSuccess callback', async () => {
    const onSuccess = vi.fn();
    const { result } = renderHook(() =>
      useAsyncAction({ action: async () => 42, onSuccess })
    );

    await act(async () => {
      await result.current.execute();
    });

    expect(onSuccess).toHaveBeenCalledWith(42);
  });

  it('resets to idle manually', async () => {
    const { result } = renderHook(() =>
      useAsyncAction({ action: async () => 'ok' })
    );

    await act(async () => {
      await result.current.execute();
    });

    expect(result.current.isSuccess).toBe(true);

    act(() => result.current.reset());
    expect(result.current.isIdle).toBe(true);
    expect(result.current.result).toBeNull();
  });
});

describe('useModal', () => {
  it('starts closed by default', () => {
    const { result } = renderHook(() => useModal());
    expect(result.current.isOpen).toBe(false);
  });

  it('starts open when initialOpen is true', () => {
    const { result } = renderHook(() => useModal({ initialOpen: true }));
    expect(result.current.isOpen).toBe(true);
  });

  it('opens and closes', () => {
    const { result } = renderHook(() => useModal());

    act(() => result.current.open());
    expect(result.current.isOpen).toBe(true);

    act(() => result.current.close());
    expect(result.current.isOpen).toBe(false);
  });

  it('toggles state', () => {
    const { result } = renderHook(() => useModal());

    act(() => result.current.toggle());
    expect(result.current.isOpen).toBe(true);

    act(() => result.current.toggle());
    expect(result.current.isOpen).toBe(false);
  });

  it('handles escape key when enabled', () => {
    const { result } = renderHook(() => useModal({ closeOnEscape: true }));

    act(() => result.current.open());
    expect(result.current.isOpen).toBe(true);

    act(() => result.current.escapeKeyHandler({ key: 'Escape' }));
    expect(result.current.isOpen).toBe(false);
  });

  it('ignores non-escape keys', () => {
    const { result } = renderHook(() => useModal({ closeOnEscape: true }));

    act(() => result.current.open());
    act(() => result.current.escapeKeyHandler({ key: 'Enter' }));
    expect(result.current.isOpen).toBe(true);
  });

  it('calls onOpen and onClose callbacks', () => {
    const onOpen = vi.fn();
    const onClose = vi.fn();

    const { result } = renderHook(() => useModal({ onOpen, onClose }));

    act(() => result.current.open());
    expect(onOpen).toHaveBeenCalledOnce();

    act(() => result.current.close());
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('closes on backdrop click when closeOnOutsideClick is true', () => {
    const { result } = renderHook(() => useModal({ closeOnOutsideClick: true }));

    act(() => result.current.open());

    const target = {};
    act(() => result.current.backdropProps.onClick({ target, currentTarget: target }));
    expect(result.current.isOpen).toBe(false);
  });

  it('does not close on content click', () => {
    const { result } = renderHook(() => useModal({ closeOnOutsideClick: true }));

    act(() => result.current.open());

    const stopPropagation = vi.fn();
    act(() => result.current.contentProps.onClick({ stopPropagation }));
    expect(stopPropagation).toHaveBeenCalled();
    expect(result.current.isOpen).toBe(true);
  });
});

describe('useConfirmDialog', () => {
  it('calls onConfirm and closes', async () => {
    const onConfirm = vi.fn().mockResolvedValue(undefined);

    const { result } = renderHook(() =>
      useConfirmDialog({ onConfirm })
    );

    act(() => result.current.open());
    expect(result.current.isOpen).toBe(true);

    await act(async () => {
      await result.current.confirm();
    });

    expect(onConfirm).toHaveBeenCalled();
    expect(result.current.isOpen).toBe(false);
  });

  it('calls onCancel and closes', () => {
    const onCancel = vi.fn();
    const { result } = renderHook(() =>
      useConfirmDialog({ onConfirm: vi.fn(), onCancel })
    );

    act(() => result.current.open());
    act(() => result.current.cancel());

    expect(onCancel).toHaveBeenCalled();
    expect(result.current.isOpen).toBe(false);
  });
});

describe('useFormField', () => {
  it('initializes with the provided value', () => {
    const { result } = renderHook(() =>
      useFormField({ initialValue: 'hello' })
    );
    expect(result.current.value).toBe('hello');
    expect(result.current.touched).toBe(false);
    expect(result.current.dirty).toBe(false);
  });

  it('updates value on change', () => {
    const { result } = renderHook(() =>
      useFormField({ initialValue: '' })
    );

    act(() => result.current.onChange('new value'));
    expect(result.current.value).toBe('new value');
    expect(result.current.dirty).toBe(true);
  });

  it('marks as touched on blur', () => {
    const { result } = renderHook(() =>
      useFormField({ initialValue: '' })
    );

    act(() => result.current.onBlur());
    expect(result.current.touched).toBe(true);
  });

  it('tracks focused state on focus and blur', () => {
    const { result } = renderHook(() =>
      useFormField({ initialValue: '' })
    );

    expect(result.current.focused).toBe(false);

    act(() => result.current.onFocus());
    expect(result.current.focused).toBe(true);

    act(() => result.current.onBlur());
    expect(result.current.focused).toBe(false);
  });

  it('validates on blur by default', () => {
    const { result } = renderHook(() =>
      useFormField({
        initialValue: '',
        validators: [required()],
      })
    );

    act(() => result.current.onBlur());
    expect(result.current.error).toBe('This field is required');
    expect(result.current.isValid).toBe(false);
  });

  it('validates on change when validateOnChange is true', () => {
    const { result } = renderHook(() =>
      useFormField({
        initialValue: '',
        validators: [required()],
        validateOnChange: true,
      })
    );

    act(() => result.current.onChange(''));
    expect(result.current.error).toBe('This field is required');
  });

  it('runs multiple validators in order', () => {
    const { result } = renderHook(() =>
      useFormField({
        initialValue: 'x',
        validators: [required(), email()],
        validateOnChange: true,
      })
    );

    act(() => result.current.onChange('x'));
    expect(result.current.error).toBe('Invalid email format');
  });

  it('clears error when value becomes valid', () => {
    const { result } = renderHook(() =>
      useFormField({
        initialValue: '',
        validators: [required()],
        validateOnChange: true,
      })
    );

    act(() => result.current.onChange(''));
    expect(result.current.error).toBeTruthy();

    act(() => result.current.onChange('valid'));
    expect(result.current.error).toBeNull();
  });

  it('applies transform function', () => {
    const { result } = renderHook(() =>
      useFormField({
        initialValue: '',
        transform: (v: string) => v.trim().toLowerCase(),
      })
    );

    act(() => result.current.onChange(' HELLO '));
    expect(result.current.value).toBe('hello');
  });

  it('resets to initial state', () => {
    const { result } = renderHook(() =>
      useFormField({ initialValue: 'initial' })
    );

    act(() => {
      result.current.onChange('changed');
      result.current.onBlur();
    });

    expect(result.current.dirty).toBe(true);
    expect(result.current.touched).toBe(true);

    act(() => result.current.reset());
    expect(result.current.value).toBe('initial');
    expect(result.current.dirty).toBe(false);
    expect(result.current.touched).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('provides a11y props', () => {
    const { result } = renderHook(() =>
      useFormField({
        initialValue: '',
        validators: [required()],
      })
    );

    // Before touching: aria-invalid should be false
    expect(result.current.a11yProps['aria-invalid']).toBe(false);

    // After touching and failing validation
    act(() => result.current.onBlur());
    expect(result.current.a11yProps['aria-invalid']).toBe(true);
  });

  it('provides inputProps for spreading', () => {
    const { result } = renderHook(() =>
      useFormField({ initialValue: 'test' })
    );

    expect(result.current.inputProps.value).toBe('test');
    expect(typeof result.current.inputProps.onChange).toBe('function');
    expect(typeof result.current.inputProps.onBlur).toBe('function');
    expect(typeof result.current.inputProps.onFocus).toBe('function');
  });
});

describe('useToast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts with empty toast queue', () => {
    const { result } = renderHook(() => useToast());
    expect(result.current.toasts).toHaveLength(0);
  });

  it('adds a toast', () => {
    const { result } = renderHook(() => useToast());

    act(() => {
      result.current.addToast({ message: 'Hello' });
    });

    expect(result.current.toasts).toHaveLength(1);
    expect(result.current.toasts[0].message).toBe('Hello');
    expect(result.current.toasts[0].type).toBe('info');
  });

  it('provides convenience methods', () => {
    const { result } = renderHook(() => useToast());

    act(() => {
      result.current.showInfo('Info');
      result.current.showSuccess('Success');
      result.current.showWarning('Warning');
      result.current.showError('Error');
    });

    expect(result.current.toasts).toHaveLength(4);
    expect(result.current.toasts[0].type).toBe('info');
    expect(result.current.toasts[1].type).toBe('success');
    expect(result.current.toasts[2].type).toBe('warning');
    expect(result.current.toasts[3].type).toBe('error');
  });

  it('removes a toast by id', () => {
    const { result } = renderHook(() => useToast());

    let toastId: string;
    act(() => {
      toastId = result.current.showInfo('Removable');
    });

    expect(result.current.toasts).toHaveLength(1);

    act(() => {
      result.current.removeToast(toastId);
    });

    expect(result.current.toasts).toHaveLength(0);
  });

  it('clears all toasts', () => {
    const { result } = renderHook(() => useToast());

    act(() => {
      result.current.showInfo('A');
      result.current.showInfo('B');
      result.current.showInfo('C');
    });

    expect(result.current.toasts).toHaveLength(3);

    act(() => result.current.clearAll());
    expect(result.current.toasts).toHaveLength(0);
  });

  it('respects maxToasts limit', () => {
    const { result } = renderHook(() => useToast({ maxToasts: 2 }));

    act(() => {
      result.current.showInfo('First');
      result.current.showInfo('Second');
      result.current.showInfo('Third');
    });

    expect(result.current.toasts).toHaveLength(2);
    // FIFO: first should be evicted
    expect(result.current.toasts[0].message).toBe('Second');
    expect(result.current.toasts[1].message).toBe('Third');
  });

  it('auto-dismisses after duration', () => {
    const { result } = renderHook(() => useToast({ defaultDuration: 1000 }));

    act(() => {
      result.current.showInfo('Temporary');
    });

    expect(result.current.toasts).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(result.current.toasts).toHaveLength(0);
  });

  it('uses longer duration for error toasts', () => {
    const { result } = renderHook(() => useToast({ defaultDuration: 1000 }));

    act(() => {
      result.current.showError('Error toast');
    });

    // Error uses max(defaultDuration, 5000) = 5000
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(result.current.toasts).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(4000);
    });
    expect(result.current.toasts).toHaveLength(0);
  });
});
