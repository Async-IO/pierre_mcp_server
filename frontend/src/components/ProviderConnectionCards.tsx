// ABOUTME: Provider connection cards for the chat interface empty state
// ABOUTME: Displays fitness providers from server with connection status and OAuth initiation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { ProviderStatus } from '../services/api/oauth';
import { Card, Badge } from './ui';

// Brand colors and hover colors for known providers
const PROVIDER_STYLES: Record<string, { brandColor: string; hoverColor: string }> = {
  strava: {
    brandColor: 'bg-[#FC4C02]',
    hoverColor: 'hover:border-[#FC4C02]',
  },
  fitbit: {
    brandColor: 'bg-[#00B0B9]',
    hoverColor: 'hover:border-[#00B0B9]',
  },
  garmin: {
    brandColor: 'bg-[#007CC3]',
    hoverColor: 'hover:border-[#007CC3]',
  },
  whoop: {
    brandColor: 'bg-[#1A1A1A]',
    hoverColor: 'hover:border-[#1A1A1A]',
  },
  terra: {
    brandColor: 'bg-[#22C55E]',
    hoverColor: 'hover:border-[#22C55E]',
  },
  synthetic: {
    brandColor: 'bg-gradient-to-br from-pierre-violet to-pierre-cyan',
    hoverColor: 'hover:border-pierre-violet',
  },
};

// Default style for unknown providers
const DEFAULT_STYLE = {
  brandColor: 'bg-pierre-gray-500',
  hoverColor: 'hover:border-pierre-gray-500',
};

// Get description based on capabilities
const getProviderDescription = (provider: ProviderStatus): string => {
  const caps = provider.capabilities;
  if (caps.includes('activities') && caps.includes('sleep')) {
    return 'Activities, sleep & recovery';
  }
  if (caps.includes('activities')) {
    return 'Activities & workouts';
  }
  if (caps.includes('sleep')) {
    return 'Sleep tracking';
  }
  return 'Fitness data';
};

// SVG icons for each provider - clean and professional
const ProviderIcon = ({ providerId, className }: { providerId: string; className?: string }) => {
  const baseClass = className || 'w-5 h-5';

  switch (providerId) {
    case 'strava':
      return (
        <svg className={baseClass} viewBox="0 0 24 24" fill="currentColor">
          <path d="M15.387 17.944l-2.089-4.116h-3.065L15.387 24l5.15-10.172h-3.066m-7.008-5.599l2.836 5.598h4.172L10.463 0l-7 13.828h4.169" />
        </svg>
      );
    case 'fitbit':
      return (
        <svg className={baseClass} viewBox="0 0 24 24" fill="currentColor">
          <circle cx="12" cy="4" r="2" />
          <circle cx="12" cy="10" r="2" />
          <circle cx="12" cy="16" r="2" />
          <circle cx="6" cy="7" r="1.5" />
          <circle cx="6" cy="13" r="1.5" />
          <circle cx="18" cy="7" r="1.5" />
          <circle cx="18" cy="13" r="1.5" />
        </svg>
      );
    case 'garmin':
      return (
        <svg className={baseClass} viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm-1-13h2v6h-2zm0 8h2v2h-2z" />
        </svg>
      );
    case 'whoop':
      return (
        <svg className={baseClass} viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 4C7.58 4 4 7.58 4 12s3.58 8 8 8 8-3.58 8-8-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6s2.69-6 6-6 6 2.69 6 6-2.69 6-6 6z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      );
    case 'terra':
      return (
        <svg className={baseClass} viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z" />
        </svg>
      );
    case 'synthetic':
      return (
        <svg className={baseClass} viewBox="0 0 24 24" fill="currentColor">
          <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-7 14c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H9V5h6v2z" />
        </svg>
      );
    default:
      return (
        <svg className={baseClass} viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z" />
        </svg>
      );
  }
};

interface ProviderConnectionCardsProps {
  onProviderConnected?: () => void;
  onConnectProvider?: (providerName: string) => void;
  connectingProvider?: string | null;
  onSkip?: () => void;
  isSkipPending?: boolean;
}

export default function ProviderConnectionCards({
  onProviderConnected,
  onConnectProvider,
  connectingProvider,
  onSkip,
  isSkipPending
}: ProviderConnectionCardsProps) {
  // Fetch providers from server (includes OAuth and non-OAuth providers)
  const { data: providersData, isLoading } = useQuery({
    queryKey: ['providers-status'],
    queryFn: () => apiService.getProvidersStatus(),
    refetchInterval: 5000,
  });

  // Handle provider card click
  const handleConnect = async (provider: ProviderStatus) => {
    // If already connected or non-OAuth provider, no action needed
    if (provider.connected || !provider.requires_oauth) return;

    // Use callback if provided (for chat-based connection flow)
    if (onConnectProvider) {
      onConnectProvider(provider.display_name);
      return;
    }

    // Fallback: Navigate directly to OAuth authorization endpoint
    try {
      const authUrl = await apiService.getOAuthAuthorizeUrl(provider.provider);
      // Open OAuth in new tab to avoid security blocks from automated browser detection
      window.open(authUrl, '_blank');
    } catch (error) {
      console.error('Failed to get OAuth authorization URL:', error);
    }
  };

  // Check if any provider is connected
  const hasAnyConnection = providersData?.providers?.some(p => p.connected) ?? false;

  // Notify parent when a connection is detected
  if (hasAnyConnection && onProviderConnected) {
    onProviderConnected();
  }

  if (isLoading) {
    return (
      <div className="w-full">
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {[1, 2, 3, 4, 5].map((i) => (
            <Card key={i} className="p-4 animate-pulse">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-lg bg-pierre-gray-100" />
                <div className="flex-1">
                  <div className="h-4 w-20 bg-pierre-gray-100 rounded mb-2" />
                  <div className="h-3 w-28 bg-pierre-gray-50 rounded" />
                </div>
              </div>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  const providers = providersData?.providers ?? [];

  return (
    <div className="w-full">
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {providers.map((provider) => {
          const style = PROVIDER_STYLES[provider.provider] ?? DEFAULT_STYLE;
          const isConnecting = connectingProvider === provider.display_name;
          const isNonOAuth = !provider.requires_oauth;

          return (
            <button
              key={provider.provider}
              type="button"
              onClick={() => handleConnect(provider)}
              disabled={provider.connected || isConnecting || !!connectingProvider || isNonOAuth}
              className="text-left focus:outline-none focus:ring-2 focus:ring-pierre-violet/50 rounded-xl disabled:cursor-default group"
              aria-label={
                provider.connected
                  ? `${provider.display_name} is connected`
                  : isNonOAuth
                    ? `${provider.display_name} - ${getProviderDescription(provider)}`
                    : `Connect to ${provider.display_name}`
              }
            >
              <Card
                className={`p-4 transition-all duration-200 h-full border-2 ${
                  provider.connected
                    ? 'bg-pierre-gray-50/50 border-emerald-200'
                    : isConnecting
                      ? 'border-pierre-violet bg-pierre-violet/5'
                      : isNonOAuth
                        ? 'border-transparent bg-pierre-gray-50/30'
                        : `border-transparent ${style.hoverColor} hover:shadow-lg hover:-translate-y-0.5`
                }`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`w-10 h-10 rounded-xl ${style.brandColor} flex items-center justify-center text-white shadow-sm`}
                  >
                    {isConnecting ? (
                      <div className="pierre-spinner w-5 h-5 border-white border-t-transparent"></div>
                    ) : (
                      <ProviderIcon providerId={provider.provider} />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-pierre-gray-900 text-sm">{provider.display_name}</span>
                      {provider.connected && (
                        <Badge variant="success">
                          Connected
                        </Badge>
                      )}
                      {isNonOAuth && !provider.connected && (
                        <Badge variant="secondary">
                          Demo
                        </Badge>
                      )}
                    </div>
                    <p className="text-xs text-pierre-gray-500 mt-0.5">{getProviderDescription(provider)}</p>
                  </div>
                  {!provider.connected && provider.requires_oauth && (
                    <svg
                      className="w-4 h-4 text-pierre-gray-300 group-hover:text-pierre-gray-500 transition-colors"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                    </svg>
                  )}
                </div>
              </Card>
            </button>
          );
        })}

        {/* Skip and start chatting - last card */}
        {onSkip && (
          <button
            type="button"
            onClick={onSkip}
            disabled={isSkipPending}
            className="text-left focus:outline-none focus:ring-2 focus:ring-pierre-violet/50 rounded-xl group"
            aria-label="Skip and start chatting"
          >
            <Card
              className="p-4 transition-all duration-200 h-full border-2 border-transparent hover:border-pierre-violet hover:shadow-lg hover:-translate-y-0.5 bg-gradient-to-br from-pierre-violet/5 to-pierre-cyan/5"
            >
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-pierre-violet to-pierre-cyan flex items-center justify-center text-white shadow-sm">
                  {isSkipPending ? (
                    <div className="pierre-spinner w-5 h-5 border-white border-t-transparent"></div>
                  ) : (
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                    </svg>
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <span className="font-semibold text-pierre-gray-900 text-sm">
                    {isSkipPending ? 'Starting...' : 'Start chatting'}
                  </span>
                  <p className="text-xs text-pierre-gray-500 mt-0.5">Connect providers later</p>
                </div>
                <svg
                  className="w-4 h-4 text-pierre-gray-300 group-hover:text-pierre-violet transition-colors"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </Card>
          </button>
        )}
      </div>
    </div>
  );
}
