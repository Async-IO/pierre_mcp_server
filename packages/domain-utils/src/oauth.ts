// ABOUTME: OAuth URL detection and provider identification utilities
// ABOUTME: Used for generating friendly link text for OAuth authorization URLs

/**
 * Supported OAuth providers
 */
export type OAuthProvider = 'strava' | 'fitbit' | 'garmin';

/**
 * Provider display configuration
 */
export interface ProviderConfig {
  name: string;
  domain: string;
  displayName: string;
  color: string;
}

/**
 * Provider configurations for OAuth detection
 */
export const OAUTH_PROVIDERS: Record<OAuthProvider, ProviderConfig> = {
  strava: {
    name: 'strava',
    domain: 'strava.com',
    displayName: 'Strava',
    color: '#FC4C02',
  },
  fitbit: {
    name: 'fitbit',
    domain: 'fitbit.com',
    displayName: 'Fitbit',
    color: '#00B0B9',
  },
  garmin: {
    name: 'garmin',
    domain: 'garmin.com',
    displayName: 'Garmin',
    color: '#007CC3',
  },
};

/**
 * Security: Check if hostname matches a trusted OAuth provider domain
 * Uses endsWith to prevent subdomain bypass attacks (e.g., strava.com.evil.com)
 */
function isTrustedOAuthDomain(hostname: string, domain: string): boolean {
  return hostname === domain || hostname.endsWith(`.${domain}`);
}

/**
 * Detect if a URL is an OAuth authorization URL for a known provider
 * Returns the provider name if detected, null otherwise
 */
export function detectOAuthProvider(url: string): OAuthProvider | null {
  try {
    const parsed = new URL(url);

    for (const [provider, config] of Object.entries(OAUTH_PROVIDERS)) {
      if (isTrustedOAuthDomain(parsed.hostname, config.domain) &&
          parsed.pathname.includes('oauth')) {
        return provider as OAuthProvider;
      }
    }

    return null;
  } catch {
    return null;
  }
}

/**
 * Get a friendly display name for OAuth URLs
 * Returns "Connect to Strava →" for OAuth URLs, or formatted URL for others
 */
export function getFriendlyUrlName(url: string): string {
  try {
    const parsed = new URL(url);
    const provider = detectOAuthProvider(url);

    if (provider) {
      return `Connect to ${OAUTH_PROVIDERS[provider].displayName} →`;
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
}

/**
 * Convert plain URLs in text to markdown links with friendly display names
 */
export function linkifyUrls(text: string): string {
  // Match URLs that aren't already in markdown link format
  const urlRegex = /(?<!\]\()(?<!\[)(https?:\/\/[^\s<>[\]()]+)/g;
  // Match existing markdown links where the text is a URL
  const markdownLinkRegex = /\[(https?:\/\/[^\]]+)\]\((https?:\/\/[^)]+)\)/g;

  // First, replace existing markdown links that have URL as text with friendly names
  let result = text.replace(markdownLinkRegex, (...groups: string[]) => {
    const href = groups[2];
    return `[${getFriendlyUrlName(href)}](${href})`;
  });

  // Then, convert any remaining plain URLs to markdown links
  result = result.replace(urlRegex, (url) => `[${getFriendlyUrlName(url)}](${url})`);

  return result;
}
