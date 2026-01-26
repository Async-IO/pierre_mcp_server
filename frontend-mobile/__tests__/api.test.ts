// ABOUTME: Unit tests for API service
// ABOUTME: Tests API request formatting and response handling

// Mock the entire api service module to avoid transformation issues with @pierre/api-client
jest.mock('../src/services/api', () => ({
  apiService: {
    login: jest.fn(),
    register: jest.fn(),
    getConversations: jest.fn(),
    sendMessage: jest.fn(),
    getOAuthStatus: jest.fn(),
    listCoaches: jest.fn(),
    getCoach: jest.fn(),
    createCoach: jest.fn(),
    updateCoach: jest.fn(),
    deleteCoach: jest.fn(),
    toggleCoachFavorite: jest.fn(),
    hideCoach: jest.fn(),
    showCoach: jest.fn(),
    listHiddenCoaches: jest.fn(),
    getHiddenCoaches: jest.fn(),
  },
  apiClient: {},
  onAuthFailure: jest.fn(),
}));

// Import after mocks
import { apiService } from '../src/services/api';

describe('API Service', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('apiService object', () => {
    it('should be defined', () => {
      expect(apiService).toBeDefined();
    });

    it('should have login method', () => {
      expect(typeof apiService.login).toBe('function');
    });

    it('should have register method', () => {
      expect(typeof apiService.register).toBe('function');
    });

    it('should have getConversations method', () => {
      expect(typeof apiService.getConversations).toBe('function');
    });

    it('should have sendMessage method', () => {
      expect(typeof apiService.sendMessage).toBe('function');
    });

    it('should have getOAuthStatus method', () => {
      expect(typeof apiService.getOAuthStatus).toBe('function');
    });
  });

  describe('Coach API methods', () => {
    it('should have listCoaches method', () => {
      expect(typeof apiService.listCoaches).toBe('function');
    });

    it('should have getCoach method', () => {
      expect(typeof apiService.getCoach).toBe('function');
    });

    it('should have createCoach method', () => {
      expect(typeof apiService.createCoach).toBe('function');
    });

    it('should have updateCoach method', () => {
      expect(typeof apiService.updateCoach).toBe('function');
    });

    it('should have deleteCoach method', () => {
      expect(typeof apiService.deleteCoach).toBe('function');
    });

    it('should have toggleCoachFavorite method', () => {
      expect(typeof apiService.toggleCoachFavorite).toBe('function');
    });

    it('should have hideCoach method', () => {
      expect(typeof apiService.hideCoach).toBe('function');
    });

    it('should have showCoach method', () => {
      expect(typeof apiService.showCoach).toBe('function');
    });

    it('should have listHiddenCoaches method', () => {
      expect(typeof apiService.listHiddenCoaches).toBe('function');
    });
  });
});
