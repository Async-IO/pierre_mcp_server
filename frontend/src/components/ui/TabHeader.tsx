// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// ABOUTME: Shared header row for user-facing tabs (Chat, Coaches, Discover, Insights)
// ABOUTME: Renders icon in gradient circle + description + optional action buttons

import React from 'react';
import { clsx } from 'clsx';

export interface TabHeaderProps {
  /** Icon element (e.g. lucide-react component) rendered inside the gradient circle */
  icon: React.ReactNode;
  /** Tailwind gradient classes for the icon circle (e.g. "from-pierre-violet to-pierre-cyan") */
  gradient: string;
  /** Description text shown next to the icon */
  description: React.ReactNode;
  /** Optional action buttons rendered on the right side */
  actions?: React.ReactNode;
}

export function TabHeader({ icon, gradient, description, actions }: TabHeaderProps) {
  return (
    <div className="p-6 border-b border-white/5 flex items-center justify-between flex-shrink-0">
      <div className="flex items-center gap-3">
        <div
          className={clsx(
            'w-10 h-10 flex items-center justify-center rounded-xl text-white shadow-glow-sm bg-gradient-to-br',
            gradient
          )}
        >
          {icon}
        </div>
        <p className="text-sm text-zinc-400">{description}</p>
      </div>
      {actions && (
        <div className="flex items-center gap-2">
          {actions}
        </div>
      )}
    </div>
  );
}
