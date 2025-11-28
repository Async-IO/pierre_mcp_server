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
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, helpText, leftIcon, rightIcon, size = 'md', className = '', id, ...props }, ref) => {
    const inputId = id || `input-${Math.random().toString(36).substring(7)}`;

    const sizeClasses = {
      sm: 'px-3 py-2 text-sm',
      md: 'px-4 py-2.5 text-sm',
      lg: 'px-4 py-3 text-base',
    };

    const baseInputClasses = `
      w-full border rounded-lg transition-all duration-base
      bg-white text-pierre-gray-900 placeholder-pierre-gray-400
      focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-20 focus:border-pierre-violet
      disabled:bg-pierre-gray-100 disabled:text-pierre-gray-500 disabled:cursor-not-allowed
    `;

    const errorClasses = error
      ? 'border-pierre-red-500 focus:ring-pierre-red-500 focus:ring-opacity-20 focus:border-pierre-red-500'
      : 'border-pierre-gray-300';

    const iconPaddingLeft = leftIcon ? 'pl-10' : '';
    const iconPaddingRight = rightIcon ? 'pr-10' : '';

    return (
      <div className="w-full">
        {label && (
          <label htmlFor={inputId} className="block text-sm font-medium text-pierre-gray-700 mb-1.5">
            {label}
          </label>
        )}
        <div className="relative">
          {leftIcon && (
            <div className="absolute inset-y-0 left-0 flex items-center pl-3 pointer-events-none text-pierre-gray-400">
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
            <div className="absolute inset-y-0 right-0 flex items-center pr-3 text-pierre-gray-400">
              {rightIcon}
            </div>
          )}
        </div>
        {error && (
          <p className="mt-1.5 text-sm text-pierre-red-500">{error}</p>
        )}
        {helpText && !error && (
          <p className="mt-1.5 text-sm text-pierre-gray-500">{helpText}</p>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';
