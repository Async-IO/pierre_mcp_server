// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach } from 'vitest'
import { apiService } from '../api'

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
  })
})