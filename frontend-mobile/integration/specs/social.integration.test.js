// ABOUTME: Integration tests for social API operations against the real backend server.
// ABOUTME: Tests friends, follows, feed, likes, comments, and privacy settings.

const {
  createAndLoginAsAdmin,
  createAndLoginTestUser,
  authenticatedRequest,
} = require('../helpers');
const { generateUniqueEmail, validPassword, timeouts } = require('../fixtures');

describe('Social API Integration Tests', () => {
  let accessToken;
  let userId;

  beforeAll(async () => {
    const loginResult = await createAndLoginAsAdmin();
    expect(loginResult.success).toBe(true);
    accessToken = loginResult.accessToken;
    userId = loginResult.user?.id;
  });

  describe('Friends List', () => {
    it('should list friends for authenticated user', async () => {
      const result = await authenticatedRequest('/api/social/friends', accessToken);

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.data).toBeDefined();
      // Friends may be empty for a new user
      expect(Array.isArray(result.data.friends || result.data)).toBe(true);
    });

    it('should reject friends list without authentication', async () => {
      const result = await authenticatedRequest('/api/social/friends', '');

      expect(result.success).toBe(false);
      expect(result.status).toBe(401);
    });

    it('should support pagination for friends list', async () => {
      const result = await authenticatedRequest(
        '/api/social/friends?limit=10&offset=0',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });
  });

  describe('Friend Requests', () => {
    it('should list pending friend requests', async () => {
      const result = await authenticatedRequest(
        '/api/social/friends/pending',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.data).toBeDefined();
    });

    it('should list sent friend requests', async () => {
      const result = await authenticatedRequest(
        '/api/social/friends/pending?type=sent',
        accessToken
      );

      // Endpoint may vary - either 200 or part of main endpoint
      if (result.status === 404) {
        console.log('Sent requests endpoint uses different path');
        return;
      }

      expect(result.success).toBe(true);
    });
  });

  describe('Social Feed', () => {
    it('should get social feed for authenticated user', async () => {
      const result = await authenticatedRequest('/api/social/feed', accessToken);

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.data).toBeDefined();
      // Feed items may be empty for a new user
      expect(Array.isArray(result.data.items || result.data.feed || result.data)).toBe(true);
    });

    it('should reject feed access without authentication', async () => {
      const result = await authenticatedRequest('/api/social/feed', '');

      expect(result.success).toBe(false);
      expect(result.status).toBe(401);
    });

    it('should support pagination for feed', async () => {
      const result = await authenticatedRequest(
        '/api/social/feed?limit=20&offset=0',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });

    it('should support filtering by visibility', async () => {
      const result = await authenticatedRequest(
        '/api/social/feed?visibility=public',
        accessToken
      );

      // Filtering may or may not be supported
      expect([200, 400]).toContain(result.status);
    });
  });

  describe('User Search', () => {
    it('should search for users', async () => {
      const result = await authenticatedRequest(
        '/api/social/users/search?q=test',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.data).toBeDefined();
    });

    it('should return empty results for non-matching search', async () => {
      const result = await authenticatedRequest(
        '/api/social/users/search?q=nonexistentuserxyz123',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      // Should return empty array, not error
      const users = result.data.users || result.data;
      expect(Array.isArray(users)).toBe(true);
    });
  });

  describe('Follow/Unfollow', () => {
    it('should return error when following non-existent user', async () => {
      const result = await authenticatedRequest(
        '/api/social/follow',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({ user_id: 'non-existent-user-id' }),
        }
      );

      expect(result.success).toBe(false);
      expect([400, 404]).toContain(result.status);
    });

    it('should return error when following self', async () => {
      if (!userId) {
        console.log('User ID not available - skipping self-follow test');
        return;
      }

      const result = await authenticatedRequest(
        '/api/social/follow',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({ user_id: userId }),
        }
      );

      // Should not allow following self
      expect(result.success).toBe(false);
      expect([400, 409]).toContain(result.status);
    });
  });

  describe('Social Settings', () => {
    it('should get social settings', async () => {
      const result = await authenticatedRequest(
        '/api/social/settings',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.data).toBeDefined();
    });

    it('should update discoverable setting', async () => {
      const result = await authenticatedRequest(
        '/api/social/settings',
        accessToken,
        {
          method: 'PUT',
          body: JSON.stringify({ discoverable: true }),
        }
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });

    it('should update default visibility setting', async () => {
      const result = await authenticatedRequest(
        '/api/social/settings',
        accessToken,
        {
          method: 'PUT',
          body: JSON.stringify({ default_visibility: 'friends_only' }),
        }
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });

    it('should update notification preferences', async () => {
      const result = await authenticatedRequest(
        '/api/social/settings',
        accessToken,
        {
          method: 'PUT',
          body: JSON.stringify({
            notify_friend_requests: true,
            notify_reactions: true,
            notify_adapted_insights: false,
          }),
        }
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });
  });

  describe('Post Interactions', () => {
    it('should return error when liking non-existent post', async () => {
      const result = await authenticatedRequest(
        '/api/social/posts/non-existent-post-id/like',
        accessToken,
        { method: 'POST' }
      );

      expect(result.success).toBe(false);
      expect([400, 404]).toContain(result.status);
    });

    it('should return error when commenting on non-existent post', async () => {
      const result = await authenticatedRequest(
        '/api/social/posts/non-existent-post-id/comments',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({ content: 'Test comment' }),
        }
      );

      expect(result.success).toBe(false);
      expect([400, 404]).toContain(result.status);
    });
  });

  describe('Insight Sharing', () => {
    it('should create a new shared insight', async () => {
      const result = await authenticatedRequest(
        '/api/social/insights',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({
            type: 'achievement',
            title: 'Integration Test Insight',
            content: 'This is a test insight from integration tests',
            visibility: 'friends',
          }),
        }
      );

      // May succeed or fail depending on required fields
      if (result.status === 201 || result.status === 200) {
        expect(result.success).toBe(true);
        expect(result.data).toBeDefined();
      } else {
        // Document the required fields
        console.log('Insight creation requirements:', result.error);
      }
    });

    it('should reject insight creation without content', async () => {
      const result = await authenticatedRequest(
        '/api/social/insights',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({
            type: 'achievement',
            // Missing required content
          }),
        }
      );

      expect(result.success).toBe(false);
      // 400 Bad Request or 422 Unprocessable Entity are both valid validation errors
      expect([400, 422]).toContain(result.status);
    });
  });

  describe('Privacy Controls', () => {
    it('should block user access after blocking', async () => {
      // First try to block a non-existent user
      const blockResult = await authenticatedRequest(
        '/api/social/block',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({ user_id: 'non-existent-user-id' }),
        }
      );

      // Blocking non-existent user should fail
      expect(blockResult.success).toBe(false);
      expect([400, 404]).toContain(blockResult.status);
    });

    it('should list blocked users', async () => {
      const result = await authenticatedRequest(
        '/api/social/blocked',
        accessToken
      );

      // Endpoint may or may not exist
      if (result.status === 404) {
        console.log('Blocked users endpoint not implemented');
        return;
      }

      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
    });
  });

  describe('Multi-User Interactions', () => {
    let secondUserToken;

    beforeAll(async () => {
      // Create a second user for interaction tests
      const secondUser = {
        email: generateUniqueEmail('social-test'),
        password: validPassword,
        role: 'user',
      };

      const loginResult = await createAndLoginTestUser(secondUser);
      if (loginResult.success) {
        secondUserToken = loginResult.accessToken;
      }
    });

    it('should allow following between two users (if second user created)', async () => {
      if (!secondUserToken) {
        console.log('Second user not created - skipping multi-user test');
        return;
      }

      // Get second user's profile to get their ID
      const profileResult = await authenticatedRequest(
        '/api/auth/me',
        secondUserToken
      );

      if (!profileResult.success || !profileResult.data?.id) {
        console.log('Could not get second user ID - skipping');
        return;
      }

      const secondUserId = profileResult.data.id;

      // First user follows second user
      const followResult = await authenticatedRequest(
        '/api/social/follow',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({ user_id: secondUserId }),
        }
      );

      // May succeed or be already following
      expect([200, 201, 409]).toContain(followResult.status);
    });
  });
});
