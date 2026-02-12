// ABOUTME: Component displaying published coaches in the store with management actions
// ABOUTME: Shows coaches in a grid with install count, published date, and unpublish option
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import { Card, Button } from './ui';
import { clsx } from 'clsx';
import { ConfirmDialog } from './ui';
import { QUERY_KEYS } from '../constants/queryKeys';

// Category colors matching SystemCoachesTab
const CATEGORY_COLORS: Record<string, string> = {
  Training: 'bg-pierre-activity/10 text-pierre-activity border-pierre-activity/20',
  Nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition border-pierre-nutrition/20',
  Recovery: 'bg-pierre-recovery/10 text-pierre-recovery border-pierre-recovery/20',
  Recipes: 'bg-pierre-yellow-500/10 text-pierre-yellow-600 border-pierre-yellow-500/20',
  Mobility: 'bg-pierre-mobility/10 text-pierre-mobility border-pierre-mobility/20',
  Custom: 'bg-pierre-violet/10 text-pierre-violet-light border-pierre-violet/20',
};

function getCategoryColorClass(category: string): string {
  const normalized = category.charAt(0).toUpperCase() + category.slice(1).toLowerCase();
  return CATEGORY_COLORS[normalized] || CATEGORY_COLORS.Custom;
}

type SortOption = 'newest' | 'most_installed';

interface PublishedCoach {
  id: string;
  title: string;
  description: string | null;
  category: string;
  tags: string[];
  sample_prompts: string[];
  token_count: number;
  install_count: number;
  icon_url: string | null;
  published_at: string | null;
  author_id: string | null;
  author_email?: string;
  system_prompt: string;
  created_at: string;
  publish_status: string;
}

export default function PublishedCoachesList() {
  const [sortBy, setSortBy] = useState<SortOption>('newest');
  const [confirmUnpublish, setConfirmUnpublish] = useState<PublishedCoach | null>(null);
  const queryClient = useQueryClient();

  // Fetch published coaches
  const { data, isLoading, error } = useQuery({
    queryKey: QUERY_KEYS.adminStore.published(sortBy),
    queryFn: () => adminApi.getPublishedStoreCoaches({ sort_by: sortBy }),
  });

  // Unpublish mutation
  const unpublishMutation = useMutation({
    mutationFn: (coachId: string) => adminApi.unpublishStoreCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.adminStore.published() });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.adminStore.stats() });
      setConfirmUnpublish(null);
    },
  });

  const formatDate = (dateString: string | null) => {
    if (!dateString) return '—';
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <div className="pierre-spinner w-8 h-8"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card variant="dark" className="text-center py-12">
        <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-pierre-red-500/20 flex items-center justify-center">
          <svg className="w-8 h-8 text-pierre-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <h3 className="text-lg font-medium text-white mb-2">Failed to Load Published Coaches</h3>
        <p className="text-zinc-400">Unable to fetch published coaches. Please try again.</p>
      </Card>
    );
  }

  const coaches = data?.coaches || [];

  if (coaches.length === 0) {
    return (
      <Card variant="dark" className="text-center py-12">
        <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-white/10 flex items-center justify-center">
          <svg className="w-8 h-8 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
        </div>
        <h3 className="text-lg font-medium text-white mb-2">No Published Coaches</h3>
        <p className="text-zinc-400">Coaches will appear here once they are approved and published.</p>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* Sort Controls */}
      <div className="flex items-center justify-between">
        <p className="text-sm text-zinc-400">
          {data?.total ?? coaches.length} published coach{coaches.length !== 1 ? 'es' : ''}
        </p>
        <div className="flex items-center gap-2">
          <span className="text-sm text-zinc-500">Sort by:</span>
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortOption)}
            className="select-dark text-sm py-1.5"
          >
            <option value="newest">Newest</option>
            <option value="most_installed">Most Installed</option>
          </select>
        </div>
      </div>

      {/* Coaches Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {coaches.map((coach) => (
          <div
            key={coach.id}
            className="bg-[rgba(30,30,46,0.6)] backdrop-blur-[16px] border border-white/10 rounded-xl p-4 hover:border-white/20 transition-all"
          >
            {/* Header */}
            <div className="flex items-start gap-3 mb-3">
              {coach.icon_url ? (
                <img
                  src={coach.icon_url}
                  alt={coach.title}
                  className="w-12 h-12 rounded-lg object-cover border border-white/10"
                />
              ) : (
                <div className="w-12 h-12 rounded-lg bg-gradient-to-br from-pierre-violet to-pierre-cyan flex items-center justify-center flex-shrink-0">
                  <span className="text-lg font-bold text-white">
                    {coach.title.charAt(0).toUpperCase()}
                  </span>
                </div>
              )}
              <div className="flex-1 min-w-0">
                <h3 className="font-semibold text-white truncate">{coach.title}</h3>
                <span className={clsx(
                  'inline-block mt-1 px-2 py-0.5 text-xs font-medium rounded-full border',
                  getCategoryColorClass(coach.category)
                )}>
                  {coach.category}
                </span>
              </div>
            </div>

            {/* Description */}
            {coach.description && (
              <p className="text-sm text-zinc-400 line-clamp-2 mb-3">
                {coach.description}
              </p>
            )}

            {/* Stats */}
            <div className="flex items-center gap-4 text-xs text-zinc-500 mb-4">
              <span className="flex items-center gap-1">
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                </svg>
                {coach.install_count.toLocaleString()} installs
              </span>
              <span className="flex items-center gap-1">
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
                </svg>
                {formatDate(coach.published_at)}
              </span>
            </div>

            {/* Author */}
            <div className="text-xs text-zinc-500 mb-4">
              by {coach.author_email || 'Unknown'}
            </div>

            {/* Actions */}
            <div className="flex items-center gap-2 pt-3 border-t border-white/10">
              <Button
                variant="secondary"
                size="sm"
                className="flex-1 text-xs"
                onClick={() => {
                  // Could open a detail view in the future
                  console.log('View coach:', coach.id);
                }}
              >
                <svg className="w-3.5 h-3.5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                </svg>
                View
              </Button>
              <Button
                variant="secondary"
                size="sm"
                className="flex-1 text-xs border-pierre-red-500/30 text-pierre-red-400 hover:bg-pierre-red-500/10"
                onClick={() => setConfirmUnpublish(coach)}
              >
                <svg className="w-3.5 h-3.5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636" />
                </svg>
                Unpublish
              </Button>
            </div>
          </div>
        ))}
      </div>

      {/* Confirm Unpublish Dialog */}
      <ConfirmDialog
        isOpen={!!confirmUnpublish}
        onClose={() => setConfirmUnpublish(null)}
        onConfirm={() => confirmUnpublish && unpublishMutation.mutate(confirmUnpublish.id)}
        title="Unpublish Coach"
        message={`Are you sure you want to unpublish "${confirmUnpublish?.title}"? It will be removed from the store and users will no longer be able to install it.`}
        confirmLabel={unpublishMutation.isPending ? 'Unpublishing...' : 'Unpublish'}
        variant="danger"
        isLoading={unpublishMutation.isPending}
      />
    </div>
  );
}
