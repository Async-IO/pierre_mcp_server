// ABOUTME: Server management utilities for mobile integration tests.
// ABOUTME: Provides health check polling and backend server readiness verification.

const BACKEND_URL = process.env.BACKEND_URL || process.env.PIERRE_API_URL || 'http://localhost:8081';

/**
 * Wait for the backend server to be healthy and ready to accept requests.
 * Polls the /health endpoint until it returns a successful response.
 *
 * @param {string} url - Health check URL
 * @param {number} maxAttempts - Maximum number of retry attempts
 * @param {number} intervalMs - Milliseconds between retries
 * @returns {Promise<{healthy: boolean, status?: string, version?: string, error?: string}>}
 */
async function waitForBackendHealth(
  url = `${BACKEND_URL}/health`,
  maxAttempts = 60,
  intervalMs = 1000
) {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const response = await fetch(url, {
        method: 'GET',
        headers: { Accept: 'application/json' },
      });

      if (response.ok) {
        const data = await response.json();
        console.log(`[Server] Backend healthy after ${attempt} attempts`);
        return {
          healthy: true,
          status: data.status,
          version: data.version,
        };
      }
    } catch (error) {
      if (attempt === maxAttempts) {
        return {
          healthy: false,
          error: `Server health check failed after ${maxAttempts} attempts: ${error.message}`,
        };
      }
    }

    await sleep(intervalMs);
  }

  return {
    healthy: false,
    error: `Server health check timed out after ${maxAttempts} attempts`,
  };
}

/**
 * Check if the backend server is ready for integration tests.
 *
 * @returns {Promise<boolean>}
 */
async function isBackendReady() {
  const result = await waitForBackendHealth();
  return result.healthy;
}

/**
 * Get the backend API base URL.
 *
 * @returns {string}
 */
function getBackendUrl() {
  return BACKEND_URL;
}

/**
 * Sleep for a specified duration.
 *
 * @param {number} ms - Milliseconds to sleep
 * @returns {Promise<void>}
 */
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

module.exports = {
  waitForBackendHealth,
  isBackendReady,
  getBackendUrl,
  sleep,
};
