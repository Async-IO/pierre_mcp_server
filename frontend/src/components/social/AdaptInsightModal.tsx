// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Modal for adapting a friend's insight to the user's training context
// ABOUTME: Shows the adapted personalized content from Pierre AI coach

import { useState, useEffect } from 'react';
import { apiService } from '../../services/api';
import { Button, Modal } from '../ui';

interface AdaptedInsight {
  id: string;
  user_id: string;
  source_insight_id: string;
  adapted_content: string;
  adaptation_context: string | null;
  created_at: string;
}

interface SharedInsight {
  id: string;
  user_id: string;
  visibility: string;
  insight_type: string;
  sport_type: string | null;
  content: string;
  title: string | null;
  training_phase: string | null;
  reaction_count: number;
  adapt_count: number;
  created_at: string;
  updated_at: string;
  expires_at: string | null;
}

interface AdaptInsightModalProps {
  insightId: string;
  onClose: () => void;
  onSuccess: () => void;
}

export default function AdaptInsightModal({ insightId, onClose, onSuccess }: AdaptInsightModalProps) {
  const [isLoading, setIsLoading] = useState(true);
  const [adaptedContent, setAdaptedContent] = useState<AdaptedInsight | null>(null);
  const [sourceInsight, setSourceInsight] = useState<SharedInsight | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const adaptInsight = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const response = await apiService.adaptInsight(insightId);
        setAdaptedContent(response.adapted);
        setSourceInsight(response.source_insight);
      } catch (err) {
        console.error('Failed to adapt insight:', err);
        setError('Failed to adapt this insight. Please try again.');
      } finally {
        setIsLoading(false);
      }
    };

    adaptInsight();
  }, [insightId]);

  const handleSave = () => {
    onSuccess();
  };

  return (
    <Modal isOpen onClose={onClose} title="Adapt to My Training" size="lg">
      <div className="space-y-6">
        {isLoading ? (
          <div className="flex flex-col items-center justify-center py-12">
            <div className="pierre-spinner mb-4"></div>
            <p className="text-zinc-400 text-sm">Pierre is personalizing this insight for you...</p>
          </div>
        ) : error ? (
          <div className="text-center py-8">
            <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-pierre-red-500/20 flex items-center justify-center">
              <svg className="w-8 h-8 text-pierre-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
            <p className="text-pierre-red-400 mb-4">{error}</p>
            <Button variant="secondary" onClick={onClose}>
              Close
            </Button>
          </div>
        ) : (
          <>
            {/* Original Insight */}
            {sourceInsight && (
              <div>
                <h3 className="text-sm font-medium text-zinc-400 mb-2">Original Insight</h3>
                <div className="p-4 bg-white/5 border border-white/10 rounded-lg">
                  {sourceInsight.title && (
                    <p className="font-medium text-zinc-300 mb-2">{sourceInsight.title}</p>
                  )}
                  <p className="text-zinc-400 text-sm">{sourceInsight.content}</p>
                </div>
              </div>
            )}

            {/* Adapted Content */}
            {adaptedContent && (
              <div>
                <h3 className="text-sm font-medium text-zinc-400 mb-2 flex items-center gap-2">
                  <svg className="w-4 h-4 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                  Personalized for You
                </h3>
                <div className="p-4 bg-pierre-violet/10 border border-pierre-violet/30 rounded-lg">
                  <p className="text-white whitespace-pre-wrap">{adaptedContent.adapted_content}</p>
                  {adaptedContent.adaptation_context && (
                    <p className="text-sm text-zinc-400 mt-3 pt-3 border-t border-white/10">
                      <span className="font-medium text-zinc-300">Context: </span>
                      {adaptedContent.adaptation_context}
                    </p>
                  )}
                </div>
              </div>
            )}

            {/* Info Box */}
            <div className="flex items-start gap-3 p-4 bg-white/5 border border-white/10 rounded-lg">
              <svg className="w-5 h-5 text-pierre-cyan flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div>
                <p className="text-sm text-zinc-300">
                  Pierre has analyzed your training history and goals to personalize this insight specifically for you.
                  This adaptation is saved to your library for future reference.
                </p>
              </div>
            </div>

            {/* Actions */}
            <div className="flex justify-end gap-3 pt-4 border-t border-white/10">
              <Button variant="secondary" onClick={onClose}>
                Close
              </Button>
              <Button variant="primary" onClick={handleSave}>
                Save to Library
              </Button>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}
