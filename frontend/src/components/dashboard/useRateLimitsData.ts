// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for fetching rate limit overview data
// ABOUTME: Owns rate-limits query for admin dashboard display

import { useQuery } from '@tanstack/react-query';
import { dashboardApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { RateLimitOverview } from '../../types/api';

export function useRateLimitsData(enabled: boolean) {
  const { data: rateLimits, isLoading } = useQuery<RateLimitOverview[]>({
    queryKey: QUERY_KEYS.dashboard.rateLimits(),
    queryFn: () => dashboardApi.getRateLimitOverview(),
    enabled,
  });

  return { rateLimits, isLoading };
}
