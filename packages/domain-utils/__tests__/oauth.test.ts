// ABOUTME: Unit tests for OAuth URL detection and provider identification
// ABOUTME: Tests detectOAuthProvider, getFriendlyUrlName, linkifyUrls

import { describe, it, expect } from 'vitest';
import {
  detectOAuthProvider,
  getFriendlyUrlName,
  linkifyUrls,
  OAUTH_PROVIDERS,
} from '../src/oauth';

describe('detectOAuthProvider', () => {
  it('detects Strava OAuth URLs', () => {
    expect(detectOAuthProvider('https://www.strava.com/oauth/authorize?client_id=123')).toBe('strava');
  });

  it('detects Fitbit OAuth URLs', () => {
    expect(detectOAuthProvider('https://www.fitbit.com/oauth2/authorize?response_type=code')).toBe('fitbit');
  });

  it('detects Garmin OAuth URLs', () => {
    expect(detectOAuthProvider('https://connect.garmin.com/oauthConfirm?token=abc')).toBe('garmin');
  });

  it('returns null for non-OAuth URLs', () => {
    expect(detectOAuthProvider('https://www.google.com')).toBeNull();
  });

  it('returns null for URLs without oauth path', () => {
    expect(detectOAuthProvider('https://www.strava.com/activities/123')).toBeNull();
  });

  it('returns null for invalid URLs', () => {
    expect(detectOAuthProvider('not-a-url')).toBeNull();
  });

  it('returns null for empty string', () => {
    expect(detectOAuthProvider('')).toBeNull();
  });

  it('prevents subdomain bypass attacks', () => {
    // strava.com.evil.com should NOT be detected
    expect(detectOAuthProvider('https://strava.com.evil.com/oauth/authorize')).toBeNull();
  });

  it('accepts valid subdomains', () => {
    expect(detectOAuthProvider('https://api.strava.com/oauth/token')).toBe('strava');
  });
});

describe('OAUTH_PROVIDERS', () => {
  it('has configuration for all three providers', () => {
    expect(OAUTH_PROVIDERS.strava.displayName).toBe('Strava');
    expect(OAUTH_PROVIDERS.fitbit.displayName).toBe('Fitbit');
    expect(OAUTH_PROVIDERS.garmin.displayName).toBe('Garmin');
  });

  it('has color codes for each provider', () => {
    expect(OAUTH_PROVIDERS.strava.color).toMatch(/^#[0-9A-F]{6}$/i);
    expect(OAUTH_PROVIDERS.fitbit.color).toMatch(/^#[0-9A-F]{6}$/i);
    expect(OAUTH_PROVIDERS.garmin.color).toMatch(/^#[0-9A-F]{6}$/i);
  });
});

describe('getFriendlyUrlName', () => {
  it('returns "Connect to Strava →" for Strava OAuth URLs', () => {
    expect(getFriendlyUrlName('https://www.strava.com/oauth/authorize?client_id=123'))
      .toBe('Connect to Strava →');
  });

  it('returns "Connect to Fitbit →" for Fitbit OAuth URLs', () => {
    expect(getFriendlyUrlName('https://www.fitbit.com/oauth2/authorize'))
      .toBe('Connect to Fitbit →');
  });

  it('returns domain for non-OAuth URLs', () => {
    expect(getFriendlyUrlName('https://www.example.com/page')).toBe('www.example.com/page');
  });

  it('truncates long paths', () => {
    const result = getFriendlyUrlName('https://example.com/a-very-long-path-that-exceeds-limit-and-more');
    expect(result.length).toBeLessThan(60);
  });

  it('handles root path without trailing slash', () => {
    expect(getFriendlyUrlName('https://example.com/')).toBe('example.com');
  });

  it('handles invalid URLs gracefully', () => {
    expect(getFriendlyUrlName('not-a-url')).toBe('not-a-url');
  });

  it('truncates long non-URL strings', () => {
    const longString = 'a'.repeat(100);
    const result = getFriendlyUrlName(longString);
    expect(result.length).toBe(50);
    expect(result).toContain('...');
  });
});

describe('linkifyUrls', () => {
  it('converts plain URLs to markdown links', () => {
    const result = linkifyUrls('Visit https://example.com for info');
    expect(result).toContain('[example.com](https://example.com)');
  });

  it('converts OAuth URLs to friendly names', () => {
    const result = linkifyUrls('Click https://www.strava.com/oauth/authorize?id=1 to connect');
    expect(result).toContain('[Connect to Strava →]');
  });

  it('replaces URL text in existing markdown links', () => {
    const result = linkifyUrls('[https://www.strava.com/oauth/authorize](https://www.strava.com/oauth/authorize)');
    expect(result).toContain('[Connect to Strava →]');
  });

  it('leaves text without URLs unchanged', () => {
    const text = 'No links here';
    expect(linkifyUrls(text)).toBe(text);
  });

  it('handles multiple URLs in text', () => {
    const text = 'See https://a.com and https://b.com';
    const result = linkifyUrls(text);
    expect(result).toContain('[a.com](https://a.com)');
    expect(result).toContain('[b.com](https://b.com)');
  });
});
