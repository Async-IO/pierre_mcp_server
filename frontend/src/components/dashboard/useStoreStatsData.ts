// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for fetching coach store statistics
// ABOUTME: Owns admin-store-stats query for pending review badge count

import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';

export interface StoreStats {
  pending_count: number;
  published_count: number;
  rejected_count: number;
}

export function useStoreStatsData(enabled: boolean) {
  const { data: storeStats, isLoading } = useQuery<StoreStats>({
    queryKey: QUERY_KEYS.adminStore.stats(),
    queryFn: () => adminApi.getStoreStats(),
    staleTime: 30_000,
    retry: false,
    enabled,
  });

  return { storeStats, isLoading };
}
