// ABOUTME: Banner component displayed when a super admin is impersonating another user
// ABOUTME: Shows target user info and provides button to end impersonation session
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useAuth } from '../hooks/useAuth';
import { Button } from './ui';

export default function ImpersonationBanner() {
  const { impersonation, endImpersonation } = useAuth();

  if (!impersonation.isImpersonating || !impersonation.targetUser) {
    return null;
  }

  const handleEndImpersonation = async () => {
    try {
      await endImpersonation();
    } catch (error) {
      console.error('Failed to end impersonation:', error);
    }
  };

  return (
    <div className="bg-pierre-yellow-500 text-white px-4 py-2 sticky top-0 z-50 shadow-lg">
      <div className="max-w-7xl mx-auto flex items-center justify-between">
        <div className="flex items-center gap-3">
          <svg
            className="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <span className="font-medium">
            You are impersonating{' '}
            <span className="font-bold">
              {impersonation.targetUser.display_name || impersonation.targetUser.email}
            </span>
            {impersonation.targetUser.display_name && (
              <span className="text-pierre-yellow-100 ml-1">
                ({impersonation.targetUser.email})
              </span>
            )}
          </span>
          <span className="text-pierre-yellow-200 text-sm">
            Role: {impersonation.targetUser.role}
          </span>
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={handleEndImpersonation}
          className="bg-white text-pierre-yellow-600 hover:bg-pierre-yellow-50 border-0"
        >
          End Impersonation
        </Button>
      </div>
    </div>
  );
}
