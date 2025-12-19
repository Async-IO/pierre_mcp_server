// ABOUTME: Provider connection cards for the chat interface empty state
// ABOUTME: Displays 5 fitness providers with connection status and OAuth initiation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card, Badge } from './ui';

interface Provider {
  id: string;
  name: string;
  description: string;
  brandColor: string;
  hoverColor: string;
}

// Provider definitions with brand colors
const PROVIDERS: Provider[] = [
  {
    id: 'strava',
    name: 'Strava',
    description: 'Running, cycling & activities',
    brandColor: 'bg-[#FC4C02]',
    hoverColor: 'hover:border-[#FC4C02]',
  },
  {
    id: 'fitbit',
    name: 'Fitbit',
    description: 'Activity, sleep & heart rate',
    brandColor: 'bg-[#00B0B9]',
    hoverColor: 'hover:border-[#00B0B9]',
  },
  {
    id: 'garmin',
    name: 'Garmin',
    description: 'Training metrics & GPS data',
    brandColor: 'bg-[#007CC3]',
    hoverColor: 'hover:border-[#007CC3]',
  },
  {
    id: 'whoop',
    name: 'WHOOP',
    description: 'Recovery & strain tracking',
    brandColor: 'bg-[#1A1A1A]',
    hoverColor: 'hover:border-[#1A1A1A]',
  },
  {
    id: 'terra',
    name: 'Terra',
    description: 'Multi-device aggregation',
    brandColor: 'bg-[#22C55E]',
    hoverColor: 'hover:border-[#22C55E]',
  },
];

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
  onSkip?: () => void;
  isSkipPending?: boolean;
}

export default function ProviderConnectionCards({ onProviderConnected, onSkip, isSkipPending }: ProviderConnectionCardsProps) {
  // Fetch OAuth connection status
  const { data: oauthStatus, isLoading } = useQuery({
    queryKey: ['oauth-status'],
    queryFn: () => apiService.getOAuthStatus(),
    refetchInterval: 5000,
  });

  // Get connection status for a provider
  const isConnected = (providerId: string): boolean => {
    if (!oauthStatus?.providers) return false;
    const provider = oauthStatus.providers.find(p => p.provider === providerId);
    return provider?.connected ?? false;
  };

  // Handle provider card click
  const handleConnect = (providerId: string) => {
    const connected = isConnected(providerId);
    if (connected) return;

    // Navigate to OAuth authorization endpoint
    const authUrl = apiService.getOAuthAuthorizeUrl(providerId);
    window.location.href = authUrl;
  };

  // Check if any provider is connected
  const hasAnyConnection = oauthStatus?.providers?.some(p => p.connected) ?? false;

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

  return (
    <div className="w-full">
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {PROVIDERS.map((provider) => {
          const connected = isConnected(provider.id);
          return (
            <button
              key={provider.id}
              type="button"
              onClick={() => handleConnect(provider.id)}
              disabled={connected}
              className="text-left focus:outline-none focus:ring-2 focus:ring-pierre-violet/50 rounded-xl disabled:cursor-default group"
              aria-label={connected ? `${provider.name} is connected` : `Connect to ${provider.name}`}
            >
              <Card
                className={`p-4 transition-all duration-200 h-full border-2 ${
                  connected
                    ? 'bg-pierre-gray-50/50 border-emerald-200'
                    : `border-transparent ${provider.hoverColor} hover:shadow-lg hover:-translate-y-0.5`
                }`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`w-10 h-10 rounded-xl ${provider.brandColor} flex items-center justify-center text-white shadow-sm`}
                  >
                    <ProviderIcon providerId={provider.id} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-pierre-gray-900 text-sm">{provider.name}</span>
                      {connected && (
                        <Badge variant="success" size="sm">
                          Connected
                        </Badge>
                      )}
                    </div>
                    <p className="text-xs text-pierre-gray-500 mt-0.5">{provider.description}</p>
                  </div>
                  {!connected && (
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

        {/* Skip and start chatting - 6th card */}
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
                    <svg className="w-5 h-5 animate-spin" viewBox="0 0 24 24" fill="none">
                      <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" strokeOpacity="0.25" />
                      <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
                    </svg>
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
