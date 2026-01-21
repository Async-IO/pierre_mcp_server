// ABOUTME: AI Chat tab component for users to interact with fitness AI assistant
// ABOUTME: Features Claude.ai-style two-column layout with sidebar and chat area
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Panel, Group as PanelGroup, Separator as PanelResizeHandle, usePanelRef } from 'react-resizable-panels';
import { ConfirmDialog } from './ui';
import { clsx } from 'clsx';
import { apiService } from '../services/api';
import Markdown from 'react-markdown';
import PromptSuggestions from './PromptSuggestions';
import ProviderConnectionCards from './ProviderConnectionCards';
import { useAuth } from '../hooks/useAuth';

// Convert plain URLs to markdown links with friendly display names
// Matches http/https URLs that aren't already in markdown link format
const urlRegex = /(?<!\]\()(?<!\[)(https?:\/\/[^\s<>[\]()]+)/g;

// Security: Check if hostname matches a trusted OAuth provider domain
// Uses endsWith to prevent subdomain bypass attacks (e.g., strava.com.evil.com)
const isTrustedOAuthDomain = (hostname: string, domain: string): boolean => {
  // Exact match or subdomain of the trusted domain
  return hostname === domain || hostname.endsWith(`.${domain}`);
};

// Generate a friendly display name for a URL
const getFriendlyUrlName = (url: string): string => {
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

// Also match existing markdown links where the text is a URL: [url](url)
const markdownLinkRegex = /\[(https?:\/\/[^\]]+)\]\((https?:\/\/[^)]+)\)/g;

const linkifyUrls = (text: string): string => {
  // First, replace existing markdown links that have URL as text with friendly names
  let result = text.replace(markdownLinkRegex, (_match, _linkText, href) => {
    return `[${getFriendlyUrlName(href)}](${href})`;
  });
  // Then, convert any remaining plain URLs to markdown links
  result = result.replace(urlRegex, (url) => `[${getFriendlyUrlName(url)}](${url})`);
  return result;
};

// Strip internal context prefixes from messages before displaying to user
const stripContextPrefix = (text: string): string => {
  return text.replace(/^\[Context:[^\]]*\]\s*/i, '');
};

interface Conversation {
  id: string;
  title: string;
  model: string;
  system_prompt?: string;
  total_tokens: number;
  message_count: number;
  created_at: string;
  updated_at: string;
}

interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  token_count?: number;
  created_at: string;
}

interface ConversationListResponse {
  conversations: Conversation[];
  total: number;
}

interface Coach {
  id: string;
  title: string;
  description: string | null;
  system_prompt: string;
  category: string;
  tags: string[];
  token_count: number;
  is_favorite: boolean;
  use_count: number;
  last_used_at: string | null;
  is_system: boolean;
  is_assigned: boolean;
}

interface ChatTabProps {
  onOpenSettings?: () => void;
}

export default function ChatTab({ onOpenSettings }: ChatTabProps) {
  const queryClient = useQueryClient();
  const { user } = useAuth();
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [newMessage, setNewMessage] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [errorCountdown, setErrorCountdown] = useState<number | null>(null);
  const [editingTitle, setEditingTitle] = useState<string | null>(null);
  const [editedTitleValue, setEditedTitleValue] = useState('');
  const [oauthNotification, setOauthNotification] = useState<{ provider: string; timestamp: number } | null>(null);
  const [deleteConfirmation, setDeleteConfirmation] = useState<{ id: string; title: string } | null>(null);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);
  const [pendingSystemPrompt, setPendingSystemPrompt] = useState<string | null>(null);
  const [showIdeas, setShowIdeas] = useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);
  const [showProviderModal, setShowProviderModal] = useState(false);
  const [pendingCoachAction, setPendingCoachAction] = useState<{ prompt: string; systemPrompt?: string } | null>(null);
  // Track model and execution time for assistant messages (for debugging/transparency)
  const [messageMetadata, setMessageMetadata] = useState<Map<string, { model: string; executionTimeMs: number }>>(new Map());
  // Coach CRUD state
  const [showCoachModal, setShowCoachModal] = useState(false);
  const [showMyCoachesPanel, setShowMyCoachesPanel] = useState(false);
  const [coachesCategoryFilter, setCoachesCategoryFilter] = useState<string | null>(null);
  const [showHiddenCoaches, setShowHiddenCoaches] = useState(false);
  const [editingCoachId, setEditingCoachId] = useState<string | null>(null);
  const [coachFormData, setCoachFormData] = useState({
    title: '',
    description: '',
    system_prompt: '',
    category: 'Training',
  });
  const [coachDeleteConfirmation, setCoachDeleteConfirmation] = useState<{ id: string; title: string } | null>(null);

  const sidebarPanelRef = usePanelRef();
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const titleInputRef = useRef<HTMLInputElement>(null);

  // Fetch conversations
  const { data: conversationsData, isLoading: conversationsLoading } = useQuery<ConversationListResponse>({
    queryKey: ['chat-conversations'],
    queryFn: () => apiService.getConversations(),
  });

  // Fetch OAuth status to determine if any providers are connected
  const { data: oauthStatus } = useQuery({
    queryKey: ['oauth-status'],
    queryFn: () => apiService.getOAuthStatus(),
  });

  // Check if any provider is connected
  const hasConnectedProvider = oauthStatus?.providers?.some(p => p.connected) ?? false;

  // Fetch coaches for My Coaches panel
  const { data: coachesData, isLoading: coachesLoading } = useQuery({
    queryKey: ['user-coaches'],
    queryFn: () => apiService.getCoaches(),
    staleTime: 5 * 60 * 1000,
    enabled: showMyCoachesPanel,
  });

  // Fetch hidden coaches for My Coaches panel
  const { data: hiddenCoachesData } = useQuery({
    queryKey: ['hidden-coaches'],
    queryFn: () => apiService.getHiddenCoaches(),
    staleTime: 5 * 60 * 1000,
    enabled: showMyCoachesPanel,
  });

  // Fetch messages for selected conversation
  const { data: messagesData, isLoading: messagesLoading } = useQuery<{ messages: Message[] }>({
    queryKey: ['chat-messages', selectedConversation],
    queryFn: () => apiService.getConversationMessages(selectedConversation!),
    enabled: !!selectedConversation,
  });

  // Create conversation mutation - auto-creates with default title and optional system prompt
  const createConversation = useMutation<{ id: string }, Error, string | void>({
    mutationFn: (systemPrompt) => {
      const now = new Date();
      const defaultTitle = `Chat ${now.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })} ${now.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })}`;
      return apiService.createConversation({
        title: defaultTitle,
        system_prompt: systemPrompt || pendingSystemPrompt || undefined,
      });
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setSelectedConversation(data.id);
      setPendingSystemPrompt(null);
    },
  });

  // Update conversation mutation for renaming
  const updateConversation = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) =>
      apiService.updateConversation(id, { title }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setEditingTitle(null);
      setEditedTitleValue('');
    },
  });

  // Delete conversation mutation
  const deleteConversation = useMutation({
    mutationFn: (id: string) => apiService.deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      if (selectedConversation) {
        setSelectedConversation(null);
      }
    },
  });

  // Create coach mutation
  const createCoach = useMutation({
    mutationFn: (data: typeof coachFormData) => apiService.createCoach(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowCoachModal(false);
      setCoachFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
    },
  });

  // Update coach mutation
  const updateCoach = useMutation({
    mutationFn: ({ id, data }: { id: string; data: typeof coachFormData }) => apiService.updateCoach(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowCoachModal(false);
      setEditingCoachId(null);
      setCoachFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
    },
  });

  // Delete coach mutation
  const deleteCoach = useMutation({
    mutationFn: (id: string) => apiService.deleteCoach(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setCoachDeleteConfirmation(null);
    },
  });

  // Hide coach mutation
  const hideCoach = useMutation({
    mutationFn: (coachId: string) => apiService.hideCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      queryClient.invalidateQueries({ queryKey: ['hidden-coaches'] });
    },
  });

  // Show coach mutation
  const showCoach = useMutation({
    mutationFn: (coachId: string) => apiService.showCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      queryClient.invalidateQueries({ queryKey: ['hidden-coaches'] });
    },
  });

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messagesData?.messages, streamingContent]);

  // Focus input when conversation is selected
  useEffect(() => {
    if (selectedConversation) {
      inputRef.current?.focus();
    }
  }, [selectedConversation]);

  // Listen for OAuth completion from popup/new tab
  // Uses a processing flag to prevent race conditions when multiple events fire
  useEffect(() => {
    // Flag to prevent duplicate processing when multiple events fire simultaneously
    let isProcessingOAuth = false;

    // Process OAuth result - extracts and removes localStorage items atomically
    // Returns the data if found and valid, null otherwise
    const extractOAuthData = () => {
      const stored = localStorage.getItem('pierre_oauth_result');
      if (!stored) return null;

      // Remove immediately to prevent duplicate processing from other handlers
      localStorage.removeItem('pierre_oauth_result');

      try {
        const result = JSON.parse(stored);
        const fiveMinutesAgo = Date.now() - 5 * 60 * 1000;

        if (result.type === 'oauth_completed' && result.success && result.timestamp > fiveMinutesAgo) {
          // Also extract related items atomically
          const savedConversation = localStorage.getItem('pierre_oauth_conversation');
          const savedCoachAction = localStorage.getItem('pierre_pending_coach_action');

          // Remove these immediately too
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

    // Process the extracted OAuth data (called after extraction to avoid races)
    const processOAuthData = (data: { result: { provider: string }; savedConversation: string | null; savedCoachAction: { prompt: string; systemPrompt?: string } | null }) => {
      if (isProcessingOAuth) return;
      isProcessingOAuth = true;

      queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
      queryClient.invalidateQueries({ queryKey: ['user-profile'] });

      // Show visible notification in chat
      const providerDisplay = data.result.provider.charAt(0).toUpperCase() + data.result.provider.slice(1);
      setOauthNotification({ provider: providerDisplay, timestamp: Date.now() });
      setConnectingProvider(null);

      // Restore the conversation that was active before OAuth redirect
      if (data.savedConversation) {
        setSelectedConversation(data.savedConversation);
      }

      // Restore pending coach action and create conversation
      if (data.savedCoachAction) {
        setPendingPrompt(data.savedCoachAction.prompt);
        if (data.savedCoachAction.systemPrompt) {
          setPendingSystemPrompt(data.savedCoachAction.systemPrompt);
        }
        createConversation.mutate(data.savedCoachAction.systemPrompt);
      }

      // Reset flag after a short delay to allow state updates to propagate
      setTimeout(() => {
        isProcessingOAuth = false;
      }, 500);
    };

    // Check localStorage for OAuth result and process if found
    const checkAndProcessOAuthResult = () => {
      const data = extractOAuthData();
      if (data) {
        processOAuthData(data);
      }
    };

    const handleOAuthMessage = (event: MessageEvent) => {
      // Validate message structure
      if (event.data?.type === 'oauth_completed') {
        const { provider, success } = event.data;
        if (success && !isProcessingOAuth) {
          // For postMessage, we don't have localStorage data, so extract what we can
          const savedConversation = localStorage.getItem('pierre_oauth_conversation');
          const savedCoachActionStr = localStorage.getItem('pierre_pending_coach_action');

          // Remove immediately
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
        // The storage event fires, but another handler might have already processed it
        // Try to extract - if extraction returns null, it was already processed
        const data = extractOAuthData();
        if (data) {
          processOAuthData(data);
        }
      }
    };

    // Check when tab becomes visible (user returns from OAuth tab)
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        checkAndProcessOAuthResult();
      }
    };

    // Check when window gains focus (alternative to visibility change)
    const handleFocus = () => {
      checkAndProcessOAuthResult();
    };

    window.addEventListener('message', handleOAuthMessage);
    window.addEventListener('storage', handleStorageChange);
    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleFocus);

    // Also check on mount in case OAuth completed while component was being rendered
    checkAndProcessOAuthResult();

    return () => {
      window.removeEventListener('message', handleOAuthMessage);
      window.removeEventListener('storage', handleStorageChange);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleFocus);
    };
  }, [queryClient]);

  // Handle sending a pending prompt when conversation is ready
  useEffect(() => {
    if (pendingPrompt && selectedConversation && !isStreaming) {
      const promptToSend = pendingPrompt;
      setPendingPrompt(null);
      setNewMessage(promptToSend);
      // Small delay to ensure state is updated before sending
      setTimeout(() => {
        const sendButton = document.querySelector('[aria-label="Send message"]') as HTMLButtonElement;
        sendButton?.click();
      }, 100);
    }
  }, [pendingPrompt, selectedConversation, isStreaming]);

  // Parse rate limit countdown from error message and manage timer
  useEffect(() => {
    if (!errorMessage) {
      setErrorCountdown(null);
      return;
    }

    // Look for "in X seconds" pattern in error message
    const match = errorMessage.match(/in (\d+) seconds/);
    if (match) {
      const seconds = parseInt(match[1], 10);
      setErrorCountdown(seconds);
    }
  }, [errorMessage]);

  // Countdown timer that auto-dismisses when reaching 0
  useEffect(() => {
    if (errorCountdown === null || errorCountdown <= 0) {
      if (errorCountdown === 0) {
        setErrorMessage(null);
        setErrorCountdown(null);
      }
      return;
    }

    const timer = setInterval(() => {
      setErrorCountdown(prev => {
        if (prev === null || prev <= 1) {
          clearInterval(timer);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(timer);
  }, [errorCountdown]);

  const handleSendMessage = useCallback(async () => {
    if (!newMessage.trim() || !selectedConversation || isStreaming) return;

    // Store conversation ID at the START if we're connecting a provider
    // This ensures the ID is saved before any OAuth links appear that user might click
    if (connectingProvider) {
      localStorage.setItem('pierre_oauth_conversation', selectedConversation);
    }

    const displayContent = newMessage.trim();
    // Add context about connected providers to help the LLM
    let messageContent = displayContent;
    if (oauthNotification) {
      // OAuth just completed - mention the newly connected provider
      messageContent = `[Context: I just connected my ${oauthNotification.provider} account successfully] ${displayContent}`;
    } else if (hasConnectedProvider && (!messagesData?.messages || messagesData.messages.length === 0)) {
      // First message in conversation with connected providers - add context
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
    setOauthNotification(null); // Clear OAuth notification when user sends a new message

    try {
      // Optimistically add user message to UI (without context prefix)
      queryClient.setQueryData(['chat-messages', selectedConversation], (old: { messages: Message[] } | undefined) => ({
        messages: [
          ...(old?.messages || []),
          {
            id: `temp-${Date.now()}`,
            role: 'user' as const,
            content: displayContent,
            created_at: new Date().toISOString(),
          },
        ],
      }));

      // Send message and stream response
      const response = await fetch(`/api/chat/conversations/${selectedConversation}/messages`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        credentials: 'include',
        body: JSON.stringify({ content: messageContent }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ message: 'Unknown error' }));
        // Show actual error message from backend
        const userMessage = errorData.message || errorData.error || 'Failed to send message';
        throw new Error(userMessage);
      }

      // Try to read response as JSON first (non-streaming endpoint returns JSON)
      const responseText = await response.text();
      let fullContent = '';
      let responseModel: string | undefined;
      let responseExecutionTimeMs: number | undefined;
      let assistantMessageId: string | undefined;

      // Check if this is a JSON response (non-streaming)
      try {
        const jsonResponse = JSON.parse(responseText);
        if (jsonResponse.assistant_message) {
          // Non-streaming JSON response from send_message endpoint
          fullContent = jsonResponse.assistant_message.content || '';
          assistantMessageId = jsonResponse.assistant_message.id;
          responseModel = jsonResponse.model;
          responseExecutionTimeMs = jsonResponse.execution_time_ms;
          setStreamingContent(fullContent);
        }
      } catch {
        // Not JSON, try SSE parsing for streaming responses
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
              // Capture metadata from done event
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

      // Store model and execution time metadata for display
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

      // Refresh messages after streaming completes
      queryClient.invalidateQueries({ queryKey: ['chat-messages', selectedConversation] });
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });

      // Auto-redirect to OAuth URL if we're connecting a provider
      if (connectingProvider && fullContent) {
        // Look for OAuth URLs in the response
        const oauthUrlMatch = fullContent.match(/https?:\/\/[^\s<>[\]()]+oauth[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*strava\.com[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*fitbit\.com[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*garmin\.com[^\s<>[\]()]*/i) ||
                             fullContent.match(/https?:\/\/[^\s<>[\]()]*whoop\.com[^\s<>[\]()]*/i);
        if (oauthUrlMatch) {
          // Conversation ID was stored at the start of handleSendMessage
          // Security: Don't log the full OAuth URL as it may contain sensitive query parameters
          console.log(`Auto-redirecting to OAuth URL for ${connectingProvider}`);
          // Small delay to let user see the response before redirect
          setTimeout(() => {
            // Security: Validate URL before redirect to prevent open redirect attacks
            try {
              const url = new URL(oauthUrlMatch[0]);
              // Only allow redirects to known OAuth providers
              const trustedDomains = ['strava.com', 'fitbit.com', 'garmin.com', 'whoop.com', 'coros.com'];
              const isTrusted = trustedDomains.some(domain =>
                url.hostname === domain || url.hostname.endsWith(`.${domain}`)
              );
              if (isTrusted && (url.protocol === 'http:' || url.protocol === 'https:')) {
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
          // No OAuth URL found, clear the connecting state
          setConnectingProvider(null);
        }
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      const errorMsg = error instanceof Error ? error.message : 'Failed to send message';
      setErrorMessage(errorMsg);
      setConnectingProvider(null);
    } finally {
      setIsStreaming(false);
      setStreamingContent('');
    }
  }, [newMessage, selectedConversation, isStreaming, queryClient, connectingProvider, oauthNotification, hasConnectedProvider, messagesData, oauthStatus]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  const handleNewChat = () => {
    // Clear selection to show welcome/onboarding screen
    // Conversation is created when user sends a message or clicks a prompt
    setSelectedConversation(null);
  };

  const handleSelectPrompt = (prompt: string, coachIdForTracking?: string, systemPrompt?: string) => {
    // coachIdForTracking is used by PromptSuggestions for usage tracking before calling this
    void coachIdForTracking; // Acknowledge the parameter is intentionally not used here

    // If no provider connected, show modal and store the action for later
    if (!hasConnectedProvider) {
      setPendingCoachAction({ prompt, systemPrompt });
      setShowProviderModal(true);
      return;
    }

    setPendingPrompt(prompt);
    if (systemPrompt) {
      setPendingSystemPrompt(systemPrompt);
    }
    createConversation.mutate(systemPrompt);
  };

  const handleFillPrompt = (prompt: string, coachIdForTracking?: string, systemPrompt?: string) => {
    // coachIdForTracking is used by PromptSuggestions for usage tracking before calling this
    void coachIdForTracking; // Acknowledge the parameter is intentionally not used here
    setNewMessage(prompt);
    if (systemPrompt) {
      setPendingSystemPrompt(systemPrompt);
    }
    setShowIdeas(false);
    inputRef.current?.focus();
  };

  const handleConnectProvider = async (providerName: string) => {
    setConnectingProvider(providerName);
    // If we have a pending coach action, store it for after OAuth completes
    if (pendingCoachAction) {
      // Store in localStorage so it persists through OAuth redirect
      localStorage.setItem('pierre_pending_coach_action', JSON.stringify(pendingCoachAction));
    }
    setShowProviderModal(false);

    try {
      // Convert provider name to lowercase ID (e.g., "Strava" -> "strava")
      const providerId = providerName.toLowerCase();
      const authUrl = await apiService.getOAuthAuthorizeUrl(providerId);
      // Open OAuth in new tab to avoid security blocks from automated browser detection
      window.open(authUrl, '_blank');
      setConnectingProvider(null);
    } catch (error) {
      console.error('Failed to get OAuth authorization URL:', error);
      setConnectingProvider(null);
    }
  };

  // Handle skip in provider modal - proceed with pending action without provider
  const handleProviderModalSkip = () => {
    setShowProviderModal(false);
    if (pendingCoachAction) {
      setPendingPrompt(pendingCoachAction.prompt);
      if (pendingCoachAction.systemPrompt) {
        setPendingSystemPrompt(pendingCoachAction.systemPrompt);
      }
      createConversation.mutate(pendingCoachAction.systemPrompt);
      setPendingCoachAction(null);
    }
  };

  // Handle close provider modal without action
  const handleProviderModalClose = () => {
    setShowProviderModal(false);
    setPendingCoachAction(null);
  };

  const handleStartRename = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setEditingTitle(conv.id);
    setEditedTitleValue(conv.title);
    setTimeout(() => titleInputRef.current?.focus(), 0);
  };

  const handleSaveRename = (convId: string) => {
    if (editedTitleValue.trim() && editedTitleValue.trim() !== conversationsData?.conversations.find(c => c.id === convId)?.title) {
      updateConversation.mutate({ id: convId, title: editedTitleValue.trim() });
    } else {
      setEditingTitle(null);
      setEditedTitleValue('');
    }
  };

  const handleCancelRename = () => {
    setEditingTitle(null);
    setEditedTitleValue('');
  };

  // Toggle sidebar collapse/expand
  const toggleSidebar = useCallback(() => {
    const panel = sidebarPanelRef.current;
    if (panel) {
      if (panel.isCollapsed()) {
        panel.expand();
        setSidebarCollapsed(false);
      } else {
        panel.collapse();
        setSidebarCollapsed(true);
      }
    }
  }, [sidebarPanelRef]);

  const handleDeleteConversation = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setDeleteConfirmation({ id: conv.id, title: conv.title });
  };

  const handleConfirmDelete = () => {
    if (deleteConfirmation) {
      deleteConversation.mutate(deleteConfirmation.id);
      setDeleteConfirmation(null);
    }
  };

  // Coach edit handler - opens modal with coach data pre-filled
  const handleEditCoach = (coach: { id: string; title: string; description: string | null; system_prompt: string; category: string }) => {
    setEditingCoachId(coach.id);
    setCoachFormData({
      title: coach.title,
      description: coach.description || '',
      system_prompt: coach.system_prompt,
      category: coach.category,
    });
    setShowCoachModal(true);
  };

  // Coach delete handler - opens confirmation dialog
  const handleDeleteCoach = (coach: { id: string; title: string }) => {
    setCoachDeleteConfirmation({ id: coach.id, title: coach.title });
  };

  // Confirm coach deletion
  const handleConfirmCoachDelete = () => {
    if (coachDeleteConfirmation) {
      deleteCoach.mutate(coachDeleteConfirmation.id);
    }
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) return 'Today';
    if (days === 1) return 'Yesterday';
    if (days < 7) return `${days} days ago`;
    return date.toLocaleDateString();
  };

  return (
    <PanelGroup
      orientation="horizontal"
      className="h-full"
    >
      {/* Left Sidebar - Conversation List (collapsible) */}
      <Panel
        panelRef={sidebarPanelRef}
        defaultSize="18%"
        minSize="12%"
        maxSize="30%"
        collapsible
        collapsedSize="0%"
        onResize={(size) => setSidebarCollapsed(size.asPercentage === 0)}
        className="bg-pierre-gray-50 flex flex-col relative"
      >
        {/* Quick Action Rows - Mobile-inspired design */}
        <div className="p-3 space-y-1">
          {/* Discussions Row */}
          <button
            onClick={() => {
              setShowMyCoachesPanel(false);
              // Show all conversations (deselect current)
            }}
            className={clsx(
              'w-full px-3 py-2.5 flex items-center gap-3 rounded-lg transition-colors',
              !showMyCoachesPanel && !selectedConversation
                ? 'bg-pierre-violet/10'
                : 'hover:bg-pierre-gray-100'
            )}
          >
            <span className="text-lg">💬</span>
            <span className="flex-1 text-left text-sm font-medium text-pierre-gray-800">Discussions</span>
            <svg className="w-4 h-4 text-pierre-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>

          {/* My Coaches Row */}
          <button
            onClick={() => {
              setSelectedConversation(null);
              setShowMyCoachesPanel(true);
            }}
            title="My Coaches"
            aria-label="My Coaches"
            className={clsx(
              'w-full px-3 py-2.5 flex items-center gap-3 rounded-lg transition-colors',
              showMyCoachesPanel
                ? 'bg-pierre-violet/10'
                : 'hover:bg-pierre-gray-100'
            )}
          >
            <span className="text-lg">🎯</span>
            <span className="flex-1 text-left text-sm font-medium text-pierre-gray-800">My Coaches</span>
            <svg className="w-4 h-4 text-pierre-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>
        </div>

        {/* Recent Chat Header */}
        <div className="px-4 py-2 border-t border-pierre-gray-200">
          <h3 className="text-xs font-semibold text-pierre-gray-500 uppercase tracking-wider">Recents</h3>
        </div>

        {/* Conversation List - with bottom padding for floating bar */}
        <div className="flex-1 overflow-y-auto pb-20">
          {conversationsLoading ? (
            <div className="p-4 text-center text-pierre-gray-500 text-sm">Loading...</div>
          ) : conversationsData?.conversations?.length === 0 ? (
            <div className="p-4 text-center text-pierre-gray-500 text-sm">No conversations yet</div>
          ) : (
            <div className="py-2">
              {conversationsData?.conversations?.map((conv) => (
                <div
                  key={conv.id}
                  onClick={() => {
                    if (editingTitle !== conv.id) {
                      setShowMyCoachesPanel(false);
                      setSelectedConversation(conv.id);
                    }
                  }}
                  className={clsx(
                    'relative px-3 py-2 mx-2 rounded-lg cursor-pointer transition-colors group',
                    selectedConversation === conv.id
                      ? 'bg-white shadow-sm'
                      : 'hover:bg-pierre-gray-100'
                  )}
                >
                  {/* Accent bar for selected state */}
                  {selectedConversation === conv.id && (
                    <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-pierre-violet rounded-r-full" />
                  )}
                  <div className="flex items-center justify-between">
                    <div className="flex-1 min-w-0 pl-1">
                      {editingTitle === conv.id ? (
                        <input
                          ref={titleInputRef}
                          type="text"
                          value={editedTitleValue}
                          onChange={(e) => setEditedTitleValue(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') handleSaveRename(conv.id);
                            if (e.key === 'Escape') handleCancelRename();
                          }}
                          onBlur={() => handleSaveRename(conv.id)}
                          className="w-full text-sm font-medium text-pierre-gray-800 bg-white border border-pierre-violet rounded px-2 py-0.5 focus:outline-none focus:ring-1 focus:ring-pierre-violet"
                          onClick={(e) => e.stopPropagation()}
                        />
                      ) : (
                        <p className="text-sm font-medium text-pierre-gray-800 truncate">
                          {conv.title}
                        </p>
                      )}
                      <p className="text-xs text-pierre-gray-500">{formatDate(conv.updated_at)}</p>
                    </div>
                    <div className="flex items-center gap-1">
                      {/* Rename button */}
                      <button
                        onClick={(e) => handleStartRename(e, conv)}
                        className="opacity-0 group-hover:opacity-100 text-pierre-gray-400 hover:text-pierre-violet transition-all p-1"
                        title="Rename"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                        </svg>
                      </button>
                      {/* Delete button */}
                      <button
                        onClick={(e) => handleDeleteConversation(e, conv)}
                        className="opacity-0 group-hover:opacity-100 text-pierre-gray-400 hover:text-pierre-red-500 transition-all p-1"
                        title="Delete"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Floating Bottom Bar - User Profile + New Chat FAB */}
        <div className="absolute bottom-3 left-3 right-3 bg-white rounded-xl shadow-lg border border-pierre-gray-200 px-3 py-2 flex items-center justify-between">
          {/* User Profile Button */}
          <button
            onClick={onOpenSettings}
            className="flex items-center gap-2 px-2 py-1 rounded-full bg-pierre-gray-100 hover:bg-pierre-gray-200 transition-colors group"
            title="Open settings"
          >
            {/* User Avatar */}
            <div className="relative flex-shrink-0">
              <div className="w-7 h-7 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-full flex items-center justify-center">
                <span className="text-xs font-bold text-white">
                  {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
                </span>
              </div>
              {/* Online status dot */}
              <div className="absolute -bottom-0.5 -right-0.5 w-2 h-2 bg-pierre-green-500 rounded-full border-2 border-white" />
            </div>
            <span className="text-sm font-medium text-pierre-gray-800 truncate max-w-[100px] group-hover:text-pierre-violet transition-colors">
              {user?.display_name || 'User'}
            </span>
          </button>

          {/* New Chat FAB */}
          <button
            onClick={() => {
              setShowMyCoachesPanel(false);
              handleNewChat();
            }}
            disabled={createConversation.isPending}
            title="New conversation"
            aria-label="New Chat"
            className="w-10 h-10 flex items-center justify-center rounded-full bg-gradient-to-br from-pierre-violet to-pierre-recovery-dark text-white shadow-md hover:shadow-lg hover:scale-105 transition-all disabled:opacity-50 disabled:hover:scale-100"
          >
            {createConversation.isPending ? (
              <div className="pierre-spinner w-4 h-4 border-white border-t-transparent"></div>
            ) : (
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M12 4v16m8-8H4" />
              </svg>
            )}
          </button>
        </div>
      </Panel>

      {/* Resize Handle with Toggle Button */}
      <PanelResizeHandle className="w-2 bg-pierre-gray-200 hover:bg-pierre-violet/50 transition-colors relative group">
        {/* Toggle button - appears on hover or when collapsed */}
        <button
          onClick={toggleSidebar}
          className={clsx(
            'absolute top-3 -left-3 w-6 h-6 rounded-full bg-white border border-pierre-gray-200 shadow-sm flex items-center justify-center text-pierre-gray-500 hover:text-pierre-violet hover:border-pierre-violet transition-all z-10',
            sidebarCollapsed ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
          )}
          title={sidebarCollapsed ? 'Show sidebar' : 'Hide sidebar'}
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            {sidebarCollapsed ? (
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            ) : (
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            )}
          </svg>
        </button>
      </PanelResizeHandle>

      {/* Main Chat Area */}
      <Panel defaultSize="82%" className="flex flex-col bg-white">
        {/* My Coaches View - shown when My Coaches button is clicked */}
        {showMyCoachesPanel && !selectedConversation ? (
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Header */}
            <div className="p-6 border-b border-pierre-gray-200 flex items-center justify-between flex-shrink-0">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 flex items-center justify-center rounded-xl bg-gradient-to-br from-pierre-violet to-pierre-recovery-dark text-white shadow-sm">
                  <span className="text-lg">🎯</span>
                </div>
                <div>
                  <h2 className="text-xl font-semibold text-pierre-gray-900">My Coaches</h2>
                  <p className="text-sm text-pierre-gray-500">Select a coach to start chatting</p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {/* Add Coach Button */}
                <button
                  onClick={() => {
                    setShowMyCoachesPanel(false);
                    setEditingCoachId(null);
                    setCoachFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
                    setShowCoachModal(true);
                  }}
                  className="flex items-center gap-1.5 px-4 py-2 text-sm font-medium text-white bg-pierre-violet rounded-lg hover:bg-pierre-violet/90 transition-colors"
                  title="Add coach"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                  </svg>
                  Add Coach
                </button>
                {/* Back button */}
                <button
                  onClick={() => {
                    setShowMyCoachesPanel(false);
                    setCoachesCategoryFilter(null);
                    setShowHiddenCoaches(false);
                  }}
                  className="p-2 text-pierre-gray-400 hover:text-pierre-gray-600 hover:bg-pierre-gray-100 rounded-lg transition-colors"
                  aria-label="Back"
                  title="Back to home"
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>

            {/* Category Filters */}
            <div className="px-6 py-4 border-b border-pierre-gray-100 flex-shrink-0">
              <div className="flex items-center gap-2 overflow-x-auto pb-1">
                <button
                  onClick={() => setCoachesCategoryFilter(null)}
                  className={clsx(
                    'px-4 py-2 text-sm font-medium rounded-full whitespace-nowrap transition-colors',
                    coachesCategoryFilter === null
                      ? 'bg-pierre-violet text-white'
                      : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
                  )}
                >
                  All
                </button>
                {['Training', 'Nutrition', 'Recovery', 'Recipes', 'Mobility', 'Analysis', 'Custom'].map((category) => (
                  <button
                    key={category}
                    onClick={() => setCoachesCategoryFilter(category)}
                    className={clsx(
                      'px-4 py-2 text-sm font-medium rounded-full whitespace-nowrap transition-colors flex items-center gap-1.5',
                      coachesCategoryFilter === category
                        ? 'bg-pierre-violet text-white'
                        : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
                    )}
                  >
                    <span>{getCategoryIcon(category)}</span>
                    {category}
                  </button>
                ))}
              </div>
            </div>

            {/* Coaches List */}
            <div className="flex-1 overflow-y-auto p-6">
              {coachesLoading ? (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {[1, 2, 3, 4, 5, 6].map((i) => (
                    <div key={i} className="h-24 bg-pierre-gray-100 rounded-xl animate-pulse" />
                  ))}
                </div>
              ) : (
                <>
                  {/* Coaches organized: User coaches first, then System coaches by category */}
                  {(() => {
                    const coaches: Coach[] = coachesData?.coaches || [];
                    const filteredCoaches = coachesCategoryFilter
                      ? coaches.filter((c: Coach) => c.category.toLowerCase() === coachesCategoryFilter.toLowerCase())
                      : coaches;

                    // Separate user coaches from system coaches
                    const userCoaches = filteredCoaches.filter((c: Coach) => !c.is_system);
                    const systemCoaches = filteredCoaches.filter((c: Coach) => c.is_system);

                    // Sort each group by favorites then use count
                    const sortByUsage = (a: Coach, b: Coach) => {
                      if (a.is_favorite !== b.is_favorite) return a.is_favorite ? -1 : 1;
                      return b.use_count - a.use_count;
                    };
                    const sortedUserCoaches = [...userCoaches].sort(sortByUsage);
                    const sortedSystemCoaches = [...systemCoaches].sort(sortByUsage);

                    if (sortedUserCoaches.length === 0 && sortedSystemCoaches.length === 0 && !showHiddenCoaches) {
                      return (
                        <div className="text-center py-12 text-pierre-gray-500">
                          <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-pierre-gray-100 flex items-center justify-center">
                            <span className="text-2xl">🎯</span>
                          </div>
                          <p className="text-base font-medium">No coaches found</p>
                          <p className="text-sm mt-1">
                            {coachesCategoryFilter ? 'Try selecting a different category' : 'Add a coach to get started'}
                          </p>
                        </div>
                      );
                    }

                    // Helper to render a coach card
                    const renderCoachCard = (coach: Coach) => (
                      <MyCoachCard
                        key={coach.id}
                        coach={coach}
                        onSelect={() => {
                          apiService.recordCoachUsage(coach.id).catch(() => {});
                          setShowMyCoachesPanel(false);
                          setPendingSystemPrompt(coach.system_prompt);
                          setPendingPrompt(coach.description || `Chat with ${coach.title}`);
                          handleNewChat();
                        }}
                        onEdit={() => {
                          setShowMyCoachesPanel(false);
                          setEditingCoachId(coach.id);
                          setCoachFormData({
                            title: coach.title,
                            description: coach.description || '',
                            system_prompt: coach.system_prompt,
                            category: coach.category,
                          });
                          setShowCoachModal(true);
                        }}
                        onDelete={() => {
                          setCoachDeleteConfirmation({ id: coach.id, title: coach.title });
                        }}
                        onHide={() => hideCoach.mutate(coach.id)}
                        isHiding={hideCoach.isPending}
                      />
                    );

                    // Group system coaches by category
                    const groupedSystemCoaches = sortedSystemCoaches.reduce<Record<string, Coach[]>>((acc, coach) => {
                      const cat = coach.category;
                      if (!acc[cat]) acc[cat] = [];
                      acc[cat].push(coach);
                      return acc;
                    }, {});

                    return (
                      <div className="space-y-8">
                        {/* Personalized (user-created) - always first */}
                        {sortedUserCoaches.length > 0 && (
                          <div>
                            <h3 className="text-sm font-semibold text-pierre-gray-700 mb-3 flex items-center gap-2">
                              <span className="text-lg">✨</span>
                              Personalized
                              <span className="text-xs text-pierre-gray-400 font-normal">({sortedUserCoaches.length})</span>
                            </h3>
                            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                              {sortedUserCoaches.map(renderCoachCard)}
                            </div>
                          </div>
                        )}

                        {/* System Coaches - grouped by category */}
                        {Object.keys(groupedSystemCoaches).length > 0 && (
                          <div className={sortedUserCoaches.length > 0 ? 'pt-4 border-t border-pierre-gray-200' : ''}>
                            <h3 className="text-sm font-semibold text-pierre-gray-500 mb-4 flex items-center gap-2">
                              <span className="text-lg">🏛️</span>
                              System Coaches
                            </h3>
                            <div className="space-y-6">
                              {Object.entries(groupedSystemCoaches).map(([category, categoryCoaches]) => (
                                <div key={category}>
                                  <h4 className="text-xs font-medium text-pierre-gray-500 mb-2 flex items-center gap-1.5 uppercase tracking-wider">
                                    <span>{getCategoryIcon(category)}</span>
                                    {category}
                                    <span className="text-pierre-gray-400 font-normal normal-case">({categoryCoaches.length})</span>
                                  </h4>
                                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                    {categoryCoaches.map(renderCoachCard)}
                                  </div>
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                      </div>
                    );
                  })()}

                  {/* Hidden Coaches Toggle */}
                  {(hiddenCoachesData?.coaches?.length ?? 0) > 0 && (
                    <div className="mt-8 pt-6 border-t border-pierre-gray-200">
                      <button
                        onClick={() => setShowHiddenCoaches(!showHiddenCoaches)}
                        className={clsx(
                          'flex items-center gap-2 text-sm font-medium transition-colors',
                          showHiddenCoaches
                            ? 'text-pierre-violet'
                            : 'text-pierre-gray-500 hover:text-pierre-gray-700'
                        )}
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          {showHiddenCoaches ? (
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                          ) : (
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                          )}
                        </svg>
                        {showHiddenCoaches ? 'Hide' : 'Show'} hidden coaches ({hiddenCoachesData?.coaches?.length})
                      </button>

                      {showHiddenCoaches && (
                        <div className="mt-4 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                          {hiddenCoachesData?.coaches?.map((coach: Coach) => (
                            <div
                              key={coach.id}
                              className="relative text-left text-sm rounded-xl border border-pierre-gray-200 px-4 py-3 opacity-60 hover:opacity-100 transition-all group bg-pierre-gray-50"
                            >
                              <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                <button
                                  onClick={() => showCoach.mutate(coach.id)}
                                  disabled={showCoach.isPending}
                                  className="p-1.5 text-pierre-gray-400 hover:text-pierre-green-600 hover:bg-pierre-green-50 rounded-lg transition-colors disabled:opacity-50"
                                  title="Show coach"
                                  aria-label="Show coach"
                                >
                                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                                  </svg>
                                </button>
                              </div>
                              <div className="flex items-center justify-between">
                                <span className="font-medium text-pierre-gray-600">{coach.title}</span>
                                <span className={clsx('text-xs px-1.5 py-0.5 rounded', getCategoryBadgeClass(coach.category))}>
                                  {getCategoryIcon(coach.category)}
                                </span>
                              </div>
                              {coach.description && (
                                <p className="text-pierre-gray-400 text-xs mt-1 line-clamp-2">{coach.description}</p>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        ) : !selectedConversation ? (
          <div className="flex-1 flex items-center justify-center overflow-y-auto py-8">
            <div className="w-full max-w-5xl px-6">
              <div className="text-center mb-8">
                {/* Show connection badge only if provider is connected */}
                {hasConnectedProvider ? (
                  <div className="inline-flex items-center gap-2 px-3 py-1.5 bg-pierre-activity/10 text-pierre-activity text-sm font-medium rounded-full mb-4">
                    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                    </svg>
                    {oauthStatus?.providers?.filter(p => p.connected).map(p =>
                      p.provider.charAt(0).toUpperCase() + p.provider.slice(1)
                    ).join(', ')} connected
                  </div>
                ) : (
                  <div className="inline-flex items-center gap-2 px-3 py-1.5 bg-pierre-gray-100 text-pierre-gray-600 text-sm font-medium rounded-full mb-4">
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                    No provider connected
                  </div>
                )}
                <h2 className="text-2xl font-semibold text-pierre-gray-900 mb-2">
                  Ready to analyze your fitness
                </h2>
                <p className="text-pierre-gray-500 text-sm">
                  {hasConnectedProvider
                    ? 'Get personalized insights from your activity data'
                    : 'Select a coach to get started - connect your data anytime'}
                </p>
              </div>

              {/* Coach selection */}
              <PromptSuggestions
                onSelectPrompt={handleSelectPrompt}
                onEditCoach={handleEditCoach}
                onDeleteCoach={handleDeleteCoach}
              />

              <div className="mt-8 text-center">
                <button
                  type="button"
                  onClick={handleNewChat}
                  disabled={createConversation.isPending}
                  className="text-pierre-gray-500 hover:text-pierre-violet text-sm font-medium transition-colors"
                >
                  Start a blank conversation
                </button>
              </div>
            </div>
          </div>
        ) : (
          <div className="h-full flex flex-col">
            {/* Messages Area */}
            <div className="flex-1 overflow-y-auto min-h-0">
              <div className="max-w-3xl mx-auto py-6 px-6">
                {messagesLoading ? (
                  <div className="text-center text-pierre-gray-500 py-8 text-sm">Loading messages...</div>
                ) : (
                  <div className="space-y-6">
                    {messagesData?.messages?.map((msg) => (
                      <div key={msg.id} className="flex gap-3">
                        {/* Avatar */}
                        <div className="flex-shrink-0">
                          {msg.role === 'user' ? (
                            <div className="w-8 h-8 rounded-full bg-pierre-gray-200 flex items-center justify-center">
                              <svg className="w-4 h-4 text-pierre-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                              </svg>
                            </div>
                          ) : (
                            <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
                          )}
                        </div>
                        {/* Message Content */}
                        <div className="flex-1 min-w-0 pt-1">
                          <div className="font-medium text-pierre-gray-900 text-sm mb-1">
                            {msg.role === 'user' ? 'You' : 'Pierre'}
                          </div>
                          <div className="text-pierre-gray-700 text-sm leading-relaxed prose prose-sm max-w-none prose-a:text-pierre-violet prose-a:underline hover:prose-a:text-pierre-violet/80">
                            <Markdown
                              components={{
                                a: ({ href, children }) => (
                                  <a href={href} target="_blank" rel="noopener noreferrer" className="break-all">
                                    {children}
                                  </a>
                                ),
                              }}
                            >
                              {linkifyUrls(stripContextPrefix(msg.content))}
                            </Markdown>
                          </div>
                          {/* Model and execution time metadata for assistant messages */}
                          {msg.role === 'assistant' && messageMetadata.get(msg.id) && (
                            <div className="mt-2 text-xs text-pierre-gray-400 flex items-center gap-2">
                              <span className="inline-flex items-center gap-1">
                                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                                </svg>
                                {messageMetadata.get(msg.id)?.model}
                              </span>
                              <span className="inline-flex items-center gap-1">
                                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                                {((messageMetadata.get(msg.id)?.executionTimeMs || 0) / 1000).toFixed(1)}s
                              </span>
                            </div>
                          )}
                        </div>
                      </div>
                    ))}

                    {/* OAuth connection notification */}
                    {oauthNotification && (
                      <div className="flex gap-3 animate-fadeIn">
                        <div className="flex-shrink-0">
                          <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
                        </div>
                        <div className="flex-1 min-w-0 pt-1">
                          <div className="font-medium text-pierre-gray-900 text-sm mb-1 flex items-center gap-2">
                            Pierre
                            <button
                              onClick={() => setOauthNotification(null)}
                              className="text-pierre-gray-400 hover:text-pierre-gray-600"
                            >
                              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                              </svg>
                            </button>
                          </div>
                          <div className="text-pierre-gray-700 text-sm leading-relaxed">
                            {oauthNotification.provider} connected successfully. You can now access your {oauthNotification.provider} data.
                          </div>
                        </div>
                      </div>
                    )}

                    {/* Streaming response */}
                    {isStreaming && streamingContent && (
                      <div className="flex gap-3">
                        <div className="flex-shrink-0">
                          <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
                        </div>
                        <div className="flex-1 min-w-0 pt-1">
                          <div className="font-medium text-pierre-gray-900 text-sm mb-1 flex items-center gap-2">
                            Pierre
                            <span className="w-1.5 h-1.5 bg-pierre-violet rounded-full animate-pulse" />
                          </div>
                          <div className="text-pierre-gray-700 text-sm leading-relaxed prose prose-sm max-w-none prose-a:text-pierre-violet prose-a:underline hover:prose-a:text-pierre-violet/80">
                            <Markdown
                              components={{
                                a: ({ href, children }) => (
                                  <a href={href} target="_blank" rel="noopener noreferrer" className="break-all">
                                    {children}
                                  </a>
                                ),
                              }}
                            >
                              {linkifyUrls(streamingContent)}
                            </Markdown>
                          </div>
                        </div>
                      </div>
                    )}

                    {/* Thinking/Loading indicator - Claude Code style spinner */}
                    {isStreaming && !streamingContent && (
                      <div className="flex gap-3">
                        <div className="flex-shrink-0">
                          <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
                        </div>
                        <div className="flex-1 pt-1">
                          <div className="font-medium text-pierre-gray-900 text-sm mb-2 flex items-center gap-2">
                            Pierre
                          </div>
                          <div className="flex items-center gap-2 text-pierre-gray-500 text-sm">
                            <div className="pierre-spinner w-4 h-4"></div>
                            <span>Thinking...</span>
                          </div>
                        </div>
                      </div>
                    )}

                    {/* Error message display */}
                    {errorMessage && !isStreaming && (
                      <div className="flex gap-3">
                        <div className="flex-shrink-0">
                          <div className="w-8 h-8 rounded-full bg-pierre-red-100 flex items-center justify-center">
                            <svg className="w-4 h-4 text-pierre-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                            </svg>
                          </div>
                        </div>
                        <div className="flex-1 pt-1">
                          <div className="bg-pierre-red-50 border border-pierre-red-100 rounded-lg px-4 py-3">
                            <p className="text-pierre-red-700 text-sm">
                              {errorCountdown !== null
                                ? errorMessage.replace(/in \d+ seconds/, `in ${errorCountdown} seconds`)
                                : errorMessage}
                            </p>
                            <button
                              onClick={() => { setErrorMessage(null); setErrorCountdown(null); }}
                              className="text-pierre-red-500 hover:text-pierre-red-700 text-xs mt-2 underline"
                            >
                              Dismiss
                            </button>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                )}
                <div ref={messagesEndRef} />
              </div>
            </div>

            {/* Input Area */}
            <div className="border-t border-pierre-gray-100 p-4 bg-white">
              <div className="max-w-3xl mx-auto">
                {/* Ideas popover */}
                {showIdeas && (
                  <div className="mb-4 p-4 bg-pierre-gray-50 rounded-xl border border-pierre-gray-200 relative">
                    <button
                      onClick={() => setShowIdeas(false)}
                      className="absolute top-2 right-2 text-pierre-gray-400 hover:text-pierre-gray-600"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                    <p className="text-xs text-pierre-gray-500 mb-3">Click a suggestion to fill the input:</p>
                    <PromptSuggestions onSelectPrompt={handleFillPrompt} />
                  </div>
                )}
                <div className="relative">
                  <textarea
                    ref={inputRef}
                    value={newMessage}
                    onChange={(e) => setNewMessage(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Message Pierre..."
                    className="w-full resize-none rounded-xl border border-pierre-gray-200 bg-pierre-gray-50 pl-4 pr-14 py-3 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent focus:bg-white text-sm transition-colors overflow-hidden"
                    rows={1}
                    disabled={isStreaming}
                  />
                  <button
                    onClick={handleSendMessage}
                    disabled={!newMessage.trim() || isStreaming}
                    aria-label="Send message"
                    className={clsx(
                      'absolute right-3 top-1/2 -translate-y-1/2 w-8 h-8 flex items-center justify-center rounded-lg transition-colors',
                      newMessage.trim() && !isStreaming
                        ? 'bg-pierre-violet text-white hover:bg-pierre-violet/90 shadow-sm'
                        : 'text-pierre-gray-400 cursor-not-allowed'
                    )}
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                    </svg>
                  </button>
                </div>
                <div className="flex items-center justify-center gap-2 mt-2">
                  <p className="text-xs text-pierre-gray-400">
                    Press Enter to send, Shift+Enter for new line
                  </p>
                  <span className="text-pierre-gray-300">|</span>
                  <button
                    onClick={() => setShowIdeas(!showIdeas)}
                    className="text-xs text-pierre-violet hover:text-pierre-violet/80 flex items-center gap-1"
                  >
                    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                    </svg>
                    Need ideas?
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}
      </Panel>

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        isOpen={!!deleteConfirmation}
        onClose={() => setDeleteConfirmation(null)}
        onConfirm={handleConfirmDelete}
        title="Delete Conversation"
        message={`Are you sure you want to delete "${deleteConfirmation?.title || 'this conversation'}"? This action cannot be undone.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={deleteConversation.isPending}
      />

      {/* Provider Connection Modal - shown when selecting coach without connected provider */}
      {showProviderModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/50 backdrop-blur-sm"
            onClick={handleProviderModalClose}
          />
          {/* Modal Content */}
          <div className="relative bg-white rounded-2xl shadow-2xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
            <div className="p-6">
              {/* Close button */}
              <button
                onClick={handleProviderModalClose}
                className="absolute top-4 right-4 p-2 text-pierre-gray-400 hover:text-pierre-gray-600 hover:bg-pierre-gray-100 rounded-lg transition-colors"
                aria-label="Close"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>

              <div className="text-center mb-6">
                <div className="w-12 h-12 bg-pierre-violet/10 rounded-xl flex items-center justify-center mx-auto mb-4">
                  <svg className="w-6 h-6 text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                </div>
                <h2 className="text-xl font-semibold text-pierre-gray-900 mb-2">
                  Connect your fitness data
                </h2>
                <p className="text-pierre-gray-500 text-sm">
                  Link a provider for personalized insights, or continue without
                </p>
              </div>

              <ProviderConnectionCards
                onConnectProvider={handleConnectProvider}
                connectingProvider={connectingProvider}
                onSkip={handleProviderModalSkip}
                isSkipPending={createConversation.isPending}
              />
            </div>
          </div>
        </div>
      )}

      {/* Coach Create/Edit Modal */}
      {showCoachModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/50 backdrop-blur-sm"
            onClick={() => {
              setShowCoachModal(false);
              setEditingCoachId(null);
              setCoachFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
            }}
          />
          {/* Modal Content */}
          <div className="relative bg-white rounded-2xl shadow-2xl max-w-lg w-full mx-4 max-h-[90vh] overflow-y-auto">
            <div className="p-6">
              {/* Close button */}
              <button
                onClick={() => {
                  setShowCoachModal(false);
                  setEditingCoachId(null);
                  setCoachFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
                }}
                className="absolute top-4 right-4 p-2 text-pierre-gray-400 hover:text-pierre-gray-600 hover:bg-pierre-gray-100 rounded-lg transition-colors"
                aria-label="Close"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>

              <div className="text-center mb-6">
                <div className="w-12 h-12 bg-pierre-violet/10 rounded-xl flex items-center justify-center mx-auto mb-4">
                  <svg className="w-6 h-6 text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                </div>
                <h2 className="text-xl font-semibold text-pierre-gray-900 mb-2">
                  {editingCoachId ? 'Edit Coach' : 'Create Custom Coach'}
                </h2>
                <p className="text-pierre-gray-500 text-sm">
                  {editingCoachId
                    ? 'Update your coaching persona settings'
                    : 'Define a specialized AI coaching persona for your training'}
                </p>
              </div>

              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  if (!coachFormData.title.trim() || !coachFormData.system_prompt.trim()) return;
                  if (editingCoachId) {
                    updateCoach.mutate({ id: editingCoachId, data: coachFormData });
                  } else {
                    createCoach.mutate(coachFormData);
                  }
                }}
                className="space-y-4"
              >
                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Coach Name
                  </label>
                  <input
                    type="text"
                    placeholder="e.g., Marathon Training Coach"
                    value={coachFormData.title}
                    onChange={(e) => setCoachFormData({ ...coachFormData, title: e.target.value })}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                    required
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Description <span className="text-pierre-gray-400">(optional)</span>
                  </label>
                  <input
                    type="text"
                    placeholder="Brief description of what this coach specializes in"
                    value={coachFormData.description}
                    onChange={(e) => setCoachFormData({ ...coachFormData, description: e.target.value })}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    System Prompt
                  </label>
                  <textarea
                    placeholder="Define your coach's personality, expertise, and communication style..."
                    value={coachFormData.system_prompt}
                    onChange={(e) => setCoachFormData({ ...coachFormData, system_prompt: e.target.value })}
                    rows={4}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent resize-none"
                    required
                  />
                  {coachFormData.system_prompt && (
                    <p className="text-xs text-pierre-gray-500 mt-1">
                      ~{Math.ceil(coachFormData.system_prompt.length / 4)} tokens ({((Math.ceil(coachFormData.system_prompt.length / 4) / 128000) * 100).toFixed(1)}% of context)
                    </p>
                  )}
                </div>

                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Category
                  </label>
                  <select
                    value={coachFormData.category}
                    onChange={(e) => setCoachFormData({ ...coachFormData, category: e.target.value })}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent bg-white"
                  >
                    <option value="Training">Training</option>
                    <option value="Nutrition">Nutrition</option>
                    <option value="Recovery">Recovery</option>
                    <option value="Recipes">Recipes</option>
                    <option value="Mobility">Mobility</option>
                    <option value="Analysis">Analysis</option>
                    <option value="Custom">Custom</option>
                  </select>
                </div>

                <div className="flex gap-3 pt-2">
                  <button
                    type="button"
                    onClick={() => {
                      setShowCoachModal(false);
                      setEditingCoachId(null);
                      setCoachFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
                    }}
                    className="flex-1 px-4 py-2 text-sm font-medium text-pierre-gray-600 bg-pierre-gray-100 rounded-lg hover:bg-pierre-gray-200 transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={(editingCoachId ? updateCoach.isPending : createCoach.isPending) || !coachFormData.title.trim() || !coachFormData.system_prompt.trim()}
                    className="flex-1 px-4 py-2 text-sm font-medium text-white bg-pierre-violet rounded-lg hover:bg-pierre-violet/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                  >
                    {editingCoachId
                      ? (updateCoach.isPending ? 'Saving...' : 'Save Changes')
                      : (createCoach.isPending ? 'Creating...' : 'Create Coach')}
                  </button>
                </div>

                {(createCoach.isError || updateCoach.isError) && (
                  <p className="text-xs text-pierre-red-500 text-center">
                    Failed to {editingCoachId ? 'update' : 'create'} coach. Please try again.
                  </p>
                )}
              </form>
            </div>
          </div>
        </div>
      )}

      {/* Coach Delete Confirmation Dialog */}
      <ConfirmDialog
        isOpen={!!coachDeleteConfirmation}
        onClose={() => setCoachDeleteConfirmation(null)}
        onConfirm={handleConfirmCoachDelete}
        title="Delete Coach"
        message={`Are you sure you want to delete "${coachDeleteConfirmation?.title || 'this coach'}"? This action cannot be undone.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={deleteCoach.isPending}
      />
    </PanelGroup>
  );
}

// Helper functions for category styling
function getCategoryBadgeClass(category: string): string {
  const classes: Record<string, string> = {
    training: 'bg-pierre-green-100 text-pierre-green-700',
    nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition',
    recovery: 'bg-pierre-blue-100 text-pierre-blue-700',
    recipes: 'bg-pierre-yellow-100 text-pierre-yellow-700',
    mobility: 'bg-pierre-mobility/10 text-pierre-mobility',
    analysis: 'bg-pierre-violet/10 text-pierre-violet',
    custom: 'bg-pierre-gray-100 text-pierre-gray-600',
  };
  return classes[category.toLowerCase()] || classes.custom;
}

function getCategoryIcon(category: string): string {
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
}

// Coach card component for My Coaches panel
interface MyCoachCardProps {
  coach: Coach;
  onSelect: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onHide: () => void;
  isHiding: boolean;
}

function MyCoachCard({ coach, onSelect, onEdit, onDelete, onHide, isHiding }: MyCoachCardProps) {
  return (
    <div
      className="relative text-left text-sm rounded-xl border border-pierre-gray-200 hover:border-pierre-violet hover:bg-pierre-violet/5 px-4 py-3 transition-all focus-within:outline-none focus-within:ring-2 focus-within:ring-pierre-violet focus-within:ring-opacity-50 group hover:shadow-sm cursor-pointer"
      onClick={onSelect}
    >
      {/* Action buttons container */}
      <div className="absolute top-1.5 right-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity z-10 bg-white/90 backdrop-blur-sm rounded-lg px-1 py-0.5 shadow-sm">
        {/* Edit/Delete for user-created coaches */}
        {!coach.is_system && (
          <>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onEdit();
              }}
              className="p-1 text-pierre-gray-400 hover:text-pierre-violet hover:bg-pierre-violet/10 rounded transition-colors"
              title="Edit coach"
              aria-label="Edit coach"
            >
              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
              </svg>
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
              className="p-1 text-pierre-gray-400 hover:text-pierre-red-500 hover:bg-pierre-red-50 rounded transition-colors"
              title="Delete coach"
              aria-label="Delete coach"
            >
              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
            </button>
          </>
        )}
        {/* Hide button for system coaches */}
        {coach.is_system && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onHide();
            }}
            disabled={isHiding}
            className="p-1 text-pierre-gray-400 hover:text-pierre-gray-600 hover:bg-pierre-gray-100 rounded transition-colors disabled:opacity-50"
            title="Hide coach"
            aria-label="Hide coach"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
            </svg>
          </button>
        )}
      </div>

      <div className="flex items-center justify-between">
        <span className="font-medium text-pierre-gray-800 group-hover:text-pierre-violet">
          {coach.title}
        </span>
        <div className="flex items-center gap-1">
          {coach.is_favorite && (
            <span className="text-pierre-yellow-500">★</span>
          )}
          <span className={`text-xs px-1.5 py-0.5 rounded ${getCategoryBadgeClass(coach.category)}`}>
            {getCategoryIcon(coach.category)}
          </span>
        </div>
      </div>
      {coach.description && (
        <p className="text-pierre-gray-500 text-xs mt-0.5 line-clamp-2">
          {coach.description}
        </p>
      )}
      <div className="flex items-center gap-2 mt-1 text-xs text-pierre-gray-400">
        {coach.is_system && (
          <span className="bg-pierre-violet bg-opacity-10 text-pierre-violet px-1.5 py-0.5 rounded">
            System
          </span>
        )}
        {coach.use_count > 0 && (
          <span>Used {coach.use_count}x</span>
        )}
      </div>
    </div>
  );
}
