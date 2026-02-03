// ABOUTME: Vitest configuration for chat-utils package tests
// ABOUTME: Configures test environment and TypeScript paths

import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['__tests__/**/*.test.ts'],
    environment: 'node',
  },
  resolve: {
    alias: {
      '@pierre/shared-types': '../shared-types/src',
    },
  },
});
