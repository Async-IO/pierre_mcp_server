// ABOUTME: Vitest configuration for mcp-types package tests
// ABOUTME: Validates type definitions are properly structured

import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['__tests__/**/*.test.ts'],
    environment: 'node',
  },
});
