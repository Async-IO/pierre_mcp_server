// ABOUTME: Unit tests for coach category utilities
// ABOUTME: Tests getCategoryConfig, getCategoryBadgeClass, getCategoryIcon, getCategoryLabel

import { describe, it, expect } from 'vitest';
import {
  COACH_CATEGORIES,
  CATEGORY_CONFIG,
  getCategoryConfig,
  getCategoryBadgeClass,
  getCategoryIcon,
  getCategoryLabel,
} from '../src/categories';
import type { CoachCategory } from '../src/categories';

describe('COACH_CATEGORIES', () => {
  it('contains all 7 categories', () => {
    expect(COACH_CATEGORIES).toHaveLength(7);
  });

  it('contains expected category values', () => {
    expect(COACH_CATEGORIES).toContain('training');
    expect(COACH_CATEGORIES).toContain('nutrition');
    expect(COACH_CATEGORIES).toContain('recovery');
    expect(COACH_CATEGORIES).toContain('recipes');
    expect(COACH_CATEGORIES).toContain('mobility');
    expect(COACH_CATEGORIES).toContain('analysis');
    expect(COACH_CATEGORIES).toContain('custom');
  });
});

describe('CATEGORY_CONFIG', () => {
  it('has config for every category', () => {
    for (const category of COACH_CATEGORIES) {
      const config = CATEGORY_CONFIG[category];
      expect(config).toBeDefined();
      expect(config.label).toBeTruthy();
      expect(config.icon).toBeTruthy();
      expect(config.webClasses).toBeTruthy();
      expect(config.colors.background).toBeTruthy();
      expect(config.colors.text).toBeTruthy();
    }
  });

  it('has proper Tailwind classes for web', () => {
    expect(CATEGORY_CONFIG.training.webClasses).toContain('bg-');
    expect(CATEGORY_CONFIG.training.webClasses).toContain('text-');
  });

  it('has rgba colors for mobile backgrounds', () => {
    for (const category of COACH_CATEGORIES) {
      expect(CATEGORY_CONFIG[category].colors.background).toMatch(/^rgba\(/);
    }
  });

  it('has hex colors for mobile text', () => {
    for (const category of COACH_CATEGORIES) {
      expect(CATEGORY_CONFIG[category].colors.text).toMatch(/^#[0-9a-f]{6}$/i);
    }
  });
});

describe('getCategoryConfig', () => {
  it('returns correct config for known categories', () => {
    const config = getCategoryConfig('training');
    expect(config.label).toBe('Training');
    expect(config.icon).toBe('🏃');
  });

  it('is case-insensitive', () => {
    expect(getCategoryConfig('Training').label).toBe('Training');
    expect(getCategoryConfig('NUTRITION').label).toBe('Nutrition');
    expect(getCategoryConfig('Recovery').label).toBe('Recovery');
  });

  it('falls back to custom for unknown categories', () => {
    const config = getCategoryConfig('unknown_category');
    expect(config.label).toBe('Custom');
    expect(config.icon).toBe('⚙️');
  });
});

describe('getCategoryBadgeClass', () => {
  it('returns Tailwind classes for training', () => {
    expect(getCategoryBadgeClass('training')).toContain('bg-');
    expect(getCategoryBadgeClass('training')).toContain('text-');
  });

  it('returns custom classes for unknown categories', () => {
    expect(getCategoryBadgeClass('unknown')).toBe(CATEGORY_CONFIG.custom.webClasses);
  });
});

describe('getCategoryIcon', () => {
  it('returns correct icons', () => {
    expect(getCategoryIcon('training')).toBe('🏃');
    expect(getCategoryIcon('nutrition')).toBe('🥗');
    expect(getCategoryIcon('recovery')).toBe('😴');
    expect(getCategoryIcon('recipes')).toBe('👨‍🍳');
    expect(getCategoryIcon('mobility')).toBe('🧘');
    expect(getCategoryIcon('analysis')).toBe('📊');
    expect(getCategoryIcon('custom')).toBe('⚙️');
  });

  it('returns custom icon for unknown category', () => {
    expect(getCategoryIcon('nonexistent')).toBe('⚙️');
  });
});

describe('getCategoryLabel', () => {
  it('returns capitalized labels', () => {
    expect(getCategoryLabel('training')).toBe('Training');
    expect(getCategoryLabel('nutrition')).toBe('Nutrition');
    expect(getCategoryLabel('recovery')).toBe('Recovery');
  });

  it('is case-insensitive for input', () => {
    expect(getCategoryLabel('MOBILITY')).toBe('Mobility');
  });

  it('returns Custom for unknown', () => {
    expect(getCategoryLabel('xyz')).toBe('Custom');
  });
});
