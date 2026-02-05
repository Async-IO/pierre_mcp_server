// ABOUTME: Main entry point for @pierre/domain-utils package
// ABOUTME: Re-exports all domain utilities for formatting, OAuth, and categories

// Formatting utilities
export {
  formatRelativeDate,
  formatFullDate,
  formatDuration,
  formatDistance,
  formatPace,
  truncateText,
} from './formatting';

// OAuth detection utilities
export {
  type OAuthProvider,
  type ProviderConfig,
  OAUTH_PROVIDERS,
  detectOAuthProvider,
  getFriendlyUrlName,
  linkifyUrls,
} from './oauth';

// Category utilities
export {
  type CoachCategory,
  type CategoryConfig,
  COACH_CATEGORIES,
  CATEGORY_CONFIG,
  getCategoryConfig,
  getCategoryBadgeClass,
  getCategoryIcon,
  getCategoryLabel,
} from './categories';
