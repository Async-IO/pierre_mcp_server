// ABOUTME: Coach category utilities for styling and display
// ABOUTME: Shared between web and mobile for consistent category presentation

/**
 * Coach category types
 */
export type CoachCategory =
  | 'training'
  | 'nutrition'
  | 'recovery'
  | 'recipes'
  | 'mobility'
  | 'analysis'
  | 'custom';

/**
 * All available coach categories
 */
export const COACH_CATEGORIES: readonly CoachCategory[] = [
  'training',
  'nutrition',
  'recovery',
  'recipes',
  'mobility',
  'analysis',
  'custom',
] as const;

/**
 * Category display configuration
 */
export interface CategoryConfig {
  label: string;
  icon: string;
  /** Tailwind color classes for web (bg + text) */
  webClasses: string;
  /** Color values for mobile (can be used with NativeWind or inline styles) */
  colors: {
    background: string;
    text: string;
  };
}

/**
 * Category configurations for consistent styling across platforms
 */
export const CATEGORY_CONFIG: Record<CoachCategory, CategoryConfig> = {
  training: {
    label: 'Training',
    icon: '🏃',
    webClasses: 'bg-pierre-green-100 text-pierre-green-700',
    colors: {
      background: 'rgba(34, 197, 94, 0.1)',
      text: '#15803d',
    },
  },
  nutrition: {
    label: 'Nutrition',
    icon: '🥗',
    webClasses: 'bg-pierre-nutrition/10 text-pierre-nutrition',
    colors: {
      background: 'rgba(245, 158, 11, 0.1)',
      text: '#d97706',
    },
  },
  recovery: {
    label: 'Recovery',
    icon: '😴',
    webClasses: 'bg-pierre-blue-100 text-pierre-blue-700',
    colors: {
      background: 'rgba(59, 130, 246, 0.1)',
      text: '#1d4ed8',
    },
  },
  recipes: {
    label: 'Recipes',
    icon: '👨‍🍳',
    webClasses: 'bg-pierre-yellow-100 text-pierre-yellow-700',
    colors: {
      background: 'rgba(234, 179, 8, 0.1)',
      text: '#a16207',
    },
  },
  mobility: {
    label: 'Mobility',
    icon: '🧘',
    webClasses: 'bg-pierre-mobility/10 text-pierre-mobility',
    colors: {
      background: 'rgba(168, 85, 247, 0.1)',
      text: '#7c3aed',
    },
  },
  analysis: {
    label: 'Analysis',
    icon: '📊',
    webClasses: 'bg-pierre-violet/10 text-pierre-violet-light',
    colors: {
      background: 'rgba(139, 92, 246, 0.1)',
      text: '#8b5cf6',
    },
  },
  custom: {
    label: 'Custom',
    icon: '⚙️',
    webClasses: 'bg-pierre-gray-100 text-pierre-gray-600',
    colors: {
      background: 'rgba(107, 114, 128, 0.1)',
      text: '#4b5563',
    },
  },
};

/**
 * Get category configuration by name (case-insensitive)
 */
export function getCategoryConfig(category: string): CategoryConfig {
  const normalized = category.toLowerCase() as CoachCategory;
  return CATEGORY_CONFIG[normalized] || CATEGORY_CONFIG.custom;
}

/**
 * Get Tailwind badge classes for a category (web)
 */
export function getCategoryBadgeClass(category: string): string {
  return getCategoryConfig(category).webClasses;
}

/**
 * Get icon for a category
 */
export function getCategoryIcon(category: string): string {
  return getCategoryConfig(category).icon;
}

/**
 * Get display label for a category
 */
export function getCategoryLabel(category: string): string {
  return getCategoryConfig(category).label;
}
