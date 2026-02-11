// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for fetching dashboard overview data
// ABOUTME: Owns dashboard-overview query with WebSocket-triggered refetch

import { useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { dashboardApi } from '../../services/api';
import { useWebSocketContext } from '../../hooks/useWebSocketContext';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { DashboardOverview } from '../../types/api';

export function useOverviewData(enabled: boolean) {
  const { lastMessage } = useWebSocketContext();

  const { data: overview, isLoading, refetch } = useQuery<DashboardOverview>({
    queryKey: QUERY_KEYS.dashboard.overview(),
    queryFn: () => dashboardApi.getDashboardOverview(),
    enabled,
  });

  // Refresh data when WebSocket updates are received
  useEffect(() => {
    if (lastMessage && enabled) {
      if (lastMessage.type === 'usage_update' || lastMessage.type === 'system_stats') {
        refetch();
      }
    }
  }, [lastMessage, refetch, enabled]);

  return { overview, isLoading };
}
