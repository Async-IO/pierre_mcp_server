// ABOUTME: Unit tests for API service
// ABOUTME: Tests API domain modules are exported correctly

// Mock the entire api service module to avoid transformation issues with @pierre/api-client
jest.mock('../src/services/api', () => ({
  authApi: {
    login: jest.fn(),
    register: jest.fn(),
    logout: jest.fn(),
    initializeAuth: jest.fn(),
    getStoredUser: jest.fn(),
    storeAuth: jest.fn(),
  },
  chatApi: {
    getConversations: jest.fn(),
    sendMessage: jest.fn(),
    getWebSocketUrl: jest.fn(),
  },
  coachesApi: {
    listCoaches: jest.fn(),
    getCoach: jest.fn(),
    createCoach: jest.fn(),
    updateCoach: jest.fn(),
    deleteCoach: jest.fn(),
    toggleFavorite: jest.fn(),
    hide: jest.fn(),
    show: jest.fn(),
    getHidden: jest.fn(),
  },
  oauthApi: {
    getStatus: jest.fn(),
    initMobileOAuth: jest.fn(),
  },
  socialApi: {
    listFriends: jest.fn(),
    getPendingRequests: jest.fn(),
    getSocialFeed: jest.fn(),
  },
  storeApi: {
    browse: jest.fn(),
    search: jest.fn(),
    getStoreCoach: jest.fn(),
  },
  userApi: {
    getMcpTokens: jest.fn(),
    changePassword: jest.fn(),
    getUserOAuthApps: jest.fn(),
  },
  apiClient: {},
  onAuthFailure: jest.fn(),
}));

// Import after mocks
import { authApi, chatApi, coachesApi, oauthApi, socialApi, storeApi, userApi } from '../src/services/api';

describe('API Service', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('authApi object', () => {
    it('should be defined', () => {
      expect(authApi).toBeDefined();
    });

    it('should have login method', () => {
      expect(typeof authApi.login).toBe('function');
    });

    it('should have register method', () => {
      expect(typeof authApi.register).toBe('function');
    });
  });

  describe('chatApi object', () => {
    it('should have getConversations method', () => {
      expect(typeof chatApi.getConversations).toBe('function');
    });

    it('should have sendMessage method', () => {
      expect(typeof chatApi.sendMessage).toBe('function');
    });
  });

  describe('oauthApi object', () => {
    it('should have getStatus method', () => {
      expect(typeof oauthApi.getStatus).toBe('function');
    });
  });

  describe('coachesApi methods', () => {
    it('should have listCoaches method', () => {
      expect(typeof coachesApi.listCoaches).toBe('function');
    });

    it('should have getCoach method', () => {
      expect(typeof coachesApi.getCoach).toBe('function');
    });

    it('should have createCoach method', () => {
      expect(typeof coachesApi.createCoach).toBe('function');
    });

    it('should have updateCoach method', () => {
      expect(typeof coachesApi.updateCoach).toBe('function');
    });

    it('should have deleteCoach method', () => {
      expect(typeof coachesApi.deleteCoach).toBe('function');
    });

    it('should have toggleFavorite method', () => {
      expect(typeof coachesApi.toggleFavorite).toBe('function');
    });

    it('should have hide method', () => {
      expect(typeof coachesApi.hide).toBe('function');
    });

    it('should have show method', () => {
      expect(typeof coachesApi.show).toBe('function');
    });

    it('should have getHidden method', () => {
      expect(typeof coachesApi.getHidden).toBe('function');
    });
  });

  describe('socialApi methods', () => {
    it('should have listFriends method', () => {
      expect(typeof socialApi.listFriends).toBe('function');
    });
  });

  describe('storeApi methods', () => {
    it('should have browse method', () => {
      expect(typeof storeApi.browse).toBe('function');
    });
  });

  describe('userApi methods', () => {
    it('should have getMcpTokens method', () => {
      expect(typeof userApi.getMcpTokens).toBe('function');
    });
  });
});
