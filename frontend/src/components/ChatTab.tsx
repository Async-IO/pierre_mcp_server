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
import PromptSuggestions, { useWelcomePrompt } from './PromptSuggestions';
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

interface ChatTabProps {
  onOpenSettings?: () => void;
}

export default function ChatTab({ onOpenSettings }: ChatTabProps) {
  const queryClient = useQueryClient();
  const { user, logout } = useAuth();
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
  const [showIdeas, setShowIdeas] = useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);
  const [skippedOnboarding, setSkippedOnboarding] = useState(false);

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

  // Fetch welcome prompt from API
  const { welcomePrompt } = useWelcomePrompt();

  // Fetch messages for selected conversation
  const { data: messagesData, isLoading: messagesLoading } = useQuery<{ messages: Message[] }>({
    queryKey: ['chat-messages', selectedConversation],
    queryFn: () => apiService.getConversationMessages(selectedConversation!),
    enabled: !!selectedConversation,
  });

  // Create conversation mutation - auto-creates with default title
  const createConversation = useMutation({
    mutationFn: () => {
      const now = new Date();
      const defaultTitle = `Chat ${now.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })} ${now.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })}`;
      return apiService.createConversation({ title: defaultTitle });
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setSelectedConversation(data.id);
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
  useEffect(() => {
    // Check localStorage for OAuth result and process if found
    const checkAndProcessOAuthResult = () => {
      try {
        const stored = localStorage.getItem('pierre_oauth_result');
        const savedConversation = localStorage.getItem('pierre_oauth_conversation');

        if (stored) {
          const result = JSON.parse(stored);
          // Only process if it's recent (within last 5 minutes)
          const fiveMinutesAgo = Date.now() - 5 * 60 * 1000;

          if (result.type === 'oauth_completed' && result.success && result.timestamp > fiveMinutesAgo) {
            queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
            queryClient.invalidateQueries({ queryKey: ['user-profile'] });
            // Show visible notification in chat
            const providerDisplay = result.provider.charAt(0).toUpperCase() + result.provider.slice(1);
            setOauthNotification({ provider: providerDisplay, timestamp: Date.now() });
            setConnectingProvider(null);
            // Restore the conversation that was active before OAuth redirect
            if (savedConversation) {
              setSelectedConversation(savedConversation);
              localStorage.removeItem('pierre_oauth_conversation');
            }
            // Clean up the storage item
            localStorage.removeItem('pierre_oauth_result');
          } else if (result.timestamp <= fiveMinutesAgo) {
            // Clean up stale entries
            localStorage.removeItem('pierre_oauth_result');
            localStorage.removeItem('pierre_oauth_conversation');
          }
        }
      } catch {
        // Ignore parse errors from localStorage
      }
    };

    const handleOAuthMessage = (event: MessageEvent) => {
      // Validate message structure
      if (event.data?.type === 'oauth_completed') {
        const { provider, success } = event.data;
        if (success) {
          // Invalidate any queries that depend on connection status
          queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
          queryClient.invalidateQueries({ queryKey: ['user-profile'] });
          // Show visible notification in chat
          const providerDisplay = provider.charAt(0).toUpperCase() + provider.slice(1);
          setOauthNotification({ provider: providerDisplay, timestamp: Date.now() });
          setConnectingProvider(null);
          // Restore the conversation that was active before OAuth redirect
          const savedConversation = localStorage.getItem('pierre_oauth_conversation');
          if (savedConversation) {
            setSelectedConversation(savedConversation);
            localStorage.removeItem('pierre_oauth_conversation');
          }
        }
      }
    };

    const handleStorageChange = (event: StorageEvent) => {
      if (event.key === 'pierre_oauth_result' && event.newValue) {
        try {
          const result = JSON.parse(event.newValue);
          if (result.type === 'oauth_completed' && result.success) {
            queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
            queryClient.invalidateQueries({ queryKey: ['user-profile'] });
            // Show visible notification in chat
            const providerDisplay = result.provider.charAt(0).toUpperCase() + result.provider.slice(1);
            setOauthNotification({ provider: providerDisplay, timestamp: Date.now() });
            setConnectingProvider(null);
            // Restore the conversation that was active before OAuth redirect
            const savedConversation = localStorage.getItem('pierre_oauth_conversation');
            if (savedConversation) {
              setSelectedConversation(savedConversation);
              localStorage.removeItem('pierre_oauth_conversation');
            }
            // Clean up the storage item
            localStorage.removeItem('pierre_oauth_result');
          }
        } catch {
          // Ignore parse errors
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

      // Handle SSE streaming
      const reader = response.body?.getReader();
      const decoder = new TextDecoder();
      let fullContent = '';

      if (reader) {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value, { stream: true });
          const lines = chunk.split('\n');

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
              } catch {
                // Skip non-JSON lines
              }
            }
          }
        }
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

  const handleSkipOnboarding = () => {
    // User explicitly skipped provider connection - show prompts
    setSkippedOnboarding(true);
    setSelectedConversation(null);
  };

  const handleSelectPrompt = (prompt: string) => {
    setPendingPrompt(prompt);
    createConversation.mutate();
  };

  const handleFillPrompt = (prompt: string) => {
    setNewMessage(prompt);
    setShowIdeas(false);
    inputRef.current?.focus();
  };

  const handleConnectProvider = (providerName: string) => {
    setConnectingProvider(providerName);
    setPendingPrompt(`Connect to ${providerName}`);
    createConversation.mutate();
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
        defaultSize="25%"
        minSize="15%"
        maxSize="40%"
        collapsible
        collapsedSize="0%"
        onResize={(size) => setSidebarCollapsed(size.asPercentage === 0)}
        className="bg-pierre-gray-50 flex flex-col"
      >
        {/* Header with New Chat Button */}
        <div className="py-2">
          <button
            onClick={handleNewChat}
            disabled={createConversation.isPending}
            title="New conversation"
            aria-label="New conversation"
            className="relative px-3 py-2 mx-2 flex items-center gap-2.5 rounded-lg hover:bg-pierre-gray-100 transition-colors disabled:opacity-50"
          >
            <div className="w-7 h-7 flex items-center justify-center rounded-lg bg-pierre-violet text-white shadow-sm flex-shrink-0">
              {createConversation.isPending ? (
                <svg className="w-3.5 h-3.5 animate-spin" viewBox="0 0 24 24" fill="none">
                  <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" strokeOpacity="0.25" />
                  <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
                </svg>
              ) : (
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M12 4v16m8-8H4" />
                </svg>
              )}
            </div>
            <span className="text-sm font-medium text-pierre-gray-800">Add chat</span>
          </button>
        </div>

        {/* Conversation List */}
        <div className="flex-1 overflow-y-auto">
          {conversationsLoading ? (
            <div className="p-4 text-center text-pierre-gray-500 text-sm">Loading...</div>
          ) : conversationsData?.conversations?.length === 0 ? (
            <div className="p-4 text-center text-pierre-gray-500 text-sm">No conversations yet</div>
          ) : (
            <div className="py-2">
              {conversationsData?.conversations?.map((conv) => (
                <div
                  key={conv.id}
                  onClick={() => editingTitle !== conv.id && setSelectedConversation(conv.id)}
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
                        className="opacity-0 group-hover:opacity-100 text-pierre-gray-400 hover:text-red-500 transition-all p-1"
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

        {/* User Profile Section - Bottom of sidebar */}
        <div className="border-t border-pierre-gray-200 px-3 py-3">
          <div className="flex items-center gap-3">
            {/* User Avatar with online indicator */}
            <div className="relative flex-shrink-0">
              <div className="w-8 h-8 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-full flex items-center justify-center">
                <span className="text-xs font-bold text-white">
                  {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
                </span>
              </div>
              {/* Online status dot */}
              <div className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 bg-green-500 rounded-full border-2 border-pierre-gray-50" />
            </div>

            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-pierre-gray-900 truncate">
                {user?.display_name || user?.email}
              </p>
              <p className="text-xs text-pierre-gray-500 truncate">
                {user?.email}
              </p>
            </div>

            {onOpenSettings && (
              <button
                onClick={onOpenSettings}
                className="p-1.5 text-pierre-gray-400 hover:text-pierre-violet hover:bg-pierre-violet/10 rounded-lg transition-colors flex-shrink-0"
                title="Settings"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </button>
            )}
            <button
              onClick={logout}
              className="p-1.5 text-pierre-gray-400 hover:text-pierre-red-500 hover:bg-pierre-red-50 rounded-lg transition-colors flex-shrink-0"
              title="Sign out"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
              </svg>
            </button>
          </div>
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
      <Panel defaultSize="75%" className="flex flex-col bg-white">
        {/* Show provider onboarding only when no conversation selected, no providers connected, and user hasn't skipped */}
        {!selectedConversation && !hasConnectedProvider && !skippedOnboarding ? (
          <div className="flex-1 flex items-center justify-center overflow-y-auto py-12">
            <div className="w-full max-w-3xl px-6">
              <div className="text-center mb-8">
                <h2 className="text-2xl font-semibold text-pierre-gray-900 mb-2">
                  Connect your fitness data
                </h2>
                <p className="text-pierre-gray-500 text-sm">
                  Link a provider to unlock personalized insights
                </p>
              </div>

              <ProviderConnectionCards
                onConnectProvider={handleConnectProvider}
                connectingProvider={connectingProvider}
                onSkip={handleSkipOnboarding}
                isSkipPending={createConversation.isPending}
              />
            </div>
          </div>
        ) : !selectedConversation && (hasConnectedProvider || skippedOnboarding) ? (
          // Welcome state when provider connected but no conversation yet
          <div className="flex-1 flex items-center justify-center overflow-y-auto py-12">
            <div className="w-full max-w-3xl px-6">
              <div className="text-center mb-8">
                <div className="inline-flex items-center gap-2 px-3 py-1.5 bg-emerald-50 text-emerald-700 text-sm font-medium rounded-full mb-4">
                  <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                  </svg>
                  {oauthStatus?.providers?.filter(p => p.connected).map(p =>
                    p.provider.charAt(0).toUpperCase() + p.provider.slice(1)
                  ).join(', ')} connected
                </div>
                <h2 className="text-2xl font-semibold text-pierre-gray-900 mb-2">
                  Ready to analyze your fitness
                </h2>
                <p className="text-pierre-gray-500 text-sm">
                  Get personalized insights from your activity data
                </p>
              </div>

              {/* Featured action: Analyze recent activities */}
              <div className="text-center mb-8">
                <button
                  type="button"
                  onClick={() => welcomePrompt && handleSelectPrompt(welcomePrompt)}
                  disabled={createConversation.isPending || !welcomePrompt}
                  className="inline-flex items-center gap-2 px-8 py-4 bg-gradient-to-r from-pierre-violet to-pierre-cyan text-white font-semibold rounded-xl shadow-lg shadow-pierre-violet/25 hover:shadow-xl hover:shadow-pierre-violet/30 hover:-translate-y-0.5 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-offset-2 disabled:opacity-50"
                >
                  {createConversation.isPending ? (
                    <>
                      <svg className="w-5 h-5 animate-spin" viewBox="0 0 24 24" fill="none">
                        <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" strokeOpacity="0.25" />
                        <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
                      </svg>
                      Analyzing...
                    </>
                  ) : (
                    <>
                      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                      </svg>
                      Analyze my last 20 activities
                    </>
                  )}
                </button>
              </div>

              {/* Divider */}
              <div className="flex items-center gap-4 my-8">
                <div className="flex-1 h-px bg-pierre-gray-200" />
                <span className="text-pierre-gray-400 text-xs uppercase tracking-wider">Or ask something else</span>
                <div className="flex-1 h-px bg-pierre-gray-200" />
              </div>

              {/* Additional prompt suggestions */}
              <PromptSuggestions onSelectPrompt={handleSelectPrompt} />

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
                            <div className="w-8 h-8 rounded-full bg-gradient-to-br from-pierre-violet to-pierre-cyan flex items-center justify-center">
                              <span className="text-white text-xs font-bold">P</span>
                            </div>
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
                        </div>
                      </div>
                    ))}

                    {/* OAuth connection notification */}
                    {oauthNotification && (
                      <div className="flex gap-3 animate-fadeIn">
                        <div className="flex-shrink-0">
                          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-pierre-violet to-pierre-cyan flex items-center justify-center">
                            <span className="text-white text-xs font-bold">P</span>
                          </div>
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
                          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-pierre-violet to-pierre-cyan flex items-center justify-center">
                            <span className="text-white text-xs font-bold">P</span>
                          </div>
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
                          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-pierre-violet to-pierre-cyan flex items-center justify-center">
                            <span className="text-white text-xs font-bold">P</span>
                          </div>
                        </div>
                        <div className="flex-1 pt-1">
                          <div className="font-medium text-pierre-gray-900 text-sm mb-2 flex items-center gap-2">
                            Pierre
                          </div>
                          <div className="flex items-center gap-2 text-pierre-gray-500 text-sm">
                            {/* Animated spinner */}
                            <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24" fill="none">
                              <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" strokeOpacity="0.25" />
                              <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
                            </svg>
                            <span>Thinking...</span>
                          </div>
                        </div>
                      </div>
                    )}

                    {/* Error message display */}
                    {errorMessage && !isStreaming && (
                      <div className="flex gap-3">
                        <div className="flex-shrink-0">
                          <div className="w-8 h-8 rounded-full bg-red-100 flex items-center justify-center">
                            <svg className="w-4 h-4 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                            </svg>
                          </div>
                        </div>
                        <div className="flex-1 pt-1">
                          <div className="bg-red-50 border border-red-100 rounded-lg px-4 py-3">
                            <p className="text-red-700 text-sm">
                              {errorCountdown !== null
                                ? errorMessage.replace(/in \d+ seconds/, `in ${errorCountdown} seconds`)
                                : errorMessage}
                            </p>
                            <button
                              onClick={() => { setErrorMessage(null); setErrorCountdown(null); }}
                              className="text-red-500 hover:text-red-700 text-xs mt-2 underline"
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
    </PanelGroup>
  );
}
