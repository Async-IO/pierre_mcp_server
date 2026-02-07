// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { apiService } from '../api/index'

// Mock the @pierre/api-client package and web adapter
vi.mock('@pierre/api-client', () => {
  const mockAxiosInstance = {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    interceptors: {
      request: { use: vi.fn() },
      response: { use: vi.fn() },
    },
  }
  return {
    createPierreApi: vi.fn(() => ({
      auth: {
        login: vi.fn(),
        loginWithFirebase: vi.fn(),
        logout: vi.fn(),
        register: vi.fn(),
        refreshToken: vi.fn().mockRejectedValue(new Error('No refresh token available')),
        getSession: vi.fn(),
      },
      chat: {
        getConversations: vi.fn(),
        createConversation: vi.fn(),
        getConversation: vi.fn(),
        updateConversation: vi.fn(),
        deleteConversation: vi.fn(),
        getConversationMessages: vi.fn(),
      },
      coaches: {
        list: vi.fn(),
        toggleFavorite: vi.fn(),
        recordUsage: vi.fn(),
        create: vi.fn(),
        update: vi.fn(),
        delete: vi.fn(),
        hide: vi.fn(),
        show: vi.fn(),
        getHidden: vi.fn(),
        fork: vi.fn(),
        getVersions: vi.fn(),
        getVersion: vi.fn(),
        revertToVersion: vi.fn(),
        getVersionDiff: vi.fn(),
        getPromptSuggestions: vi.fn(),
        generateFromConversation: vi.fn(),
      },
      oauth: {
        getStatus: vi.fn(),
        getAuthorizeUrl: vi.fn(),
      },
      social: {
        listFriends: vi.fn(),
        searchUsers: vi.fn(),
        getPendingRequests: vi.fn(),
        sendFriendRequest: vi.fn(),
        acceptFriendRequest: vi.fn(),
        declineFriendRequest: vi.fn(),
        removeFriend: vi.fn(),
        blockUser: vi.fn(),
        getFeed: vi.fn(),
        shareInsight: vi.fn(),
        deleteInsight: vi.fn(),
        addReaction: vi.fn(),
        removeReaction: vi.fn(),
        adaptInsight: vi.fn(),
        getAdaptedInsights: vi.fn(),
        getSettings: vi.fn(),
        updateSettings: vi.fn(),
      },
      store: {
        browse: vi.fn(),
        search: vi.fn(),
        get: vi.fn(),
        getCategories: vi.fn(),
        install: vi.fn(),
        uninstall: vi.fn(),
        getInstallations: vi.fn(),
      },
      user: {
        getStats: vi.fn(),
        updateProfile: vi.fn(),
        createMcpToken: vi.fn(),
        getMcpTokens: vi.fn(),
        revokeMcpToken: vi.fn(),
        getOAuthApps: vi.fn(),
        registerOAuthApp: vi.fn(),
        deleteOAuthApp: vi.fn(),
        getLlmSettings: vi.fn(),
        saveLlmCredentials: vi.fn(),
        validateLlmCredentials: vi.fn(),
        deleteLlmCredentials: vi.fn(),
      },
      axios: mockAxiosInstance,
      adapter: {
        authStorage: {
          setCsrfToken: vi.fn(),
          getCsrfToken: vi.fn(),
          setUser: vi.fn(),
          getUser: vi.fn(),
          clear: vi.fn(),
          getToken: vi.fn(),
          setToken: vi.fn(),
          removeToken: vi.fn(),
          getRefreshToken: vi.fn(),
          setRefreshToken: vi.fn(),
        },
        httpConfig: { baseURL: '' },
        authFailure: { onAuthFailure: vi.fn() },
      },
    })),
  }
})

vi.mock('@pierre/api-client/adapters/web', () => ({
  createWebAdapter: vi.fn(() => ({
    authStorage: {
      setCsrfToken: vi.fn(),
      getCsrfToken: vi.fn(),
      setUser: vi.fn(),
      getUser: vi.fn(),
      clear: vi.fn(),
      getToken: vi.fn(),
      setToken: vi.fn(),
      removeToken: vi.fn(),
      getRefreshToken: vi.fn(),
      setRefreshToken: vi.fn(),
    },
    httpConfig: { baseURL: '' },
    authFailure: { onAuthFailure: vi.fn() },
  })),
}))

// Mock axios for domain modules that still import it via client.ts
vi.mock('axios', () => {
  const mockAxiosInstance = {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    interceptors: {
      request: { use: vi.fn() },
      response: { use: vi.fn() },
    },
  }
  return {
    default: {
      ...mockAxiosInstance,
      create: vi.fn(() => mockAxiosInstance),
      defaults: {
        baseURL: '',
        withCredentials: true,
        headers: { common: {} },
      },
    },
  }
})

describe('API Service', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  describe('API Methods', () => {
    it('should have all required methods', () => {
      expect(typeof apiService.login).toBe('function')
      expect(typeof apiService.register).toBe('function')
      expect(typeof apiService.createApiKey).toBe('function')
      expect(typeof apiService.getApiKeys).toBe('function')
      expect(typeof apiService.deactivateApiKey).toBe('function')
      expect(typeof apiService.getDashboardOverview).toBe('function')
      expect(typeof apiService.getUsageAnalytics).toBe('function')
      expect(typeof apiService.getRateLimitOverview).toBe('function')
    })

    it('should have coach hide/show methods', () => {
      expect(typeof apiService.hideCoach).toBe('function')
      expect(typeof apiService.showCoach).toBe('function')
      expect(typeof apiService.getHiddenCoaches).toBe('function')
    })

    it('should have coach CRUD methods', () => {
      expect(typeof apiService.getCoaches).toBe('function')
      expect(typeof apiService.createCoach).toBe('function')
      expect(typeof apiService.updateCoach).toBe('function')
      expect(typeof apiService.deleteCoach).toBe('function')
      expect(typeof apiService.toggleCoachFavorite).toBe('function')
    })

    it('should have store API methods', () => {
      expect(typeof apiService.browseStoreCoaches).toBe('function')
      expect(typeof apiService.searchStoreCoaches).toBe('function')
      expect(typeof apiService.getStoreCoach).toBe('function')
      expect(typeof apiService.getStoreCategories).toBe('function')
      expect(typeof apiService.installStoreCoach).toBe('function')
      expect(typeof apiService.uninstallStoreCoach).toBe('function')
      expect(typeof apiService.getStoreInstallations).toBe('function')
    })
  })
})
