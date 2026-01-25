// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach, vi } from 'vitest'
import axios from 'axios'
import { apiService } from '../api/index'

// Mock axios - must be complete for apiClient initialization
vi.mock('axios', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    interceptors: {
      request: { use: vi.fn() },
      response: { use: vi.fn() },
    },
    defaults: {
      baseURL: '',
      withCredentials: true,
      headers: {
        common: {},
      },
    },
  },
}))

describe('API Service', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  describe('CSRF Token Management', () => {
    it('should manage CSRF tokens in memory', () => {
      expect(apiService.getCsrfToken()).toBeNull()

      apiService.setCsrfToken('test-csrf-token')
      expect(apiService.getCsrfToken()).toBe('test-csrf-token')

      apiService.clearCsrfToken()
      expect(apiService.getCsrfToken()).toBeNull()
    })

    it('should manage user info in localStorage', () => {
      const testUser = { id: 'user-123', email: 'test@example.com', display_name: 'Test User' }

      expect(apiService.getUser()).toBeNull()

      apiService.setUser(testUser)
      expect(apiService.getUser()).toEqual(testUser)
      expect(localStorage.getItem('user')).toBe(JSON.stringify(testUser))

      apiService.clearUser()
      expect(apiService.getUser()).toBeNull()
      expect(localStorage.getItem('user')).toBeNull()
    })
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

  describe('Store API URL Patterns', () => {
    const mockResponse = { data: { message: 'success' } }

    beforeEach(() => {
      vi.clearAllMocks()
      vi.mocked(axios.post).mockResolvedValue(mockResponse)
      vi.mocked(axios.delete).mockResolvedValue(mockResponse)
      vi.mocked(axios.get).mockResolvedValue(mockResponse)
    })

    it('installStoreCoach should call correct URL with coach ID', async () => {
      const coachId = 'test-coach-123'
      await apiService.installStoreCoach(coachId)

      expect(axios.post).toHaveBeenCalledWith(`/api/store/coaches/${coachId}/install`)
      // Verify the URL includes the slash before coachId
      const calledUrl = vi.mocked(axios.post).mock.calls[0][0]
      expect(calledUrl).toContain('/coaches/')
      expect(calledUrl).not.toContain('/coachestest') // Would fail if slash missing
    })

    it('uninstallStoreCoach should call correct URL with coach ID', async () => {
      const coachId = 'test-coach-456'
      await apiService.uninstallStoreCoach(coachId)

      expect(axios.delete).toHaveBeenCalledWith(`/api/store/coaches/${coachId}/install`)
      // Verify the URL includes the slash before coachId
      const calledUrl = vi.mocked(axios.delete).mock.calls[0][0]
      expect(calledUrl).toContain('/coaches/')
      expect(calledUrl).not.toContain('/coachestest') // Would fail if slash missing
    })

    it('getStoreCoach should call correct URL with coach ID', async () => {
      const coachId = 'test-coach-789'
      await apiService.getStoreCoach(coachId)

      expect(axios.get).toHaveBeenCalledWith(`/api/store/coaches/${coachId}`)
    })

    it('browseStoreCoaches should call correct URL with query params', async () => {
      await apiService.browseStoreCoaches({ category: 'training', sort_by: 'popular' })

      // API builds query string directly into URL
      const calledUrl = vi.mocked(axios.get).mock.calls[0][0]
      expect(calledUrl).toContain('/api/store/coaches')
      expect(calledUrl).toContain('category=training')
      expect(calledUrl).toContain('sort_by=popular')
    })

    it('searchStoreCoaches should call correct URL with query', async () => {
      await apiService.searchStoreCoaches('marathon', 10)

      // API builds query string directly into URL
      const calledUrl = vi.mocked(axios.get).mock.calls[0][0]
      expect(calledUrl).toContain('/api/store/search')
      expect(calledUrl).toContain('q=marathon')
      expect(calledUrl).toContain('limit=10')
    })

    it('getStoreCategories should call correct URL', async () => {
      await apiService.getStoreCategories()

      expect(axios.get).toHaveBeenCalledWith('/api/store/categories')
    })

    it('getStoreInstallations should call correct URL', async () => {
      await apiService.getStoreInstallations()

      expect(axios.get).toHaveBeenCalledWith('/api/store/installations')
    })
  })
})