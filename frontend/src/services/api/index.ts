// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: API service entry point - re-exports all domain modules for backward compatibility
// ABOUTME: Maintains the apiService interface while using modular domain-specific implementations

import { apiClient } from './client';
import { authApi } from './auth';
import { keysApi } from './keys';
import { dashboardApi } from './dashboard';
import { chatApi } from './chat';
import { userApi } from './user';
import { coachesApi } from './coaches';
import { oauthApi } from './oauth';
import { a2aApi } from './a2a';
import { adminApi } from './admin';
import { storeApi } from './store';
import { socialApi } from './social';

// Export individual API modules for direct import
export { apiClient } from './client';
export { authApi } from './auth';
export { keysApi } from './keys';
export { dashboardApi } from './dashboard';
export { chatApi } from './chat';
export { userApi } from './user';
export { coachesApi } from './coaches';
export { oauthApi } from './oauth';
export { a2aApi } from './a2a';
export { adminApi } from './admin';
export { storeApi } from './store';
export { socialApi } from './social';

// Export types
export type { Coach } from './coaches';
export type { StoreCoach } from './store';

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
  // Client utilities
  getCsrfToken: () => apiClient.getCsrfToken(),
  setCsrfToken: (token: string) => apiClient.setCsrfToken(token),
  clearCsrfToken: () => apiClient.clearCsrfToken(),
  getUser: () => apiClient.getUser(),
  setUser: (user: { id: string; email: string; display_name?: string }) => apiClient.setUser(user),
  clearUser: () => apiClient.clearUser(),

  // Auth
  login: authApi.login,
  loginWithFirebase: authApi.loginWithFirebase,
  logout: authApi.logout,
  register: authApi.register,
  refreshToken: authApi.refreshToken,

  // API Keys
  createApiKey: keysApi.createApiKey,
  createTrialKey: keysApi.createTrialKey,
  getApiKeys: keysApi.getApiKeys,
  deactivateApiKey: keysApi.deactivateApiKey,
  getApiKeyUsage: keysApi.getApiKeyUsage,

  // Dashboard
  getDashboardOverview: dashboardApi.getDashboardOverview,
  getUsageAnalytics: dashboardApi.getUsageAnalytics,
  getRateLimitOverview: dashboardApi.getRateLimitOverview,
  getRequestLogs: dashboardApi.getRequestLogs,
  getRequestStats: dashboardApi.getRequestStats,
  getToolUsageBreakdown: dashboardApi.getToolUsageBreakdown,

  // Chat
  getConversations: chatApi.getConversations,
  createConversation: chatApi.createConversation,
  getConversation: chatApi.getConversation,
  updateConversation: chatApi.updateConversation,
  deleteConversation: chatApi.deleteConversation,
  getConversationMessages: chatApi.getConversationMessages,

  // User
  updateProfile: userApi.updateProfile,
  getUserStats: userApi.getUserStats,
  createMcpToken: userApi.createMcpToken,
  getMcpTokens: userApi.getMcpTokens,
  revokeMcpToken: userApi.revokeMcpToken,
  getUserOAuthApps: userApi.getUserOAuthApps,
  registerUserOAuthApp: userApi.registerUserOAuthApp,
  deleteUserOAuthApp: userApi.deleteUserOAuthApp,
  getLlmSettings: userApi.getLlmSettings,
  saveLlmCredentials: userApi.saveLlmCredentials,
  validateLlmCredentials: userApi.validateLlmCredentials,
  deleteLlmCredentials: userApi.deleteLlmCredentials,

  // Coaches
  getCoaches: coachesApi.getCoaches,
  toggleCoachFavorite: coachesApi.toggleCoachFavorite,
  recordCoachUsage: coachesApi.recordCoachUsage,
  createCoach: coachesApi.createCoach,
  updateCoach: coachesApi.updateCoach,
  deleteCoach: coachesApi.deleteCoach,
  hideCoach: coachesApi.hideCoach,
  showCoach: coachesApi.showCoach,
  getHiddenCoaches: coachesApi.getHiddenCoaches,
  getCoachVersions: coachesApi.getCoachVersions,
  getCoachVersion: coachesApi.getCoachVersion,
  revertCoachToVersion: coachesApi.revertCoachToVersion,
  getCoachVersionDiff: coachesApi.getCoachVersionDiff,
  getPromptSuggestions: coachesApi.getPromptSuggestions,

  // OAuth
  getOAuthStatus: oauthApi.getOAuthStatus,
  getOAuthAuthorizeUrl: oauthApi.getOAuthAuthorizeUrl,

  // A2A
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

  // Admin
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

  // Store
  browseStoreCoaches: storeApi.browseStoreCoaches,
  searchStoreCoaches: storeApi.searchStoreCoaches,
  getStoreCoach: storeApi.getStoreCoach,
  getStoreCategories: storeApi.getStoreCategories,
  installStoreCoach: storeApi.installStoreCoach,
  uninstallStoreCoach: storeApi.uninstallStoreCoach,
  getStoreInstallations: storeApi.getStoreInstallations,

  // Social
  listFriends: socialApi.listFriends,
  searchUsers: socialApi.searchUsers,
  getPendingFriendRequests: socialApi.getPendingFriendRequests,
  sendFriendRequest: socialApi.sendFriendRequest,
  acceptFriendRequest: socialApi.acceptFriendRequest,
  rejectFriendRequest: socialApi.rejectFriendRequest,
  removeFriend: socialApi.removeFriend,
  blockUser: socialApi.blockUser,
  getSocialFeed: socialApi.getSocialFeed,
  shareInsight: socialApi.shareInsight,
  deleteSharedInsight: socialApi.deleteSharedInsight,
  addReaction: socialApi.addReaction,
  removeReaction: socialApi.removeReaction,
  adaptInsight: socialApi.adaptInsight,
  getAdaptedInsights: socialApi.getAdaptedInsights,
  getInsightSuggestions: socialApi.getInsightSuggestions,
  shareFromActivity: socialApi.shareFromActivity,
  getSocialSettings: socialApi.getSocialSettings,
  updateSocialSettings: socialApi.updateSocialSettings,
};
