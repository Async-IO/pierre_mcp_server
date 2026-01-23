// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import React from 'react';
import { clsx } from 'clsx';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'gradient' | 'secondary' | 'danger' | 'success' | 'outline' | 'pill' | 'activity' | 'nutrition' | 'recovery';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  children: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  variant = 'primary',
  size = 'md',
  loading = false,
  disabled,
  children,
  className,
  ...props
}) => {
  const classes = clsx(
    'btn-base',
    {
      'btn-primary': variant === 'primary',
      'btn-gradient': variant === 'gradient',
      'btn-secondary': variant === 'secondary',
      'btn-danger': variant === 'danger',
      'btn-success': variant === 'success',
      'btn-outline': variant === 'outline',
      'btn-pill': variant === 'pill',
      'btn-activity': variant === 'activity',
      'btn-nutrition': variant === 'nutrition',
      'btn-recovery': variant === 'recovery',
      'btn-sm': size === 'sm',
      'btn-lg': size === 'lg',
    },
    className
  );

  return (
    <button
      className={classes}
      disabled={disabled || loading}
      {...props}
    >
      {loading && (
        <div className="pierre-spinner w-4 h-4 mr-2" />
      )}
      {children}
    </button>
  );
};