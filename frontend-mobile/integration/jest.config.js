// ABOUTME: Jest configuration for mobile integration tests against real backend.
// ABOUTME: Configures sequential test execution, longer timeouts, and global setup/teardown.

const path = require('path');

/** @type {import('@jest/types').Config.InitialOptions} */
module.exports = {
  // Run from the frontend-mobile directory
  rootDir: '..',

  // Only run tests in the integration/specs folder
  testMatch: ['<rootDir>/integration/specs/**/*.test.js'],

  // Longer timeout for API calls to real server
  testTimeout: 30000,

  // Sequential execution - tests may share database state
  maxWorkers: 1,

  // Global setup/teardown for server health checks and user creation
  globalSetup: path.join(__dirname, 'helpers/global-setup.js'),
  globalTeardown: path.join(__dirname, 'helpers/global-teardown.js'),

  // Use Node.js environment (no React Native runtime needed)
  testEnvironment: 'node',

  // Verbose output for debugging
  verbose: true,

  // Clear mocks between tests
  clearMocks: true,

  // Collect coverage from the integration test helpers
  collectCoverageFrom: ['<rootDir>/integration/helpers/**/*.js'],

  // Custom reporters
  reporters: ['default'],

  // Setup files to run before tests
  setupFilesAfterEnv: [],

  // Module name mapping (if needed)
  moduleNameMapper: {},

  // Transform settings (none needed for plain JS)
  transform: {},

  // Don't transform node_modules
  transformIgnorePatterns: ['/node_modules/'],
};
