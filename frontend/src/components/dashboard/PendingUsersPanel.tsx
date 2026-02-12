// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Pending users panel showing users awaiting approval
// ABOUTME: Owns its own useQuery call for pending users list

import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { User } from '../../types/api';
import { Card } from '../ui';

interface PendingUsersPanelProps {
  onNavigate?: (tab: string) => void;
}

export default function PendingUsersPanel({ onNavigate }: PendingUsersPanelProps) {
  const { data: pendingUsers = [], isLoading } = useQuery<User[]>({
    queryKey: QUERY_KEYS.adminUsers.pending(),
    queryFn: () => adminApi.getPendingUsers(),
    staleTime: 30_000,
    retry: false,
  });

  if (isLoading) {
    return (
      <Card variant="dark" className="!p-4">
        <div className="flex justify-center py-4">
          <div className="pierre-spinner"></div>
        </div>
      </Card>
    );
  }

  if (pendingUsers.length === 0) {
    return null;
  }

  return (
    <button
      onClick={() => onNavigate?.('users')}
      className="w-full flex items-center justify-between p-3 rounded-lg bg-pierre-nutrition/15 border border-pierre-nutrition/30 hover:bg-pierre-nutrition/25 transition-colors"
    >
      <div className="flex items-center gap-2">
        <div className="w-2 h-2 rounded-full bg-pierre-nutrition animate-pulse" />
        <span className="text-sm font-medium text-white">
          {pendingUsers.length} user{pendingUsers.length !== 1 ? 's' : ''} awaiting approval
        </span>
      </div>
      <svg className="w-4 h-4 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
      </svg>
    </button>
  );
}
