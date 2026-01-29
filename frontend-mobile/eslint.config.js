// ABOUTME: ESLint configuration for Pierre Mobile app (React Native/Expo)
// ABOUTME: Uses shared @pierre/eslint-config for consistent standards

import js from '@eslint/js';
import tsParser from '@typescript-eslint/parser';
import tsPlugin from '@typescript-eslint/eslint-plugin';
import reactPlugin from 'eslint-plugin-react';
import reactHooksPlugin from 'eslint-plugin-react-hooks';
// Import shared ESLint rules via relative path (mobile is outside npm workspaces)
import {
  baseTypeScriptRules,
  baseReactRules,
  reactHooksRules,
  testFileRules,
} from '../packages/eslint-config-pierre/index.js';

// React Native / Browser globals
const rnGlobals = {
  console: 'readonly',
  process: 'readonly',
  __dirname: 'readonly',
  module: 'readonly',
  require: 'readonly',
  exports: 'readonly',
  setTimeout: 'readonly',
  clearTimeout: 'readonly',
  setInterval: 'readonly',
  clearInterval: 'readonly',
  fetch: 'readonly',
  FormData: 'readonly',
  URLSearchParams: 'readonly',
  URL: 'readonly',
  AbortController: 'readonly',
  Headers: 'readonly',
  Request: 'readonly',
  Response: 'readonly',
  WebSocket: 'readonly',
  Blob: 'readonly',
  File: 'readonly',
  FileReader: 'readonly',
  alert: 'readonly',
  requestAnimationFrame: 'readonly',
  cancelAnimationFrame: 'readonly',
};

// Jest globals for test files
const jestGlobals = {
  jest: 'readonly',
  describe: 'readonly',
  it: 'readonly',
  test: 'readonly',
  expect: 'readonly',
  beforeEach: 'readonly',
  afterEach: 'readonly',
  beforeAll: 'readonly',
  afterAll: 'readonly',
  global: 'readonly',
  Event: 'readonly',
  MessageEvent: 'readonly',
  CloseEvent: 'readonly',
};

export default [
  js.configs.recommended,
  // Main source files
  {
    files: ['**/*.{ts,tsx}'],
    ignores: ['**/__tests__/**', '**/*.test.{ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
      globals: rnGlobals,
    },
    plugins: {
      '@typescript-eslint': tsPlugin,
      'react': reactPlugin,
      'react-hooks': reactHooksPlugin,
    },
    rules: {
      // Shared rules from @pierre/eslint-config
      ...baseTypeScriptRules,
      ...baseReactRules,
      ...reactHooksRules,
      // Mobile-specific rules
      'no-console': 'off',
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
  },
  // Test files - more relaxed rules
  {
    files: ['**/__tests__/**/*.{ts,tsx}', '**/*.test.{ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
      globals: {
        ...rnGlobals,
        ...jestGlobals,
      },
    },
    plugins: {
      '@typescript-eslint': tsPlugin,
      'react': reactPlugin,
      'react-hooks': reactHooksPlugin,
    },
    rules: {
      // Shared test file rules from @pierre/eslint-config
      ...testFileRules,
      'react-hooks/rules-of-hooks': 'error',
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
  },
  // Global ignores
  {
    ignores: [
      'node_modules/',
      '.expo/',
      'app.config.js',
      'babel.config.js',
      'metro.config.js',
      'jest.config.js',
      'jest.setup.js',
      'react-native.config.js',
      'tailwind.config.js',
      '.detoxrc.js',
      'e2e/',
      'integration/',
      'eslint.config.js',
      'plugins/',
    ],
  },
];
