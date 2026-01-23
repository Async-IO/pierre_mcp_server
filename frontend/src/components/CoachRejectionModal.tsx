// ABOUTME: Modal component for rejecting coach submissions with reason selection
// ABOUTME: Provides dropdown for rejection reason and optional notes textarea
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Button, Card } from './ui';

const REJECTION_REASONS = [
  { value: 'inappropriate_content', label: 'Inappropriate content' },
  { value: 'quality_standards', label: 'Quality standards not met' },
  { value: 'duplicate_submission', label: 'Duplicate submission' },
  { value: 'incomplete_information', label: 'Incomplete information' },
  { value: 'other', label: 'Other' },
];

interface Coach {
  id: string;
  title: string;
  author_email?: string;
}

interface CoachRejectionModalProps {
  coach: Coach | null;
  isOpen: boolean;
  onClose: () => void;
  onComplete: () => void;
}

export default function CoachRejectionModal({
  coach,
  isOpen,
  onClose,
  onComplete,
}: CoachRejectionModalProps) {
  const [reason, setReason] = useState('');
  const [notes, setNotes] = useState('');
  const queryClient = useQueryClient();

  const rejectMutation = useMutation({
    mutationFn: ({ coachId, rejectionReason, rejectionNotes }: {
      coachId: string;
      rejectionReason: string;
      rejectionNotes?: string;
    }) => apiService.rejectStoreCoach(coachId, rejectionReason, rejectionNotes),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-store-review-queue'] });
      queryClient.invalidateQueries({ queryKey: ['admin-store-stats'] });
      queryClient.invalidateQueries({ queryKey: ['admin-store-rejected'] });
      setReason('');
      setNotes('');
      onComplete();
    },
  });

  const handleSubmit = () => {
    if (coach && reason) {
      rejectMutation.mutate({
        coachId: coach.id,
        rejectionReason: reason,
        rejectionNotes: notes.trim() || undefined,
      });
    }
  };

  const handleClose = () => {
    setReason('');
    setNotes('');
    onClose();
  };

  if (!isOpen || !coach) return null;

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[60]">
      <div className="bg-[rgba(30,30,46,0.95)] backdrop-blur-[16px] rounded-xl border border-white/10 max-w-md w-full mx-4 shadow-2xl">
        {/* Red accent bar */}
        <div className="h-1 bg-gradient-to-r from-pierre-red-500 to-pierre-red-400 rounded-t-xl" />

        <div className="p-6">
          {/* Header */}
          <div className="flex justify-between items-start mb-4">
            <div>
              <h2 className="text-xl font-semibold text-white">Reject Coach</h2>
              <p className="text-sm text-zinc-400 mt-1">
                "{coach.title}" by {coach.author_email || 'Unknown'}
              </p>
            </div>
            <button
              onClick={handleClose}
              aria-label="Close modal"
              className="text-zinc-400 hover:text-white transition-colors"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          {/* Warning Card */}
          <Card variant="dark" className="mb-4 p-3 bg-pierre-red-500/10 border-pierre-red-500/30">
            <div className="flex items-start gap-3">
              <svg className="w-5 h-5 text-pierre-red-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              <p className="text-sm text-pierre-red-300">
                This action will reject the coach submission. The author will be notified of the rejection reason.
              </p>
            </div>
          </Card>

          {/* Reason Select */}
          <div className="mb-4">
            <label htmlFor="rejection-reason" className="block text-sm font-medium text-zinc-300 mb-2">
              Rejection Reason <span className="text-pierre-red-400">*</span>
            </label>
            <select
              id="rejection-reason"
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              className="select-dark w-full"
            >
              <option value="">Select a reason...</option>
              {REJECTION_REASONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          {/* Notes Textarea */}
          <div className="mb-6">
            <label htmlFor="rejection-notes" className="block text-sm font-medium text-zinc-300 mb-2">
              Additional Notes <span className="text-zinc-500">(optional)</span>
            </label>
            <textarea
              id="rejection-notes"
              rows={3}
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              className="input-dark w-full"
              placeholder="Provide additional context for the rejection..."
            />
          </div>

          {/* Action Buttons */}
          <div className="flex gap-3">
            <Button
              variant="outline"
              onClick={handleClose}
              disabled={rejectMutation.isPending}
              className="flex-1"
            >
              Cancel
            </Button>
            <Button
              onClick={handleSubmit}
              disabled={!reason || rejectMutation.isPending}
              className="flex-1 bg-pierre-red-500 hover:bg-pierre-red-600 text-white"
            >
              {rejectMutation.isPending ? (
                <span className="flex items-center justify-center">
                  <div className="pierre-spinner w-4 h-4 mr-2 border-white border-t-transparent" />
                  Rejecting...
                </span>
              ) : (
                'Reject Coach'
              )}
            </Button>
          </div>

          {/* Error Message */}
          {rejectMutation.isError && (
            <div className="mt-4 p-3 bg-pierre-red-500/15 border border-pierre-red-500/30 rounded-md">
              <p className="text-sm text-pierre-red-400">
                Failed to reject coach. Please try again.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
