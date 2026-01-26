// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Card component displaying a coach-generated insight suggestion
// ABOUTME: Shows preview with type badge, content, relevance, and share action

import { clsx } from 'clsx';
import type { InsightType, TrainingPhase } from '../../types/social';

interface InsightSuggestion {
  insight_type: InsightType;
  suggested_content: string;
  suggested_title?: string;
  relevance_score: number;
  sport_type?: string;
  training_phase?: TrainingPhase;
  source_activity_id?: string;
}

interface SuggestionCardProps {
  suggestion: InsightSuggestion;
  onShare: (suggestion: InsightSuggestion) => void;
  isSelected?: boolean;
}

const INSIGHT_TYPE_CONFIG: Record<InsightType, { icon: string; color: string; label: string }> = {
  achievement: { icon: '🏆', color: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30', label: 'Achievement' },
  milestone: { icon: '🚩', color: 'bg-amber-500/20 text-amber-400 border-amber-500/30', label: 'Milestone' },
  training_tip: { icon: '⚡', color: 'bg-indigo-500/20 text-indigo-400 border-indigo-500/30', label: 'Training Tip' },
  recovery: { icon: '🌙', color: 'bg-violet-500/20 text-violet-400 border-violet-500/30', label: 'Recovery' },
  motivation: { icon: '☀️', color: 'bg-orange-500/20 text-orange-400 border-orange-500/30', label: 'Motivation' },
  coaching_insight: { icon: '🎯', color: 'bg-pierre-violet/20 text-pierre-violet border-pierre-violet/30', label: 'Coaching Insight' },
};

export default function SuggestionCard({ suggestion, onShare, isSelected = false }: SuggestionCardProps) {
  const config = INSIGHT_TYPE_CONFIG[suggestion.insight_type] || INSIGHT_TYPE_CONFIG.motivation;
  const relevancePercentage = Math.round(suggestion.relevance_score * 100);

  const getRelevanceColor = (percentage: number) => {
    if (percentage >= 70) return 'bg-emerald-500/20 text-emerald-400';
    if (percentage >= 40) return 'bg-amber-500/20 text-amber-400';
    return 'bg-zinc-500/20 text-zinc-400';
  };

  return (
    <button
      className={clsx(
        'w-full text-left rounded-xl p-4 mb-3 transition-all',
        isSelected
          ? 'bg-pierre-violet/20 border-2 border-pierre-violet'
          : 'bg-white/5 border border-white/10 hover:bg-white/10'
      )}
      onClick={() => onShare(suggestion)}
    >
      {/* Header with type badge and relevance */}
      <div className="flex items-center justify-between mb-3">
        {/* Type badge */}
        <span className={clsx(
          'inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium border',
          config.color
        )}>
          <span>{config.icon}</span>
          <span>{config.label}</span>
        </span>

        {/* Relevance indicator */}
        <div className="flex items-center gap-1.5">
          <span className="text-xs text-zinc-500">Relevance</span>
          <span className={clsx(
            'px-2 py-0.5 rounded text-xs font-semibold',
            getRelevanceColor(relevancePercentage)
          )}>
            {relevancePercentage}%
          </span>
        </div>
      </div>

      {/* Optional title */}
      {suggestion.suggested_title && (
        <h4 className="text-white font-semibold mb-1 line-clamp-1">{suggestion.suggested_title}</h4>
      )}

      {/* Content preview */}
      <p className="text-zinc-300 text-sm leading-relaxed mb-3 line-clamp-3">
        {suggestion.suggested_content}
      </p>

      {/* Context badges */}
      {(suggestion.sport_type || suggestion.training_phase) && (
        <div className="flex flex-wrap gap-2 mb-3">
          {suggestion.sport_type && (
            <span className="px-2 py-1 text-xs bg-white/10 text-zinc-400 rounded">
              {suggestion.sport_type}
            </span>
          )}
          {suggestion.training_phase && (
            <span className="px-2 py-1 text-xs bg-white/10 text-zinc-400 rounded capitalize">
              {suggestion.training_phase} phase
            </span>
          )}
        </div>
      )}

      {/* Share button indicator */}
      <div className="flex justify-end">
        <span className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-pierre-violet text-white text-sm font-semibold">
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
          </svg>
          Share This
        </span>
      </div>
    </button>
  );
}
