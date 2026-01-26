// ABOUTME: Pierre Design System tokens shared across web and mobile
// ABOUTME: Brand colors, semantic colors, provider colors, and typography scales

// ========== PIERRE BRAND COLORS ==========

/** Pierre primary brand colors */
export const PIERRE_COLORS = {
  violet: '#7C3AED',
  cyan: '#06B6D4',
  dark: '#0F0F1A',
  slate: '#1E1E2E',
} as const;

/** Three pillars semantic accent colors (brightened for dark mode) */
export const PILLAR_COLORS = {
  activity: '#10B981',   // Emerald - Movement, fitness, energy
  nutrition: '#FBBF24',  // Amber - Brightened for dark mode contrast
  recovery: '#818CF8',   // Indigo - Brightened for dark mode contrast
} as const;

// ========== PRIMARY COLOR PALETTE ==========

/** Primary brand color palette (sky blue) */
export const PRIMARY_PALETTE = {
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
} as const;

// ========== DARK THEME BACKGROUNDS ==========

/** Dark theme background colors */
export const BACKGROUND_COLORS = {
  primary: '#0F0F1A',    // pierre-dark - deepest background
  secondary: '#1E1E2E',  // pierre-slate - cards, elevated surfaces
  tertiary: '#2A2A3E',   // slightly lighter for hover states
  elevated: '#363650',   // elevated components like modals
} as const;

// ========== TEXT COLORS ==========

/** Text colors for dark theme */
export const TEXT_COLORS = {
  primary: '#ffffff',
  secondary: '#a1a1aa',  // zinc-400
  tertiary: '#71717a',   // zinc-500
  accent: '#7C3AED',     // pierre-violet for accent
} as const;

// ========== BORDER COLORS ==========

/** Border colors with subtle white opacity */
export const BORDER_COLORS = {
  subtle: 'rgba(255, 255, 255, 0.05)',   // white/5
  default: 'rgba(255, 255, 255, 0.1)',   // white/10
  strong: 'rgba(255, 255, 255, 0.15)',   // white/15
} as const;

// ========== SEMANTIC COLORS ==========

/** Semantic colors for feedback states */
export const SEMANTIC_COLORS = {
  success: '#22c55e',  // pierre-green-500
  warning: '#f59e0b',  // pierre-yellow-500
  error: '#ef4444',    // pierre-red-500
  info: '#3b82f6',     // pierre-blue-500
} as const;

// ========== PROVIDER BRAND COLORS ==========

/** OAuth provider brand colors */
export const PROVIDER_COLORS = {
  strava: '#FC4C02',
  garmin: '#007CC3',
  fitbit: '#00B0B9',
  whoop: '#00D46A',
  terra: '#6366F1',
  google: '#4285F4',
} as const;

// ========== GRADIENT DEFINITIONS ==========

/** Gradient color stops for premium effects */
export const GRADIENT_COLORS = {
  violetIndigo: {
    start: 'rgba(124, 59, 237, 0.15)',
    end: 'rgba(79, 70, 229, 0.05)',
  },
  violetCyan: {
    start: '#7C3AED',
    end: '#06B6D4',
  },
  darkOverlay: {
    start: 'rgba(15, 15, 26, 0)',
    end: 'rgba(15, 15, 26, 0.8)',
  },
  aiGradient: {
    start: 'rgba(124, 58, 237, 0.08)',
    end: 'rgba(30, 30, 46, 0.6)',
  },
} as const;

// ========== GLASSMORPHISM ==========

/** Glassmorphism card style values */
export const GLASS_CARD = {
  background: 'rgba(124, 59, 237, 0.08)',
  borderColor: 'rgba(255, 255, 255, 0.1)',
  borderWidth: 1,
  shadowColor: '#7C3AED',
  shadowOpacity: 0.15,
  shadowRadius: 12,
} as const;

// ========== AI GLOW EFFECTS ==========

/** AI glow effect values for various intensities */
export const AI_GLOW = {
  ambient: {
    shadowColor: '#7C3AED',
    shadowOpacity: 0.15,
    shadowRadius: 20,
  },
  strong: {
    shadowColor: '#7C3AED',
    shadowOpacity: 0.25,
    shadowRadius: 30,
  },
  avatar: {
    shadowColor: '#7C3AED',
    shadowOpacity: 0.4,
    shadowRadius: 20,
  },
  thinking: {
    shadowColor: '#7C3AED',
    shadowOpacity: 0.5,
    shadowRadius: 25,
  },
  response: {
    shadowColor: '#7C3AED',
    shadowOpacity: 0.3,
    shadowRadius: 15,
  },
} as const;

/** Button glow effect values */
export const BUTTON_GLOW = {
  shadowColor: '#7C3AED',
  shadowOpacity: 0.4,
  shadowRadius: 20,
} as const;

// ========== SPACING SCALE ==========

/** Spacing scale (platform-agnostic values) */
export const SPACING = {
  xs: 4,
  sm: 8,
  md: 16,
  lg: 24,
  xl: 32,
  xxl: 48,
} as const;

// ========== BORDER RADIUS SCALE ==========

/** Border radius scale */
export const BORDER_RADIUS = {
  sm: 4,
  md: 8,
  lg: 12,
  xl: 16,
  full: 9999,
} as const;

// ========== FONT SIZE SCALE ==========

/** Font size scale */
export const FONT_SIZE = {
  xs: 12,
  sm: 14,
  md: 16,
  lg: 18,
  xl: 20,
  xxl: 24,
  xxxl: 32,
} as const;

// ========== FONT WEIGHT ==========

/** Font weight values */
export const FONT_WEIGHT = {
  normal: '400',
  medium: '500',
  semibold: '600',
  bold: '700',
} as const;

// ========== COMBINED THEME EXPORT ==========

/** Complete design system theme */
export const DESIGN_SYSTEM = {
  colors: {
    pierre: PIERRE_COLORS,
    pillars: PILLAR_COLORS,
    primary: PRIMARY_PALETTE,
    background: BACKGROUND_COLORS,
    text: TEXT_COLORS,
    border: BORDER_COLORS,
    semantic: SEMANTIC_COLORS,
    providers: PROVIDER_COLORS,
  },
  gradients: GRADIENT_COLORS,
  effects: {
    glassCard: GLASS_CARD,
    aiGlow: AI_GLOW,
    buttonGlow: BUTTON_GLOW,
  },
  spacing: SPACING,
  borderRadius: BORDER_RADIUS,
  fontSize: FONT_SIZE,
  fontWeight: FONT_WEIGHT,
} as const;

/** Type for the complete design system */
export type DesignSystem = typeof DESIGN_SYSTEM;

