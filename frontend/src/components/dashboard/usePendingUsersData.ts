// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for fetching pending users awaiting approval
// ABOUTME: Owns pending-users query with badge count for admin sidebar

import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { User } from '../../types/api';

export function usePendingUsersData(enabled: boolean) {
  const { data: pendingUsers = [], isLoading } = useQuery<User[]>({
    queryKey: QUERY_KEYS.adminUsers.pending(),
    queryFn: () => adminApi.getPendingUsers(),
    staleTime: 30_000,
    retry: false,
    enabled,
  });

  return { pendingUsers, isLoading };
}
