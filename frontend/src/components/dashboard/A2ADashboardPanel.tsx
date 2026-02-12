// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: A2A (Agent-to-Agent) dashboard panel with protocol stats
// ABOUTME: Owns its own useQuery call for A2A overview data

import { useQuery } from '@tanstack/react-query';
import { a2aApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import { Card } from '../ui';

export default function A2ADashboardPanel() {
  const { data: a2aOverview, isLoading } = useQuery({
    queryKey: QUERY_KEYS.a2a.dashboardOverview(),
    queryFn: () => a2aApi.getA2ADashboardOverview(),
  });

  if (isLoading) {
    return (
      <Card variant="dark" className="!p-5">
        <div className="flex justify-center py-4">
          <div className="pierre-spinner"></div>
        </div>
      </Card>
    );
  }

  if (!a2aOverview) {
    return null;
  }

  return (
    <Card variant="dark" className="!p-5">
      <h3 className="text-base font-semibold text-white mb-4">A2A Protocol</h3>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <p className="text-sm text-zinc-400">Total Clients</p>
          <p className="text-2xl font-bold text-white">{a2aOverview.total_clients}</p>
        </div>
        <div>
          <p className="text-sm text-zinc-400">Active Clients</p>
          <p className="text-2xl font-bold text-pierre-activity">{a2aOverview.active_clients}</p>
        </div>
        <div>
          <p className="text-sm text-zinc-400">Today's Requests</p>
          <p className="text-2xl font-bold text-pierre-nutrition">{a2aOverview.requests_today.toLocaleString()}</p>
        </div>
        <div>
          <p className="text-sm text-zinc-400">This Month</p>
          <p className="text-2xl font-bold text-pierre-recovery">{a2aOverview.requests_this_month.toLocaleString()}</p>
        </div>
      </div>
    </Card>
  );
}
