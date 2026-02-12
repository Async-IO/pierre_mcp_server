// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Overview panel for the admin dashboard with aggregated stats
// ABOUTME: Owns its own useQuery calls for dashboard overview, rate limits, and usage analytics

import { useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { dashboardApi, adminApi, a2aApi } from '../../services/api';
import { useWebSocketContext } from '../../hooks/useWebSocketContext';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { DashboardOverview, RateLimitOverview, User } from '../../types/api';
import type { AnalyticsData } from '../../types/chart';
import OverviewTab from '../OverviewTab';

interface OverviewPanelProps {
  onNavigate?: (tab: string) => void;
}

export default function OverviewPanel({ onNavigate }: OverviewPanelProps) {
  const { lastMessage } = useWebSocketContext();

  const { data: overview, isLoading: overviewLoading, refetch: refetchOverview } = useQuery<DashboardOverview>({
    queryKey: QUERY_KEYS.dashboard.overview(),
    queryFn: () => dashboardApi.getDashboardOverview(),
  });

  const { data: rateLimits } = useQuery<RateLimitOverview[]>({
    queryKey: QUERY_KEYS.dashboard.rateLimits(),
    queryFn: () => dashboardApi.getRateLimitOverview(),
  });

  const { data: weeklyUsage } = useQuery<AnalyticsData>({
    queryKey: QUERY_KEYS.dashboard.usageAnalytics(7),
    queryFn: () => dashboardApi.getUsageAnalytics(7),
  });

  const { data: a2aOverview } = useQuery({
    queryKey: QUERY_KEYS.a2a.dashboardOverview(),
    queryFn: () => a2aApi.getA2ADashboardOverview(),
  });

  const { data: pendingUsers = [] } = useQuery<User[]>({
    queryKey: QUERY_KEYS.adminUsers.pending(),
    queryFn: () => adminApi.getPendingUsers(),
    staleTime: 30_000,
    retry: false,
  });

  const { data: storeStats } = useQuery({
    queryKey: QUERY_KEYS.adminStore.stats(),
    queryFn: () => adminApi.getStoreStats(),
    staleTime: 30_000,
    retry: false,
  });

  // Refresh data when WebSocket updates are received
  useEffect(() => {
    if (lastMessage) {
      if (lastMessage.type === 'usage_update' || lastMessage.type === 'system_stats') {
        refetchOverview();
      }
    }
  }, [lastMessage, refetchOverview]);

  return (
    <OverviewTab
      overview={overview}
      overviewLoading={overviewLoading}
      rateLimits={rateLimits}
      weeklyUsage={weeklyUsage}
      a2aOverview={a2aOverview}
      pendingUsersCount={pendingUsers.length}
      pendingCoachReviews={storeStats?.pending_count ?? 0}
      onNavigate={onNavigate}
    />
  );
}
