// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import React from 'react';
import { clsx } from 'clsx';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  variant?: 'default' | 'stat' | 'dark';
  onClick?: () => void;
}

export const Card: React.FC<CardProps> = ({
  children,
  className,
  variant = 'default',
  onClick
}) => {
  const classes = clsx(
    {
      'card': variant === 'default',
      'stat-card': variant === 'stat',
      'card-dark': variant === 'dark',
    },
    className
  );

  return <div className={classes} onClick={onClick}>{children}</div>;
};

interface CardHeaderProps {
  title: string;
  subtitle?: string;
  children?: React.ReactNode;
}

export const CardHeader: React.FC<CardHeaderProps> = ({ 
  title, 
  subtitle, 
  children 
}) => (
  <div className="card-header">
    <div className="flex justify-between items-start">
      <div>
        <h3 className="text-lg font-semibold text-pierre-gray-900 m-0">{title}</h3>
        {subtitle && (
          <p className="text-sm text-pierre-gray-500 mt-1 m-0">{subtitle}</p>
        )}
      </div>
      {children}
    </div>
  </div>
);