// ABOUTME: MMKV storage adapter for React Query persistence
// ABOUTME: Provides fast synchronous storage for offline caching of activities and training data

import type { Persister, PersistedClient } from '@tanstack/react-query-persist-client';

// Dynamic import for MMKV to handle test environment
// In production, this will use the native MMKV module
// In tests, we'll mock this module
let queryCacheStorage: {
  getString: (key: string) => string | undefined;
  set: (key: string, value: string) => void;
  delete: (key: string) => void;
  clearAll: () => void;
};

try {
  const { MMKV } = require('react-native-mmkv');
  queryCacheStorage = new MMKV({
    id: 'pierre-query-cache',
  });
} catch {
  // Fallback for test environment - use in-memory storage
  const inMemoryStorage = new Map<string, string>();
  queryCacheStorage = {
    getString: (key: string) => inMemoryStorage.get(key),
    set: (key: string, value: string) => {
      inMemoryStorage.set(key, value);
    },
    delete: (key: string) => {
      inMemoryStorage.delete(key);
    },
    clearAll: () => {
      inMemoryStorage.clear();
    },
  };
}

// Storage key for persisted queries
const QUERY_CACHE_KEY = 'REACT_QUERY_CACHE';

/**
 * MMKV-based persister for React Query
 *
 * This persister stores the React Query cache in MMKV storage,
 * enabling offline access to cached data. MMKV is chosen over
 * AsyncStorage for:
 * - Synchronous reads (faster initial load)
 * - Better performance for larger payloads
 * - No JSON parsing overhead on hot path
 */
export const mmkvPersister: Persister = {
  persistClient: async (client: PersistedClient) => {
    const serialized = JSON.stringify(client);
    queryCacheStorage.set(QUERY_CACHE_KEY, serialized);
  },
  restoreClient: async (): Promise<PersistedClient | undefined> => {
    const cached = queryCacheStorage.getString(QUERY_CACHE_KEY);
    if (!cached) {
      return undefined;
    }
    try {
      return JSON.parse(cached) as PersistedClient;
    } catch {
      // Corrupted cache, clear and return undefined
      queryCacheStorage.delete(QUERY_CACHE_KEY);
      return undefined;
    }
  },
  removeClient: async () => {
    queryCacheStorage.delete(QUERY_CACHE_KEY);
  },
};

/**
 * Clear the query cache storage
 *
 * Call this on user logout to ensure no data leaks between users.
 */
export function clearQueryCache(): void {
  queryCacheStorage.delete(QUERY_CACHE_KEY);
}

/**
 * Clear all MMKV storage for this app
 *
 * Use this for debugging or complete data reset.
 */
export function clearAllStorage(): void {
  queryCacheStorage.clearAll();
}

// Cache configuration constants
export const CACHE_TIMES = {
  /** Activities: 7 days for offline access */
  ACTIVITIES_GC_TIME: 7 * 24 * 60 * 60 * 1000, // 7 days in ms
  /** Activities: Consider stale after 5 minutes */
  ACTIVITIES_STALE_TIME: 5 * 60 * 1000, // 5 minutes in ms
  /** Training load: Consider stale after 15 minutes */
  TRAINING_LOAD_STALE_TIME: 15 * 60 * 1000, // 15 minutes in ms
  /** Recovery scores: Consider stale after 30 minutes */
  RECOVERY_STALE_TIME: 30 * 60 * 1000, // 30 minutes in ms
  /** Default stale time for general queries */
  DEFAULT_STALE_TIME: 5 * 60 * 1000, // 5 minutes in ms
  /** Maximum cache age before garbage collection */
  MAX_CACHE_AGE: 7 * 24 * 60 * 60 * 1000, // 7 days in ms
} as const;

/**
 * Query key factory for consistent cache key generation
 */
export const queryKeys = {
  activities: {
    all: ['activities'] as const,
    list: (params?: { days?: number; limit?: number }) =>
      ['activities', 'list', params] as const,
    detail: (id: string) => ['activities', 'detail', id] as const,
  },
  trainingLoad: {
    all: ['training-load'] as const,
    current: () => ['training-load', 'current'] as const,
    history: (days?: number) => ['training-load', 'history', days] as const,
  },
  recovery: {
    all: ['recovery'] as const,
    current: () => ['recovery', 'current'] as const,
    scores: (days?: number) => ['recovery', 'scores', days] as const,
  },
} as const;
