// ABOUTME: Headless form field hook for validation and state management
// ABOUTME: Shared form field logic for web and mobile inputs

import { useState, useCallback, useMemo } from 'react';

/**
 * Validation result
 */
export interface ValidationResult {
  valid: boolean;
  message?: string;
}

/**
 * Validator function type
 */
export type Validator<T> = (value: T) => ValidationResult;

/**
 * Props for useFormField hook
 */
export interface UseFormFieldProps<T> {
  /** Initial value */
  initialValue: T;
  /** Array of validator functions */
  validators?: Validator<T>[];
  /** Validate on every change (default: false, validate on blur) */
  validateOnChange?: boolean;
  /** Transform value before setting (e.g., trim, lowercase) */
  transform?: (value: T) => T;
}

/**
 * Return type for useFormField hook
 */
export interface UseFormFieldReturn<T> {
  /** Current field value */
  value: T;
  /** Set field value */
  setValue: (value: T) => void;
  /** Whether field has been touched (focused then blurred) */
  touched: boolean;
  /** Whether field is currently focused */
  focused: boolean;
  /** Whether field has been modified from initial value */
  dirty: boolean;
  /** Current validation error message (null if valid) */
  error: string | null;
  /** Whether field is currently valid */
  isValid: boolean;
  /** Handler for value change */
  onChange: (value: T) => void;
  /** Handler for blur event */
  onBlur: () => void;
  /** Handler for focus event */
  onFocus: () => void;
  /** Validate the field manually */
  validate: () => boolean;
  /** Reset field to initial state */
  reset: () => void;
  /** Props to spread on input (web) */
  inputProps: {
    value: T;
    onChange: (value: T) => void;
    onBlur: () => void;
    onFocus: () => void;
  };
  /** Accessibility props */
  a11yProps: {
    'aria-invalid': boolean;
    'aria-describedby'?: string;
  };
}

/**
 * Headless form field hook
 *
 * Manages form field state and validation:
 * - Value state with optional transform
 * - Touched/dirty tracking
 * - Validation with multiple validators
 * - Blur and change validation modes
 *
 * @example
 * const emailField = useFormField({
 *   initialValue: '',
 *   validators: [
 *     (v) => ({ valid: v.length > 0, message: 'Email is required' }),
 *     (v) => ({ valid: v.includes('@'), message: 'Invalid email format' }),
 *   ],
 * });
 *
 * return (
 *   <Input
 *     value={emailField.value}
 *     onChangeText={emailField.onChange}
 *     onBlur={emailField.onBlur}
 *     error={emailField.touched ? emailField.error : null}
 *   />
 * );
 */
export function useFormField<T>({
  initialValue,
  validators = [],
  validateOnChange = false,
  transform,
}: UseFormFieldProps<T>): UseFormFieldReturn<T> {
  const [value, setValueState] = useState<T>(initialValue);
  const [touched, setTouched] = useState(false);
  const [focused, setFocused] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const dirty = useMemo(() => value !== initialValue, [value, initialValue]);

  const runValidation = useCallback((val: T): boolean => {
    for (const validator of validators) {
      const result = validator(val);
      if (!result.valid) {
        setError(result.message || 'Invalid value');
        return false;
      }
    }
    setError(null);
    return true;
  }, [validators]);

  const setValue = useCallback((newValue: T) => {
    const transformedValue = transform ? transform(newValue) : newValue;
    setValueState(transformedValue);

    if (validateOnChange) {
      runValidation(transformedValue);
    }
  }, [transform, validateOnChange, runValidation]);

  const onChange = useCallback((newValue: T) => {
    setValue(newValue);
  }, [setValue]);

  const onBlur = useCallback(() => {
    setFocused(false);
    setTouched(true);

    // Validate on blur if not validating on change
    if (!validateOnChange) {
      runValidation(value);
    }
  }, [value, validateOnChange, runValidation]);

  const onFocus = useCallback(() => {
    setFocused(true);
  }, []);

  const validate = useCallback((): boolean => {
    setTouched(true);
    return runValidation(value);
  }, [value, runValidation]);

  const reset = useCallback(() => {
    setValueState(initialValue);
    setTouched(false);
    setFocused(false);
    setError(null);
  }, [initialValue]);

  const isValid = error === null;

  const inputProps = useMemo(() => ({
    value,
    onChange,
    onBlur,
    onFocus,
  }), [value, onChange, onBlur, onFocus]);

  const a11yProps = useMemo(() => ({
    'aria-invalid': touched && !isValid,
    ...(error && touched ? { 'aria-describedby': 'field-error' } : {}),
  }), [touched, isValid, error]);

  return {
    value,
    setValue,
    touched,
    focused,
    dirty,
    error,
    isValid,
    onChange,
    onBlur,
    onFocus,
    validate,
    reset,
    inputProps,
    a11yProps,
  };
}

// ============================================================================
// Common Validators
// ============================================================================

/**
 * Required field validator
 */
export function required(message = 'This field is required'): Validator<string> {
  return (value) => ({
    valid: value.trim().length > 0,
    message,
  });
}

/**
 * Minimum length validator
 */
export function minLength(min: number, message?: string): Validator<string> {
  return (value) => ({
    valid: value.length >= min,
    message: message || `Must be at least ${min} characters`,
  });
}

/**
 * Maximum length validator
 */
export function maxLength(max: number, message?: string): Validator<string> {
  return (value) => ({
    valid: value.length <= max,
    message: message || `Must be at most ${max} characters`,
  });
}

/**
 * Email format validator
 */
export function email(message = 'Invalid email format'): Validator<string> {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return (value) => ({
    valid: value.length === 0 || emailRegex.test(value),
    message,
  });
}

/**
 * Numeric range validator
 */
export function range(min: number, max: number, message?: string): Validator<number> {
  return (value) => ({
    valid: value >= min && value <= max,
    message: message || `Must be between ${min} and ${max}`,
  });
}

/**
 * Pattern/regex validator
 */
export function pattern(regex: RegExp, message = 'Invalid format'): Validator<string> {
  return (value) => ({
    valid: value.length === 0 || regex.test(value),
    message,
  });
}
