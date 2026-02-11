// ABOUTME: Admin settings tab for system configuration
// ABOUTME: Provides toggles for auto-approval, social insights config, and other admin settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import { Card } from './ui';
import type { SocialInsightsConfig } from '../types/api';
import { QUERY_KEYS } from '../constants/queryKeys';

export default function AdminSettings() {
  const queryClient = useQueryClient();
  const [showSocialInsightsConfig, setShowSocialInsightsConfig] = useState(false);

  const { data: autoApprovalData, isLoading, error } = useQuery({
    queryKey: QUERY_KEYS.adminSettings.autoApproval(),
    queryFn: () => adminApi.getAutoApprovalSetting(),
    retry: 1,
  });

  const {
    data: socialInsightsConfig,
    isLoading: socialInsightsLoading,
    error: socialInsightsError,
  } = useQuery({
    queryKey: QUERY_KEYS.adminSettings.socialInsightsConfig(),
    queryFn: () => adminApi.getSocialInsightsConfig(),
    retry: 1,
  });

  const updateAutoApprovalMutation = useMutation({
    mutationFn: (enabled: boolean) => adminApi.updateAutoApprovalSetting(enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.adminSettings.autoApproval() });
    },
  });

  const updateSocialInsightsMutation = useMutation({
    mutationFn: (config: SocialInsightsConfig) => adminApi.updateSocialInsightsConfig(config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.adminSettings.socialInsightsConfig() });
    },
  });

  const resetSocialInsightsMutation = useMutation({
    mutationFn: () => adminApi.resetSocialInsightsConfig(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.adminSettings.socialInsightsConfig() });
    },
  });

  const handleToggleAutoApproval = () => {
    if (autoApprovalData) {
      updateAutoApprovalMutation.mutate(!autoApprovalData.enabled);
    }
  };

  const handleSocialInsightsChange = (field: string, value: number) => {
    if (!socialInsightsConfig) return;

    const updatedConfig = { ...socialInsightsConfig };

    // Handle nested fields
    if (field === 'min_relevance_score') {
      updatedConfig.min_relevance_score = value;
    } else if (field === 'insight_context_limit') {
      updatedConfig.activity_fetch_limits = {
        ...updatedConfig.activity_fetch_limits,
        insight_context_limit: value,
      };
    } else if (field === 'training_context_limit') {
      updatedConfig.activity_fetch_limits = {
        ...updatedConfig.activity_fetch_limits,
        training_context_limit: value,
      };
    } else if (field === 'max_client_limit') {
      updatedConfig.activity_fetch_limits = {
        ...updatedConfig.activity_fetch_limits,
        max_client_limit: value,
      };
    } else if (field === 'streak_lookback_days') {
      updatedConfig.streak_config = {
        ...updatedConfig.streak_config,
        lookback_days: value,
      };
    } else if (field === 'streak_min_for_sharing') {
      updatedConfig.streak_config = {
        ...updatedConfig.streak_config,
        min_for_sharing: value,
      };
    } else if (field === 'min_activities_for_milestone') {
      updatedConfig.milestone_thresholds = {
        ...updatedConfig.milestone_thresholds,
        min_activities_for_milestone: value,
      };
    }

    updateSocialInsightsMutation.mutate(updatedConfig);
  };

  const handleResetSocialInsights = () => {
    if (window.confirm('Are you sure you want to reset social insights configuration to defaults?')) {
      resetSocialInsightsMutation.mutate();
    }
  };

  return (
    <div className="space-y-6 max-w-3xl">
      {/* User Registration Settings */}
      <Card variant="dark">
        <h2 className="text-lg font-semibold text-white mb-4">User Registration</h2>

        <div className="space-y-4">
          {/* Auto-Approval Toggle */}
          <div className="flex items-start justify-between p-4 bg-white/5 rounded-lg border border-white/10">
            <div className="flex-1 mr-4">
              <h3 className="font-medium text-white">Auto-Approve Registrations</h3>
              <p className="text-sm text-zinc-400 mt-1">
                When enabled, new user registrations are automatically approved without requiring admin review.
                This is useful for public platforms but may pose security risks.
              </p>
              {autoApprovalData?.description && (
                <p className="text-xs text-zinc-500 mt-2">
                  {autoApprovalData.description}
                </p>
              )}
            </div>
            <div className="flex-shrink-0">
              {isLoading ? (
                <div className="w-11 h-6 bg-white/10 rounded-full animate-pulse" />
              ) : error ? (
                <span className="text-xs text-pierre-red-400">Error loading</span>
              ) : (
                <button
                  onClick={handleToggleAutoApproval}
                  disabled={updateAutoApprovalMutation.isPending}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-offset-2 focus:ring-offset-pierre-slate ${
                    autoApprovalData?.enabled
                      ? 'bg-pierre-activity'
                      : 'bg-zinc-600'
                  } ${updateAutoApprovalMutation.isPending ? 'opacity-50 cursor-not-allowed' : ''}`}
                  role="switch"
                  aria-checked={autoApprovalData?.enabled}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow-sm ${
                      autoApprovalData?.enabled ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
              )}
            </div>
          </div>

          {/* Status indicator */}
          {autoApprovalData && (
            <div className={`flex items-center gap-2 p-3 rounded-lg text-sm ${
              autoApprovalData.enabled
                ? 'bg-pierre-activity/15 text-pierre-activity border border-pierre-activity/30'
                : 'bg-white/5 text-zinc-400 border border-white/10'
            }`}>
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                {autoApprovalData.enabled ? (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                )}
              </svg>
              <span>
                {autoApprovalData.enabled
                  ? 'Auto-approval is enabled. New registrations will be approved automatically.'
                  : 'Auto-approval is disabled. New registrations require admin approval.'}
              </span>
            </div>
          )}

          {/* Mutation status */}
          {updateAutoApprovalMutation.isSuccess && (
            <div className="p-3 rounded-lg bg-pierre-activity/15 text-pierre-activity text-sm border border-pierre-activity/30">
              Setting updated successfully.
            </div>
          )}
          {updateAutoApprovalMutation.isError && (
            <div className="p-3 rounded-lg bg-pierre-red-500/15 text-pierre-red-400 text-sm border border-pierre-red-500/30">
              Failed to update setting. Please try again.
            </div>
          )}
        </div>
      </Card>

      {/* System Information */}
      <Card variant="dark">
        <h2 className="text-lg font-semibold text-white mb-4">System Information</h2>

        <div className="space-y-3">
          <div className="flex justify-between items-center py-2 border-b border-white/10">
            <span className="text-zinc-400">Application</span>
            <span className="text-white">Pierre Fitness Intelligence</span>
          </div>
          <div className="flex justify-between items-center py-2 border-b border-white/10">
            <span className="text-zinc-400">Version</span>
            <span className="text-white">0.2.0</span>
          </div>
          <div className="flex justify-between items-center py-2">
            <span className="text-zinc-400">Environment</span>
            <span className="px-2 py-1 bg-pierre-activity/20 text-pierre-activity rounded-full text-xs font-medium border border-pierre-activity/30">
              {import.meta.env.MODE}
            </span>
          </div>
        </div>
      </Card>

      {/* Social Insights Configuration */}
      <Card variant="dark">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-white">Social Insights Configuration</h2>
          <button
            onClick={() => setShowSocialInsightsConfig(!showSocialInsightsConfig)}
            className="text-sm text-pierre-violet-light hover:text-pierre-cyan-light transition-colors"
          >
            {showSocialInsightsConfig ? 'Hide Details' : 'Show Details'}
          </button>
        </div>

        <p className="text-sm text-zinc-400 mb-4">
          Configure thresholds and limits for coach-mediated social sharing features.
        </p>

        {socialInsightsLoading ? (
          <div className="p-4 flex justify-center">
            <div className="w-8 h-8 border-2 border-pierre-violet border-t-transparent rounded-full animate-spin" />
          </div>
        ) : socialInsightsError ? (
          <div className="p-3 rounded-lg bg-pierre-red-500/15 text-pierre-red-400 text-sm border border-pierre-red-500/30">
            Failed to load social insights configuration.
          </div>
        ) : socialInsightsConfig && showSocialInsightsConfig ? (
          <div className="space-y-4">
            {/* Min Relevance Score */}
            <div className="p-4 bg-white/5 rounded-lg border border-white/10">
              <div className="flex items-center justify-between mb-2">
                <label className="font-medium text-white">Minimum Relevance Score</label>
                <span className="text-sm text-zinc-400">{socialInsightsConfig.min_relevance_score}%</span>
              </div>
              <p className="text-xs text-zinc-500 mb-3">
                Suggestions with lower relevance scores will be filtered out.
              </p>
              <input
                type="range"
                min="0"
                max="100"
                value={socialInsightsConfig.min_relevance_score}
                onChange={(e) => handleSocialInsightsChange('min_relevance_score', parseInt(e.target.value))}
                className="w-full accent-pierre-violet"
                disabled={updateSocialInsightsMutation.isPending}
              />
            </div>

            {/* Activity Fetch Limits */}
            <div className="p-4 bg-white/5 rounded-lg border border-white/10">
              <h3 className="font-medium text-white mb-3">Activity Fetch Limits</h3>

              <div className="space-y-3">
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-sm text-zinc-300">Insight Context Limit</label>
                    <input
                      type="number"
                      min="1"
                      max="500"
                      value={socialInsightsConfig.activity_fetch_limits.insight_context_limit}
                      onChange={(e) => handleSocialInsightsChange('insight_context_limit', parseInt(e.target.value) || 1)}
                      className="w-20 px-2 py-1 bg-white/10 border border-white/20 rounded text-white text-sm text-right"
                      disabled={updateSocialInsightsMutation.isPending}
                    />
                  </div>
                  <p className="text-xs text-zinc-500">Activities to analyze for insight generation</p>
                </div>

                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-sm text-zinc-300">Training Context Limit</label>
                    <input
                      type="number"
                      min="1"
                      max="100"
                      value={socialInsightsConfig.activity_fetch_limits.training_context_limit}
                      onChange={(e) => handleSocialInsightsChange('training_context_limit', parseInt(e.target.value) || 1)}
                      className="w-20 px-2 py-1 bg-white/10 border border-white/20 rounded text-white text-sm text-right"
                      disabled={updateSocialInsightsMutation.isPending}
                    />
                  </div>
                  <p className="text-xs text-zinc-500">Activities for training context analysis</p>
                </div>

                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-sm text-zinc-300">Max Client Request Limit</label>
                    <input
                      type="number"
                      min="1"
                      max="1000"
                      value={socialInsightsConfig.activity_fetch_limits.max_client_limit}
                      onChange={(e) => handleSocialInsightsChange('max_client_limit', parseInt(e.target.value) || 1)}
                      className="w-20 px-2 py-1 bg-white/10 border border-white/20 rounded text-white text-sm text-right"
                      disabled={updateSocialInsightsMutation.isPending}
                    />
                  </div>
                  <p className="text-xs text-zinc-500">Maximum activities clients can request</p>
                </div>
              </div>
            </div>

            {/* Streak Configuration */}
            <div className="p-4 bg-white/5 rounded-lg border border-white/10">
              <h3 className="font-medium text-white mb-3">Streak Configuration</h3>

              <div className="space-y-3">
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-sm text-zinc-300">Lookback Days</label>
                    <input
                      type="number"
                      min="7"
                      max="365"
                      value={socialInsightsConfig.streak_config.lookback_days}
                      onChange={(e) => handleSocialInsightsChange('streak_lookback_days', parseInt(e.target.value) || 7)}
                      className="w-20 px-2 py-1 bg-white/10 border border-white/20 rounded text-white text-sm text-right"
                      disabled={updateSocialInsightsMutation.isPending}
                    />
                  </div>
                  <p className="text-xs text-zinc-500">Days to analyze for streak calculation</p>
                </div>

                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-sm text-zinc-300">Min Days for Sharing</label>
                    <input
                      type="number"
                      min="1"
                      max="30"
                      value={socialInsightsConfig.streak_config.min_for_sharing}
                      onChange={(e) => handleSocialInsightsChange('streak_min_for_sharing', parseInt(e.target.value) || 1)}
                      className="w-20 px-2 py-1 bg-white/10 border border-white/20 rounded text-white text-sm text-right"
                      disabled={updateSocialInsightsMutation.isPending}
                    />
                  </div>
                  <p className="text-xs text-zinc-500">Minimum streak length to suggest sharing</p>
                </div>
              </div>
            </div>

            {/* Milestone Configuration */}
            <div className="p-4 bg-white/5 rounded-lg border border-white/10">
              <h3 className="font-medium text-white mb-3">Milestone Configuration</h3>

              <div>
                <div className="flex items-center justify-between mb-1">
                  <label className="text-sm text-zinc-300">Min Activities for Milestone</label>
                  <input
                    type="number"
                    min="1"
                    max="100"
                    value={socialInsightsConfig.milestone_thresholds.min_activities_for_milestone}
                    onChange={(e) => handleSocialInsightsChange('min_activities_for_milestone', parseInt(e.target.value) || 1)}
                    className="w-20 px-2 py-1 bg-white/10 border border-white/20 rounded text-white text-sm text-right"
                    disabled={updateSocialInsightsMutation.isPending}
                  />
                </div>
                <p className="text-xs text-zinc-500">Minimum activity count before suggesting milestones</p>
              </div>
            </div>

            {/* Reset to Defaults */}
            <div className="flex justify-end pt-2">
              <button
                onClick={handleResetSocialInsights}
                disabled={resetSocialInsightsMutation.isPending}
                className="px-4 py-2 text-sm text-zinc-400 hover:text-white hover:bg-white/10 rounded-lg transition-colors disabled:opacity-50"
              >
                {resetSocialInsightsMutation.isPending ? 'Resetting...' : 'Reset to Defaults'}
              </button>
            </div>

            {/* Mutation Status */}
            {updateSocialInsightsMutation.isSuccess && (
              <div className="p-3 rounded-lg bg-pierre-activity/15 text-pierre-activity text-sm border border-pierre-activity/30">
                Configuration updated successfully.
              </div>
            )}
            {(updateSocialInsightsMutation.isError || resetSocialInsightsMutation.isError) && (
              <div className="p-3 rounded-lg bg-pierre-red-500/15 text-pierre-red-400 text-sm border border-pierre-red-500/30">
                Failed to update configuration. Please try again.
              </div>
            )}
          </div>
        ) : socialInsightsConfig && !showSocialInsightsConfig ? (
          <div className="p-3 rounded-lg bg-white/5 text-zinc-400 text-sm border border-white/10">
            Click "Show Details" to configure social insights parameters.
          </div>
        ) : null}
      </Card>

      {/* Security Recommendations */}
      <Card variant="dark" className="border-pierre-nutrition/30">
        <div className="flex items-start gap-3">
          <div className="flex-shrink-0 p-2 bg-pierre-nutrition/20 rounded-lg">
            <svg className="w-5 h-5 text-pierre-nutrition" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <div>
            <h3 className="font-medium text-white">Security Recommendations</h3>
            <ul className="mt-2 space-y-1 text-sm text-zinc-400">
              <li>Keep auto-approval disabled for production environments.</li>
              <li>Regularly review pending user registrations.</li>
              <li>Monitor the Users tab for suspicious registrations.</li>
            </ul>
          </div>
        </div>
      </Card>
    </div>
  );
}
