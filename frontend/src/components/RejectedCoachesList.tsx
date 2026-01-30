// ABOUTME: Component displaying rejected coach submissions with rejection details
// ABOUTME: Lists rejected coaches with reason, date, and option to re-review
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import { Card } from './ui';
import { clsx } from 'clsx';

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

// Map rejection reason codes to human-readable labels
const REJECTION_REASON_LABELS: Record<string, string> = {
  inappropriate_content: 'Inappropriate content',
  quality_standards: 'Quality standards not met',
  duplicate_submission: 'Duplicate submission',
  incomplete_information: 'Incomplete information',
  other: 'Other',
};

function getReasonLabel(reason: string): string {
  return REJECTION_REASON_LABELS[reason] || reason;
}

export default function RejectedCoachesList() {
  // Fetch rejected coaches
  const { data, isLoading, error } = useQuery({
    queryKey: ['admin-store-rejected'],
    queryFn: () => adminApi.getRejectedStoreCoaches(),
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
        <h3 className="text-lg font-medium text-white mb-2">Failed to Load Rejected Coaches</h3>
        <p className="text-zinc-400">Unable to fetch rejected coaches. Please try again.</p>
      </Card>
    );
  }

  const coaches = data?.coaches || [];

  if (coaches.length === 0) {
    return (
      <Card variant="dark" className="text-center py-12">
        <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-white/10 flex items-center justify-center">
          <svg className="w-8 h-8 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <h3 className="text-lg font-medium text-white mb-2">No Rejected Coaches</h3>
        <p className="text-zinc-400">Rejected coach submissions will appear here.</p>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* Count */}
      <p className="text-sm text-zinc-400">
        {data?.total ?? coaches.length} rejected submission{coaches.length !== 1 ? 's' : ''}
      </p>

      {/* Rejected Coaches List */}
      <div className="space-y-3">
        {coaches.map((coach) => (
          <div
            key={coach.id}
            className="bg-[rgba(30,30,46,0.6)] backdrop-blur-[16px] border border-white/10 rounded-xl p-4"
          >
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1 min-w-0">
                {/* Header */}
                <div className="flex items-center gap-3 mb-2">
                  {coach.icon_url ? (
                    <img
                      src={coach.icon_url}
                      alt={coach.title}
                      className="w-10 h-10 rounded-lg object-cover border border-white/10 flex-shrink-0"
                    />
                  ) : (
                    <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-pierre-violet/50 to-pierre-cyan/50 flex items-center justify-center flex-shrink-0 opacity-60">
                      <span className="text-sm font-bold text-white">
                        {coach.title.charAt(0).toUpperCase()}
                      </span>
                    </div>
                  )}
                  <div className="min-w-0">
                    <h3 className="font-semibold text-white truncate">{coach.title}</h3>
                    <div className="flex items-center gap-2 mt-1">
                      <span className={clsx(
                        'px-2 py-0.5 text-xs font-medium rounded-full border',
                        getCategoryColorClass(coach.category)
                      )}>
                        {coach.category}
                      </span>
                      <span className="text-xs text-zinc-500">
                        by {coach.author_email || 'Unknown'}
                      </span>
                    </div>
                  </div>
                </div>

                {/* Rejection Info */}
                <div className="mt-3 p-3 bg-pierre-red-500/10 border border-pierre-red-500/20 rounded-lg">
                  <div className="flex items-start gap-2">
                    <svg className="w-4 h-4 text-pierre-red-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-sm font-medium text-pierre-red-300">
                          {getReasonLabel(coach.rejection_reason)}
                        </span>
                        <span className="text-xs text-zinc-500 flex-shrink-0">
                          {formatDate(coach.rejected_at)}
                        </span>
                      </div>
                      {coach.rejection_notes && (
                        <p className="text-xs text-zinc-400 mt-1">
                          {coach.rejection_notes}
                        </p>
                      )}
                    </div>
                  </div>
                </div>

                {/* Tags */}
                {coach.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1 mt-3">
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

              {/* Metadata */}
              <div className="flex-shrink-0 text-right">
                <span className="text-xs text-zinc-500">
                  {coach.token_count.toLocaleString()} tokens
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
