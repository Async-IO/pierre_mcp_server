// ABOUTME: Vitest configuration for ui-logic package tests
// ABOUTME: Uses jsdom environment for React hook testing

import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['__tests__/**/*.test.ts'],
    environment: 'jsdom',
  },
});
