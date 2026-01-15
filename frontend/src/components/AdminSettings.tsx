// ABOUTME: Admin settings tab for system configuration
// ABOUTME: Provides toggles for auto-approval and other admin-configurable settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card } from './ui';

export default function AdminSettings() {
  const queryClient = useQueryClient();

  const { data: autoApprovalData, isLoading, error } = useQuery({
    queryKey: ['auto-approval-setting'],
    queryFn: () => apiService.getAutoApprovalSetting(),
    retry: 1,
  });

  const updateAutoApprovalMutation = useMutation({
    mutationFn: (enabled: boolean) => apiService.updateAutoApprovalSetting(enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['auto-approval-setting'] });
    },
  });

  const handleToggleAutoApproval = () => {
    if (autoApprovalData) {
      updateAutoApprovalMutation.mutate(!autoApprovalData.enabled);
    }
  };

  return (
    <div className="space-y-6 max-w-3xl">
      {/* User Registration Settings */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">User Registration</h2>

        <div className="space-y-4">
          {/* Auto-Approval Toggle */}
          <div className="flex items-start justify-between p-4 bg-pierre-gray-50 rounded-lg">
            <div className="flex-1 mr-4">
              <h3 className="font-medium text-pierre-gray-900">Auto-Approve Registrations</h3>
              <p className="text-sm text-pierre-gray-600 mt-1">
                When enabled, new user registrations are automatically approved without requiring admin review.
                This is useful for public platforms but may pose security risks.
              </p>
              {autoApprovalData?.description && (
                <p className="text-xs text-pierre-gray-400 mt-2">
                  {autoApprovalData.description}
                </p>
              )}
            </div>
            <div className="flex-shrink-0">
              {isLoading ? (
                <div className="w-11 h-6 bg-pierre-gray-200 rounded-full animate-pulse" />
              ) : error ? (
                <span className="text-xs text-pierre-red-500">Error loading</span>
              ) : (
                <button
                  onClick={handleToggleAutoApproval}
                  disabled={updateAutoApprovalMutation.isPending}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-offset-2 ${
                    autoApprovalData?.enabled
                      ? 'bg-pierre-activity'
                      : 'bg-pierre-gray-300'
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
                ? 'bg-pierre-activity-light/20 text-pierre-activity'
                : 'bg-pierre-gray-100 text-pierre-gray-600'
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
            <div className="p-3 rounded-lg bg-pierre-activity-light/30 text-pierre-activity text-sm">
              Setting updated successfully.
            </div>
          )}
          {updateAutoApprovalMutation.isError && (
            <div className="p-3 rounded-lg bg-pierre-red-50 text-pierre-red-600 text-sm">
              Failed to update setting. Please try again.
            </div>
          )}
        </div>
      </Card>

      {/* System Information */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">System Information</h2>

        <div className="space-y-3">
          <div className="flex justify-between items-center py-2 border-b border-pierre-gray-100">
            <span className="text-pierre-gray-600">Application</span>
            <span className="text-pierre-gray-900">Pierre Fitness Intelligence</span>
          </div>
          <div className="flex justify-between items-center py-2 border-b border-pierre-gray-100">
            <span className="text-pierre-gray-600">Version</span>
            <span className="text-pierre-gray-900">0.2.0</span>
          </div>
          <div className="flex justify-between items-center py-2">
            <span className="text-pierre-gray-600">Environment</span>
            <span className="px-2 py-1 bg-pierre-activity-light/20 text-pierre-activity rounded-full text-xs font-medium">
              {import.meta.env.MODE}
            </span>
          </div>
        </div>
      </Card>

      {/* Security Recommendations */}
      <Card className="border-pierre-nutrition-light">
        <div className="flex items-start gap-3">
          <div className="flex-shrink-0 p-2 bg-pierre-nutrition-light/20 rounded-lg">
            <svg className="w-5 h-5 text-pierre-nutrition" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <div>
            <h3 className="font-medium text-pierre-gray-900">Security Recommendations</h3>
            <ul className="mt-2 space-y-1 text-sm text-pierre-gray-600">
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
