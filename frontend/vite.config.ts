/// <reference types="vitest" />
import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const backendUrl = env.VITE_BACKEND_URL || 'http://localhost:8081'

  return {
    plugins: [react()],
    server: {
      proxy: {
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
