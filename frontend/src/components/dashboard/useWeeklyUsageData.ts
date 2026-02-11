// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for fetching weekly usage analytics data
// ABOUTME: Owns usage-analytics query for admin dashboard charts

import { useQuery } from '@tanstack/react-query';
import { dashboardApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { AnalyticsData } from '../../types/chart';

export function useWeeklyUsageData(enabled: boolean) {
  const { data: weeklyUsage, isLoading } = useQuery<AnalyticsData>({
    queryKey: QUERY_KEYS.dashboard.usageAnalytics(7),
    queryFn: () => dashboardApi.getUsageAnalytics(7),
    enabled,
  });

  return { weeklyUsage, isLoading };
}
