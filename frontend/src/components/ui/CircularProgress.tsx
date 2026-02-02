// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Circular progress indicator component for visual stat displays
// ABOUTME: Supports different color variants matching Pierre's design system

import React from 'react';
import { clsx } from 'clsx';

export interface CircularProgressProps {
  value: number;
  max: number;
  size?: 'sm' | 'md' | 'lg';
  variant?: 'violet' | 'cyan' | 'activity' | 'nutrition' | 'recovery' | 'gradient';
  showLabel?: boolean;
  label?: string;
  className?: string;
}

const sizeConfig = {
  sm: { diameter: 60, stroke: 6 },
  md: { diameter: 80, stroke: 8 },
  lg: { diameter: 100, stroke: 10 },
};

const variantColors = {
  violet: { stroke: '#8B5CF6', bg: 'rgba(139, 92, 246, 0.1)' },
  cyan: { stroke: '#22D3EE', bg: 'rgba(34, 211, 238, 0.1)' },
  activity: { stroke: '#4ADE80', bg: 'rgba(74, 222, 128, 0.1)' },
  nutrition: { stroke: '#F59E0B', bg: 'rgba(245, 158, 11, 0.1)' },
  recovery: { stroke: '#818CF8', bg: 'rgba(129, 140, 248, 0.1)' },
  gradient: { stroke: 'url(#progressGradient)', bg: 'rgba(139, 92, 246, 0.1)' },
};

export const CircularProgress: React.FC<CircularProgressProps> = ({
  value,
  max,
  size = 'md',
  variant = 'violet',
  showLabel = true,
  label,
  className,
}) => {
  const { diameter, stroke } = sizeConfig[size];
  const radius = (diameter - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const percentage = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  const offset = circumference - (percentage / 100) * circumference;
  const colors = variantColors[variant];

  return (
    <div className={clsx('relative inline-flex items-center justify-center', className)}>
      <svg width={diameter} height={diameter} className="transform -rotate-90">
        <defs>
          <linearGradient id="progressGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#8B5CF6" />
            <stop offset="100%" stopColor="#22D3EE" />
          </linearGradient>
        </defs>
        {/* Background circle */}
        <circle
          cx={diameter / 2}
          cy={diameter / 2}
          r={radius}
          fill="none"
          stroke={colors.bg}
          strokeWidth={stroke}
        />
        {/* Progress circle */}
        <circle
          cx={diameter / 2}
          cy={diameter / 2}
          r={radius}
          fill="none"
          stroke={colors.stroke}
          strokeWidth={stroke}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          className="transition-all duration-500 ease-out"
        />
      </svg>
      {showLabel && (
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className={clsx(
            'font-bold text-pierre-gray-900',
            size === 'sm' && 'text-xs',
            size === 'md' && 'text-sm',
            size === 'lg' && 'text-base'
          )}>
            {Math.round(percentage)}%
          </span>
          {label && (
            <span className={clsx(
              'text-pierre-gray-500',
              size === 'sm' && 'text-[8px]',
              size === 'md' && 'text-[10px]',
              size === 'lg' && 'text-xs'
            )}>
              {label}
            </span>
          )}
        </div>
      )}
    </div>
  );
};
