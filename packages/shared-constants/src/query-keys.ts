// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Centralized query key constants for React Query
// ABOUTME: Ensures type safety and consistency across all query/mutation operations

/**
 * Centralized query keys for React Query.
 *
 * Benefits:
 * - Type-safe query key management
 * - Easy refactoring - change keys in one place
 * - Prevents typos and inconsistencies
 * - Better IDE autocomplete support
 *
 * Usage:
 * ```typescript
 * import { QUERY_KEYS } from '@pierre/shared-constants';
 *
 * // In useQuery
 * useQuery({ queryKey: QUERY_KEYS.coaches.list() })
 *
 * // In invalidateQueries
 * queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.all })
 * ```
 */

export const QUERY_KEYS = {
  // ==================== AUTH & USER ====================
  user: {
    all: ['user'] as const,
    profile: () => ['user-profile'] as const,
    stats: () => ['userStats'] as const,
    oauthApps: () => ['user-oauth-apps'] as const,
    providerConnections: () => ['provider-connections'] as const,
  },

  // ==================== OAUTH ====================
  oauth: {
    all: ['oauth'] as const,
    status: () => ['oauth-status'] as const,
    connections: () => ['connections'] as const,
  },

  // ==================== PROVIDERS ====================
  providers: {
    all: ['providers'] as const,
    status: () => ['providers-status'] as const,
  },

  // ==================== CHAT ====================
  chat: {
    all: ['chat'] as const,
    conversations: () => ['chat-conversations'] as const,
    messages: (conversationId: string | null) => ['chat-messages', conversationId] as const,
  },

  // ==================== COACHES ====================
  coaches: {
    all: ['coaches'] as const,
    list: (category?: string, favoritesOnly?: boolean) =>
      ['user-coaches', category, favoritesOnly] as const,
    listWithHidden: () => ['user-coaches', 'include-hidden'] as const,
    hidden: () => ['hidden-coaches'] as const,
    versions: (coachId: string) => ['coach-versions', coachId] as const,
    versionDiff: (coachId: string, fromVersion?: number, toVersion?: number) =>
      ['coach-version-diff', coachId, fromVersion, toVersion] as const,
    assignments: (coachId?: string) => ['coach-assignments', coachId] as const,
  },

  // ==================== COACH STORE ====================
  store: {
    all: ['store'] as const,
    coaches: (category?: string, sort?: string) =>
      ['store-coaches', category, sort] as const,
    search: (query: string) => ['store-search', query] as const,
    coach: (coachId: string) => ['store-coach', coachId] as const,
    coachDetail: (coachId?: string) => ['store-coach', coachId] as const,
    installations: () => ['store-installations'] as const,
  },

  // ==================== ADMIN - STORE ====================
  adminStore: {
    all: ['admin-store'] as const,
    stats: () => ['admin-store-stats'] as const,
    reviewQueue: () => ['admin-store-review-queue'] as const,
    published: (sortBy?: string) => ['admin-store-published', sortBy] as const,
    rejected: () => ['admin-store-rejected'] as const,
  },

  // ==================== ADMIN - COACHES ====================
  adminCoaches: {
    all: ['admin-coaches'] as const,
    system: () => ['admin-system-coaches'] as const,
    allUsers: () => ['admin-all-users'] as const,
  },

  // ==================== ADMIN - USERS ====================
  adminUsers: {
    all: ['admin-users'] as const,
    list: () => ['all-users'] as const,
    pending: () => ['pending-users'] as const,
    rateLimit: (userId?: string) => ['user-rate-limit', userId] as const,
    activity: (userId?: string) => ['user-activity', userId] as const,
  },

  // ==================== ADMIN - TOKENS ====================
  adminTokens: {
    all: ['admin-tokens'] as const,
    list: (includeInactive?: boolean) => ['admin-tokens', includeInactive] as const,
    audit: (tokenId: string) => ['admin-token-audit', tokenId] as const,
    usageStats: (tokenId: string) => ['admin-token-usage-stats', tokenId] as const,
    provisionedKeys: (tokenId: string) => ['admin-token-provisioned-keys', tokenId] as const,
  },

  // ==================== ADMIN - CONFIG ====================
  adminConfig: {
    all: ['admin-config'] as const,
    catalog: () => ['admin-config-catalog'] as const,
    audit: () => ['admin-config-audit'] as const,
  },

  // ==================== ADMIN - SETTINGS ====================
  adminSettings: {
    all: ['admin-settings'] as const,
    autoApproval: () => ['auto-approval-setting'] as const,
    socialInsightsConfig: () => ['social-insights-config'] as const,
  },

  // ==================== ADMIN - TOOLS ====================
  adminTools: {
    all: ['admin-tools'] as const,
    globalDisabled: () => ['global-disabled-tools'] as const,
    tenant: (tenantId: string) => ['tenant-tools', tenantId] as const,
    summary: (tenantId: string) => ['tool-availability-summary', tenantId] as const,
  },

  // ==================== DASHBOARD ====================
  dashboard: {
    all: ['dashboard'] as const,
    overview: () => ['dashboard-overview'] as const,
    rateLimits: () => ['rate-limits'] as const,
    usageAnalytics: (days?: number) => ['usage-analytics', days] as const,
    toolUsage: (apiKeyId?: string, timeRange?: string) =>
      ['tool-usage-breakdown', apiKeyId, timeRange] as const,
    requestLogs: (apiKeyId?: string, filter?: unknown) =>
      ['request-logs', apiKeyId, filter] as const,
    requestStats: (apiKeyId?: string, timeRange?: string) =>
      ['request-stats', apiKeyId, timeRange] as const,
  },

  // ==================== A2A (Agent-to-Agent) ====================
  a2a: {
    all: ['a2a'] as const,
    dashboardOverview: () => ['a2a-dashboard-overview'] as const,
    agentCard: () => ['a2a-agent-card'] as const,
    clients: () => ['a2a-clients'] as const,
    clientUsage: (clientId?: string) => ['a2a-client-usage', clientId] as const,
    clientRateLimit: (clientId?: string) => ['a2a-client-rate-limit', clientId] as const,
  },

  // ==================== MCP TOKENS ====================
  mcpTokens: {
    all: ['mcp-tokens'] as const,
    list: () => ['mcp-tokens'] as const,
  },

  // ==================== LLM SETTINGS ====================
  llmSettings: {
    all: ['llm-settings'] as const,
    list: () => ['llm-settings'] as const,
  },

  // ==================== SOCIAL ====================
  social: {
    all: ['social'] as const,
    feed: () => ['social-feed'] as const,
    friends: () => ['social-friends'] as const,
    friendRequests: () => ['social-friend-requests'] as const,
    settings: () => ['social-settings'] as const,
  },

  // ==================== PROMPTS ====================
  prompts: {
    all: ['prompts'] as const,
    suggestions: () => ['prompt-suggestions'] as const,
  },
} as const;

/** Type helpers for query key inference */
export type QueryKeys = typeof QUERY_KEYS;
