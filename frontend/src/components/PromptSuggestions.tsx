// ABOUTME: Coach selector component for the chat interface
// ABOUTME: Fetches user's available coaches from API and displays them with help tooltip
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, memo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { X, Users, Eye, EyeOff, Pencil, Trash2 } from 'lucide-react';
import { coachesApi } from '../services/api';
import { Card } from './ui';
import type { Coach } from '@pierre/shared-types';
import { QUERY_KEYS } from '../constants/queryKeys';

interface PromptSuggestionsProps {
  onSelectPrompt: (prompt: string, coachId?: string, systemPrompt?: string) => void;
  onEditCoach?: (coach: Coach) => void;
  onDeleteCoach?: (coach: Coach) => void;
}

export default function PromptSuggestions({ onSelectPrompt, onEditCoach, onDeleteCoach }: PromptSuggestionsProps) {
  const [showHidden, setShowHidden] = useState(false);
  const queryClient = useQueryClient();

  const {
    data: coachesData,
    isLoading,
    error,
  } = useQuery({
    queryKey: QUERY_KEYS.coaches.list(),
    queryFn: () => coachesApi.list(),
    staleTime: 5 * 60 * 1000, // Cache for 5 minutes
    retry: 2,
  });

  const {
    data: hiddenCoachesData,
  } = useQuery({
    queryKey: QUERY_KEYS.coaches.hidden(),
    queryFn: () => coachesApi.getHidden(),
    staleTime: 5 * 60 * 1000,
    enabled: showHidden, // Only fetch when showing hidden coaches
  });

  const hideCoach = useMutation({
    mutationFn: (coachId: string) => coachesApi.hide(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.all });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.hidden() });
    },
  });

  const showCoach = useMutation({
    mutationFn: (coachId: string) => coachesApi.show(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.all });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.hidden() });
    },
  });

  if (isLoading) {
    return (
      <Card variant="dark" className="p-3 mt-4 animate-pulse">
        <div className="flex items-center gap-2 mb-3">
          <div className="w-8 h-8 rounded-lg bg-white/10" />
          <div className="h-5 w-20 bg-white/10 rounded" />
          <div className="w-5 h-5 rounded-full bg-white/10" />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i} className="h-20 bg-white/5 rounded-xl" />
          ))}
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <div className="mt-4 text-center">
        <Card variant="dark" className="p-6 border-red-500/30 bg-red-500/10">
          <div className="text-red-400 mb-2">
            <svg
              className="w-8 h-8 mx-auto mb-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <p className="font-medium">Failed to load coaches</p>
            <p className="text-sm text-red-400/80 mt-1">
              {error instanceof Error ? error.message : 'Please try refreshing the page'}
            </p>
          </div>
        </Card>
      </div>
    );
  }

  const coaches = coachesData?.coaches || [];
  const hiddenCoaches: Coach[] = hiddenCoachesData?.coaches || [];

  if (coaches.length === 0 && hiddenCoaches.length === 0) {
    return (
      <div className="mt-4 text-center text-zinc-400">
        <p>No coaches available yet</p>
        <p className="text-sm mt-2 text-zinc-500">Ask your admin to assign some coaching personas to get started.</p>
      </div>
    );
  }

  // Separate user coaches (non-system) from system coaches
  const userCoaches = coaches.filter((c) => !c.is_system);
  const systemCoaches = coaches.filter((c) => c.is_system);

  // Sort each group: favorites first, then by use_count
  const sortByUsage = (a: Coach, b: Coach) => {
    if (a.is_favorite !== b.is_favorite) return a.is_favorite ? -1 : 1;
    return b.use_count - a.use_count;
  };
  const sortedUserCoaches = [...userCoaches].sort(sortByUsage);
  const sortedSystemCoaches = [...systemCoaches].sort(sortByUsage);

  const handleHideCoach = (coach: Coach) => {
    hideCoach.mutate(coach.id);
  };

  const handleShowCoach = (coach: Coach) => {
    showCoach.mutate(coach.id);
  };

  return (
    <CoachesSection
      userCoaches={sortedUserCoaches}
      systemCoaches={sortedSystemCoaches}
      hiddenCoaches={hiddenCoaches}
      showHidden={showHidden}
      onToggleShowHidden={() => setShowHidden(!showHidden)}
      onSelectPrompt={onSelectPrompt}
      onEditCoach={onEditCoach}
      onDeleteCoach={onDeleteCoach}
      onHideCoach={handleHideCoach}
      onShowCoach={handleShowCoach}
      isHiding={hideCoach.isPending}
      isShowing={showCoach.isPending}
    />
  );
}

// Help tooltip popover component
function HelpTooltip({ isVisible, onClose }: { isVisible: boolean; onClose: () => void }) {
  if (!isVisible) return null;

  return (
    <div className="absolute top-full left-0 mt-2 z-50">
      <div className="bg-pierre-slate rounded-lg shadow-lg border border-white/10 p-4 max-w-sm">
        <div className="flex items-start justify-between gap-2">
          <div>
            <p className="text-sm text-white font-medium mb-2">
              AI Coaching Personas
            </p>
            <p className="text-xs text-zinc-400">
              Coaches are specialized AI assistants trained to help with specific aspects of your fitness journey.
              Select a coach to start a conversation focused on their expertise area.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="text-zinc-500 hover:text-white flex-shrink-0 transition-colors"
            aria-label="Close help"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

// Coaches section with header and help button
function CoachesSection({
  userCoaches,
  systemCoaches,
  hiddenCoaches,
  showHidden,
  onToggleShowHidden,
  onSelectPrompt,
  onEditCoach,
  onDeleteCoach,
  onHideCoach,
  onShowCoach,
  isHiding,
  isShowing,
}: {
  userCoaches: Coach[];
  systemCoaches: Coach[];
  hiddenCoaches: Coach[];
  showHidden: boolean;
  onToggleShowHidden: () => void;
  onSelectPrompt: (prompt: string, coachId?: string, systemPrompt?: string) => void;
  onEditCoach?: (coach: Coach) => void;
  onDeleteCoach?: (coach: Coach) => void;
  onHideCoach: (coach: Coach) => void;
  onShowCoach: (coach: Coach) => void;
  isHiding: boolean;
  isShowing: boolean;
}) {
  const [showHelp, setShowHelp] = useState(false);

  return (
    <Card variant="dark" className="p-3 mt-4">
      {/* Header with help button */}
      <div className="flex items-center gap-2 mb-3 relative">
        <div
          className="w-8 h-8 rounded-lg bg-gradient-to-br from-pierre-violet to-purple-600 flex items-center justify-center shadow-glow-sm"
          role="img"
          aria-label="Coaches"
        >
          <Users className="w-4 h-4 text-white" />
        </div>
        <h3 className="font-medium text-white">Coaches</h3>
        <button
          type="button"
          onClick={() => setShowHelp(!showHelp)}
          className="w-5 h-5 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-zinc-400 hover:text-white transition-colors"
          aria-label="What are coaches?"
        >
          <span className="text-xs font-medium">?</span>
        </button>
        <HelpTooltip isVisible={showHelp} onClose={() => setShowHelp(false)} />

        {/* Show hidden coaches toggle - only show if there are hidden coaches */}
        {(hiddenCoaches.length > 0 || showHidden) && (
          <button
            type="button"
            onClick={onToggleShowHidden}
            className={`ml-auto flex items-center gap-1.5 px-2 py-1 text-xs rounded-lg transition-colors ${
              showHidden
                ? 'bg-pierre-violet/20 text-pierre-violet-light'
                : 'bg-white/10 text-zinc-400 hover:bg-white/15 hover:text-zinc-300'
            }`}
            title={showHidden ? 'Hide hidden coaches' : 'Show hidden coaches'}
          >
            {showHidden ? (
              <Eye className="w-3.5 h-3.5" />
            ) : (
              <EyeOff className="w-3.5 h-3.5" />
            )}
            <span>{hiddenCoaches.length} hidden</span>
          </button>
        )}
      </div>

      {/* Personalized section (user-created coaches) - always first */}
      {userCoaches.length > 0 && (
        <div className="mb-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-2 flex items-center gap-2">
            <span className="text-base">✨</span>
            Personalized
            <span className="text-xs text-zinc-500 font-normal">({userCoaches.length})</span>
          </h4>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {userCoaches.map((coach) => (
              <CoachCard
                key={coach.id}
                coach={coach}
                onSelectPrompt={onSelectPrompt}
                onEditCoach={onEditCoach}
                onDeleteCoach={onDeleteCoach}
                onHideCoach={onHideCoach}
                isHiding={isHiding}
              />
            ))}
          </div>
        </div>
      )}

      {/* System Coaches section - below user coaches */}
      {systemCoaches.length > 0 && (
        <div className={userCoaches.length > 0 ? 'pt-3 border-t border-white/10' : ''}>
          <h4 className="text-sm font-medium text-zinc-400 mb-2 flex items-center gap-2">
            <span className="text-base">🏛️</span>
            System Coaches
            <span className="text-xs text-zinc-500 font-normal">({systemCoaches.length})</span>
          </h4>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {systemCoaches.map((coach) => (
              <CoachCard
                key={coach.id}
                coach={coach}
                onSelectPrompt={onSelectPrompt}
                onEditCoach={onEditCoach}
                onDeleteCoach={onDeleteCoach}
                onHideCoach={onHideCoach}
                isHiding={isHiding}
              />
            ))}
          </div>
        </div>
      )}

      {/* Hidden coaches section */}
      {showHidden && hiddenCoaches.length > 0 && (
        <div className="mt-4 pt-4 border-t border-white/10">
          <h4 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <EyeOff className="w-4 h-4" />
            Hidden Coaches
          </h4>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {hiddenCoaches.map((coach) => (
              <HiddenCoachCard
                key={coach.id}
                coach={coach}
                onShowCoach={onShowCoach}
                isShowing={isShowing}
              />
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}

// Individual coach card component - memoized to prevent unnecessary re-renders
interface CoachCardProps {
  coach: Coach;
  onSelectPrompt: (prompt: string, coachId?: string, systemPrompt?: string) => void;
  onEditCoach?: (coach: Coach) => void;
  onDeleteCoach?: (coach: Coach) => void;
  onHideCoach: (coach: Coach) => void;
  isHiding: boolean;
}

const CoachCard = memo(function CoachCard({
  coach,
  onSelectPrompt,
  onEditCoach,
  onDeleteCoach,
  onHideCoach,
  isHiding,
}: CoachCardProps) {
  return (
    <div
      className="relative text-left text-sm rounded-xl border border-white/10 bg-white/5 hover:border-pierre-violet/50 hover:bg-pierre-violet/10 px-4 py-3 transition-all focus-within:outline-none focus-within:ring-2 focus-within:ring-pierre-violet focus-within:ring-opacity-50 group hover:shadow-glow-sm"
    >
      {/* Action buttons container */}
      <div className="absolute top-1.5 right-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity z-10 bg-pierre-slate/90 backdrop-blur-sm rounded-lg px-1 py-0.5 shadow-sm border border-white/10">
        {/* Edit/Delete for user-created coaches */}
        {!coach.is_system && onEditCoach && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onEditCoach(coach);
            }}
            className="p-1 text-zinc-400 hover:text-pierre-violet hover:bg-pierre-violet/20 rounded transition-colors"
            title="Edit coach"
            aria-label="Edit coach"
          >
            <Pencil className="w-3.5 h-3.5" />
          </button>
        )}
        {!coach.is_system && onDeleteCoach && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onDeleteCoach(coach);
            }}
            className="p-1 text-zinc-400 hover:text-red-400 hover:bg-red-500/20 rounded transition-colors"
            title="Delete coach"
            aria-label="Delete coach"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        )}
        {/* Hide button for system coaches */}
        {coach.is_system && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onHideCoach(coach);
            }}
            disabled={isHiding}
            className="p-1 text-zinc-400 hover:text-zinc-200 hover:bg-white/10 rounded transition-colors disabled:opacity-50"
            title="Hide coach"
            aria-label="Hide coach"
          >
            <EyeOff className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
      <button
        type="button"
        onClick={() => {
          coachesApi.recordUsage(coach.id).catch(() => {
            // Silently ignore usage tracking errors
          });
          onSelectPrompt(
            coach.description || `Chat with ${coach.title}`,
            coach.id,
            coach.system_prompt
          );
        }}
        className="w-full text-left"
      >
        <div className="flex items-center justify-between">
          <span className="font-medium text-white group-hover:text-pierre-violet transition-colors">
            {coach.title}
          </span>
          <div className="flex items-center gap-1">
            {coach.is_favorite && (
              <span className="text-amber-400">★</span>
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
            <span className="bg-pierre-violet/20 text-pierre-violet-light px-1.5 py-0.5 rounded">
              System
            </span>
          )}
          {coach.use_count > 0 && (
            <span>Used {coach.use_count}x</span>
          )}
        </div>
      </button>
    </div>
  );
});

// Hidden coach card component (dimmed, with show button) - memoized to prevent unnecessary re-renders
interface HiddenCoachCardProps {
  coach: Coach;
  onShowCoach: (coach: Coach) => void;
  isShowing: boolean;
}

const HiddenCoachCard = memo(function HiddenCoachCard({
  coach,
  onShowCoach,
  isShowing,
}: HiddenCoachCardProps) {
  return (
    <div
      className="relative text-left text-sm rounded-xl border border-white/5 px-4 py-3 opacity-50 hover:opacity-100 transition-all group bg-white/5"
    >
      {/* Show button */}
      <div className="absolute top-1.5 right-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity z-10 bg-pierre-slate/90 backdrop-blur-sm rounded-lg px-1 py-0.5 shadow-sm border border-white/10">
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onShowCoach(coach);
          }}
          disabled={isShowing}
          className="p-1 text-zinc-400 hover:text-emerald-400 hover:bg-emerald-500/20 rounded transition-colors disabled:opacity-50"
          title="Show coach"
          aria-label="Show coach"
        >
          <Eye className="w-3.5 h-3.5" />
        </button>
      </div>
      <div className="flex items-center justify-between">
        <span className="font-medium text-zinc-400">
          {coach.title}
        </span>
        <div className="flex items-center gap-1">
          <span className={`text-xs px-1.5 py-0.5 rounded ${getCategoryBadgeClass(coach.category)}`}>
            {getCategoryIcon(coach.category)}
          </span>
        </div>
      </div>
      {coach.description && (
        <p className="text-zinc-500 text-xs mt-0.5 line-clamp-2">
          {coach.description}
        </p>
      )}
      <div className="flex items-center gap-2 mt-1 text-xs text-zinc-500">
        {coach.is_system && (
          <span className="bg-white/10 text-zinc-400 px-1.5 py-0.5 rounded">
            System
          </span>
        )}
      </div>
    </div>
  );
});

// Helper functions for category styling (dark theme)
function getCategoryBadgeClass(category: string): string {
  const classes: Record<string, string> = {
    training: 'bg-emerald-500/20 text-emerald-400',
    nutrition: 'bg-amber-500/20 text-amber-400',
    recovery: 'bg-indigo-500/20 text-indigo-400',
    recipes: 'bg-orange-500/20 text-orange-400',
    mobility: 'bg-pink-500/20 text-pink-400',
    analysis: 'bg-pierre-violet/20 text-pierre-violet-light',
    custom: 'bg-white/10 text-zinc-400',
  };
  return classes[category.toLowerCase()] || classes.custom;
}

function getCategoryIcon(category: string): string {
  const icons: Record<string, string> = {
    training: '🏃',
    nutrition: '🥗',
    recovery: '😴',
    recipes: '👨‍🍳',
    mobility: '🧘',
    analysis: '📊',
    custom: '⚙️',
  };
  return icons[category.toLowerCase()] || icons.custom;
}
