// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for handling message sending and streaming in chat
// ABOUTME: Manages streaming state, error handling, OAuth URL detection, and message metadata

import { useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { QUERY_KEYS } from '../../constants/queryKeys';

interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  token_count?: number;
  created_at: string;
}

interface MessageMetadata {
  model: string;
  executionTimeMs: number;
}

interface OAuthProvider {
  provider: string;
  connected: boolean;
  last_sync: string | null;
}

interface UseMessageStreamingOptions {
  selectedConversation: string | null;
  connectingProvider: string | null;
  oauthNotification: { provider: string; timestamp: number } | null;
  hasConnectedProvider: boolean;
  messagesData: { messages: Message[] } | undefined;
  oauthStatus: { providers: OAuthProvider[] } | undefined;
  setOauthNotification: (notification: { provider: string; timestamp: number } | null) => void;
  setConnectingProvider: (provider: string | null) => void;
}

interface UseMessageStreamingReturn {
  // State
  newMessage: string;
  isStreaming: boolean;
  streamingContent: string;
  errorMessage: string | null;
  errorCountdown: number | null;
  messageMetadata: Map<string, MessageMetadata>;

  // Setters
  setNewMessage: React.Dispatch<React.SetStateAction<string>>;
  setErrorMessage: React.Dispatch<React.SetStateAction<string | null>>;

  // Handlers
  handleSendMessage: () => Promise<void>;
  handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
}

// List of trusted OAuth provider domains
const TRUSTED_OAUTH_DOMAINS = ['strava.com', 'fitbit.com', 'garmin.com', 'whoop.com', 'coros.com'];

// Check if hostname matches a trusted domain
const isTrustedDomain = (hostname: string): boolean => {
  return TRUSTED_OAUTH_DOMAINS.some(domain =>
    hostname === domain || hostname.endsWith(`.${domain}`)
  );
};

export function useMessageStreaming(options: UseMessageStreamingOptions): UseMessageStreamingReturn {
  const {
    selectedConversation,
    connectingProvider,
    oauthNotification,
    hasConnectedProvider,
    messagesData,
    oauthStatus,
    setOauthNotification,
    setConnectingProvider,
  } = options;

  const queryClient = useQueryClient();

  // State
  const [newMessage, setNewMessage] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [errorCountdown, setErrorCountdown] = useState<number | null>(null);
  const [messageMetadata, setMessageMetadata] = useState<Map<string, MessageMetadata>>(new Map());

  // Parse rate limit countdown from error message
  const parseRateLimitCountdown = useCallback((error: string | null) => {
    if (!error) {
      setErrorCountdown(null);
      return;
    }

    const match = error.match(/in (\d+) seconds/);
    if (match) {
      const seconds = parseInt(match[1], 10);
      setErrorCountdown(seconds);

      // Start countdown timer
      const timer = setInterval(() => {
        setErrorCountdown(prev => {
          if (prev === null || prev <= 1) {
            clearInterval(timer);
            setErrorMessage(null);
            return null;
          }
          return prev - 1;
        });
      }, 1000);
    }
  }, []);

  const handleSendMessage = useCallback(async () => {
    if (!newMessage.trim() || !selectedConversation || isStreaming) return;

    // Store conversation ID if connecting a provider
    if (connectingProvider) {
      localStorage.setItem('pierre_oauth_conversation', selectedConversation);
    }

    const displayContent = newMessage.trim();

    // Add context about connected providers
    let messageContent = displayContent;
    if (oauthNotification) {
      messageContent = `[Context: I just connected my ${oauthNotification.provider} account successfully] ${displayContent}`;
    } else if (hasConnectedProvider && (!messagesData?.messages || messagesData.messages.length === 0)) {
      const connectedProviders = oauthStatus?.providers
        ?.filter(p => p.connected)
        .map(p => p.provider.charAt(0).toUpperCase() + p.provider.slice(1))
        .join(', ');
      if (connectedProviders) {
        messageContent = `[Context: I have ${connectedProviders} connected] ${displayContent}`;
      }
    }

    setNewMessage('');
    setIsStreaming(true);
    setStreamingContent('');
    setErrorMessage(null);
    setOauthNotification(null);

    try {
      // Optimistically add user message to UI
      queryClient.setQueryData(
        QUERY_KEYS.chat.messages(selectedConversation),
        (old: { messages: Message[] } | undefined) => ({
          messages: [
            ...(old?.messages || []),
            {
              id: `temp-${Date.now()}`,
              role: 'user' as const,
              content: displayContent,
              created_at: new Date().toISOString(),
            },
          ],
        })
      );

      // Send message and stream response
      const response = await fetch(`/api/chat/conversations/${selectedConversation}/messages`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ content: messageContent }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ message: 'Unknown error' }));
        throw new Error(errorData.message || errorData.error || 'Failed to send message');
      }

      const responseText = await response.text();
      let fullContent = '';
      let responseModel: string | undefined;
      let responseExecutionTimeMs: number | undefined;
      let assistantMessageId: string | undefined;

      // Try JSON first (non-streaming)
      try {
        const jsonResponse = JSON.parse(responseText);
        if (jsonResponse.assistant_message) {
          fullContent = jsonResponse.assistant_message.content || '';
          assistantMessageId = jsonResponse.assistant_message.id;
          responseModel = jsonResponse.model;
          responseExecutionTimeMs = jsonResponse.execution_time_ms;
          setStreamingContent(fullContent);
        }
      } catch {
        // SSE parsing for streaming responses
        const lines = responseText.split('\n');
        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') continue;

            try {
              const parsed = JSON.parse(data);
              if (parsed.delta) {
                fullContent += parsed.delta;
                setStreamingContent(fullContent);
              }
              if (parsed.type === 'done' && parsed.message) {
                assistantMessageId = parsed.message.id;
                responseModel = parsed.model;
                responseExecutionTimeMs = parsed.execution_time_ms;
              }
            } catch {
              // Skip non-JSON lines
            }
          }
        }
      }

      // Store metadata
      if (assistantMessageId && (responseModel || responseExecutionTimeMs)) {
        setMessageMetadata(prev => {
          const updated = new Map(prev);
          updated.set(assistantMessageId!, {
            model: responseModel || 'unknown',
            executionTimeMs: responseExecutionTimeMs || 0,
          });
          return updated;
        });
      }

      // Refresh messages
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.chat.messages(selectedConversation) });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.chat.conversations() });

      // Auto-redirect to OAuth URL if connecting provider
      if (connectingProvider && fullContent) {
        const oauthUrlMatch = fullContent.match(/https?:\/\/[^\s<>[\]()]+oauth[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*strava\.com[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*fitbit\.com[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*garmin\.com[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*whoop\.com[^\s<>[\]()]*/i);

        if (oauthUrlMatch) {
          console.log(`Auto-redirecting to OAuth URL for ${connectingProvider}`);
          setTimeout(() => {
            try {
              const url = new URL(oauthUrlMatch[0]);
              if (isTrustedDomain(url.hostname) && (url.protocol === 'http:' || url.protocol === 'https:')) {
                window.location.href = url.href;
              } else {
                console.warn('OAuth redirect blocked: URL not from trusted domain');
                setConnectingProvider(null);
              }
            } catch {
              console.warn('OAuth redirect blocked: Invalid URL format');
              setConnectingProvider(null);
            }
          }, 500);
        } else {
          setConnectingProvider(null);
        }
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      const errorMsg = error instanceof Error ? error.message : 'Failed to send message';
      setErrorMessage(errorMsg);
      parseRateLimitCountdown(errorMsg);
      setConnectingProvider(null);
    } finally {
      setIsStreaming(false);
      setStreamingContent('');
    }
  }, [
    newMessage,
    selectedConversation,
    isStreaming,
    queryClient,
    connectingProvider,
    oauthNotification,
    hasConnectedProvider,
    messagesData,
    oauthStatus,
    setOauthNotification,
    setConnectingProvider,
    parseRateLimitCountdown,
  ]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  }, [handleSendMessage]);

  return {
    // State
    newMessage,
    isStreaming,
    streamingContent,
    errorMessage,
    errorCountdown,
    messageMetadata,

    // Setters
    setNewMessage,
    setErrorMessage,

    // Handlers
    handleSendMessage,
    handleKeyDown,
  };
}
