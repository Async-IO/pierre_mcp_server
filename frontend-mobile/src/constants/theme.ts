// ABOUTME: Theme constants for Pierre Mobile app
// ABOUTME: Re-exports shared design system and adds platform-specific styles

// Note: Using relative imports for Jest compatibility since mobile is isolated from workspaces
// Metro bundler resolves @pierre/* via extraNodeModules in metro.config.js at runtime
import {
  PIERRE_COLORS,
  PILLAR_COLORS,
  PRIMARY_PALETTE,
  BACKGROUND_COLORS,
  TEXT_COLORS,
  BORDER_COLORS,
  SEMANTIC_COLORS,
  PROVIDER_COLORS,
  GRADIENT_COLORS,
  GLASS_CARD,
  AI_GLOW,
  BUTTON_GLOW,
  SPACING,
  BORDER_RADIUS,
  FONT_SIZE,
  FONT_WEIGHT,
} from '../../../packages/shared-constants/src/design-system';

// Re-export spacing and typography from shared constants
export const spacing = SPACING;
export const borderRadius = BORDER_RADIUS;
export const fontSize = FONT_SIZE;
export const fontWeight = FONT_WEIGHT;

// Combined colors object for mobile (maintains existing API)
export const colors = {
  // Pierre Design System - Primary brand colors with pillars
  pierre: {
    ...PIERRE_COLORS,
    // Three Pillars - included in pierre for backward compatibility
    activity: PILLAR_COLORS.activity,
    nutrition: PILLAR_COLORS.nutrition,
    recovery: PILLAR_COLORS.recovery,
    // Additional colors used by Stitch UX components
    red: '#FF6B6B',  // Errors, destructive actions
  },

  // Primary brand color (sky blue palette)
  primary: PRIMARY_PALETTE,

  // Dark theme backgrounds - Pierre Design System
  background: BACKGROUND_COLORS,

  // Text colors
  text: TEXT_COLORS,

  // Border colors
  border: BORDER_COLORS,

  // Semantic colors (mapped to Pierre Design System)
  success: SEMANTIC_COLORS.success,
  warning: SEMANTIC_COLORS.warning,
  error: SEMANTIC_COLORS.error,
  info: SEMANTIC_COLORS.info,

  // Provider brand colors
  providers: PROVIDER_COLORS,

  // Google brand color (for OAuth button)
  google: PROVIDER_COLORS.google,
} as const;

// Glassmorphism card styles for premium look (platform-specific with shadows)
export const glassCard = {
  background: GLASS_CARD.background,
  borderColor: GLASS_CARD.borderColor,
  borderWidth: GLASS_CARD.borderWidth,
  // Shadow for depth (iOS)
  shadowColor: GLASS_CARD.shadowColor,
  shadowOffset: { width: 0, height: 4 },
  shadowOpacity: GLASS_CARD.shadowOpacity,
  shadowRadius: GLASS_CARD.shadowRadius,
  // Android elevation
  elevation: 8,
} as const;

// Glow effect for primary buttons (platform-specific)
export const buttonGlow = {
  shadowColor: BUTTON_GLOW.shadowColor,
  shadowOffset: { width: 0, height: 0 },
  shadowOpacity: BUTTON_GLOW.shadowOpacity,
  shadowRadius: BUTTON_GLOW.shadowRadius,
  elevation: 12,
} as const;

// Gradient colors for premium effects (as arrays for LinearGradient)
export const gradients = {
  violetIndigo: [GRADIENT_COLORS.violetIndigo.start, GRADIENT_COLORS.violetIndigo.end],
  violetCyan: [GRADIENT_COLORS.violetCyan.start, GRADIENT_COLORS.violetCyan.end],
  darkOverlay: [GRADIENT_COLORS.darkOverlay.start, GRADIENT_COLORS.darkOverlay.end],
  aiGradient: [GRADIENT_COLORS.aiGradient.start, GRADIENT_COLORS.aiGradient.end],
} as const;

// AI Intelligence Glow Effects - Reinforces Pierre's AI-first brand identity
// Use with Animated API or react-native-reanimated for animated effects
export const aiGlow = {
  // Subtle ambient glow for AI elements
  ambient: {
    shadowColor: AI_GLOW.ambient.shadowColor,
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: AI_GLOW.ambient.shadowOpacity,
    shadowRadius: AI_GLOW.ambient.shadowRadius,
    elevation: 6,
  },
  // Strong glow for prominent AI elements
  strong: {
    shadowColor: AI_GLOW.strong.shadowColor,
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: AI_GLOW.strong.shadowOpacity,
    shadowRadius: AI_GLOW.strong.shadowRadius,
    elevation: 10,
  },
  // Avatar/icon glow
  avatar: {
    shadowColor: AI_GLOW.avatar.shadowColor,
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: AI_GLOW.avatar.shadowOpacity,
    shadowRadius: AI_GLOW.avatar.shadowRadius,
    elevation: 8,
  },
  // Thinking/processing state glow
  thinking: {
    shadowColor: AI_GLOW.thinking.shadowColor,
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: AI_GLOW.thinking.shadowOpacity,
    shadowRadius: AI_GLOW.thinking.shadowRadius,
    elevation: 12,
  },
  // Response glow for new AI messages
  response: {
    shadowColor: AI_GLOW.response.shadowColor,
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: AI_GLOW.response.shadowOpacity,
    shadowRadius: AI_GLOW.response.shadowRadius,
    elevation: 8,
  },
} as const;

// AI card style with gradient background and glow
export const aiCard = {
  backgroundColor: GRADIENT_COLORS.aiGradient.start,
  borderColor: 'rgba(139, 92, 246, 0.2)',
  borderWidth: 1,
  borderRadius: borderRadius.xl,
  ...aiGlow.ambient,
} as const;
