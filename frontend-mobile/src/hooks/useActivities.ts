// ABOUTME: React Query hooks for activities with offline caching
// ABOUTME: Implements stale-while-revalidate for instant UI with background refresh

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';
import { queryKeys, CACHE_TIMES } from '../utils/mmkvStorage';

// Activity types (aligned with backend models)
interface Activity {
  id: string;
  user_id: string;
  provider: string;
  activity_type: string;
  start_time: string;
  end_time?: string;
  duration_seconds?: number;
  distance_meters?: number;
  calories?: number;
  average_heart_rate?: number;
  max_heart_rate?: number;
  average_power?: number;
  normalized_power?: number;
  training_load?: number;
  intensity_factor?: number;
  raw_data?: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

interface TrainingLoadData {
  ctl: number; // Chronic Training Load (fitness)
  atl: number; // Acute Training Load (fatigue)
  tsb: number; // Training Stress Balance (form)
  ramp_rate: number;
  trend: 'increasing' | 'stable' | 'decreasing';
}

interface RecoveryScore {
  score: number; // 0-100
  recovery_status: 'poor' | 'fair' | 'good' | 'excellent';
  hrv?: number;
  resting_hr?: number;
  sleep_quality?: number;
  readiness?: number;
}

interface ActivitiesParams {
  days?: number;
  limit?: number;
}

// Placeholder API functions - to be replaced with actual API client calls
// These simulate the API structure for activities
async function fetchActivities(params?: ActivitiesParams): Promise<Activity[]> {
  // This would call the actual API endpoint
  // For now, return empty array as activities API is not yet implemented
  const response = await fetch(
    `/api/activities?days=${params?.days ?? 7}&limit=${params?.limit ?? 50}`
  );
  if (!response.ok) {
    throw new Error('Failed to fetch activities');
  }
  return response.json();
}

async function fetchTrainingLoad(): Promise<TrainingLoadData> {
  const response = await fetch('/api/training-load/current');
  if (!response.ok) {
    throw new Error('Failed to fetch training load');
  }
  return response.json();
}

async function fetchRecoveryScore(): Promise<RecoveryScore> {
  const response = await fetch('/api/recovery/current');
  if (!response.ok) {
    throw new Error('Failed to fetch recovery score');
  }
  return response.json();
}

/**
 * Hook for fetching activities with offline caching
 *
 * Implements stale-while-revalidate:
 * - Returns cached data immediately (if available)
 * - Refetches in background when data is stale (>5 minutes old)
 * - Caches data for 7 days for offline access
 *
 * @param params - Filter parameters for activities
 * @param params.days - Number of days to fetch (default: 7)
 * @param params.limit - Maximum activities to return (default: 50)
 */
export function useActivities(params?: ActivitiesParams) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.activities.list(params),
    queryFn: () => fetchActivities(params),
    staleTime: CACHE_TIMES.ACTIVITIES_STALE_TIME,
    gcTime: CACHE_TIMES.ACTIVITIES_GC_TIME,
    // Placeholder data while loading (shows empty state gracefully)
    placeholderData: (previousData) => previousData ?? [],
  });

  const prefetchNextPage = useCallback(
    async (nextParams: ActivitiesParams) => {
      await queryClient.prefetchQuery({
        queryKey: queryKeys.activities.list(nextParams),
        queryFn: () => fetchActivities(nextParams),
        staleTime: CACHE_TIMES.ACTIVITIES_STALE_TIME,
      });
    },
    [queryClient]
  );

  const invalidate = useCallback(async () => {
    await queryClient.invalidateQueries({
      queryKey: queryKeys.activities.all,
    });
  }, [queryClient]);

  return {
    activities: query.data ?? [],
    isLoading: query.isLoading,
    isRefetching: query.isRefetching,
    isFetching: query.isFetching,
    isStale: query.isStale,
    isError: query.isError,
    error: query.error,
    refetch: query.refetch,
    prefetchNextPage,
    invalidate,
    // Useful for showing "Last updated" UI
    dataUpdatedAt: query.dataUpdatedAt,
    // True when showing cached data while fetching fresh data
    isShowingCachedData: query.isStale && !query.isLoading && query.data,
  };
}

/**
 * Hook for fetching current training load metrics
 *
 * Training load includes:
 * - CTL (Chronic Training Load / Fitness)
 * - ATL (Acute Training Load / Fatigue)
 * - TSB (Training Stress Balance / Form)
 */
export function useTrainingLoad() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.trainingLoad.current(),
    queryFn: fetchTrainingLoad,
    staleTime: CACHE_TIMES.TRAINING_LOAD_STALE_TIME,
    gcTime: CACHE_TIMES.ACTIVITIES_GC_TIME,
  });

  const invalidate = useCallback(async () => {
    await queryClient.invalidateQueries({
      queryKey: queryKeys.trainingLoad.all,
    });
  }, [queryClient]);

  return {
    trainingLoad: query.data,
    isLoading: query.isLoading,
    isRefetching: query.isRefetching,
    isError: query.isError,
    error: query.error,
    refetch: query.refetch,
    invalidate,
    dataUpdatedAt: query.dataUpdatedAt,
    isShowingCachedData: query.isStale && !query.isLoading && query.data,
  };
}

/**
 * Hook for fetching current recovery score
 *
 * Recovery includes:
 * - Overall recovery score (0-100)
 * - HRV, resting HR (if available from provider)
 * - Sleep quality metrics
 * - Readiness assessment
 */
export function useRecoveryScore() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.recovery.current(),
    queryFn: fetchRecoveryScore,
    staleTime: CACHE_TIMES.RECOVERY_STALE_TIME,
    gcTime: CACHE_TIMES.ACTIVITIES_GC_TIME,
  });

  const invalidate = useCallback(async () => {
    await queryClient.invalidateQueries({
      queryKey: queryKeys.recovery.all,
    });
  }, [queryClient]);

  return {
    recovery: query.data,
    isLoading: query.isLoading,
    isRefetching: query.isRefetching,
    isError: query.isError,
    error: query.error,
    refetch: query.refetch,
    invalidate,
    dataUpdatedAt: query.dataUpdatedAt,
    isShowingCachedData: query.isStale && !query.isLoading && query.data,
  };
}

// Export types for consumers
export type { Activity, TrainingLoadData, RecoveryScore, ActivitiesParams };
