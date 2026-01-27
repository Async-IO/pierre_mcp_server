// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Coach card component for My Coaches panel
// ABOUTME: Memoized for performance when rendering many coaches

import { memo } from 'react';
import { Pencil, Trash2 } from 'lucide-react';
import type { Coach } from './types';
import { getCategoryBadgeClass, getCategoryIcon } from './utils';

interface MyCoachCardProps {
  coach: Coach;
  onSelect: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onHide: () => void;
  isHiding: boolean;
}

const MyCoachCard = memo(function MyCoachCard({
  coach,
  onSelect,
  onEdit,
  onDelete,
  onHide,
  isHiding,
}: MyCoachCardProps) {
  return (
    <div
      className="relative text-left text-sm rounded-xl border border-white/10 bg-white/5 hover:border-pierre-violet/50 hover:bg-white/10 px-4 py-3 transition-all focus-within:outline-none focus-within:ring-2 focus-within:ring-pierre-violet focus-within:ring-opacity-50 group hover:shadow-glow-sm cursor-pointer"
      onClick={onSelect}
    >
      {/* Action buttons container */}
      <div className="absolute top-1.5 right-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity z-10 bg-pierre-slate/90 backdrop-blur-sm rounded-lg px-1 py-0.5 shadow-sm border border-white/10">
        {/* Edit/Delete for user-created coaches */}
        {!coach.is_system && (
          <>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onEdit();
              }}
              className="p-1 text-zinc-400 hover:text-pierre-violet hover:bg-pierre-violet/10 rounded transition-colors"
              title="Edit coach"
              aria-label="Edit coach"
            >
              <Pencil className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
              className="p-1 text-zinc-400 hover:text-pierre-red-500 hover:bg-pierre-red-500/10 rounded transition-colors"
              title="Delete coach"
              aria-label="Delete coach"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </>
        )}
        {/* Hide button for system coaches */}
        {coach.is_system && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onHide();
            }}
            disabled={isHiding}
            className="p-1 text-zinc-400 hover:text-zinc-200 hover:bg-white/10 rounded transition-colors disabled:opacity-50"
            title="Hide coach"
            aria-label="Hide coach"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
            </svg>
          </button>
        )}
      </div>

      <div className="flex items-center justify-between">
        <span className="font-medium text-zinc-200 group-hover:text-pierre-violet">
          {coach.title}
        </span>
        <div className="flex items-center gap-1">
          {coach.is_favorite && (
            <span className="text-pierre-yellow-500">★</span>
          )}
          <span className={`text-xs px-1.5 py-0.5 rounded ${getCategoryBadgeClass(coach.category)}`}>
            {getCategoryIcon(coach.category)}
          </span>
        </div>
      </div>
      {coach.description && (
        <p className="text-zinc-400 text-xs mt-0.5 line-clamp-2">
          {coach.description}
        </p>
      )}
      <div className="flex items-center gap-2 mt-1 text-xs text-zinc-500">
        {coach.is_system && (
          <span className="bg-pierre-violet/20 text-pierre-violet px-1.5 py-0.5 rounded">
            System
          </span>
        )}
        {coach.use_count > 0 && (
          <span>Used {coach.use_count}x</span>
        )}
      </div>
    </div>
  );
});

export default MyCoachCard;
