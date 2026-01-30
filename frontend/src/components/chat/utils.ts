// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Utility functions for chat components
// ABOUTME: URL processing, text formatting, and category helpers

// Convert plain URLs to markdown links with friendly display names
// Matches http/https URLs that aren't already in markdown link format
const urlRegex = /(?<!\]\()(?<!\[)(https?:\/\/[^\s<>[\]()]+)/g;

// Also match existing markdown links where the text is a URL: [url](url)
const markdownLinkRegex = /\[(https?:\/\/[^\]]+)\]\((https?:\/\/[^)]+)\)/g;

// Security: Check if hostname matches a trusted OAuth provider domain
// Uses endsWith to prevent subdomain bypass attacks (e.g., strava.com.evil.com)
const isTrustedOAuthDomain = (hostname: string, domain: string): boolean => {
  // Exact match or subdomain of the trusted domain
  return hostname === domain || hostname.endsWith(`.${domain}`);
};

// Generate a friendly display name for a URL
export const getFriendlyUrlName = (url: string): string => {
  try {
    const parsed = new URL(url);
    // Special handling for OAuth URLs - use strict domain validation
    if (isTrustedOAuthDomain(parsed.hostname, 'strava.com') && parsed.pathname.includes('oauth')) {
      return 'Connect to Strava →';
    }
    if (isTrustedOAuthDomain(parsed.hostname, 'fitbit.com') && parsed.pathname.includes('oauth')) {
      return 'Connect to Fitbit →';
    }
    if (isTrustedOAuthDomain(parsed.hostname, 'garmin.com') && parsed.pathname.includes('oauth')) {
      return 'Connect to Garmin →';
    }
    // For other URLs, show domain + truncated path
    const path = parsed.pathname.length > 20
      ? parsed.pathname.slice(0, 20) + '...'
      : parsed.pathname;
    return `${parsed.hostname}${path !== '/' ? path : ''}`;
  } catch {
    // If URL parsing fails, truncate to reasonable length
    return url.length > 50 ? url.slice(0, 47) + '...' : url;
  }
};

export const linkifyUrls = (text: string): string => {
  // First, replace existing markdown links that have URL as text with friendly names
  let result = text.replace(markdownLinkRegex, (_match, _linkText, href) => {
    return `[${getFriendlyUrlName(href)}](${href})`;
  });
  // Then, convert any remaining plain URLs to markdown links
  result = result.replace(urlRegex, (url) => `[${getFriendlyUrlName(url)}](${url})`);
  return result;
};

// Strip internal context prefixes from messages before displaying to user
export const stripContextPrefix = (text: string): string => {
  return text.replace(/^\[Context:[^\]]*\]\s*/i, '');
};

// Format date for conversation list
export const formatDate = (dateString: string): string => {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) {
    return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  } else if (diffDays === 1) {
    return 'Yesterday';
  } else if (diffDays < 7) {
    return date.toLocaleDateString('en-US', { weekday: 'short' });
  } else {
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }
};

// Category styling helpers
export const getCategoryBadgeClass = (category: string): string => {
  const classes: Record<string, string> = {
    training: 'bg-pierre-green-100 text-pierre-green-700',
    nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition',
    recovery: 'bg-pierre-blue-100 text-pierre-blue-700',
    recipes: 'bg-pierre-yellow-100 text-pierre-yellow-700',
    mobility: 'bg-pierre-mobility/10 text-pierre-mobility',
    analysis: 'bg-pierre-violet/10 text-pierre-violet-light',
    custom: 'bg-pierre-gray-100 text-pierre-gray-600',
  };
  return classes[category.toLowerCase()] || classes.custom;
};

export const getCategoryIcon = (category: string): string => {
  const icons: Record<string, string> = {
    training: '🏃',
    nutrition: '🥗',
    recovery: '😴',
    recipes: '👨‍🍳',
    mobility: '🧘',
    analysis: '📊',
    custom: '⚙️',
  };
  return icons[category.toLowerCase()] || icons.custom;
};

// Coach category list
export const COACH_CATEGORIES = ['Training', 'Nutrition', 'Recovery', 'Recipes', 'Mobility', 'Analysis', 'Custom'] as const;
