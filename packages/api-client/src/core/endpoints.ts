// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: API endpoint URL constants shared between platforms
// ABOUTME: Centralizes all endpoint definitions to avoid duplication

/**
 * API endpoint constants.
 * All endpoints are relative to the base URL configured in the platform adapter.
 */
export const ENDPOINTS = {
  // ==================== AUTH ====================
  AUTH: {
    /** OAuth token endpoint (login) */
    TOKEN: '/oauth/token',
    /** Firebase authentication */
    FIREBASE: '/api/auth/firebase',
    /** Logout */
    LOGOUT: '/api/auth/logout',
    /** User registration */
    REGISTER: '/api/auth/register',
    /** Token refresh */
    REFRESH: '/api/auth/refresh',
  },

  // ==================== CHAT ====================
  CHAT: {
    /** List/create conversations */
    CONVERSATIONS: '/api/chat/conversations',
    /** Get/update/delete a conversation */
    CONVERSATION: (id: string) => `/api/chat/conversations/${id}`,
    /** Get/send messages in a conversation */
    MESSAGES: (id: string) => `/api/chat/conversations/${id}/messages`,
  },

  // ==================== COACHES ====================
  COACHES: {
    /** List/create coaches */
    LIST: '/api/coaches',
    /** Get/update/delete a coach */
    COACH: (id: string) => `/api/coaches/${id}`,
    /** Toggle favorite status */
    FAVORITE: (id: string) => `/api/coaches/${id}/favorite`,
    /** Record coach usage */
    USAGE: (id: string) => `/api/coaches/${id}/usage`,
    /** Record coach use (alias) */
    USE: (id: string) => `/api/coaches/${id}/use`,
    /** Hide/show a coach */
    HIDE: (id: string) => `/api/coaches/${id}/hide`,
    /** List hidden coaches */
    HIDDEN: '/api/coaches/hidden',
    /** Fork a coach */
    FORK: (id: string) => `/api/coaches/${id}/fork`,
    /** List versions */
    VERSIONS: (id: string) => `/api/coaches/${id}/versions`,
    /** Get specific version */
    VERSION: (id: string, version: number) => `/api/coaches/${id}/versions/${version}`,
    /** Revert to version */
    VERSION_REVERT: (id: string, version: number) =>
      `/api/coaches/${id}/versions/${version}/revert`,
    /** Diff between versions */
    VERSION_DIFF: (id: string, fromVersion: number, toVersion: number) =>
      `/api/coaches/${id}/versions/${fromVersion}/diff/${toVersion}`,
    /** Coach assignments */
    ASSIGNMENTS: (id: string) => `/api/coaches/${id}/assignments`,
    /** Generate coach from conversation */
    GENERATE: '/api/coaches/generate',
  },

  // ==================== PROMPTS ====================
  PROMPTS: {
    /** Get prompt suggestions */
    SUGGESTIONS: '/api/prompts/suggestions',
  },

  // ==================== OAUTH ====================
  OAUTH: {
    /** Get OAuth connection status */
    STATUS: '/api/oauth/status',
    /** Get OAuth authorize URL (web) */
    AUTHORIZE: (provider: string) => `/api/oauth/${provider}/authorize`,
    /** Initialize mobile OAuth flow */
    MOBILE_INIT: (provider: string) => `/api/oauth/mobile/init/${provider}`,
    /** Disconnect provider (revoke tokens) */
    DISCONNECT: (provider: string) => `/api/oauth/providers/${provider}/disconnect`,
  },

  // ==================== PROVIDERS ====================
  PROVIDERS: {
    /** Get all providers with connection status (OAuth and non-OAuth) */
    STATUS: '/api/providers',
  },

  // ==================== SOCIAL ====================
  SOCIAL: {
    /** List friends */
    FRIENDS: '/api/social/friends',
    /** Pending friend requests (received) */
    FRIENDS_PENDING: '/api/social/friends/pending',
    /** Friend requests (sent/received) */
    FRIENDS_REQUESTS: '/api/social/friends/requests',
    /** Specific friend request */
    FRIEND_REQUEST: (id: string) => `/api/social/friends/requests/${id}`,
    /** Accept friend request */
    FRIEND_REQUEST_ACCEPT: (id: string) => `/api/social/friends/requests/${id}/accept`,
    /** Reject/decline friend request */
    FRIEND_REQUEST_REJECT: (id: string) => `/api/social/friends/requests/${id}/reject`,
    /** Specific friend (for removal) */
    FRIEND: (id: string) => `/api/social/friends/${id}`,
    /** Block a user */
    FRIEND_BLOCK: (id: string) => `/api/social/friends/${id}/block`,
    /** Search users */
    USER_SEARCH: '/api/social/users/search',
    /** Social feed */
    FEED: '/api/social/feed',
    /** Share an insight */
    SHARE: '/api/social/share',
    /** Specific shared insight */
    SHARED_INSIGHT: (id: string) => `/api/social/share/${id}`,
    /** List my insights */
    INSIGHTS: '/api/social/insights',
    /** Specific insight */
    INSIGHT: (id: string) => `/api/social/insights/${id}`,
    /** Reactions on an insight */
    INSIGHT_REACTIONS: (id: string) => `/api/social/insights/${id}/reactions`,
    /** Adapt an insight */
    INSIGHT_ADAPT: (id: string) => `/api/social/insights/${id}/adapt`,
    /** Get adapted insight */
    ADAPT: (id: string) => `/api/social/adapt/${id}`,
    /** List adapted insights */
    ADAPTED: '/api/social/adapted',
    /** Social settings */
    SETTINGS: '/api/social/settings',
    /** Insight suggestions (coach-generated) */
    SUGGESTIONS: '/api/social/insights/suggestions',
    /** Share insight from activity */
    FROM_ACTIVITY: '/api/social/insights/from-activity',
  },

  // ==================== STORE ====================
  STORE: {
    /** Browse/list store coaches */
    COACHES: '/api/store/coaches',
    /** Get specific store coach */
    COACH: (id: string) => `/api/store/coaches/${id}`,
    /** Search store coaches */
    SEARCH: '/api/store/search',
    /** Get store categories */
    CATEGORIES: '/api/store/categories',
    /** Install/uninstall a coach */
    INSTALL: (id: string) => `/api/store/coaches/${id}/install`,
    /** List installed coaches */
    INSTALLATIONS: '/api/store/installations',
  },

  // ==================== USER ====================
  USER: {
    /** User profile */
    PROFILE: '/api/user/profile',
    /** User stats */
    STATS: '/api/user/stats',
    /** MCP tokens */
    MCP_TOKENS: '/api/user/mcp-tokens',
    /** Specific MCP token */
    MCP_TOKEN: (id: string) => `/api/user/mcp-tokens/${id}`,
    /** Change password */
    CHANGE_PASSWORD: '/api/user/change-password',
    /** LLM settings */
    LLM_SETTINGS: '/api/user/llm-settings',
    /** Validate LLM settings */
    LLM_SETTINGS_VALIDATE: '/api/user/llm-settings/validate',
    /** Provider-specific LLM settings */
    LLM_SETTINGS_PROVIDER: (provider: string) => `/api/user/llm-settings/${provider}`,
    /** User OAuth apps */
    OAUTH_APPS: '/api/users/oauth-apps',
    /** Specific OAuth app */
    OAUTH_APP: (provider: string) => `/api/users/oauth-apps/${provider}`,
  },
} as const;

/** Type for endpoint keys */
export type EndpointKeys = typeof ENDPOINTS;
