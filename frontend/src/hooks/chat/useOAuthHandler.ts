// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for handling OAuth completion flows in chat
// ABOUTME: Detects OAuth callbacks, restores conversation state, manages notifications

import { useState, useEffect, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { oauthApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';

interface OAuthNotification {
  provider: string;
  timestamp: number;
}

interface PendingCoachAction {
  prompt: string;
  systemPrompt?: string;
}

interface OAuthData {
  result: { provider: string };
  savedConversation: string | null;
  savedCoachAction: PendingCoachAction | null;
}

interface UseOAuthHandlerReturn {
  // State
  oauthNotification: OAuthNotification | null;
  connectingProvider: string | null;
  pendingCoachAction: PendingCoachAction | null;

  // Setters
  setOauthNotification: React.Dispatch<React.SetStateAction<OAuthNotification | null>>;
  setConnectingProvider: React.Dispatch<React.SetStateAction<string | null>>;
  setPendingCoachAction: React.Dispatch<React.SetStateAction<PendingCoachAction | null>>;

  // Handlers
  handleConnectProvider: (providerName: string) => Promise<void>;

  // Callback to be invoked when OAuth completes
  onOAuthComplete: (
    setSelectedConversation: (id: string | null) => void,
    setPendingPrompt: (prompt: string | null) => void,
    setPendingSystemPrompt: (prompt: string | null) => void,
    createConversation: { mutate: (systemPrompt?: string) => void }
  ) => void;
}

export function useOAuthHandler(): UseOAuthHandlerReturn {
  const queryClient = useQueryClient();

  // State
  const [oauthNotification, setOauthNotification] = useState<OAuthNotification | null>(null);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);
  const [pendingCoachAction, setPendingCoachAction] = useState<PendingCoachAction | null>(null);

  // OAuth completion callback
  const [oauthCallbackData, setOAuthCallbackData] = useState<{
    setSelectedConversation: (id: string | null) => void;
    setPendingPrompt: (prompt: string | null) => void;
    setPendingSystemPrompt: (prompt: string | null) => void;
    createConversation: { mutate: (systemPrompt?: string) => void };
  } | null>(null);

  // Register callback for OAuth completion
  const onOAuthComplete = useCallback((
    setSelectedConversation: (id: string | null) => void,
    setPendingPrompt: (prompt: string | null) => void,
    setPendingSystemPrompt: (prompt: string | null) => void,
    createConversation: { mutate: (systemPrompt?: string) => void }
  ) => {
    setOAuthCallbackData({ setSelectedConversation, setPendingPrompt, setPendingSystemPrompt, createConversation });
  }, []);

  // Handle connecting to a provider
  const handleConnectProvider = useCallback(async (providerName: string) => {
    setConnectingProvider(providerName);

    // If we have a pending coach action, store it for after OAuth completes
    if (pendingCoachAction) {
      localStorage.setItem('pierre_pending_coach_action', JSON.stringify(pendingCoachAction));
    }

    try {
      const providerId = providerName.toLowerCase();
      const authUrl = await oauthApi.getAuthorizeUrl(providerId);
      // Open OAuth in new tab to avoid security blocks from automated browser detection
      window.open(authUrl, '_blank');
      setConnectingProvider(null);
    } catch (error) {
      console.error('Failed to get OAuth authorization URL:', error);
      setConnectingProvider(null);
    }
  }, [pendingCoachAction]);

  // Listen for OAuth completion from popup/new tab
  useEffect(() => {
    if (!oauthCallbackData) return;

    let isProcessingOAuth = false;

    // Process OAuth result - extracts and removes localStorage items atomically
    const extractOAuthData = (): OAuthData | null => {
      const stored = localStorage.getItem('pierre_oauth_result');
      if (!stored) return null;

      // Remove immediately to prevent duplicate processing
      localStorage.removeItem('pierre_oauth_result');

      try {
        const result = JSON.parse(stored);
        const fiveMinutesAgo = Date.now() - 5 * 60 * 1000;

        if (result.type === 'oauth_completed' && result.success && result.timestamp > fiveMinutesAgo) {
          const savedConversation = localStorage.getItem('pierre_oauth_conversation');
          const savedCoachAction = localStorage.getItem('pierre_pending_coach_action');

          // Remove immediately
          if (savedConversation) localStorage.removeItem('pierre_oauth_conversation');
          if (savedCoachAction) localStorage.removeItem('pierre_pending_coach_action');

          return {
            result,
            savedConversation,
            savedCoachAction: savedCoachAction ? JSON.parse(savedCoachAction) : null,
          };
        } else if (result.timestamp <= fiveMinutesAgo) {
          // Clean up stale entries
          localStorage.removeItem('pierre_oauth_conversation');
          localStorage.removeItem('pierre_pending_coach_action');
        }
      } catch {
        // Ignore parse errors
      }
      return null;
    };

    // Process the extracted OAuth data
    const processOAuthData = (data: OAuthData) => {
      if (isProcessingOAuth) return;
      isProcessingOAuth = true;

      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.oauth.status() });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.user.profile() });

      // Show visible notification in chat
      const providerDisplay = data.result.provider.charAt(0).toUpperCase() + data.result.provider.slice(1);
      setOauthNotification({ provider: providerDisplay, timestamp: Date.now() });
      setConnectingProvider(null);

      // Restore the conversation that was active before OAuth redirect
      if (data.savedConversation) {
        oauthCallbackData.setSelectedConversation(data.savedConversation);
      }

      // Restore pending coach action and create conversation
      if (data.savedCoachAction) {
        oauthCallbackData.setPendingPrompt(data.savedCoachAction.prompt);
        if (data.savedCoachAction.systemPrompt) {
          oauthCallbackData.setPendingSystemPrompt(data.savedCoachAction.systemPrompt);
        }
        oauthCallbackData.createConversation.mutate(data.savedCoachAction.systemPrompt);
      }

      // Reset flag after a short delay
      setTimeout(() => {
        isProcessingOAuth = false;
      }, 500);
    };

    const checkAndProcessOAuthResult = () => {
      const data = extractOAuthData();
      if (data) {
        processOAuthData(data);
      }
    };

    const handleOAuthMessage = (event: MessageEvent) => {
      if (event.data?.type === 'oauth_completed') {
        const { provider, success } = event.data;
        if (success && !isProcessingOAuth) {
          const savedConversation = localStorage.getItem('pierre_oauth_conversation');
          const savedCoachActionStr = localStorage.getItem('pierre_pending_coach_action');

          if (savedConversation) localStorage.removeItem('pierre_oauth_conversation');
          if (savedCoachActionStr) localStorage.removeItem('pierre_pending_coach_action');

          let savedCoachAction = null;
          if (savedCoachActionStr) {
            try {
              savedCoachAction = JSON.parse(savedCoachActionStr);
            } catch {
              // Ignore parse errors
            }
          }

          processOAuthData({
            result: { provider },
            savedConversation,
            savedCoachAction,
          });
        }
      }
    };

    const handleStorageChange = (event: StorageEvent) => {
      if (event.key === 'pierre_oauth_result' && event.newValue) {
        const data = extractOAuthData();
        if (data) {
          processOAuthData(data);
        }
      }
    };

    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        checkAndProcessOAuthResult();
      }
    };

    const handleFocus = () => {
      checkAndProcessOAuthResult();
    };

    window.addEventListener('message', handleOAuthMessage);
    window.addEventListener('storage', handleStorageChange);
    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleFocus);

    // Check on mount
    checkAndProcessOAuthResult();

    return () => {
      window.removeEventListener('message', handleOAuthMessage);
      window.removeEventListener('storage', handleStorageChange);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleFocus);
    };
  }, [queryClient, oauthCallbackData]);

  return {
    // State
    oauthNotification,
    connectingProvider,
    pendingCoachAction,

    // Setters
    setOauthNotification,
    setConnectingProvider,
    setPendingCoachAction,

    // Handlers
    handleConnectProvider,
    onOAuthComplete,
  };
}
