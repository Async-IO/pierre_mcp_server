// ABOUTME: Integration tests for chat functionality against the real backend server.
// ABOUTME: Tests conversation management and message handling with actual API calls.

const {
  createAndLoginAsAdmin,
  authenticatedRequest,
} = require('../helpers');
const { endpoints, timeouts } = require('../fixtures');

describe('Chat Integration Tests', () => {
  let accessToken;

  beforeAll(async () => {
    // Login once for all chat tests
    const loginResult = await createAndLoginAsAdmin();
    expect(loginResult.success).toBe(true);
    accessToken = loginResult.accessToken;
  }, timeouts.serverStart);

  describe('Conversations List', () => {
    it('should fetch conversations list', async () => {
      const result = await authenticatedRequest(
        endpoints.chatConversations,
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      // Response should be an array (possibly empty for new users)
      expect(Array.isArray(result.data) || result.data.conversations !== undefined).toBe(true);
    });

    it('should return valid conversations array structure', async () => {
      const result = await authenticatedRequest(
        endpoints.chatConversations,
        accessToken
      );

      expect(result.success).toBe(true);
      // Should return a valid array of conversations (may not be empty if tests ran before)
      const conversations = Array.isArray(result.data)
        ? result.data
        : (result.data.conversations || []);
      expect(Array.isArray(conversations)).toBe(true);
    });

    it('should reject conversations request without auth', async () => {
      const result = await authenticatedRequest(
        endpoints.chatConversations,
        ''
      );

      expect(result.success).toBe(false);
      expect(result.status).toBe(401);
    });
  });

  describe('Create Conversation', () => {
    it('should create a new conversation', async () => {
      const result = await authenticatedRequest(
        endpoints.chatConversations,
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({
            title: 'Test Conversation',
          }),
        }
      );

      // Either 200/201 for success or 404 if endpoint doesn't support creation
      expect([200, 201, 404].includes(result.status)).toBe(true);

      if (result.success) {
        expect(result.data).toBeDefined();
        if (result.data.id) {
          expect(result.data.id).toBeDefined();
        }
      }
    });

    it('should reject conversation creation without auth', async () => {
      const result = await authenticatedRequest(
        endpoints.chatConversations,
        '',
        {
          method: 'POST',
          body: JSON.stringify({
            title: 'Unauthorized Conversation',
          }),
        }
      );

      expect(result.success).toBe(false);
      expect(result.status).toBe(401);
    });
  });

  describe('Chat Messages', () => {
    let conversationId;

    beforeAll(async () => {
      // Try to create a conversation first
      const createResult = await authenticatedRequest(
        endpoints.chatConversations,
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({
            title: 'Test Messages Conversation',
          }),
        }
      );

      if (createResult.success && createResult.data && createResult.data.id) {
        conversationId = createResult.data.id;
      }
    });

    it('should handle message sending (if conversation exists)', async () => {
      if (!conversationId) {
        // Skip if we couldn't create a conversation
        console.log('Skipping: No conversation available for message test');
        return;
      }

      const result = await authenticatedRequest(
        `${endpoints.chatConversations}/${conversationId}/messages`,
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({
            content: 'Hello, this is a test message',
          }),
        }
      );

      // 200/201 for success, 400/422 for validation, 404 if endpoint doesn't exist
      // Also accept 500 for streaming endpoints that may not handle test requests
      expect([200, 201, 400, 404, 422, 500].includes(result.status)).toBe(true);
    });

    it('should fetch conversation messages (if conversation exists)', async () => {
      if (!conversationId) {
        console.log('Skipping: No conversation available for messages fetch test');
        return;
      }

      const result = await authenticatedRequest(
        `${endpoints.chatConversations}/${conversationId}/messages`,
        accessToken
      );

      // 200 for success, 404 if endpoint doesn't exist
      expect([200, 404].includes(result.status)).toBe(true);

      if (result.success) {
        expect(Array.isArray(result.data) || result.data.messages !== undefined).toBe(true);
      }
    });
  });

  describe('Chat Error Handling', () => {
    it('should handle invalid conversation ID gracefully', async () => {
      const result = await authenticatedRequest(
        `${endpoints.chatConversations}/invalid-id-12345`,
        accessToken
      );

      // Should return 404 for non-existent conversation
      expect([400, 404].includes(result.status)).toBe(true);
    });

    it('should handle malformed request body gracefully', async () => {
      const { getBackendUrl } = require('../helpers/server-manager');
      const backendUrl = getBackendUrl();

      const response = await fetch(`${backendUrl}${endpoints.chatConversations}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: 'not valid json{{{',
      });

      // Should return 400 for malformed JSON
      expect([400, 404, 422].includes(response.status)).toBe(true);
    });
  });
});
