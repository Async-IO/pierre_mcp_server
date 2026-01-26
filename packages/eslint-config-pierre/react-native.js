// ABOUTME: ESLint configuration preset for React Native/Expo applications
// ABOUTME: Extends base config with React Native-specific settings

import js from '@eslint/js';
import tsParser from '@typescript-eslint/parser';
import tsPlugin from '@typescript-eslint/eslint-plugin';
import reactPlugin from 'eslint-plugin-react';
import reactHooksPlugin from 'eslint-plugin-react-hooks';
import {
  baseTypeScriptRules,
  baseReactRules,
  reactHooksRules,
  testFileRules,
} from './index.js';

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

/**
 * Creates an ESLint configuration for React Native/Expo applications.
 *
 * @param {Object} options - Configuration options
 * @param {string[]} options.ignores - Additional patterns to ignore
 * @returns {Array} ESLint flat config array
 */
export function createMobileConfig(options = {}) {
  const defaultIgnores = [
    'node_modules/',
    '.expo/',
    'babel.config.js',
    'metro.config.js',
    'jest.config.js',
    'jest.setup.js',
    'tailwind.config.js',
    '.detoxrc.js',
    'e2e/',
    'integration/',
    'eslint.config.js',
    'plugins/',
  ];

  const ignores = [...defaultIgnores, ...(options.ignores || [])];

  return [
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
        ...baseTypeScriptRules,
        ...baseReactRules,
        ...reactHooksRules,
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
      ignores,
    },
  ];
}

export default { createMobileConfig };
