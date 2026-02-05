// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Re-exports shared utilities for chat components
// ABOUTME: Sources from @pierre/domain-utils and @pierre/chat-utils packages

// OAuth-aware URL formatting
export { getFriendlyUrlName, linkifyUrls } from '@pierre/domain-utils';

// Message processing
export { stripContextPrefix } from '@pierre/chat-utils';

// Category styling helpers
export { getCategoryBadgeClass, getCategoryIcon } from '@pierre/domain-utils';

// Date formatting for conversation list
export { formatRelativeDate as formatDate } from '@pierre/domain-utils';

// Coach category list (title-case labels for display)
export const COACH_CATEGORIES = ['Training', 'Nutrition', 'Recovery', 'Recipes', 'Mobility', 'Analysis', 'Custom'] as const;
