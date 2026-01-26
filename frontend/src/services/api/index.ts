// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: API service entry point - uses @pierre/api-client for shared modules
// ABOUTME: Web-only modules (admin, keys, dashboard, a2a) remain local

import { apiClient, pierreApi } from './client';
import { keysApi } from './keys';
import { dashboardApi } from './dashboard';
import { a2aApi } from './a2a';
import { adminApi } from './admin';
import { socialApi as localSocialApi } from './social';

// Export the shared Pierre API instance for direct access
export { pierreApi } from './client';

// Export individual API modules - shared modules from @pierre/api-client
export const authApi = pierreApi.auth;
export const chatApi = pierreApi.chat;
export const coachesApi = pierreApi.coaches;
export const oauthApi = pierreApi.oauth;
export const socialApi = pierreApi.social;
export const storeApi = pierreApi.store;
export const userApi = pierreApi.user;

// Export web-only modules from local implementations
export { apiClient } from './client';
export { keysApi } from './keys';
export { dashboardApi } from './dashboard';
export { a2aApi } from './a2a';
export { adminApi } from './admin';

// Export types from shared package
export type { Coach, StoreCoach } from '@pierre/shared-types';

/**
 * Unified API service that combines all domain-specific APIs.
 *
 * This provides backward compatibility with existing code that imports apiService.
 * New code should prefer importing specific domain APIs directly:
 *
 * @example
 * // Preferred - import specific domain API
 * import { authApi } from '@/services/api';
 * await authApi.login(email, password);
 *
 * // Also works - use unified service (backward compatible)
 * import { apiService } from '@/services/api';
 * await apiService.login(email, password);
 */
export const apiService = {
  // Client utilities (legacy)
  getCsrfToken: () => apiClient.getCsrfToken(),
  setCsrfToken: (token: string) => apiClient.setCsrfToken(token),
  clearCsrfToken: () => apiClient.clearCsrfToken(),
  getUser: () => apiClient.getUser(),
  setUser: (user: { id: string; email: string; display_name?: string }) => apiClient.setUser(user),
  clearUser: () => apiClient.clearUser(),

  // Auth (from @pierre/api-client)
  login: pierreApi.auth.login.bind(pierreApi.auth),
  loginWithFirebase: pierreApi.auth.loginWithFirebase.bind(pierreApi.auth),
  logout: pierreApi.auth.logout.bind(pierreApi.auth),
  register: pierreApi.auth.register.bind(pierreApi.auth),
  refreshToken: pierreApi.auth.refreshToken.bind(pierreApi.auth),

  // API Keys (web-only)
  createApiKey: keysApi.createApiKey,
  createTrialKey: keysApi.createTrialKey,
  getApiKeys: keysApi.getApiKeys,
  deactivateApiKey: keysApi.deactivateApiKey,
  getApiKeyUsage: keysApi.getApiKeyUsage,

  // Dashboard (web-only)
  getDashboardOverview: dashboardApi.getDashboardOverview,
  getUsageAnalytics: dashboardApi.getUsageAnalytics,
  getRateLimitOverview: dashboardApi.getRateLimitOverview,
  getRequestLogs: dashboardApi.getRequestLogs,
  getRequestStats: dashboardApi.getRequestStats,
  getToolUsageBreakdown: dashboardApi.getToolUsageBreakdown,

  // Chat (from @pierre/api-client)
  getConversations: pierreApi.chat.getConversations.bind(pierreApi.chat),
  createConversation: pierreApi.chat.createConversation.bind(pierreApi.chat),
  getConversation: pierreApi.chat.getConversation.bind(pierreApi.chat),
  updateConversation: pierreApi.chat.updateConversation.bind(pierreApi.chat),
  deleteConversation: pierreApi.chat.deleteConversation.bind(pierreApi.chat),
  getConversationMessages: pierreApi.chat.getConversationMessages.bind(pierreApi.chat),

  // User (from @pierre/api-client)
  updateProfile: pierreApi.user.updateProfile.bind(pierreApi.user),
  getUserStats: pierreApi.user.getStats.bind(pierreApi.user),
  createMcpToken: pierreApi.user.createMcpToken.bind(pierreApi.user),
  getMcpTokens: pierreApi.user.getMcpTokens.bind(pierreApi.user),
  revokeMcpToken: pierreApi.user.revokeMcpToken.bind(pierreApi.user),
  getUserOAuthApps: pierreApi.user.getOAuthApps.bind(pierreApi.user),
  registerUserOAuthApp: pierreApi.user.registerOAuthApp.bind(pierreApi.user),
  deleteUserOAuthApp: pierreApi.user.deleteOAuthApp.bind(pierreApi.user),
  getLlmSettings: async () => {
    const response = await pierreApi.user.getLlmSettings();
    // Transform to expected frontend format
    const PROVIDER_NAMES: Record<string, string> = {
      gemini: 'Google Gemini',
      groq: 'Groq (Llama/Mixtral)',
      local: 'Local LLM (Ollama/vLLM)',
    };
    const providers = response.settings.map((s: { provider: string; api_key?: string; model?: string; enabled: boolean }) => ({
      provider: s.provider,
      name: s.provider,
      display_name: PROVIDER_NAMES[s.provider] || s.provider,
      has_api_key: !!s.api_key,
      has_credentials: !!s.api_key,
      credential_source: s.api_key ? 'user_specific' : undefined,
      default_model: s.model,
      enabled: s.enabled,
    }));
    const enabledProvider = response.settings.find((s: { enabled: boolean }) => s.enabled);
    return {
      providers,
      current_provider: enabledProvider?.provider || null,
    };
  },
  saveLlmCredentials: async (data: { provider: string; api_key: string; base_url?: string; default_model?: string }) => {
    const { provider, ...settings } = data;
    const result = await pierreApi.user.updateLlmSettings(provider, settings);
    return { ...result, message: 'Credentials saved successfully' };
  },
  validateLlmCredentials: async (data: { provider: string; api_key: string; base_url?: string }) => {
    const result = await pierreApi.user.validateLlmSettings(data.provider, data.api_key);
    return result;
  },
  deleteLlmCredentials: async (provider: string) => {
    await pierreApi.user.updateLlmSettings(provider, { enabled: false });
    return { message: 'Credentials deleted successfully' };
  },

  // Coaches (from @pierre/api-client)
  getCoaches: pierreApi.coaches.list.bind(pierreApi.coaches),
  toggleCoachFavorite: pierreApi.coaches.toggleFavorite.bind(pierreApi.coaches),
  recordCoachUsage: pierreApi.coaches.recordUsage.bind(pierreApi.coaches),
  createCoach: pierreApi.coaches.create.bind(pierreApi.coaches),
  updateCoach: pierreApi.coaches.update.bind(pierreApi.coaches),
  deleteCoach: pierreApi.coaches.delete.bind(pierreApi.coaches),
  hideCoach: pierreApi.coaches.hide.bind(pierreApi.coaches),
  showCoach: pierreApi.coaches.show.bind(pierreApi.coaches),
  getHiddenCoaches: pierreApi.coaches.getHidden.bind(pierreApi.coaches),
  getCoachVersions: pierreApi.coaches.getVersions.bind(pierreApi.coaches),
  getCoachVersion: pierreApi.coaches.getVersion.bind(pierreApi.coaches),
  revertCoachToVersion: pierreApi.coaches.revertToVersion.bind(pierreApi.coaches),
  getCoachVersionDiff: pierreApi.coaches.getVersionDiff.bind(pierreApi.coaches),
  getPromptSuggestions: pierreApi.coaches.getPromptSuggestions.bind(pierreApi.coaches),

  // OAuth (from @pierre/api-client)
  getOAuthStatus: pierreApi.oauth.getStatus.bind(pierreApi.oauth),
  getOAuthAuthorizeUrl: pierreApi.oauth.getAuthorizeUrl.bind(pierreApi.oauth),

  // A2A (web-only)
  registerA2AClient: a2aApi.registerA2AClient,
  getA2AClients: a2aApi.getA2AClients,
  getA2AClient: a2aApi.getA2AClient,
  updateA2AClient: a2aApi.updateA2AClient,
  deactivateA2AClient: a2aApi.deactivateA2AClient,
  getA2AClientUsage: a2aApi.getA2AClientUsage,
  getA2AClientRateLimit: a2aApi.getA2AClientRateLimit,
  getA2ASessions: a2aApi.getA2ASessions,
  getA2ADashboardOverview: a2aApi.getA2ADashboardOverview,
  getA2AUsageAnalytics: a2aApi.getA2AUsageAnalytics,
  getA2AAgentCard: a2aApi.getA2AAgentCard,
  getA2ARequestLogs: a2aApi.getA2ARequestLogs,

  // Admin (web-only)
  getSetupStatus: adminApi.getSetupStatus,
  getAdminTokens: adminApi.getAdminTokens,
  createAdminToken: adminApi.createAdminToken,
  getAdminTokenDetails: adminApi.getAdminTokenDetails,
  revokeAdminToken: adminApi.revokeAdminToken,
  rotateAdminToken: adminApi.rotateAdminToken,
  getAdminTokenInfo: adminApi.getAdminTokenInfo,
  getAdminHealth: adminApi.getAdminHealth,
  getAdminTokenAudit: adminApi.getAdminTokenAudit,
  getAdminTokenUsageStats: adminApi.getAdminTokenUsageStats,
  getAdminTokenProvisionedKeys: adminApi.getAdminTokenProvisionedKeys,
  provisionApiKey: adminApi.provisionApiKey,
  revokeApiKey: adminApi.revokeApiKey,
  listApiKeys: adminApi.listApiKeys,
  getTierDefaults: adminApi.getTierDefaults,
  getPendingUsers: adminApi.getPendingUsers,
  approveUser: adminApi.approveUser,
  suspendUser: adminApi.suspendUser,
  getAllUsers: adminApi.getAllUsers,
  resetUserPassword: adminApi.resetUserPassword,
  getUserRateLimit: adminApi.getUserRateLimit,
  getUserActivity: adminApi.getUserActivity,
  getAutoApprovalSetting: adminApi.getAutoApprovalSetting,
  updateAutoApprovalSetting: adminApi.updateAutoApprovalSetting,
  getConfigCatalog: adminApi.getConfigCatalog,
  getConfigAuditLog: adminApi.getConfigAuditLog,
  updateConfig: adminApi.updateConfig,
  resetConfig: adminApi.resetConfig,
  startImpersonation: adminApi.startImpersonation,
  endImpersonation: adminApi.endImpersonation,
  getImpersonationSessions: adminApi.getImpersonationSessions,
  getSystemCoaches: adminApi.getSystemCoaches,
  createSystemCoach: adminApi.createSystemCoach,
  getSystemCoach: adminApi.getSystemCoach,
  updateSystemCoach: adminApi.updateSystemCoach,
  deleteSystemCoach: adminApi.deleteSystemCoach,
  assignCoachToUsers: adminApi.assignCoachToUsers,
  unassignCoachFromUsers: adminApi.unassignCoachFromUsers,
  getCoachAssignments: adminApi.getCoachAssignments,
  getToolCatalog: adminApi.getToolCatalog,
  getToolCatalogEntry: adminApi.getToolCatalogEntry,
  getGlobalDisabledTools: adminApi.getGlobalDisabledTools,
  getTenantTools: adminApi.getTenantTools,
  setToolOverride: adminApi.setToolOverride,
  removeToolOverride: adminApi.removeToolOverride,
  getToolAvailabilitySummary: adminApi.getToolAvailabilitySummary,
  getStoreReviewQueue: adminApi.getStoreReviewQueue,
  getPublishedStoreCoaches: adminApi.getPublishedStoreCoaches,
  getRejectedStoreCoaches: adminApi.getRejectedStoreCoaches,
  getStoreStats: adminApi.getStoreStats,
  approveStoreCoach: adminApi.approveStoreCoach,
  rejectStoreCoach: adminApi.rejectStoreCoach,
  unpublishStoreCoach: adminApi.unpublishStoreCoach,
  getSocialInsightsConfig: adminApi.getSocialInsightsConfig,
  updateSocialInsightsConfig: adminApi.updateSocialInsightsConfig,
  resetSocialInsightsConfig: adminApi.resetSocialInsightsConfig,

  // Store (from @pierre/api-client)
  browseStoreCoaches: pierreApi.store.browse.bind(pierreApi.store),
  searchStoreCoaches: pierreApi.store.search.bind(pierreApi.store),
  getStoreCoach: pierreApi.store.get.bind(pierreApi.store),
  getStoreCategories: pierreApi.store.getCategories.bind(pierreApi.store),
  installStoreCoach: pierreApi.store.install.bind(pierreApi.store),
  uninstallStoreCoach: pierreApi.store.uninstall.bind(pierreApi.store),
  getStoreInstallations: pierreApi.store.getInstallations.bind(pierreApi.store),

  // Social (from @pierre/api-client)
  listFriends: pierreApi.social.listFriends.bind(pierreApi.social),
  searchUsers: pierreApi.social.searchUsers.bind(pierreApi.social),
  getPendingFriendRequests: pierreApi.social.getPendingRequests.bind(pierreApi.social),
  sendFriendRequest: pierreApi.social.sendFriendRequest.bind(pierreApi.social),
  acceptFriendRequest: pierreApi.social.acceptFriendRequest.bind(pierreApi.social),
  rejectFriendRequest: pierreApi.social.declineFriendRequest.bind(pierreApi.social),
  removeFriend: pierreApi.social.removeFriend.bind(pierreApi.social),
  blockUser: pierreApi.social.blockUser.bind(pierreApi.social),
  getSocialFeed: pierreApi.social.getFeed.bind(pierreApi.social),
  shareInsight: pierreApi.social.shareInsight.bind(pierreApi.social),
  deleteSharedInsight: pierreApi.social.deleteInsight.bind(pierreApi.social),
  addReaction: pierreApi.social.addReaction.bind(pierreApi.social),
  removeReaction: pierreApi.social.removeReaction.bind(pierreApi.social),
  adaptInsight: pierreApi.social.adaptInsight.bind(pierreApi.social),
  getAdaptedInsights: pierreApi.social.getAdaptedInsights.bind(pierreApi.social),
  getSocialSettings: pierreApi.social.getSettings.bind(pierreApi.social),
  updateSocialSettings: pierreApi.social.updateSettings.bind(pierreApi.social),
  // Web-only social methods (coach-generated insights)
  getInsightSuggestions: localSocialApi.getInsightSuggestions,
  shareFromActivity: localSocialApi.shareFromActivity,
};
