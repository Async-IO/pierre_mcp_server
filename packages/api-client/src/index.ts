// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Main entry point for @pierre/api-client shared package
// ABOUTME: Exports API factories, domain APIs, and platform adapters

// Re-export types
export type {
  PlatformAdapter,
  AuthStorage,
  AuthFailureHandler,
  HttpClientConfig,
  ApiClientOptions,
  ApiMetadata,
  CursorPaginatedResponse,
  OffsetPaginatedResponse,
} from './types/platform';

// Re-export core
export { createApiClient, createAxiosClient } from './core/client';
export type { ApiClient } from './core/client';
export { ENDPOINTS } from './core/endpoints';
export type { EndpointKeys } from './core/endpoints';

// Re-export domain API factories
export { createAuthApi } from './domains/auth';
export type { AuthApi, LoginCredentials, RegisterCredentials } from './domains/auth';

export { createChatApi } from './domains/chat';
export type {
  ChatApi,
  Conversation,
  Message,
  ConversationsResponse,
  MessagesResponse,
  SendMessageResponse,
  CreateConversationOptions,
} from './domains/chat';

export { createCoachesApi } from './domains/coaches';
export type {
  CoachesApi,
  Coach,
  ListCoachesOptions,
  PromptSuggestion,
  PromptSuggestionsResponse,
  ForkCoachResponse,
  GenerateCoachRequest,
  GenerateCoachResponse,
} from './domains/coaches';

export { createOAuthApi } from './domains/oauth';
export type {
  OAuthApi,
  OAuthProvider,
  OAuthStatusResponse,
  MobileOAuthInitResponse,
} from './domains/oauth';

export { createSocialApi } from './domains/social';
export type {
  SocialApi,
  FriendsResponse,
  FriendRequestsResponse,
  FeedResponse,
  InsightsResponse,
  ShareInsightRequest,
  UserSearchResponse,
} from './domains/social';

export { createStoreApi } from './domains/store';
export type { StoreApi, BrowseOptions } from './domains/store';

export { createUserApi } from './domains/user';
export type {
  UserApi,
  UserStats,
  McpToken,
  McpTokensResponse,
  CreateMcpTokenRequest,
  UserOAuthApp,
  LlmSettings,
} from './domains/user';

// Re-export platform adapters
export { createMobileAdapter } from './adapters/mobile';
export type { AsyncStorageLike, MobileAdapterOptions } from './adapters/mobile';
export { createWebAdapter } from './adapters/web';
export type { WebAdapterOptions } from './adapters/web';

// Import for unified API service
import type { AxiosInstance } from 'axios';
import type { PlatformAdapter } from './types/platform';
import { createAxiosClient } from './core/client';
import { createAuthApi } from './domains/auth';
import { createChatApi } from './domains/chat';
import { createCoachesApi } from './domains/coaches';
import { createOAuthApi } from './domains/oauth';
import { createSocialApi } from './domains/social';
import { createStoreApi } from './domains/store';
import { createUserApi } from './domains/user';

/**
 * Complete API service combining all domain APIs.
 * Provides a unified interface for all Pierre API operations.
 */
export interface PierreApiService {
  /** Authentication API */
  auth: ReturnType<typeof createAuthApi>;
  /** Chat API */
  chat: ReturnType<typeof createChatApi>;
  /** Coaches API */
  coaches: ReturnType<typeof createCoachesApi>;
  /** OAuth API */
  oauth: ReturnType<typeof createOAuthApi>;
  /** Social API */
  social: ReturnType<typeof createSocialApi>;
  /** Store API */
  store: ReturnType<typeof createStoreApi>;
  /** User API */
  user: ReturnType<typeof createUserApi>;
  /** Underlying axios instance for custom requests */
  axios: AxiosInstance;
  /** Platform adapter */
  adapter: PlatformAdapter;
}

/**
 * Creates a complete API service with all domain APIs.
 *
 * @example
 * // Web usage
 * import { createPierreApi } from '@pierre/api-client';
 * import { createWebAdapter } from '@pierre/api-client/adapters/web';
 *
 * const adapter = createWebAdapter();
 * const api = createPierreApi(adapter);
 *
 * // Use domain APIs
 * const coaches = await api.coaches.list();
 * const user = await api.auth.login({ email, password });
 *
 * @example
 * // Mobile usage
 * import { createPierreApi } from '@pierre/api-client';
 * import { createMobileAdapter } from '@pierre/api-client/adapters/mobile';
 * import AsyncStorage from '@react-native-async-storage/async-storage';
 *
 * const adapter = createMobileAdapter({ asyncStorage: AsyncStorage });
 * const api = createPierreApi(adapter);
 */
export function createPierreApi(adapter: PlatformAdapter): PierreApiService {
  const axios = createAxiosClient(adapter);
  const getBaseUrl = () => adapter.httpConfig.baseURL;

  return {
    auth: createAuthApi(axios, adapter.authStorage),
    chat: createChatApi(axios, getBaseUrl),
    coaches: createCoachesApi(axios),
    oauth: createOAuthApi(axios, getBaseUrl),
    social: createSocialApi(axios),
    store: createStoreApi(axios),
    user: createUserApi(axios),
    axios,
    adapter,
  };
}
