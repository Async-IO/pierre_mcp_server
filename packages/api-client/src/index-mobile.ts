// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: React Native entry point for @pierre/api-client package
// ABOUTME: Excludes web adapter to avoid import.meta compatibility issues with Hermes

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

// Re-export mobile adapter only (web adapter excluded for Hermes compatibility)
export { createMobileAdapter } from './adapters/mobile';
export type { AsyncStorageLike, MobileAdapterOptions } from './adapters/mobile';

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

/**
 * Legacy unified API service interface for backward compatibility.
 * Maps all domain methods to a flat object structure.
 */
export function createLegacyApiService(adapter: PlatformAdapter) {
  const api = createPierreApi(adapter);

  return {
    // Auth - with mobile-compatible positional argument wrappers
    login: (email: string, password: string) => api.auth.login({ email, password }),
    loginWithFirebase: (idToken: string) => api.auth.loginWithFirebase({ idToken }),
    logout: api.auth.logout.bind(api.auth),
    register: (email: string, password: string, displayName?: string) =>
      api.auth.register({ email, password, display_name: displayName }),
    refreshToken: api.auth.refreshToken.bind(api.auth),
    getStoredUser: api.auth.getStoredUser.bind(api.auth),
    storeAuth: api.auth.storeAuth.bind(api.auth),
    clearStoredAuth: api.auth.clearStoredAuth.bind(api.auth),
    initializeAuth: api.auth.initializeAuth.bind(api.auth),

    // Chat
    getConversations: api.chat.getConversations.bind(api.chat),
    createConversation: api.chat.createConversation.bind(api.chat),
    getConversation: api.chat.getConversation.bind(api.chat),
    updateConversation: api.chat.updateConversation.bind(api.chat),
    deleteConversation: api.chat.deleteConversation.bind(api.chat),
    getConversationMessages: api.chat.getConversationMessages.bind(api.chat),
    sendMessage: api.chat.sendMessage.bind(api.chat),
    getWebSocketUrl: api.chat.getWebSocketUrl.bind(api.chat),

    // Coaches
    listCoaches: api.coaches.list.bind(api.coaches),
    getCoaches: api.coaches.list.bind(api.coaches),
    getCoach: api.coaches.get.bind(api.coaches),
    createCoach: api.coaches.create.bind(api.coaches),
    updateCoach: api.coaches.update.bind(api.coaches),
    deleteCoach: api.coaches.delete.bind(api.coaches),
    toggleCoachFavorite: api.coaches.toggleFavorite.bind(api.coaches),
    recordCoachUsage: api.coaches.recordUsage.bind(api.coaches),
    hideCoach: api.coaches.hide.bind(api.coaches),
    showCoach: api.coaches.show.bind(api.coaches),
    listHiddenCoaches: api.coaches.getHidden.bind(api.coaches),
    getHiddenCoaches: api.coaches.getHidden.bind(api.coaches),
    forkCoach: api.coaches.fork.bind(api.coaches),
    getCoachVersions: api.coaches.getVersions.bind(api.coaches),
    getCoachVersion: api.coaches.getVersion.bind(api.coaches),
    revertCoachToVersion: api.coaches.revertToVersion.bind(api.coaches),
    getCoachVersionDiff: api.coaches.getVersionDiff.bind(api.coaches),
    getPromptSuggestions: api.coaches.getPromptSuggestions.bind(api.coaches),

    // OAuth / Providers
    getOAuthStatus: api.oauth.getStatus.bind(api.oauth),
    getProvidersStatus: api.oauth.getProvidersStatus.bind(api.oauth),
    getOAuthAuthorizeUrl: api.oauth.getAuthorizeUrl.bind(api.oauth),
    initMobileOAuth: api.oauth.initMobileOAuth.bind(api.oauth),

    // Social
    listFriends: api.social.listFriends.bind(api.social),
    getPendingRequests: api.social.getPendingRequests.bind(api.social),
    getPendingFriendRequests: api.social.getPendingRequests.bind(api.social),
    sendFriendRequest: api.social.sendFriendRequest.bind(api.social),
    acceptFriendRequest: api.social.acceptFriendRequest.bind(api.social),
    declineFriendRequest: api.social.declineFriendRequest.bind(api.social),
    rejectFriendRequest: api.social.declineFriendRequest.bind(api.social),
    removeFriend: api.social.removeFriend.bind(api.social),
    blockUser: api.social.blockUser.bind(api.social),
    searchUsers: api.social.searchUsers.bind(api.social),
    getSocialFeed: api.social.getFeed.bind(api.social),
    shareInsight: api.social.shareInsight.bind(api.social),
    listMyInsights: api.social.listMyInsights.bind(api.social),
    deleteInsight: api.social.deleteInsight.bind(api.social),
    deleteSharedInsight: api.social.deleteInsight.bind(api.social),
    addReaction: api.social.addReaction.bind(api.social),
    removeReaction: api.social.removeReaction.bind(api.social),
    adaptInsight: api.social.adaptInsight.bind(api.social),
    getAdaptedInsights: api.social.getAdaptedInsights.bind(api.social),
    getSocialSettings: api.social.getSettings.bind(api.social),
    updateSocialSettings: api.social.updateSettings.bind(api.social),
    getInsightSuggestions: api.social.getInsightSuggestions.bind(api.social),
    shareFromActivity: api.social.shareFromActivity.bind(api.social),

    // Store
    browseStoreCoaches: api.store.browse.bind(api.store),
    searchStoreCoaches: api.store.search.bind(api.store),
    getStoreCoach: api.store.get.bind(api.store),
    getStoreCategories: api.store.getCategories.bind(api.store),
    installStoreCoach: api.store.install.bind(api.store),
    uninstallStoreCoach: api.store.uninstall.bind(api.store),
    getStoreInstallations: api.store.getInstallations.bind(api.store),
    getInstalledCoaches: api.store.getInstallations.bind(api.store),

    // User
    getProfile: api.user.getProfile.bind(api.user),
    updateProfile: api.user.updateProfile.bind(api.user),
    getUserStats: api.user.getStats.bind(api.user),
    changePassword: api.user.changePassword.bind(api.user),
    getMcpTokens: api.user.getMcpTokens.bind(api.user),
    createMcpToken: api.user.createMcpToken.bind(api.user),
    revokeMcpToken: api.user.revokeMcpToken.bind(api.user),
    getUserOAuthApps: api.user.getOAuthApps.bind(api.user),
    registerUserOAuthApp: api.user.registerOAuthApp.bind(api.user),
    deleteUserOAuthApp: api.user.deleteOAuthApp.bind(api.user),
    getLlmSettings: api.user.getLlmSettings.bind(api.user),
    updateLlmSettings: api.user.updateLlmSettings.bind(api.user),
    validateLlmSettings: api.user.validateLlmSettings.bind(api.user),
  };
}
