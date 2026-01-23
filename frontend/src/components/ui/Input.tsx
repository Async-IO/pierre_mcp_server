// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Reusable Input component with Pierre design system styling
// ABOUTME: Supports error states, help text, icons, and consistent focus rings

import React, { forwardRef } from 'react';

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'> {
  label?: string;
  error?: string;
  helpText?: string;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
  size?: 'sm' | 'md' | 'lg';
  variant?: 'light' | 'dark' | 'glass';
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, helpText, leftIcon, rightIcon, size = 'md', variant = 'light', className = '', id, ...props }, ref) => {
    const inputId = id || `input-${Math.random().toString(36).substring(7)}`;

    const sizeClasses = {
      sm: 'px-3 py-2 text-sm',
      md: 'px-4 py-2.5 text-sm',
      lg: 'px-4 py-3 text-base',
    };

    const variantClasses = {
      light: `
        bg-white text-pierre-gray-900 placeholder-pierre-gray-400
        disabled:bg-pierre-gray-100 disabled:text-pierre-gray-500
      `,
      dark: `
        bg-[#151520] text-white placeholder-zinc-500
        disabled:bg-zinc-900 disabled:text-zinc-600
      `,
      glass: `
        input-glass
      `,
    };

    const baseInputClasses = `
      w-full border rounded-lg transition-all duration-base
      focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-30 focus:border-pierre-violet
      disabled:cursor-not-allowed
      ${variantClasses[variant]}
    `;

    const errorClasses = error
      ? 'border-red-500/50 focus:ring-red-500 focus:ring-opacity-20 focus:border-red-500'
      : variant === 'dark' || variant === 'glass'
        ? 'border-white/10'
        : 'border-pierre-gray-300';

    const iconPaddingLeft = leftIcon ? 'pl-10' : '';
    const iconPaddingRight = rightIcon ? 'pr-10' : '';

    const labelClasses = variant === 'dark' || variant === 'glass'
      ? 'block text-sm font-medium text-zinc-300 mb-1.5'
      : 'block text-sm font-medium text-pierre-gray-700 mb-1.5';

    const iconClasses = variant === 'dark' || variant === 'glass' ? 'text-zinc-500' : 'text-pierre-gray-400';

    return (
      <div className="w-full">
        {label && (
          <label htmlFor={inputId} className={labelClasses}>
            {label}
          </label>
        )}
        <div className="relative">
          {leftIcon && (
            <div className={`absolute inset-y-0 left-0 flex items-center pl-3 pointer-events-none ${iconClasses}`}>
              {leftIcon}
            </div>
          )}
          <input
            ref={ref}
            id={inputId}
            className={`${baseInputClasses} ${sizeClasses[size]} ${errorClasses} ${iconPaddingLeft} ${iconPaddingRight} ${className}`}
            {...props}
          />
          {rightIcon && (
            <div className={`absolute inset-y-0 right-0 flex items-center pr-3 ${iconClasses}`}>
              {rightIcon}
            </div>
          )}
        </div>
        {error && (
          <p className={`mt-1.5 text-sm ${variant === 'dark' || variant === 'glass' ? 'text-red-400' : 'text-pierre-red-500'}`}>{error}</p>
        )}
        {helpText && !error && (
          <p className={`mt-1.5 text-sm ${variant === 'dark' || variant === 'glass' ? 'text-zinc-500' : 'text-pierre-gray-500'}`}>{helpText}</p>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';
