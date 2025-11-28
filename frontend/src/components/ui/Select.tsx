// ABOUTME: Reusable Select dropdown component with Pierre design system styling
// ABOUTME: Supports error states, custom arrow icon, and consistent styling

import React, { forwardRef } from 'react';

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'size'> {
  label?: string;
  error?: string;
  helpText?: string;
  options: SelectOption[];
  placeholder?: string;
  size?: 'sm' | 'md' | 'lg';
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ label, error, helpText, options, placeholder, size = 'md', className = '', id, ...props }, ref) => {
    const selectId = id || `select-${Math.random().toString(36).substring(7)}`;

    const sizeClasses = {
      sm: 'px-3 py-2 text-sm',
      md: 'px-4 py-2.5 text-sm',
      lg: 'px-4 py-3 text-base',
    };

    const baseSelectClasses = `
      w-full border rounded-lg transition-all duration-base appearance-none cursor-pointer
      bg-white text-pierre-gray-900
      focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-20 focus:border-pierre-violet
      disabled:bg-pierre-gray-100 disabled:text-pierre-gray-500 disabled:cursor-not-allowed
    `;

    const errorClasses = error
      ? 'border-pierre-red-500 focus:ring-pierre-red-500 focus:ring-opacity-20 focus:border-pierre-red-500'
      : 'border-pierre-gray-300';

    return (
      <div className="w-full">
        {label && (
          <label htmlFor={selectId} className="block text-sm font-medium text-pierre-gray-700 mb-1.5">
            {label}
          </label>
        )}
        <div className="relative">
          <select
            ref={ref}
            id={selectId}
            className={`${baseSelectClasses} ${sizeClasses[size]} ${errorClasses} pr-10 ${className}`}
            {...props}
          >
            {placeholder && (
              <option value="" disabled>
                {placeholder}
              </option>
            )}
            {options.map((option) => (
              <option key={option.value} value={option.value} disabled={option.disabled}>
                {option.label}
              </option>
            ))}
          </select>
          <div className="absolute inset-y-0 right-0 flex items-center pr-3 pointer-events-none text-pierre-gray-400">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </div>
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

Select.displayName = 'Select';
