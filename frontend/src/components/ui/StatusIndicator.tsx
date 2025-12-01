// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import React from 'react';
import { clsx } from 'clsx';

interface StatusIndicatorProps {
  status: 'online' | 'offline' | 'error';
  label?: string;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export const StatusIndicator: React.FC<StatusIndicatorProps> = ({ status, label, size = 'md', className }) => {
  const dotClasses = clsx('status-dot', {
    'status-online': status === 'online',
    'status-offline': status === 'offline',
    'status-error': status === 'error',
    'status-sm': size === 'sm',
    'status-lg': size === 'lg',
  }, className);

  if (!label) {
    return <span className={dotClasses} />;
  }

  return (
    <div className="flex items-center">
      <span className={dotClasses} />
      <span className="text-sm">{label}</span>
    </div>
  );
};