// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for fetching A2A (Agent-to-Agent) dashboard overview
// ABOUTME: Owns a2a-dashboard-overview query for admin dashboard display

import { useQuery } from '@tanstack/react-query';
import { a2aApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { A2ADashboardOverview } from '../../types/api';

export function useA2ADashboardData(enabled: boolean) {
  const { data: a2aOverview, isLoading } = useQuery<A2ADashboardOverview>({
    queryKey: QUERY_KEYS.a2a.dashboardOverview(),
    queryFn: () => a2aApi.getA2ADashboardOverview(),
    enabled,
  });

  return { a2aOverview, isLoading };
}
