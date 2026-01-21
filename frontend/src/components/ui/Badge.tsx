// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import React from 'react';
import { clsx } from 'clsx';

interface BadgeProps {
  variant: 'success' | 'warning' | 'error' | 'info' | 'trial' | 'starter' | 'professional' | 'enterprise' | 'destructive' | 'secondary' | 'mobility';
  children: React.ReactNode;
  className?: string;
}

export const Badge: React.FC<BadgeProps> = ({ variant, children, className }) => {
  const classes = clsx(
    'badge',
    {
      'badge-success': variant === 'success',
      'badge-warning': variant === 'warning',
      'badge-error': variant === 'error',
      'badge-info': variant === 'info',
      'badge-trial': variant === 'trial',
      'badge-starter': variant === 'starter',
      'badge-professional': variant === 'professional',
      'badge-enterprise': variant === 'enterprise',
      'badge-destructive': variant === 'destructive',
      'badge-secondary': variant === 'secondary',
      'badge-mobility': variant === 'mobility',
    },
    className
  );

  return <span className={classes}>{children}</span>;
};