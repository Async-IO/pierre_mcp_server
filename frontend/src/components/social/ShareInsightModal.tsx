// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Modal for sharing coach-generated insight suggestions with friends
// ABOUTME: Shows coach suggestions, allows editing, and publishes to social feed

import { useState, useEffect, useCallback } from 'react';
import { clsx } from 'clsx';
import { socialApi } from '../../services/api';
import { Button, Modal } from '../ui';
import SuggestionCard from './SuggestionCard';
import type { InsightType, TrainingPhase, ShareVisibility } from '../../types/social';

interface InsightSuggestion {
  insight_type: InsightType;
  suggested_content: string;
  suggested_title?: string;
  relevance_score: number;
  sport_type?: string;
  training_phase?: TrainingPhase;
  source_activity_id?: string;
}

type FlowState = 'loading' | 'suggestions' | 'editing' | 'submitting' | 'error';

interface ShareInsightModalProps {
  onClose: () => void;
  onSuccess: () => void;
  /** Optional activity ID to filter suggestions for a specific activity */
  activityId?: string;
}

export default function ShareInsightModal({ onClose, onSuccess, activityId }: ShareInsightModalProps) {
  // Flow state
  const [flowState, setFlowState] = useState<FlowState>('loading');
  const [error, setError] = useState<string | null>(null);

  // Suggestions from coach
  const [suggestions, setSuggestions] = useState<InsightSuggestion[]>([]);

  // Selected suggestion for editing
  const [selectedSuggestion, setSelectedSuggestion] = useState<InsightSuggestion | null>(null);
  const [editedContent, setEditedContent] = useState('');
  const [visibility, setVisibility] = useState<ShareVisibility>('friends_only');

  // Fetch suggestions on mount (optionally filtered by activityId)
  const fetchSuggestions = useCallback(async () => {
    try {
      setError(null);
      setFlowState('loading');

      const response = await socialApi.getInsightSuggestions({
        limit: 10,
        activity_id: activityId,
      });
      setSuggestions(response.suggestions as InsightSuggestion[]);

      if (response.suggestions.length > 0) {
        setFlowState('suggestions');
      } else {
        setError('No suggestions available. Complete some activities to unlock sharing!');
        setFlowState('error');
      }
    } catch (err) {
      console.error('Failed to fetch suggestions:', err);
      setError('Failed to load coach suggestions. Please try again.');
      setFlowState('error');
    }
  }, [activityId]);

  useEffect(() => {
    fetchSuggestions();
  }, [fetchSuggestions]);

  // Handle selecting a suggestion to edit
  const handleSelectSuggestion = useCallback((suggestion: InsightSuggestion) => {
    setSelectedSuggestion(suggestion);
    setEditedContent(suggestion.suggested_content);
    setFlowState('editing');
  }, []);

  // Handle going back from editing to suggestions
  const handleBackToSuggestions = useCallback(() => {
    setSelectedSuggestion(null);
    setEditedContent('');
    setFlowState('suggestions');
  }, []);

  // Handle sharing the insight
  const handleShare = async () => {
    if (!selectedSuggestion || !editedContent.trim()) return;

    try {
      setFlowState('submitting');

      await socialApi.shareFromActivity({
        activity_id: selectedSuggestion.source_activity_id,
        insight_type: selectedSuggestion.insight_type,
        content: editedContent.trim(),
        visibility,
      });

      onSuccess();
    } catch (err) {
      console.error('Failed to share insight:', err);
      setError('Failed to share. Please try again.');
      setFlowState('editing');
    }
  };

  const canSubmit = editedContent.trim().length >= 10;

  // Loading state
  if (flowState === 'loading') {
    return (
      <Modal isOpen onClose={onClose} title="Share Insight" size="lg">
        <div className="flex flex-col items-center justify-center py-12">
          <div className="pierre-spinner mb-4"></div>
          <p className="text-zinc-400">Loading coach suggestions...</p>
        </div>
      </Modal>
    );
  }

  // Error state with no suggestions
  if (flowState === 'error' && suggestions.length === 0) {
    return (
      <Modal isOpen onClose={onClose} title="Share Insight" size="lg">
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-zinc-800 flex items-center justify-center">
            <svg className="w-8 h-8 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
            </svg>
          </div>
          <h3 className="text-lg font-semibold text-white mb-2">No Insights Available</h3>
          <p className="text-zinc-400 mb-6 max-w-sm">
            {error || 'Complete some activities to unlock coach-mediated sharing!'}
          </p>
          <Button variant="primary" onClick={fetchSuggestions}>
            Refresh
          </Button>
        </div>
      </Modal>
    );
  }

  // Editing state - show selected suggestion with edit capability
  if (flowState === 'editing' || flowState === 'submitting') {
    return (
      <Modal isOpen onClose={onClose} title="Edit & Share" size="lg">
        <div className="space-y-5">
          {/* Back button */}
          <button
            onClick={handleBackToSuggestions}
            disabled={flowState === 'submitting'}
            className="flex items-center gap-2 text-sm text-zinc-400 hover:text-white transition-colors disabled:opacity-50"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
            Back to suggestions
          </button>

          {/* Type indicator */}
          {selectedSuggestion && (
            <div className="text-sm text-zinc-500 uppercase tracking-wide">
              Coach Suggestion: {selectedSuggestion.insight_type.replace('_', ' ')}
            </div>
          )}

          {/* Editable content */}
          <div>
            <label className="block text-sm font-medium text-zinc-400 mb-2">Content</label>
            <textarea
              value={editedContent}
              onChange={(e) => setEditedContent(e.target.value)}
              placeholder="Edit your coach insight... (min 10 characters)"
              maxLength={500}
              rows={6}
              disabled={flowState === 'submitting'}
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-pierre-violet/50 resize-none disabled:opacity-50"
            />
            <div className="flex justify-between mt-1">
              <span className="text-xs text-zinc-500">
                {editedContent.length < 10 ? `${10 - editedContent.length} more characters needed` : ''}
              </span>
              <span className="text-xs text-zinc-500">{editedContent.length}/500</span>
            </div>
          </div>

          {/* Privacy Note */}
          <div className="flex items-start gap-3 p-4 bg-pierre-violet/10 border border-pierre-violet/20 rounded-lg">
            <svg className="w-5 h-5 text-pierre-violet-light flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
            <div>
              <p className="text-sm font-medium text-white">Privacy Protected</p>
              <p className="text-sm text-zinc-400 mt-1">
                Your insight is automatically sanitized. GPS coordinates, exact pace, recovery scores, and other private data are never shared.
              </p>
            </div>
          </div>

          {/* Visibility */}
          <div>
            <label className="block text-sm font-medium text-zinc-400 mb-2">Visibility</label>
            <div className="flex gap-3">
              <button
                onClick={() => setVisibility('friends_only')}
                disabled={flowState === 'submitting'}
                className={clsx(
                  'flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg border transition-colors',
                  visibility === 'friends_only'
                    ? 'bg-pierre-violet/20 border-pierre-violet text-pierre-violet-light'
                    : 'bg-white/5 border-white/10 text-zinc-400 hover:bg-white/10'
                )}
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
                </svg>
                Friends Only
              </button>
              <button
                onClick={() => setVisibility('public')}
                disabled={flowState === 'submitting'}
                className={clsx(
                  'flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg border transition-colors',
                  visibility === 'public'
                    ? 'bg-pierre-violet/20 border-pierre-violet text-pierre-violet-light'
                    : 'bg-white/5 border-white/10 text-zinc-400 hover:bg-white/10'
                )}
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Public
              </button>
            </div>
          </div>

          {/* Error message */}
          {error && (
            <div className="p-3 bg-pierre-red-500/20 border border-pierre-red-500/30 rounded-lg text-pierre-red-400 text-sm">
              {error}
            </div>
          )}

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4 border-t border-white/10">
            <Button variant="secondary" onClick={onClose} disabled={flowState === 'submitting'}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleShare}
              disabled={!canSubmit}
              loading={flowState === 'submitting'}
            >
              Share Insight
            </Button>
          </div>
        </div>
      </Modal>
    );
  }

  // Suggestions state - show list of coach suggestions
  return (
    <Modal isOpen onClose={onClose} title="Coach Suggestions" size="lg">
      <div className="space-y-4">
        {/* Intro text */}
        <div className="mb-4">
          <p className="text-white font-medium">Your coach noticed some achievements!</p>
          <p className="text-sm text-zinc-400 mt-1">
            Select a suggestion to share with friends.
          </p>
        </div>

        {/* Suggestions list */}
        <div className="max-h-[400px] overflow-y-auto pr-2 -mr-2">
          {suggestions.map((suggestion, index) => (
            <SuggestionCard
              key={`suggestion-${index}-${suggestion.insight_type}`}
              suggestion={suggestion}
              onShare={handleSelectSuggestion}
            />
          ))}
        </div>

        {/* Refresh button */}
        <div className="flex justify-center pt-2">
          <button
            onClick={fetchSuggestions}
            className="text-sm text-zinc-400 hover:text-white transition-colors flex items-center gap-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            Refresh suggestions
          </button>
        </div>
      </div>
    </Modal>
  );
}
