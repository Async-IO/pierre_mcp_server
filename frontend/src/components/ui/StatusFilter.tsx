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
    <div className={`inline-flex rounded-lg border border-pierre-gray-200 bg-pierre-gray-50 p-1 ${className}`}>
      {options.map((option) => (
        <button
          key={option.value}
          onClick={() => onChange(option.value)}
          className={`
            px-3 py-1.5 text-sm font-medium rounded-md transition-all duration-150
            ${value === option.value
              ? 'bg-white text-pierre-gray-900 shadow-sm'
              : 'text-pierre-gray-600 hover:text-pierre-gray-900'
            }
          `}
        >
          {option.label}
          {option.count !== undefined && (
            <span className={`ml-1.5 text-xs ${value === option.value ? 'text-pierre-gray-500' : 'text-pierre-gray-400'}`}>
              ({option.count})
            </span>
          )}
        </button>
      ))}
    </div>
  );
};
