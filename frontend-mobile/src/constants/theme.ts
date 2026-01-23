// ABOUTME: Theme constants for Pierre Mobile app
// ABOUTME: Colors, spacing, and typography matching Pierre design system

export const colors = {
  // Pierre Design System - Primary brand colors
  pierre: {
    violet: '#7C3AED',
    cyan: '#06B6D4',
    // Three Pillars - Semantic accent colors (brightened for dark mode per Stitch)
    activity: '#10B981',   // Emerald - Movement, fitness, energy
    nutrition: '#FBBF24',  // Amber - Brightened for dark mode contrast
    recovery: '#818CF8',   // Indigo - Brightened for dark mode contrast
    // Dark theme
    dark: '#0F0F1A',
    slate: '#1E1E2E',
  },

  // Primary brand color (sky blue palette)
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

  // Dark theme backgrounds - Pierre Design System
  background: {
    primary: '#0F0F1A',    // pierre-dark - deepest background
    secondary: '#1E1E2E',  // pierre-slate - cards, elevated surfaces
    tertiary: '#2A2A3E',   // slightly lighter for hover states
    elevated: '#363650',   // elevated components like modals
  },

  // Text colors - using zinc palette for secondary/muted
  text: {
    primary: '#ffffff',
    secondary: '#a1a1aa',  // zinc-400
    tertiary: '#71717a',   // zinc-500
    accent: '#7C3AED',     // pierre-violet for accent
  },

  // Border colors - subtle white opacity borders
  border: {
    subtle: 'rgba(255, 255, 255, 0.05)',   // white/5
    default: 'rgba(255, 255, 255, 0.1)',   // white/10
    strong: 'rgba(255, 255, 255, 0.15)',   // white/15
  },

  // Semantic colors (mapped to Pierre Design System)
  success: '#22c55e',  // pierre-green-500
  warning: '#f59e0b',  // pierre-yellow-500 (nutrition)
  error: '#ef4444',    // pierre-red-500
  info: '#3b82f6',     // pierre-blue-500

  // Provider brand colors
  providers: {
    strava: '#FC4C02',
    garmin: '#007CC3',
    fitbit: '#00B0B9',
    whoop: '#00D46A',
    terra: '#6366F1',
  },

  // Google brand color (for OAuth button)
  google: '#4285F4',
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

// Glassmorphism card styles for premium look
export const glassCard = {
  // Base glass card style - use with StyleSheet
  background: 'rgba(124, 59, 237, 0.08)',  // Subtle violet tint
  borderColor: 'rgba(255, 255, 255, 0.1)',
  borderWidth: 1,
  // Shadow for depth (iOS)
  shadowColor: '#7C3AED',
  shadowOffset: { width: 0, height: 4 },
  shadowOpacity: 0.15,
  shadowRadius: 12,
  // Android elevation
  elevation: 8,
} as const;

// Glow effect for primary buttons
export const buttonGlow = {
  shadowColor: '#7C3AED',
  shadowOffset: { width: 0, height: 0 },
  shadowOpacity: 0.4,
  shadowRadius: 20,
  elevation: 12,
} as const;

// Gradient colors for premium effects
export const gradients = {
  violetIndigo: ['rgba(124, 59, 237, 0.15)', 'rgba(79, 70, 229, 0.05)'],
  violetCyan: ['#7C3AED', '#06B6D4'],
  darkOverlay: ['rgba(15, 15, 26, 0)', 'rgba(15, 15, 26, 0.8)'],
  aiGradient: ['rgba(124, 58, 237, 0.08)', 'rgba(30, 30, 46, 0.6)'],
} as const;

// AI Intelligence Glow Effects - Reinforces Pierre's AI-first brand identity
// Use with Animated API or react-native-reanimated for animated effects
export const aiGlow = {
  // Subtle ambient glow for AI elements
  ambient: {
    shadowColor: '#7C3AED',
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0.15,
    shadowRadius: 20,
    elevation: 6,
  },
  // Strong glow for prominent AI elements
  strong: {
    shadowColor: '#7C3AED',
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0.25,
    shadowRadius: 30,
    elevation: 10,
  },
  // Avatar/icon glow
  avatar: {
    shadowColor: '#7C3AED',
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0.4,
    shadowRadius: 20,
    elevation: 8,
  },
  // Thinking/processing state glow
  thinking: {
    shadowColor: '#7C3AED',
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0.5,
    shadowRadius: 25,
    elevation: 12,
  },
  // Response glow for new AI messages
  response: {
    shadowColor: '#7C3AED',
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0.3,
    shadowRadius: 15,
    elevation: 8,
  },
} as const;

// AI card style with gradient background and glow
export const aiCard = {
  backgroundColor: 'rgba(124, 58, 237, 0.08)',
  borderColor: 'rgba(124, 58, 237, 0.2)',
  borderWidth: 1,
  borderRadius: borderRadius.xl,
  ...aiGlow.ambient,
} as const;
