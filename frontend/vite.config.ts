// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// <reference types="vitest" />
import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const backendUrl = env.VITE_BACKEND_URL || 'http://localhost:8081'

  // Disable proxy during E2E tests since Playwright mocks all API routes
  const isE2EMode = process.env.E2E_TEST === 'true'

  return {
    plugins: [react()],
    server: isE2EMode
      ? {}
      : {
          proxy: {
            // Proxy backend OAuth endpoints but NOT /oauth-callback (frontend route)
            '/oauth': {
              target: backendUrl,
              changeOrigin: true,
              bypass: (req) => {
                // Don't proxy /oauth-callback - it's a frontend route
                if (req.url?.startsWith('/oauth-callback')) {
                  return req.url;
                }
                return undefined;
              },
            },
            '/api': {
              target: backendUrl,
              changeOrigin: true,
            },
            '/admin': {
              target: backendUrl,
              changeOrigin: true,
            },
            '/a2a': {
              target: backendUrl,
              changeOrigin: true,
            },
            '/ws': {
              target: backendUrl,
              ws: true,
              changeOrigin: true,
            },
          },
        },
    test: {
      globals: true,
      environment: 'jsdom',
      setupFiles: './src/test/setup.ts',
      include: ['src/**/*.{test,spec}.{ts,tsx}'],
      exclude: ['node_modules', 'e2e', 'dist'],
      coverage: {
        provider: 'v8',
        reporter: ['text', 'json', 'html', 'lcov'],
        exclude: [
          'node_modules/',
          'src/test/',
          '**/*.test.{ts,tsx}',
          '**/*.config.{ts,js}',
          'dist/',
        ],
      },
      // CI-friendly configuration
      watch: false,
    },
  }
})
