// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unified status filter component for list views
// ABOUTME: Provides consistent Active/All/Inactive filtering across Admin Tokens, API Keys, and A2A Clients

import React from 'react';

export type StatusFilterValue = 'active' | 'all' | 'inactive';

export interface StatusFilterProps {
  value: StatusFilterValue;
  onChange: (value: StatusFilterValue) => void;
  activeCount?: number;
  inactiveCount?: number;
  totalCount?: number;
  className?: string;
}

export const StatusFilter: React.FC<StatusFilterProps> = ({
  value,
  onChange,
  activeCount,
  inactiveCount,
  totalCount,
  className = '',
}) => {
  const options: { value: StatusFilterValue; label: string; count?: number }[] = [
    { value: 'active', label: 'Active', count: activeCount },
    { value: 'all', label: 'All', count: totalCount },
    { value: 'inactive', label: 'Inactive', count: inactiveCount },
  ];

  return (
    <div className={`inline-flex rounded-lg border border-white/10 bg-pierre-slate/60 p-1 ${className}`}>
      {options.map((option) => (
        <button
          key={option.value}
          onClick={() => onChange(option.value)}
          className={`
            px-3 py-1.5 text-sm font-medium rounded-md transition-all duration-150
            ${value === option.value
              ? 'bg-white/10 text-white shadow-sm'
              : 'text-zinc-400 hover:text-white hover:bg-white/5'
            }
          `}
        >
          {option.label}
          {option.count !== undefined && (
            <span className={`ml-1.5 text-xs ${value === option.value ? 'text-zinc-500' : 'text-zinc-500'}`}>
              ({option.count})
            </span>
          )}
        </button>
      ))}
    </div>
  );
};
