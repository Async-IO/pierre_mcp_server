// ABOUTME: Shared base ESLint configuration for Pierre frontend applications
// ABOUTME: Enforces consistent TypeScript and code quality standards

/**
 * Base TypeScript rules shared across all Pierre frontend applications.
 * These rules enforce strict type safety and consistent code quality.
 */
export const baseTypeScriptRules = {
  // CRITICAL: Enforce no explicit any - use unknown with type guards instead
  '@typescript-eslint/no-explicit-any': 'error',

  // Unused variables are errors, except those prefixed with _
  '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],

  // Allow inference for return types (explicit is optional)
  '@typescript-eslint/explicit-function-return-type': 'off',
  '@typescript-eslint/explicit-module-boundary-types': 'off',

  // Disable base rule in favor of TypeScript version
  'no-unused-vars': 'off',
};

/**
 * Base React rules shared across web and mobile applications.
 */
export const baseReactRules = {
  // React 17+ doesn't need React in scope
  'react/react-in-jsx-scope': 'off',

  // We use TypeScript for prop validation
  'react/prop-types': 'off',
};

/**
 * React Hooks rules - critical for correctness
 */
export const reactHooksRules = {
  'react-hooks/rules-of-hooks': 'error',
  'react-hooks/exhaustive-deps': 'warn',
};

/**
 * Relaxed rules for test files
 */
export const testFileRules = {
  '@typescript-eslint/no-unused-vars': 'off',
  '@typescript-eslint/no-explicit-any': 'off',
  'react-hooks/exhaustive-deps': 'off',
  'no-unused-vars': 'off',
};

export default {
  baseTypeScriptRules,
  baseReactRules,
  reactHooksRules,
  testFileRules,
};
