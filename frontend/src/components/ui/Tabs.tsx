// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Reusable Tabs component with Pierre design system styling
// ABOUTME: Supports icons, badges, and active state with violet underline

import React, { useCallback } from 'react';

export interface Tab {
  id: string;
  label: string;
  icon?: React.ReactNode;
  badge?: string | number;
  disabled?: boolean;
}

export interface TabsProps {
  tabs: Tab[];
  activeTab: string;
  onChange: (tabId: string) => void;
  variant?: 'underline' | 'pills' | 'bordered';
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export const Tabs: React.FC<TabsProps> = ({
  tabs,
  activeTab,
  onChange,
  variant = 'underline',
  size = 'md',
  className = '',
}) => {
  const handleTabClick = useCallback(
    (tabId: string, disabled?: boolean) => {
      if (!disabled) {
        onChange(tabId);
      }
    },
    [onChange]
  );

  const sizeClasses = {
    sm: 'text-sm px-3 py-2',
    md: 'text-sm px-4 py-3',
    lg: 'text-base px-5 py-4',
  };

  const getTabClasses = (tab: Tab) => {
    const isActive = tab.id === activeTab;
    const baseClasses = `
      flex items-center gap-2 font-medium transition-all duration-base
      ${sizeClasses[size]}
      ${tab.disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}
    `;

    switch (variant) {
      case 'pills':
        return `${baseClasses} rounded-lg ${
          isActive
            ? 'bg-pierre-violet text-white'
            : 'text-zinc-400 hover:bg-white/10 hover:text-white'
        }`;

      case 'bordered':
        return `${baseClasses} border-2 rounded-lg ${
          isActive
            ? 'border-pierre-violet text-pierre-violet bg-pierre-violet/5'
            : 'border-transparent text-zinc-400 hover:border-white/10 hover:text-white'
        }`;

      case 'underline':
      default:
        return `${baseClasses} border-b-2 ${
          isActive
            ? 'border-pierre-violet text-pierre-violet'
            : 'border-transparent text-zinc-400 hover:text-white hover:border-zinc-600'
        }`;
    }
  };

  const containerClasses = {
    underline: 'flex border-b border-white/10',
    pills: 'flex gap-2 p-1 bg-pierre-slate/60 rounded-lg',
    bordered: 'flex gap-2',
  };

  return (
    <div className={`${containerClasses[variant]} ${className}`} role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={tab.id === activeTab}
          aria-disabled={tab.disabled}
          onClick={() => handleTabClick(tab.id, tab.disabled)}
          className={getTabClasses(tab)}
        >
          {tab.icon && <span className="flex-shrink-0">{tab.icon}</span>}
          <span>{tab.label}</span>
          {tab.badge !== undefined && (
            <span
              className={`
                px-2 py-0.5 text-xs font-semibold rounded-full
                ${
                  tab.id === activeTab
                    ? variant === 'pills'
                      ? 'bg-white/20 text-white'
                      : 'bg-pierre-violet/10 text-pierre-violet-light'
                    : 'bg-white/10 text-zinc-400'
                }
              `}
            >
              {tab.badge}
            </span>
          )}
        </button>
      ))}
    </div>
  );
};

// Tab Panel component for content
export interface TabPanelProps {
  id: string;
  activeTab: string;
  children: React.ReactNode;
  className?: string;
}

export const TabPanel: React.FC<TabPanelProps> = ({ id, activeTab, children, className = '' }) => {
  if (id !== activeTab) return null;

  return (
    <div role="tabpanel" aria-labelledby={`tab-${id}`} className={`animate-fade-in ${className}`}>
      {children}
    </div>
  );
};
