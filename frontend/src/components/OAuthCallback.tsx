// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useEffect } from 'react';

interface OAuthCallbackProps {
  provider: string;
  success: boolean;
  error?: string;
  onClose?: () => void;
}

/**
 * OAuth callback result page shown after OAuth redirect from provider.
 * Displays success or error state and allows user to continue to dashboard.
 */
export default function OAuthCallback({ provider, success, error, onClose }: OAuthCallbackProps) {
  const providerDisplay = provider.charAt(0).toUpperCase() + provider.slice(1);

  // Store OAuth result in localStorage so ChatTab can display connection status
  // Security: Only store non-sensitive data - error messages may contain sensitive info
  useEffect(() => {
    const result = {
      type: 'oauth_completed',
      provider,
      success,
      // Don't store raw error messages in localStorage - they may contain sensitive info
      // Store a generic error indicator instead
      hasError: !success && !!error,
      timestamp: Date.now(),
    };
    localStorage.setItem('pierre_oauth_result', JSON.stringify(result));
  }, [provider, success, error]);


  return (
    <div className="min-h-screen bg-pierre-gray-50 flex items-center justify-center px-4">
      <div className="max-w-md w-full bg-white rounded-xl shadow-lg overflow-hidden">
        {/* Top gradient bar */}
        <div
          className={`h-1 w-full ${
            success
              ? 'bg-gradient-to-r from-pierre-activity to-pierre-activity-dark'
              : 'bg-gradient-to-r from-red-500 to-red-600'
          }`}
        />

        <div className="px-8 py-10 text-center">
          {/* Logo */}
          <div className="mb-6">
            <svg
              width="80"
              height="80"
              viewBox="0 0 120 120"
              xmlns="http://www.w3.org/2000/svg"
              className="mx-auto"
            >
              <defs>
                <linearGradient id="pg" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" style={{ stopColor: '#8B5CF6' }} />
                  <stop offset="100%" style={{ stopColor: '#22D3EE' }} />
                </linearGradient>
                <linearGradient id="ag" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" style={{ stopColor: '#4ADE80' }} />
                  <stop offset="100%" style={{ stopColor: '#059669' }} />
                </linearGradient>
                <linearGradient id="ng" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" style={{ stopColor: '#F59E0B' }} />
                  <stop offset="100%" style={{ stopColor: '#D97706' }} />
                </linearGradient>
                <linearGradient id="rg" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" style={{ stopColor: '#6366F1' }} />
                  <stop offset="100%" style={{ stopColor: '#4F46E5' }} />
                </linearGradient>
              </defs>
              <g strokeWidth="2" opacity="0.5" strokeLinecap="round">
                <line x1="40" y1="30" x2="52" y2="42" stroke="url(#ag)" />
                <line x1="52" y1="42" x2="70" y2="35" stroke="url(#ag)" />
                <line x1="52" y1="42" x2="48" y2="55" stroke="url(#pg)" />
                <line x1="48" y1="55" x2="75" y2="52" stroke="url(#ng)" />
                <line x1="48" y1="55" x2="55" y2="72" stroke="url(#pg)" />
                <line x1="55" y1="72" x2="35" y2="85" stroke="url(#rg)" />
                <line x1="55" y1="72" x2="72" y2="82" stroke="url(#rg)" />
              </g>
              <circle cx="40" cy="30" r="7" fill="url(#ag)" />
              <circle cx="52" cy="42" r="5" fill="url(#ag)" />
              <circle cx="70" cy="35" r="3.5" fill="url(#ag)" />
              <circle cx="48" cy="55" r="6" fill="url(#pg)" />
              <circle cx="48" cy="55" r="3" fill="#fff" opacity="0.9" />
              <circle cx="75" cy="52" r="4.5" fill="url(#ng)" />
              <circle cx="88" cy="60" r="3.5" fill="url(#ng)" />
              <circle cx="55" cy="72" r="5" fill="url(#rg)" />
              <circle cx="35" cy="85" r="4" fill="url(#rg)" />
              <circle cx="72" cy="82" r="4" fill="url(#rg)" />
            </svg>
          </div>

          <div className="text-lg font-bold text-pierre-gray-900 mb-6">Pierre Fitness Intelligence</div>

          {success ? (
            <>
              {/* Success icon */}
              <div className="w-16 h-16 bg-pierre-activity-light rounded-full flex items-center justify-center mx-auto mb-4">
                <svg
                  className="w-8 h-8 text-pierre-activity"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              </div>

              <h1 className="text-xl font-bold text-pierre-activity mb-2">
                {providerDisplay} Connected
              </h1>
              <p className="text-sm text-pierre-gray-600 mb-6">
                Your {providerDisplay} account has been successfully connected to Pierre.
              </p>
            </>
          ) : (
            <>
              {/* Error icon */}
              <div className="w-16 h-16 bg-pierre-red-100 rounded-full flex items-center justify-center mx-auto mb-4">
                <svg
                  className="w-8 h-8 text-pierre-red-500"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </div>

              <h1 className="text-xl font-bold text-pierre-red-600 mb-2">Connection Failed</h1>
              <p className="text-sm text-pierre-gray-600 mb-6">
                {error || `Failed to connect your ${providerDisplay} account. Please try again.`}
              </p>
            </>
          )}

          {onClose ? (
            <button
              onClick={onClose}
              className="btn-primary w-full"
            >
              Continue to Dashboard
            </button>
          ) : (
            <p className="text-xs text-pierre-gray-500">
              You can close this tab and return to your conversation.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
