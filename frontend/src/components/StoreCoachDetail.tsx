// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Coach Store detail screen showing full coach info with install/uninstall actions
// ABOUTME: Displays system prompt preview, sample prompts, tags, and install count

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { apiService } from '../services/api';
import { ConfirmDialog } from './ui';

// Coach category colors
const COACH_CATEGORY_COLORS: Record<string, string> = {
  training: 'bg-emerald-100 text-emerald-700',
  nutrition: 'bg-amber-100 text-amber-700',
  recovery: 'bg-indigo-100 text-indigo-700',
  recipes: 'bg-orange-100 text-orange-700',
  mobility: 'bg-pink-100 text-pink-700',
  custom: 'bg-violet-100 text-violet-700',
};

interface StoreCoachDetailProps {
  coachId: string;
  onBack: () => void;
  onNavigateToLibrary?: () => void;
}

export default function StoreCoachDetail({ coachId, onBack, onNavigateToLibrary }: StoreCoachDetailProps) {
  const queryClient = useQueryClient();
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Fetch coach details
  const { data: coach, isLoading, error } = useQuery({
    queryKey: ['store-coach', coachId],
    queryFn: () => apiService.getStoreCoach(coachId),
    staleTime: 60_000,
  });

  // Check if coach is installed
  const { data: installations } = useQuery({
    queryKey: ['store-installations'],
    queryFn: () => apiService.getStoreInstallations(),
    staleTime: 30_000,
  });

  const isInstalled = installations?.coaches.some((c) => c.id === coachId) ?? false;

  // Install mutation
  const installMutation = useMutation({
    mutationFn: () => apiService.installStoreCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['store-installations'] });
      queryClient.invalidateQueries({ queryKey: ['store-coach', coachId] });
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setSuccessMessage(`"${coach?.title}" has been added to your coaches.`);
    },
  });

  // Uninstall mutation
  const uninstallMutation = useMutation({
    mutationFn: () => apiService.uninstallStoreCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['store-installations'] });
      queryClient.invalidateQueries({ queryKey: ['store-coach', coachId] });
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowUninstallConfirm(false);
      setSuccessMessage('Coach has been removed from My Coaches.');
    },
  });

  const handleInstall = () => {
    installMutation.mutate();
  };

  const handleUninstall = () => {
    uninstallMutation.mutate();
  };

  const categoryColors = coach ? COACH_CATEGORY_COLORS[coach.category] ?? 'bg-gray-100 text-gray-700' : '';

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center bg-white">
        <div className="text-center">
          <div className="w-8 h-8 border-2 border-pierre-violet border-t-transparent rounded-full animate-spin mx-auto" />
          <p className="mt-3 text-sm text-pierre-gray-500">Loading coach details...</p>
        </div>
      </div>
    );
  }

  if (error || !coach) {
    return (
      <div className="h-full flex flex-col items-center justify-center bg-white p-6">
        <svg
          className="w-12 h-12 text-pierre-gray-300 mb-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <h3 className="text-lg font-medium text-pierre-gray-900">Coach not found</h3>
        <p className="text-sm text-pierre-gray-500 mt-1">This coach may have been removed or is no longer available.</p>
        <button
          onClick={onBack}
          className="mt-4 px-4 py-2 bg-pierre-violet text-white rounded-lg hover:bg-pierre-violet/90 transition-colors"
        >
          Go Back
        </button>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-white">
      {/* Header */}
      <div className="flex items-center gap-4 px-6 py-4 border-b border-pierre-gray-200">
        <button
          onClick={onBack}
          className="p-2 rounded-lg hover:bg-pierre-gray-100 transition-colors"
          title="Back to Store"
        >
          <svg className="w-5 h-5 text-pierre-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <h1 className="text-xl font-semibold text-pierre-gray-900 flex-1 truncate">{coach.title}</h1>
      </div>

      {/* Success Message */}
      {successMessage && (
        <div className="mx-6 mt-4 p-4 bg-emerald-50 border border-emerald-200 rounded-lg flex items-start gap-3">
          <svg className="w-5 h-5 text-emerald-600 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <div className="flex-1">
            <p className="text-sm text-emerald-800">{successMessage}</p>
            {successMessage.includes('added') && onNavigateToLibrary && (
              <button
                onClick={onNavigateToLibrary}
                className="text-sm text-emerald-700 hover:text-emerald-800 font-medium mt-1"
              >
                View My Coaches →
              </button>
            )}
          </div>
          <button
            onClick={() => setSuccessMessage(null)}
            className="text-emerald-600 hover:text-emerald-800"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {/* Category & Stats */}
        <div className="flex items-center justify-between mb-4">
          <span className={clsx('px-3 py-1 text-sm font-medium rounded-full capitalize', categoryColors)}>
            {coach.category}
          </span>
          <span className="text-sm text-pierre-gray-500">
            {coach.install_count} {coach.install_count === 1 ? 'user' : 'users'}
          </span>
        </div>

        {/* Title */}
        <h2 className="text-2xl font-bold text-pierre-gray-900 mb-3">{coach.title}</h2>

        {/* Description */}
        {coach.description && (
          <p className="text-pierre-gray-600 mb-6 leading-relaxed">{coach.description}</p>
        )}

        {/* Tags */}
        {coach.tags.length > 0 && (
          <div className="mb-6">
            <h3 className="text-sm font-semibold text-pierre-gray-500 uppercase tracking-wide mb-2">Tags</h3>
            <div className="flex flex-wrap gap-2">
              {coach.tags.map((tag, index) => (
                <span
                  key={index}
                  className="px-3 py-1 text-sm bg-pierre-gray-100 text-pierre-gray-700 rounded-full"
                >
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Sample Prompts */}
        {coach.sample_prompts.length > 0 && (
          <div className="mb-6">
            <h3 className="text-sm font-semibold text-pierre-gray-500 uppercase tracking-wide mb-2">Sample Prompts</h3>
            <div className="space-y-2">
              {coach.sample_prompts.map((prompt, index) => (
                <div
                  key={index}
                  className="p-3 bg-pierre-gray-50 border border-pierre-gray-200 rounded-lg text-sm text-pierre-gray-700"
                >
                  {prompt}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* System Prompt Preview */}
        <div className="mb-6">
          <h3 className="text-sm font-semibold text-pierre-gray-500 uppercase tracking-wide mb-2">System Prompt</h3>
          <div className="p-4 bg-pierre-gray-50 border border-pierre-gray-200 rounded-lg">
            <pre className="text-sm text-pierre-gray-600 whitespace-pre-wrap font-mono leading-relaxed max-h-48 overflow-y-auto">
              {coach.system_prompt.length > 500
                ? `${coach.system_prompt.slice(0, 500)}...`
                : coach.system_prompt}
            </pre>
            {coach.system_prompt.length > 500 && (
              <p className="text-xs text-pierre-gray-500 mt-2 italic">
                ...and more ({coach.token_count} tokens)
              </p>
            )}
          </div>
        </div>

        {/* Details */}
        <div className="mb-6">
          <h3 className="text-sm font-semibold text-pierre-gray-500 uppercase tracking-wide mb-2">Details</h3>
          <div className="bg-pierre-gray-50 border border-pierre-gray-200 rounded-lg overflow-hidden">
            <div className="flex justify-between items-center px-4 py-3 border-b border-pierre-gray-200">
              <span className="text-sm text-pierre-gray-600">Token Count</span>
              <span className="text-sm font-medium text-pierre-gray-900">{coach.token_count.toLocaleString()}</span>
            </div>
            {coach.published_at && (
              <div className="flex justify-between items-center px-4 py-3">
                <span className="text-sm text-pierre-gray-600">Published</span>
                <span className="text-sm font-medium text-pierre-gray-900">
                  {new Date(coach.published_at).toLocaleDateString()}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Install/Uninstall Button */}
        <div className="mb-6">
          {isInstalled ? (
            <button
              onClick={() => setShowUninstallConfirm(true)}
              disabled={uninstallMutation.isPending}
              className="px-6 py-2.5 bg-pierre-gray-100 text-pierre-gray-700 rounded-lg font-medium hover:bg-pierre-gray-200 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
            >
              {uninstallMutation.isPending ? (
                <div className="w-5 h-5 border-2 border-pierre-gray-600 border-t-transparent rounded-full animate-spin" />
              ) : (
                'Remove'
              )}
            </button>
          ) : (
            <button
              onClick={handleInstall}
              disabled={installMutation.isPending}
              className="px-6 py-2.5 bg-pierre-violet text-white rounded-lg font-medium hover:bg-pierre-violet/90 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
            >
              {installMutation.isPending ? (
                <div className="w-5 h-5 border-2 border-white border-t-transparent rounded-full animate-spin" />
              ) : (
                'Add Coach'
              )}
            </button>
          )}
        </div>
      </div>

      {/* Uninstall Confirmation Dialog */}
      <ConfirmDialog
        isOpen={showUninstallConfirm}
        onClose={() => setShowUninstallConfirm(false)}
        onConfirm={handleUninstall}
        title="Remove Coach?"
        message={`Remove "${coach.title}" from your coaches? You can always add it again later.`}
        confirmText="Remove"
        confirmVariant="danger"
        isLoading={uninstallMutation.isPending}
      />
    </div>
  );
}
