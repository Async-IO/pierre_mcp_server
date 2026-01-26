// ABOUTME: Main entry point for @pierre/shared-constants package
// ABOUTME: Re-exports all shared constants for convenient importing

// Social constants (colors, labels, emojis for insights and reactions)
export {
  REACTION_EMOJIS,
  INSIGHT_TYPE_COLORS,
  INSIGHT_TYPE_LABELS,
  INSIGHT_TYPE_ICONS,
  REACTION_TYPE_LABELS,
} from './social.js';

// Design system (colors, typography, spacing, effects)
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
  SPACING,
  BORDER_RADIUS,
  FONT_SIZE,
  FONT_WEIGHT,
  DESIGN_SYSTEM,
} from './design-system.js';

export type { DesignSystem } from './design-system.js';

// React Query keys (for consistent cache key management)
export { QUERY_KEYS } from './query-keys.js';
export type { QueryKeys } from './query-keys.js';
