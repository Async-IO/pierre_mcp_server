// ABOUTME: Jest configuration for Pierre Mobile app testing
// ABOUTME: Uses jest-expo preset with React Native Testing Library

module.exports = {
  preset: 'jest-expo',
  setupFilesAfterEnv: [
    '@testing-library/jest-native/extend-expect',
    '<rootDir>/jest.setup.js',
  ],
  transformIgnorePatterns: [
    'node_modules/(?!((jest-)?react-native|@react-native(-community)?)|expo(nent)?|@expo(nent)?/.*|@expo-google-fonts/.*|react-navigation|@react-navigation/.*|@unimodules/.*|unimodules|sentry-expo|native-base|react-native-svg|@pierre/.*)',
  ],
  moduleFileExtensions: ['ts', 'tsx', 'js', 'jsx'],
  testMatch: ['**/__tests__/**/*.(ts|tsx|js)', '**/?(*.)+(spec|test).(ts|tsx|js)'],
  testPathIgnorePatterns: ['/node_modules/', '/e2e/', '/integration/'],
  collectCoverageFrom: [
    'src/**/*.{ts,tsx}',
    '!src/**/*.d.ts',
    '!src/**/index.ts',
  ],
  coverageThreshold: {
    global: {
      branches: 0,
      functions: 0,
      lines: 0,
      statements: 0,
    },
  },
  testEnvironment: 'node',
  moduleNameMapper: {
    '^@/(.*)$': '<rootDir>/src/$1',
    '^@pierre/api-client$': '<rootDir>/../packages/api-client/src/index.ts',
    '^@pierre/shared-types$': '<rootDir>/../packages/shared-types/src/index.ts',
    // Mock expo virtual modules for packages outside node_modules
    '^expo/virtual/(.*)$': '<rootDir>/jest.setup.js',
  },
};
