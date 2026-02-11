// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hooks for dashboard badge data (pending users, store stats)
// ABOUTME: Enables sidebar badges to share query data with panel components

import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { User } from '../../types/api';

/** Hook to get pending users count for badge display */
export function usePendingUsersCount(): number {
  const { data: pendingUsers = [] } = useQuery<User[]>({
    queryKey: QUERY_KEYS.adminUsers.pending(),
    queryFn: () => adminApi.getPendingUsers(),
    staleTime: 30_000,
    retry: false,
  });
  return pendingUsers.length;
}

/** Hook to get pending coach count for badge display */
export function useStoreStatsPendingCount(): number {
  const { data: storeStats } = useQuery({
    queryKey: QUERY_KEYS.adminStore.stats(),
    queryFn: () => adminApi.getStoreStats(),
    staleTime: 30_000,
    retry: false,
  });
  return storeStats?.pending_count ?? 0;
}
