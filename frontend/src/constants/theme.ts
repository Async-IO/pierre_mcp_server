// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Theme constants for Pierre Web app
// ABOUTME: Re-exports shared design system and adds platform-specific styles

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
} from '@pierre/shared-constants';

// Re-export spacing and typography from shared constants
export const spacing = SPACING;
export const borderRadius = BORDER_RADIUS;
export const fontSize = FONT_SIZE;
export const fontWeight = FONT_WEIGHT;

// Combined colors object for web (maintains consistent API with mobile)
export const colors = {
  // Pierre Design System - Primary brand colors with pillars
  pierre: {
    ...PIERRE_COLORS,
    // Three Pillars - included in pierre for backward compatibility
    activity: PILLAR_COLORS.activity,
    nutrition: PILLAR_COLORS.nutrition,
    recovery: PILLAR_COLORS.recovery,
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

// Glassmorphism card styles for premium look (web CSS)
export const glassCard = {
  background: GLASS_CARD.background,
  borderColor: GLASS_CARD.borderColor,
  borderWidth: `${GLASS_CARD.borderWidth}px`,
  boxShadow: `0 4px ${GLASS_CARD.shadowRadius}px ${GLASS_CARD.shadowColor}`,
  backdropFilter: 'blur(12px)',
} as const;

// Glow effect for primary buttons (web CSS)
export const buttonGlow = {
  boxShadow: `0 0 ${BUTTON_GLOW.shadowRadius}px ${BUTTON_GLOW.shadowColor}`,
} as const;

// Gradient colors for premium effects (as CSS strings)
export const gradients = {
  violetIndigo: `linear-gradient(135deg, ${GRADIENT_COLORS.violetIndigo.start}, ${GRADIENT_COLORS.violetIndigo.end})`,
  violetCyan: `linear-gradient(135deg, ${GRADIENT_COLORS.violetCyan.start}, ${GRADIENT_COLORS.violetCyan.end})`,
  darkOverlay: `linear-gradient(180deg, ${GRADIENT_COLORS.darkOverlay.start}, ${GRADIENT_COLORS.darkOverlay.end})`,
  aiGradient: `linear-gradient(135deg, ${GRADIENT_COLORS.aiGradient.start}, ${GRADIENT_COLORS.aiGradient.end})`,
} as const;

// AI Intelligence Glow Effects - Reinforces Pierre's AI-first brand identity
// Use with CSS animations or @keyframes for animated effects
export const aiGlow = {
  // Subtle ambient glow for AI elements
  ambient: {
    boxShadow: `0 0 ${AI_GLOW.ambient.shadowRadius}px ${AI_GLOW.ambient.shadowColor}`,
  },
  // Strong glow for prominent AI elements
  strong: {
    boxShadow: `0 0 ${AI_GLOW.strong.shadowRadius}px ${AI_GLOW.strong.shadowColor}`,
  },
  // Avatar/icon glow
  avatar: {
    boxShadow: `0 0 ${AI_GLOW.avatar.shadowRadius}px ${AI_GLOW.avatar.shadowColor}`,
  },
  // Thinking/processing state glow
  thinking: {
    boxShadow: `0 0 ${AI_GLOW.thinking.shadowRadius}px ${AI_GLOW.thinking.shadowColor}`,
  },
  // Response glow for new AI messages
  response: {
    boxShadow: `0 0 ${AI_GLOW.response.shadowRadius}px ${AI_GLOW.response.shadowColor}`,
  },
} as const;

// AI card style with gradient background and glow (web CSS)
export const aiCard = {
  background: gradients.aiGradient,
  border: '1px solid rgba(124, 58, 237, 0.2)',
  borderRadius: `${borderRadius.xl}px`,
  ...aiGlow.ambient,
} as const;

// Re-export raw shared constants for direct access
export {
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
};
