// ABOUTME: ESLint configuration preset for React web applications (Vite)
// ABOUTME: Extends base config with browser-specific settings

import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import { baseTypeScriptRules, baseReactRules, reactHooksRules } from './index.js';

/**
 * Creates an ESLint configuration for React web applications.
 *
 * @param {Object} options - Configuration options
 * @param {string[]} options.ignores - Additional patterns to ignore
 * @returns {Array} ESLint flat config array
 */
export function createWebConfig(options = {}) {
  const ignores = ['dist', 'coverage', ...(options.ignores || [])];

  return tseslint.config(
    { ignores },
    {
      extends: [js.configs.recommended, ...tseslint.configs.recommended],
      files: ['**/*.{ts,tsx}'],
      languageOptions: {
        ecmaVersion: 2020,
        globals: {
          window: 'readonly',
          document: 'readonly',
          navigator: 'readonly',
          console: 'readonly',
          fetch: 'readonly',
          URL: 'readonly',
          URLSearchParams: 'readonly',
          FormData: 'readonly',
          Headers: 'readonly',
          Request: 'readonly',
          Response: 'readonly',
          AbortController: 'readonly',
          setTimeout: 'readonly',
          clearTimeout: 'readonly',
          setInterval: 'readonly',
          clearInterval: 'readonly',
          requestAnimationFrame: 'readonly',
          cancelAnimationFrame: 'readonly',
          localStorage: 'readonly',
          sessionStorage: 'readonly',
          WebSocket: 'readonly',
        },
      },
      plugins: {
        'react-hooks': reactHooks,
        'react-refresh': reactRefresh,
      },
      rules: {
        ...baseTypeScriptRules,
        ...baseReactRules,
        ...reactHooksRules,
        'react-refresh/only-export-components': [
          'warn',
          { allowConstantExport: true },
        ],
      },
    },
  );
}

export default { createWebConfig };
