// ABOUTME: Component displaying pending coach submissions for admin review
// ABOUTME: Lists coaches sorted by submission date (FIFO) with click to open review drawer
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import { Card } from './ui';
import { clsx } from 'clsx';
import CoachReviewDrawer from './CoachReviewDrawer';

// Category colors matching SystemCoachesTab
const CATEGORY_COLORS: Record<string, string> = {
  Training: 'bg-pierre-activity/10 text-pierre-activity border-pierre-activity/20',
  Nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition border-pierre-nutrition/20',
  Recovery: 'bg-pierre-recovery/10 text-pierre-recovery border-pierre-recovery/20',
  Recipes: 'bg-pierre-yellow-500/10 text-pierre-yellow-600 border-pierre-yellow-500/20',
  Mobility: 'bg-pierre-mobility/10 text-pierre-mobility border-pierre-mobility/20',
  Custom: 'bg-pierre-violet/10 text-pierre-violet border-pierre-violet/20',
};

function getCategoryColorClass(category: string): string {
  const normalized = category.charAt(0).toUpperCase() + category.slice(1).toLowerCase();
  return CATEGORY_COLORS[normalized] || CATEGORY_COLORS.Custom;
}

interface PendingCoach {
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
  submitted_at: string;
  publish_status: string;
}

export default function CoachReviewQueue() {
  const [selectedCoach, setSelectedCoach] = useState<PendingCoach | null>(null);

  // Fetch pending coaches
  const { data, isLoading, error } = useQuery({
    queryKey: ['admin-store-review-queue'],
    queryFn: () => adminApi.getStoreReviewQueue(),
  });

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const getTimeAgo = (dateString: string) => {
    const now = new Date();
    const date = new Date(dateString);
    const diffMs = now.getTime() - date.getTime();
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffHours / 24);

    if (diffDays > 0) {
      return `${diffDays}d ago`;
    }
    if (diffHours > 0) {
      return `${diffHours}h ago`;
    }
    return 'Just now';
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
        <h3 className="text-lg font-medium text-white mb-2">Failed to Load Review Queue</h3>
        <p className="text-zinc-400">Unable to fetch pending coach submissions. Please try again.</p>
      </Card>
    );
  }

  const coaches = data?.coaches || [];

  if (coaches.length === 0) {
    return (
      <Card variant="dark" className="text-center py-12">
        <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-pierre-activity/20 flex items-center justify-center">
          <svg className="w-8 h-8 text-pierre-activity" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
          </svg>
        </div>
        <h3 className="text-lg font-medium text-white mb-2">All Caught Up!</h3>
        <p className="text-zinc-400">There are no coaches pending review at this time.</p>
      </Card>
    );
  }

  return (
    <>
      <div className="space-y-3">
        {coaches.map((coach, index) => (
          <button
            key={coach.id}
            onClick={() => setSelectedCoach(coach)}
            className="w-full text-left bg-[rgba(30,30,46,0.6)] backdrop-blur-[16px] border border-white/10 rounded-xl p-4 hover:border-pierre-violet/30 transition-all group"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-3 mb-2">
                  {/* Queue position indicator */}
                  <span className="flex-shrink-0 w-6 h-6 rounded-full bg-pierre-violet/20 text-pierre-violet-light text-xs font-bold flex items-center justify-center">
                    {index + 1}
                  </span>
                  <h3 className="font-semibold text-white truncate group-hover:text-pierre-violet-light transition-colors">
                    {coach.title}
                  </h3>
                  <span className={clsx(
                    'flex-shrink-0 px-2 py-0.5 text-xs font-medium rounded-full border',
                    getCategoryColorClass(coach.category)
                  )}>
                    {coach.category}
                  </span>
                </div>

                {coach.description && (
                  <p className="text-sm text-zinc-400 line-clamp-1 mb-2 ml-9">
                    {coach.description}
                  </p>
                )}

                <div className="flex items-center gap-4 ml-9 text-xs text-zinc-500">
                  <span className="flex items-center gap-1">
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                    </svg>
                    {coach.author_email || 'Unknown author'}
                  </span>
                  <span className="flex items-center gap-1">
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    Submitted {getTimeAgo(coach.submitted_at)}
                  </span>
                  <span className="flex items-center gap-1">
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                    </svg>
                    {coach.token_count.toLocaleString()} tokens
                  </span>
                </div>

                {coach.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1 mt-2 ml-9">
                    {coach.tags.slice(0, 4).map((tag) => (
                      <span key={tag} className="px-2 py-0.5 text-xs bg-white/10 text-zinc-400 rounded">
                        {tag}
                      </span>
                    ))}
                    {coach.tags.length > 4 && (
                      <span className="px-2 py-0.5 text-xs bg-white/10 text-zinc-500 rounded">
                        +{coach.tags.length - 4}
                      </span>
                    )}
                  </div>
                )}
              </div>

              <div className="flex-shrink-0 ml-4 flex items-center gap-2 text-zinc-500 group-hover:text-pierre-violet-light transition-colors">
                <span className="text-xs hidden sm:inline">{formatDate(coach.submitted_at)}</span>
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </div>
          </button>
        ))}
      </div>

      {/* Review Drawer */}
      <CoachReviewDrawer
        coach={selectedCoach}
        isOpen={!!selectedCoach}
        onClose={() => setSelectedCoach(null)}
      />
    </>
  );
}
