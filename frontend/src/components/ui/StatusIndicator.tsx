import React from 'react';
import { clsx } from 'clsx';

interface StatusIndicatorProps {
  status: 'online' | 'offline' | 'error';
  label: string;
}

export const StatusIndicator: React.FC<StatusIndicatorProps> = ({ status, label }) => {
  const dotClasses = clsx('status-dot', {
    'status-online': status === 'online',
    'status-offline': status === 'offline',
    'status-error': status === 'error',
  });

  return (
    <div className="flex items-center">
      <span className={dotClasses} />
      <span className="text-sm">{label}</span>
    </div>
  );
};