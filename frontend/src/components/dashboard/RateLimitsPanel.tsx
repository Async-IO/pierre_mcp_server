// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Rate limits panel displaying API usage against limits
// ABOUTME: Owns its own useQuery call for rate limit overview data

import { useQuery } from '@tanstack/react-query';
import { dashboardApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { RateLimitOverview } from '../../types/api';
import { Card, CircularProgress } from '../ui';
import { clsx } from 'clsx';

const tierConfig: Record<string, { color: string; bg: string; icon: string }> = {
  trial: { color: 'text-zinc-400', bg: 'bg-white/5', icon: 'T' },
  starter: { color: 'text-pierre-activity', bg: 'bg-pierre-activity/10', icon: 'S' },
  professional: { color: 'text-pierre-violet-light', bg: 'bg-pierre-violet/15', icon: 'P' },
  enterprise: { color: 'text-pierre-cyan', bg: 'bg-pierre-cyan/15', icon: 'E' },
};

export default function RateLimitsPanel() {
  const { data: rateLimits, isLoading } = useQuery<RateLimitOverview[]>({
    queryKey: QUERY_KEYS.dashboard.rateLimits(),
    queryFn: () => dashboardApi.getRateLimitOverview(),
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

  if (!rateLimits || rateLimits.length === 0) {
    return null;
  }

  const totalCapacity = rateLimits.reduce((sum, item) => sum + (item.limit || 0), 0);
  const totalUsed = rateLimits.reduce((sum, item) => sum + item.current_usage, 0);

  return (
    <Card variant="dark" className="!p-5">
      <div className="flex justify-between items-center mb-4">
        <div>
          <h3 className="text-base font-semibold text-white">Rate Limits</h3>
          <p className="text-xs text-zinc-500 mt-0.5">
            {totalCapacity > 0 ? `${Math.round((totalUsed / totalCapacity) * 100)}% of capacity used` : 'Monitoring usage'}
          </p>
        </div>
        {totalCapacity > 0 && (
          <CircularProgress value={totalUsed} max={totalCapacity} size="md" variant="gradient" />
        )}
      </div>
      <div className="space-y-3 max-h-[200px] overflow-y-auto pr-2 scrollbar-dark">
        {rateLimits.slice(0, 5).map((item: RateLimitOverview) => (
          <div key={item.api_key_id} className="flex items-center gap-3">
            <div className={clsx(
              'w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold',
              tierConfig[item.tier]?.bg || 'bg-white/5',
              tierConfig[item.tier]?.color || 'text-zinc-400'
            )}>
              {tierConfig[item.tier]?.icon || 'T'}
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-white truncate">{item.api_key_name}</p>
              <div className="flex items-center gap-2">
                <div className="flex-1 h-1.5 bg-white/10 rounded-full overflow-hidden">
                  <div
                    className={clsx(
                      'h-full rounded-full transition-all duration-300',
                      item.usage_percentage > 90 ? 'bg-pierre-red-500' :
                      item.usage_percentage > 70 ? 'bg-pierre-nutrition' : 'bg-pierre-activity'
                    )}
                    style={{ width: `${Math.min(item.usage_percentage, 100)}%` }}
                  />
                </div>
                <span className="text-xs text-zinc-500 w-12 text-right">
                  {item.limit ? `${Math.round(item.usage_percentage)}%` : '-'}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
