// ABOUTME: Theme constants for Pierre Mobile app
// ABOUTME: Colors, spacing, and typography matching Pierre design system

export const colors = {
  // Primary brand color
  primary: {
    50: '#f0f9ff',
    100: '#e0f2fe',
    200: '#bae6fd',
    300: '#7dd3fc',
    400: '#38bdf8',
    500: '#0ea5e9',
    600: '#0284c7',
    700: '#0369a1',
    800: '#075985',
    900: '#0c4a6e',
    950: '#082f49',
  },

  // Dark theme backgrounds
  background: {
    primary: '#0f0f0f',
    secondary: '#1a1a1a',
    tertiary: '#2a2a2a',
    elevated: '#333333',
  },

  // Text colors
  text: {
    primary: '#ffffff',
    secondary: '#a1a1a1',
    tertiary: '#6b6b6b',
    accent: '#0ea5e9',
  },

  // Border colors
  border: {
    subtle: '#2a2a2a',
    default: '#3a3a3a',
    strong: '#4a4a4a',
  },

  // Semantic colors
  success: '#22c55e',
  warning: '#f59e0b',
  error: '#ef4444',
  info: '#3b82f6',

  // Provider brand colors
  providers: {
    strava: '#FC4C02',
    garmin: '#007CC3',
    fitbit: '#00B0B9',
    whoop: '#00D46A',
    terra: '#6366F1',
  },
} as const;

export const spacing = {
  xs: 4,
  sm: 8,
  md: 16,
  lg: 24,
  xl: 32,
  xxl: 48,
} as const;

export const borderRadius = {
  sm: 4,
  md: 8,
  lg: 12,
  xl: 16,
  full: 9999,
} as const;

export const fontSize = {
  xs: 12,
  sm: 14,
  md: 16,
  lg: 18,
  xl: 20,
  xxl: 24,
  xxxl: 32,
} as const;

export const fontWeight = {
  normal: '400' as const,
  medium: '500' as const,
  semibold: '600' as const,
  bold: '700' as const,
};
