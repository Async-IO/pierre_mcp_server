// ABOUTME: Headless button hook for shared button state and behavior
// ABOUTME: Platform-agnostic logic for loading, disabled states and press handling

import { useCallback, useMemo } from 'react';

/**
 * Button variant types supported by both web and mobile
 */
export type ButtonVariant =
  | 'primary'
  | 'secondary'
  | 'gradient'
  | 'danger'
  | 'success'
  | 'outline'
  | 'ghost'
  | 'pill'
  | 'activity'
  | 'nutrition'
  | 'recovery';

/**
 * Button size types
 */
export type ButtonSize = 'sm' | 'md' | 'lg';

/**
 * Props for useButton hook
 */
export interface UseButtonProps {
  /** Whether the button is in loading state */
  loading?: boolean;
  /** Whether the button is disabled */
  disabled?: boolean;
  /** Click/press handler */
  onClick?: () => void;
  /** Button variant for styling hints */
  variant?: ButtonVariant;
  /** Button size for styling hints */
  size?: ButtonSize;
}

/**
 * Return type for useButton hook
 */
export interface UseButtonReturn {
  /** Combined disabled state (disabled OR loading) */
  isDisabled: boolean;
  /** Whether button is currently loading */
  isLoading: boolean;
  /** Safe press handler that respects disabled state */
  handlePress: () => void;
  /** Accessibility state for screen readers */
  accessibilityState: {
    disabled: boolean;
    busy: boolean;
  };
  /** ARIA props for web (spread onto button element) */
  ariaProps: {
    'aria-disabled': boolean;
    'aria-busy': boolean;
  };
  /** Style hints derived from variant/size (for platform-specific styling) */
  styleHints: {
    variant: ButtonVariant;
    size: ButtonSize;
  };
}

/**
 * Headless button hook
 *
 * Provides shared button logic for both web and mobile:
 * - Combined disabled/loading state
 * - Safe press handling
 * - Accessibility attributes
 *
 * @example
 * // Web usage
 * const { isDisabled, handlePress, ariaProps } = useButton({ loading, disabled, onClick });
 * return <button onClick={handlePress} disabled={isDisabled} {...ariaProps}>{children}</button>;
 *
 * @example
 * // Mobile usage
 * const { isDisabled, handlePress, accessibilityState } = useButton({ loading, disabled, onClick: onPress });
 * return <TouchableOpacity onPress={handlePress} disabled={isDisabled} accessibilityState={accessibilityState} />;
 */
export function useButton({
  loading = false,
  disabled = false,
  onClick,
  variant = 'primary',
  size = 'md',
}: UseButtonProps): UseButtonReturn {
  const isDisabled = disabled || loading;
  const isLoading = loading;

  const handlePress = useCallback(() => {
    if (!isDisabled && onClick) {
      onClick();
    }
  }, [isDisabled, onClick]);

  const accessibilityState = useMemo(() => ({
    disabled: isDisabled,
    busy: isLoading,
  }), [isDisabled, isLoading]);

  const ariaProps = useMemo(() => ({
    'aria-disabled': isDisabled,
    'aria-busy': isLoading,
  }), [isDisabled, isLoading]);

  const styleHints = useMemo(() => ({
    variant,
    size,
  }), [variant, size]);

  return {
    isDisabled,
    isLoading,
    handlePress,
    accessibilityState,
    ariaProps,
    styleHints,
  };
}
