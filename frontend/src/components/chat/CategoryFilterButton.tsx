// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Category filter button for coaches panel
// ABOUTME: Memoized for performance when rendering multiple category filters

import { memo } from 'react';
import { clsx } from 'clsx';
import { getCategoryIcon } from './utils';

interface CategoryFilterButtonProps {
  category: string | null;
  label: string;
  isSelected: boolean;
  onClick: () => void;
  showIcon?: boolean;
}

const CategoryFilterButton = memo(function CategoryFilterButton({
  category,
  label,
  isSelected,
  onClick,
  showIcon = true,
}: CategoryFilterButtonProps) {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'px-4 py-2 text-sm font-medium rounded-full whitespace-nowrap transition-colors flex items-center gap-1.5',
        isSelected
          ? 'bg-pierre-violet text-white shadow-glow-sm'
          : 'bg-white/5 text-zinc-400 hover:bg-white/10 hover:text-white'
      )}
    >
      {showIcon && category && <span>{getCategoryIcon(category)}</span>}
      {label}
    </button>
  );
});

export default CategoryFilterButton;
