// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Store stats panel showing coach store pending reviews
// ABOUTME: Owns its own useQuery call for store statistics

import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import { Card } from '../ui';

interface StoreStatsPanelProps {
  onNavigate?: (tab: string) => void;
}

export default function StoreStatsPanel({ onNavigate }: StoreStatsPanelProps) {
  const { data: storeStats, isLoading } = useQuery({
    queryKey: QUERY_KEYS.adminStore.stats(),
    queryFn: () => adminApi.getStoreStats(),
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

  const pendingCount = storeStats?.pending_count ?? 0;

  if (pendingCount === 0) {
    return null;
  }

  return (
    <button
      onClick={() => onNavigate?.('coach-store')}
      className="w-full flex items-center justify-between p-3 rounded-lg bg-pierre-violet/15 border border-pierre-violet/30 hover:bg-pierre-violet/25 transition-colors"
    >
      <div className="flex items-center gap-2">
        <div className="w-2 h-2 rounded-full bg-pierre-violet animate-pulse" />
        <span className="text-sm font-medium text-white">
          {pendingCount} coach{pendingCount !== 1 ? 'es' : ''} pending review
        </span>
      </div>
      <svg className="w-4 h-4 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
      </svg>
    </button>
  );
}
