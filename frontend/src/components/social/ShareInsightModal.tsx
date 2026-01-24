// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Modal for sharing coach insights with friends
// ABOUTME: Allows selecting insight type, content, sport, and visibility

import { useState } from 'react';
import { clsx } from 'clsx';
import { apiService } from '../../services/api';
import { Button, Modal } from '../ui';

type InsightType = 'achievement' | 'milestone' | 'training_tip' | 'recovery' | 'motivation';
type ShareVisibility = 'friends_only' | 'public';
type TrainingPhase = 'base' | 'build' | 'peak' | 'recovery';

const INSIGHT_TYPES: Array<{ key: InsightType; label: string; icon: string; color: string }> = [
  { key: 'achievement', label: 'Achievement', icon: '🏆', color: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30' },
  { key: 'milestone', label: 'Milestone', icon: '🚩', color: 'bg-amber-500/20 text-amber-400 border-amber-500/30' },
  { key: 'training_tip', label: 'Training Tip', icon: '⚡', color: 'bg-indigo-500/20 text-indigo-400 border-indigo-500/30' },
  { key: 'recovery', label: 'Recovery', icon: '🌙', color: 'bg-violet-500/20 text-violet-400 border-violet-500/30' },
  { key: 'motivation', label: 'Motivation', icon: '☀️', color: 'bg-orange-500/20 text-orange-400 border-orange-500/30' },
];

const SPORT_TYPES = ['Running', 'Cycling', 'Swimming', 'Triathlon', 'Strength', 'Other'];

const TRAINING_PHASES: Array<{ key: TrainingPhase; label: string }> = [
  { key: 'base', label: 'Base Building' },
  { key: 'build', label: 'Build Phase' },
  { key: 'peak', label: 'Peak/Race' },
  { key: 'recovery', label: 'Recovery' },
];

interface ShareInsightModalProps {
  onClose: () => void;
  onSuccess: () => void;
}

export default function ShareInsightModal({ onClose, onSuccess }: ShareInsightModalProps) {
  const [insightType, setInsightType] = useState<InsightType>('achievement');
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [sportType, setSportType] = useState<string | null>(null);
  const [trainingPhase, setTrainingPhase] = useState<TrainingPhase | null>(null);
  const [visibility, setVisibility] = useState<ShareVisibility>('friends_only');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canSubmit = content.trim().length >= 10;

  const handleSubmit = async () => {
    if (!canSubmit) return;

    try {
      setIsSubmitting(true);
      setError(null);

      await apiService.shareInsight({
        insight_type: insightType,
        title: title.trim() || undefined,
        content: content.trim(),
        sport_type: sportType || undefined,
        training_phase: trainingPhase || undefined,
        visibility,
      });

      onSuccess();
    } catch (err) {
      console.error('Failed to share insight:', err);
      setError('Failed to share insight. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal isOpen onClose={onClose} title="Share Insight" size="lg">
      <div className="space-y-6">
        {/* Insight Type */}
        <div>
          <label className="block text-sm font-medium text-zinc-400 mb-3">Type</label>
          <div className="flex flex-wrap gap-2">
            {INSIGHT_TYPES.map((type) => (
              <button
                key={type.key}
                onClick={() => setInsightType(type.key)}
                className={clsx(
                  'flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium border transition-colors',
                  insightType === type.key
                    ? type.color
                    : 'bg-white/5 text-zinc-400 border-white/10 hover:bg-white/10'
                )}
              >
                <span>{type.icon}</span>
                <span>{type.label}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Title */}
        <div>
          <label className="block text-sm font-medium text-zinc-400 mb-2">
            Title <span className="text-zinc-600">(optional)</span>
          </label>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Give your insight a catchy title..."
            maxLength={100}
            className="w-full px-4 py-2 bg-white/5 border border-white/10 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-pierre-violet/50"
          />
        </div>

        {/* Content */}
        <div>
          <label className="block text-sm font-medium text-zinc-400 mb-2">
            Content <span className="text-pierre-red-500">*</span>
          </label>
          <textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="Share your coach insight... (min 10 characters)"
            maxLength={500}
            rows={4}
            className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-pierre-violet/50 resize-none"
          />
          <div className="flex justify-between mt-1">
            <span className="text-xs text-zinc-500">
              {content.length < 10 ? `${10 - content.length} more characters needed` : ''}
            </span>
            <span className="text-xs text-zinc-500">{content.length}/500</span>
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

        {/* Sport Type */}
        <div>
          <label className="block text-sm font-medium text-zinc-400 mb-2">
            Sport <span className="text-zinc-600">(optional)</span>
          </label>
          <div className="flex flex-wrap gap-2">
            {SPORT_TYPES.map((sport) => (
              <button
                key={sport}
                onClick={() => setSportType(sportType === sport ? null : sport)}
                className={clsx(
                  'px-3 py-1.5 rounded-full text-sm font-medium transition-colors',
                  sportType === sport
                    ? 'bg-pierre-violet text-white'
                    : 'bg-white/5 text-zinc-400 hover:bg-white/10'
                )}
              >
                {sport}
              </button>
            ))}
          </div>
        </div>

        {/* Training Phase */}
        <div>
          <label className="block text-sm font-medium text-zinc-400 mb-2">
            Training Phase <span className="text-zinc-600">(optional)</span>
          </label>
          <div className="flex flex-wrap gap-2">
            {TRAINING_PHASES.map((phase) => (
              <button
                key={phase.key}
                onClick={() => setTrainingPhase(trainingPhase === phase.key ? null : phase.key)}
                className={clsx(
                  'px-3 py-1.5 rounded-full text-sm font-medium transition-colors',
                  trainingPhase === phase.key
                    ? 'bg-pierre-violet text-white'
                    : 'bg-white/5 text-zinc-400 hover:bg-white/10'
                )}
              >
                {phase.label}
              </button>
            ))}
          </div>
        </div>

        {/* Visibility */}
        <div>
          <label className="block text-sm font-medium text-zinc-400 mb-2">Visibility</label>
          <div className="flex gap-3">
            <button
              onClick={() => setVisibility('friends_only')}
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

        {/* Error */}
        {error && (
          <div className="p-3 bg-pierre-red-500/20 border border-pierre-red-500/30 rounded-lg text-pierre-red-400 text-sm">
            {error}
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-3 pt-4 border-t border-white/10">
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={handleSubmit}
            disabled={!canSubmit}
            loading={isSubmitting}
          >
            Share Insight
          </Button>
        </div>
      </div>
    </Modal>
  );
}
