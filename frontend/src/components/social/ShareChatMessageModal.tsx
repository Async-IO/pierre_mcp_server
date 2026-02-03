// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Modal for sharing an insight to the social feed
// ABOUTME: Simple modal with visibility selection and direct sharing

import { useState } from 'react';
import { socialApi } from '../../services/api';
import { Button, Modal, InsightPreview } from '../ui';
import type { ShareVisibility } from '../../types/social';

interface ShareChatMessageModalProps {
  /** Whether the modal is open (defaults to true when component is rendered) */
  isOpen?: boolean;
  /** The message content to share */
  content: string;
  /** Callback when modal is closed */
  onClose: () => void;
  /** Callback when share is successful */
  onSuccess: () => void;
}

const VISIBILITY_OPTIONS: { value: ShareVisibility; label: string; description: string }[] = [
  { value: 'friends_only', label: 'Friends Only', description: 'Only your friends can see this' },
  { value: 'public', label: 'Public', description: 'Anyone can see this' },
];

export default function ShareChatMessageModal({
  isOpen = true,
  content,
  onClose,
  onSuccess,
}: ShareChatMessageModalProps) {
  const [visibility, setVisibility] = useState<ShareVisibility>('friends_only');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleShare = async () => {
    setIsSubmitting(true);
    setError(null);

    try {
      await socialApi.shareFromActivity({
        content,
        insight_type: 'coaching_insight',
        visibility,
      });
      onSuccess();
      onClose();
    } catch (err) {
      console.error('Failed to share message:', err);
      setError(err instanceof Error ? err.message : 'Failed to share. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Share Insight" size="2xl">
      <div className="space-y-6">
        {/* Message Preview */}
        <InsightPreview
          content={content}
          label="Message to Share"
          maxHeight="max-h-80"
        />

        {/* Visibility Selection */}
        <div>
          <label className="block text-sm font-medium text-zinc-300 mb-3">
            Who can see this?
          </label>
          <div className="space-y-2">
            {VISIBILITY_OPTIONS.map((option) => (
              <label
                key={option.value}
                className={`flex items-start p-3 rounded-lg cursor-pointer transition-colors ${
                  visibility === option.value
                    ? 'bg-pierre-violet/20 border-pierre-violet border'
                    : 'bg-white/5 border-transparent border hover:bg-white/10'
                }`}
              >
                <input
                  type="radio"
                  name="visibility"
                  value={option.value}
                  checked={visibility === option.value}
                  onChange={(e) => setVisibility(e.target.value as ShareVisibility)}
                  className="mt-1 mr-3"
                />
                <div>
                  <div className="text-sm font-medium text-zinc-100">{option.label}</div>
                  <div className="text-xs text-zinc-400">{option.description}</div>
                </div>
              </label>
            ))}
          </div>
        </div>

        {/* Error Message */}
        {error && (
          <div className="bg-pierre-red-500/20 border border-pierre-red-500/30 rounded-lg p-3">
            <p className="text-sm text-pierre-red-400">{error}</p>
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-3 justify-end">
          <Button variant="secondary" onClick={onClose} disabled={isSubmitting}>
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={handleShare}
            disabled={isSubmitting}
          >
            {isSubmitting ? 'Sharing...' : 'Share'}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
