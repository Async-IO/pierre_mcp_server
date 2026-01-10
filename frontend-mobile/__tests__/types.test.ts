// ABOUTME: Type validation tests for Pierre Mobile app
// ABOUTME: Ensures type definitions match expected shape

import type {
  User,
  Conversation,
  Message,
  ProviderStatus,
  McpToken,
  PromptCategory,
} from '../src/types';

describe('Type Definitions', () => {
  describe('User type', () => {
    it('should have required fields', () => {
      const user: User = {
        user_id: '123',
        email: 'test@example.com',
        is_admin: false,
        role: 'user',
        user_status: 'active',
      };
      expect(user.user_id).toBe('123');
      expect(user.email).toBe('test@example.com');
    });

    it('should accept optional display_name', () => {
      const user: User = {
        user_id: '123',
        email: 'test@example.com',
        display_name: 'Test User',
        is_admin: false,
        role: 'user',
        user_status: 'active',
      };
      expect(user.display_name).toBe('Test User');
    });
  });

  describe('Conversation type', () => {
    it('should have required fields', () => {
      const conversation: Conversation = {
        id: 'conv-123',
        title: 'Test Conversation',
        model: 'gpt-4',
        total_tokens: 100,
        message_count: 5,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      };
      expect(conversation.id).toBe('conv-123');
      expect(conversation.title).toBe('Test Conversation');
    });
  });

  describe('Message type', () => {
    it('should accept user role', () => {
      const message: Message = {
        id: 'msg-123',
        role: 'user',
        content: 'Hello',
        created_at: '2024-01-01T00:00:00Z',
      };
      expect(message.role).toBe('user');
    });

    it('should accept assistant role', () => {
      const message: Message = {
        id: 'msg-123',
        role: 'assistant',
        content: 'Hello',
        created_at: '2024-01-01T00:00:00Z',
      };
      expect(message.role).toBe('assistant');
    });
  });

  describe('ProviderStatus type', () => {
    it('should track connection status', () => {
      const status: ProviderStatus = {
        provider: 'strava',
        connected: true,
        last_sync: '2024-01-01T00:00:00Z',
      };
      expect(status.connected).toBe(true);
    });
  });

  describe('PromptCategory type', () => {
    it('should have category and prompts', () => {
      const category: PromptCategory = {
        category_key: 'training',
        category_title: 'Training',
        category_icon: '🏃',
        pillar: 'activity',
        prompts: ['What is my fitness level?'],
      };
      expect(category.prompts).toHaveLength(1);
    });
  });
});
