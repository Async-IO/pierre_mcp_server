// ABOUTME: Banner component displayed when server connection is lost
// ABOUTME: Shows reconnection status and provides manual retry button
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect } from 'react';
import { useWebSocketContext } from '../hooks/useWebSocketContext';
import { Button } from './ui';

export default function ConnectionBanner() {
  const { isConnected, reconnect } = useWebSocketContext();
  const [isRetrying, setIsRetrying] = useState(false);
  const [showBanner, setShowBanner] = useState(false);

  // Debounce showing the banner to avoid flashing during brief disconnects
  useEffect(() => {
    let timeout: NodeJS.Timeout;

    if (!isConnected) {
      // Show banner after 2 seconds of disconnection
      timeout = setTimeout(() => {
        setShowBanner(true);
      }, 2000);
    } else {
      // Hide immediately when reconnected
      setShowBanner(false);
      setIsRetrying(false);
    }

    return () => {
      if (timeout) {
        clearTimeout(timeout);
      }
    };
  }, [isConnected]);

  const handleRetry = async () => {
    setIsRetrying(true);
    reconnect();
    // Reset retry state after a delay if still disconnected
    setTimeout(() => {
      setIsRetrying(false);
    }, 3000);
  };

  if (!showBanner) {
    return null;
  }

  return (
    <div className="bg-pierre-red-500 text-white px-4 py-2 sticky top-0 z-50 shadow-lg animate-pulse">
      <div className="max-w-7xl mx-auto flex items-center justify-between">
        <div className="flex items-center gap-3">
          <svg
            className="w-5 h-5 flex-shrink-0"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <span className="font-medium">
            Server connection lost
          </span>
          <span className="text-red-200 text-sm hidden sm:inline">
            {isRetrying ? 'Attempting to reconnect...' : 'Real-time updates unavailable'}
          </span>
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={handleRetry}
          disabled={isRetrying}
          className="bg-white text-pierre-red-600 hover:bg-red-50 border-0 disabled:opacity-50"
        >
          {isRetrying ? (
            <span className="flex items-center gap-2">
              <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              Retrying...
            </span>
          ) : (
            'Retry Now'
          )}
        </Button>
      </div>
    </div>
  );
}
