// ABOUTME: Shared constants for social features (coach-mediated sharing)
// ABOUTME: Colors, labels, emojis, and icons for insights and reactions

import type { InsightType, ReactionType } from '@pierre/shared-types';

/**
 * Emoji mapping for reaction types
 */
export const REACTION_EMOJIS: Record<ReactionType, string> = {
  like: '\u{1F44D}',      // thumbs up
  celebrate: '\u{1F389}', // party popper
  inspire: '\u{1F4AA}',   // flexed biceps
  support: '\u{1F917}',   // hugging face
} as const;

/**
 * Color mapping for insight types (Pierre brand colors)
 *
 * These are the canonical colors for the Pierre platform.
 * Use these values across all frontends (web, mobile, etc.)
 */
export const INSIGHT_TYPE_COLORS: Record<InsightType, string> = {
  achievement: '#10B981', // emerald-500
  milestone: '#F59E0B',   // amber-500
  training_tip: '#6366F1', // indigo-500
  recovery: '#8B5CF6',     // violet-500
  motivation: '#F97316',   // orange-500
  coaching_insight: '#7C3AED', // pierre-violet
} as const;

/**
 * Human-readable labels for insight types
 */
export const INSIGHT_TYPE_LABELS: Record<InsightType, string> = {
  achievement: 'Achievement',
  milestone: 'Milestone',
  training_tip: 'Training Tip',
  recovery: 'Recovery',
  motivation: 'Motivation',
  coaching_insight: 'Coach Chat',
} as const;

/**
 * Icon names for insight types (for SVG icon libraries)
 *
 * These correspond to common icon libraries like Lucide, Heroicons, etc.
 */
export const INSIGHT_TYPE_ICONS: Record<InsightType, string> = {
  achievement: 'award',
  milestone: 'flag',
  training_tip: 'zap',
  recovery: 'moon',
  motivation: 'sun',
  coaching_insight: 'message-circle',
} as const;

/**
 * Human-readable labels for reaction types
 */
export const REACTION_TYPE_LABELS: Record<ReactionType, string> = {
  like: 'Like',
  celebrate: 'Celebrate',
  inspire: 'Inspire',
  support: 'Support',
} as const;
