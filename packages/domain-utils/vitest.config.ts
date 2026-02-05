// ABOUTME: Vitest configuration for domain-utils package tests
// ABOUTME: Tests formatting, OAuth detection, and category utilities

import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['__tests__/**/*.test.ts'],
    environment: 'node',
  },
});
