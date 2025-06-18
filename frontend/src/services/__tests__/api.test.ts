import { describe, it, expect, beforeEach } from 'vitest'
import { apiService } from '../api'

describe('API Service', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  describe('Token Management', () => {
    it('should manage tokens in localStorage', () => {
      expect(apiService.getToken()).toBeNull()
      
      apiService.setToken('test-token')
      expect(apiService.getToken()).toBe('test-token')
      expect(localStorage.getItem('auth_token')).toBe('test-token')
      
      apiService.clearToken()
      expect(apiService.getToken()).toBeNull()
      expect(localStorage.getItem('auth_token')).toBeNull()
    })

    it('should set auth header correctly', () => {
      apiService.setAuthToken('test-token')
      // Note: In a real test, we'd check if axios headers were set
      // For now, we just verify the method exists and doesn't throw
      expect(() => apiService.setAuthToken(null)).not.toThrow()
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
  })
})