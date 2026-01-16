// ABOUTME: Coach selector component for the chat interface
// ABOUTME: Fetches user's available coaches from API and displays them with help tooltip
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card } from './ui';

interface Coach {
  id: string;
  title: string;
  description?: string;
  system_prompt: string;
  category: string;
  tags: string[];
  token_count: number;
  is_favorite: boolean;
  use_count: number;
  last_used_at?: string;
  is_system: boolean;
  is_assigned: boolean;
}

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
    queryKey: ['user-coaches'],
    queryFn: () => apiService.getCoaches(),
    staleTime: 5 * 60 * 1000, // Cache for 5 minutes
    retry: 2,
  });

  const {
    data: hiddenCoachesData,
  } = useQuery({
    queryKey: ['hidden-coaches'],
    queryFn: () => apiService.getHiddenCoaches(),
    staleTime: 5 * 60 * 1000,
    enabled: showHidden, // Only fetch when showing hidden coaches
  });

  const hideCoach = useMutation({
    mutationFn: (coachId: string) => apiService.hideCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      queryClient.invalidateQueries({ queryKey: ['hidden-coaches'] });
    },
  });

  const showCoach = useMutation({
    mutationFn: (coachId: string) => apiService.showCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      queryClient.invalidateQueries({ queryKey: ['hidden-coaches'] });
    },
  });

  if (isLoading) {
    return (
      <Card className="p-3 mt-4 animate-pulse">
        <div className="flex items-center gap-2 mb-3">
          <div className="w-8 h-8 rounded-lg bg-pierre-gray-200" />
          <div className="h-5 w-20 bg-pierre-gray-200 rounded" />
          <div className="w-5 h-5 rounded-full bg-pierre-gray-200" />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i} className="h-20 bg-pierre-gray-100 rounded-xl" />
          ))}
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <div className="mt-4 text-center">
        <Card className="p-6 border-pierre-red-200 bg-pierre-red-50">
          <div className="text-pierre-red-600 mb-2">
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
            <p className="text-sm text-pierre-red-500 mt-1">
              {error instanceof Error ? error.message : 'Please try refreshing the page'}
            </p>
          </div>
        </Card>
      </div>
    );
  }

  const coaches = coachesData?.coaches || [];
  // Map hidden coaches to convert null values to undefined for Coach type compatibility
  // API returns string | null for optional fields but Coach type uses string | undefined
  const hiddenCoaches: Coach[] = (hiddenCoachesData?.coaches || []).map((c) => ({
    ...c,
    description: c.description ?? undefined,
    last_used_at: c.last_used_at ?? undefined,
  }));

  if (coaches.length === 0 && hiddenCoaches.length === 0) {
    return (
      <div className="mt-4 text-center text-pierre-gray-500">
        <p>No coaches available yet</p>
        <p className="text-sm mt-2">Ask your admin to assign some coaching personas to get started.</p>
      </div>
    );
  }

  // Sort coaches: favorites first, then by use_count
  const sortedCoaches = [...coaches].sort((a, b) => {
    if (a.is_favorite !== b.is_favorite) return a.is_favorite ? -1 : 1;
    return b.use_count - a.use_count;
  });

  const handleHideCoach = (coach: Coach) => {
    hideCoach.mutate(coach.id);
  };

  const handleShowCoach = (coach: Coach) => {
    showCoach.mutate(coach.id);
  };

  return (
    <CoachesSection
      coaches={sortedCoaches}
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
      <div className="bg-white rounded-lg shadow-lg border border-pierre-gray-200 p-4 max-w-sm">
        <div className="flex items-start justify-between gap-2">
          <div>
            <p className="text-sm text-pierre-gray-700 font-medium mb-2">
              AI Coaching Personas
            </p>
            <p className="text-xs text-pierre-gray-500">
              Coaches are specialized AI assistants trained to help with specific aspects of your fitness journey.
              Select a coach to start a conversation focused on their expertise area.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="text-pierre-gray-400 hover:text-pierre-gray-600 flex-shrink-0"
            aria-label="Close help"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}

// Coaches section with header and help button
function CoachesSection({
  coaches,
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
  coaches: Coach[];
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
    <Card className="p-3 mt-4">
      {/* Header with help button */}
      <div className="flex items-center gap-2 mb-3 relative">
        <div
          className="w-8 h-8 rounded-lg bg-gradient-to-br from-pierre-violet to-purple-600 flex items-center justify-center"
          role="img"
          aria-label="Coaches"
        >
          <svg className="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
          </svg>
        </div>
        <h3 className="font-medium text-pierre-gray-900">Coaches</h3>
        <button
          type="button"
          onClick={() => setShowHelp(!showHelp)}
          className="w-5 h-5 rounded-full bg-pierre-gray-100 hover:bg-pierre-gray-200 flex items-center justify-center text-pierre-gray-500 hover:text-pierre-gray-700 transition-colors"
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
                ? 'bg-pierre-violet/10 text-pierre-violet'
                : 'bg-pierre-gray-100 text-pierre-gray-500 hover:bg-pierre-gray-200'
            }`}
            title={showHidden ? 'Hide hidden coaches' : 'Show hidden coaches'}
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              {showHidden ? (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
              ) : (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
              )}
            </svg>
            <span>{hiddenCoaches.length} hidden</span>
          </button>
        )}
      </div>

      {/* Coach list - responsive grid: 1 col mobile, 2 col tablet, 3 col desktop */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {coaches.map((coach) => (
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

      {/* Hidden coaches section */}
      {showHidden && hiddenCoaches.length > 0 && (
        <div className="mt-4 pt-4 border-t border-pierre-gray-200">
          <h4 className="text-sm font-medium text-pierre-gray-500 mb-3 flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
            </svg>
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

// Individual coach card component
function CoachCard({
  coach,
  onSelectPrompt,
  onEditCoach,
  onDeleteCoach,
  onHideCoach,
  isHiding,
}: {
  coach: Coach;
  onSelectPrompt: (prompt: string, coachId?: string, systemPrompt?: string) => void;
  onEditCoach?: (coach: Coach) => void;
  onDeleteCoach?: (coach: Coach) => void;
  onHideCoach: (coach: Coach) => void;
  isHiding: boolean;
}) {
  return (
    <div
      className="relative text-left text-sm rounded-xl border border-pierre-gray-200 hover:border-pierre-violet hover:bg-pierre-violet/5 px-4 py-3 transition-all focus-within:outline-none focus-within:ring-2 focus-within:ring-pierre-violet focus-within:ring-opacity-50 group hover:shadow-sm"
    >
      {/* Action buttons container */}
      <div className="absolute top-1.5 right-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity z-10 bg-white/90 backdrop-blur-sm rounded-lg px-1 py-0.5 shadow-sm">
        {/* Edit/Delete for user-created coaches */}
        {!coach.is_system && onEditCoach && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onEditCoach(coach);
            }}
            className="p-1 text-pierre-gray-400 hover:text-pierre-violet hover:bg-pierre-violet/10 rounded transition-colors"
            title="Edit coach"
            aria-label="Edit coach"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
            </svg>
          </button>
        )}
        {!coach.is_system && onDeleteCoach && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onDeleteCoach(coach);
            }}
            className="p-1 text-pierre-gray-400 hover:text-pierre-red-500 hover:bg-pierre-red-50 rounded transition-colors"
            title="Delete coach"
            aria-label="Delete coach"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
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
            className="p-1 text-pierre-gray-400 hover:text-pierre-gray-600 hover:bg-pierre-gray-100 rounded transition-colors disabled:opacity-50"
            title="Hide coach"
            aria-label="Hide coach"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
            </svg>
          </button>
        )}
      </div>
      <button
        type="button"
        onClick={() => {
          apiService.recordCoachUsage(coach.id).catch(() => {
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
          <span className="font-medium text-pierre-gray-800 group-hover:text-pierre-violet">
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
          <p className="text-pierre-gray-500 text-xs mt-0.5 line-clamp-2">
            {coach.description}
          </p>
        )}
        <div className="flex items-center gap-2 mt-1 text-xs text-pierre-gray-400">
          {coach.is_system && (
            <span className="bg-pierre-violet bg-opacity-10 text-pierre-violet px-1.5 py-0.5 rounded">
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
}

// Hidden coach card component (dimmed, with show button)
function HiddenCoachCard({
  coach,
  onShowCoach,
  isShowing,
}: {
  coach: Coach;
  onShowCoach: (coach: Coach) => void;
  isShowing: boolean;
}) {
  return (
    <div
      className="relative text-left text-sm rounded-xl border border-pierre-gray-200 px-4 py-3 opacity-60 hover:opacity-100 transition-all group bg-pierre-gray-50"
    >
      {/* Show button */}
      <div className="absolute top-1.5 right-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity z-10 bg-white/90 backdrop-blur-sm rounded-lg px-1 py-0.5 shadow-sm">
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onShowCoach(coach);
          }}
          disabled={isShowing}
          className="p-1 text-pierre-gray-400 hover:text-pierre-green-600 hover:bg-pierre-green-50 rounded transition-colors disabled:opacity-50"
          title="Show coach"
          aria-label="Show coach"
        >
          <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
          </svg>
        </button>
      </div>
      <div className="flex items-center justify-between">
        <span className="font-medium text-pierre-gray-600">
          {coach.title}
        </span>
        <div className="flex items-center gap-1">
          <span className={`text-xs px-1.5 py-0.5 rounded ${getCategoryBadgeClass(coach.category)}`}>
            {getCategoryIcon(coach.category)}
          </span>
        </div>
      </div>
      {coach.description && (
        <p className="text-pierre-gray-400 text-xs mt-0.5 line-clamp-2">
          {coach.description}
        </p>
      )}
      <div className="flex items-center gap-2 mt-1 text-xs text-pierre-gray-400">
        {coach.is_system && (
          <span className="bg-pierre-gray-200 text-pierre-gray-500 px-1.5 py-0.5 rounded">
            System
          </span>
        )}
      </div>
    </div>
  );
}

// Helper functions for category styling
function getCategoryBadgeClass(category: string): string {
  const classes: Record<string, string> = {
    training: 'bg-pierre-green-100 text-pierre-green-700',
    nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition',
    recovery: 'bg-pierre-blue-100 text-pierre-blue-700',
    recipes: 'bg-pierre-yellow-100 text-pierre-yellow-700',
    analysis: 'bg-pierre-violet/10 text-pierre-violet',
    custom: 'bg-pierre-gray-100 text-pierre-gray-600',
  };
  return classes[category.toLowerCase()] || classes.custom;
}

function getCategoryIcon(category: string): string {
  const icons: Record<string, string> = {
    training: '🏃',
    nutrition: '🥗',
    recovery: '😴',
    recipes: '👨‍🍳',
    analysis: '📊',
    custom: '⚙️',
  };
  return icons[category.toLowerCase()] || icons.custom;
}
