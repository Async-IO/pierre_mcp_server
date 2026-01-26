// ABOUTME: AI Chat tab component for users to interact with fitness AI assistant
// ABOUTME: Pure chat interface - navigation handled by Dashboard
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { ConfirmDialog } from './ui';
import { clsx } from 'clsx';
import { apiService } from '../services/api';
import Markdown from 'react-markdown';
import PromptSuggestions from './PromptSuggestions';
import ProviderConnectionCards from './ProviderConnectionCards';
import { MessageCircle, Plus, ChevronDown, Pencil, Trash2, X, Check, Share2 } from 'lucide-react';
import { ShareChatMessageModal } from './social';

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

export default function ChatTab() {
  const queryClient = useQueryClient();
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [newMessage, setNewMessage] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [errorCountdown, setErrorCountdown] = useState<number | null>(null);
  const [oauthNotification, setOauthNotification] = useState<{ provider: string; timestamp: number } | null>(null);
  const [deleteConfirmation, setDeleteConfirmation] = useState<{ id: string; title: string } | null>(null);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);
  const [pendingSystemPrompt, setPendingSystemPrompt] = useState<string | null>(null);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);
  const [showProviderModal, setShowProviderModal] = useState(false);
  const [pendingCoachAction, setPendingCoachAction] = useState<{ prompt: string; systemPrompt?: string } | null>(null);
  // Track model and execution time for assistant messages (for debugging/transparency)
  const [messageMetadata, setMessageMetadata] = useState<Map<string, { model: string; executionTimeMs: number }>>(new Map());
  // Conversation selector dropdown
  const [showConversationDropdown, setShowConversationDropdown] = useState(false);
  const [editingConversationId, setEditingConversationId] = useState<string | null>(null);
  const [editedTitle, setEditedTitle] = useState('');

  // Share message modal state
  const [shareMessageContent, setShareMessageContent] = useState<string | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Fetch conversations
  const { data: conversationsData } = useQuery<ConversationListResponse>({
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
      setEditingConversationId(null);
      setEditedTitle('');
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

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowConversationDropdown(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Listen for OAuth completion from popup/new tab
  useEffect(() => {
    let isProcessingOAuth = false;

    const extractOAuthData = () => {
      const stored = localStorage.getItem('pierre_oauth_result');
      if (!stored) return null;

      localStorage.removeItem('pierre_oauth_result');

      try {
        const result = JSON.parse(stored);
        const fiveMinutesAgo = Date.now() - 5 * 60 * 1000;

        if (result.type === 'oauth_completed' && result.success && result.timestamp > fiveMinutesAgo) {
          const savedConversation = localStorage.getItem('pierre_oauth_conversation');
          const savedCoachAction = localStorage.getItem('pierre_pending_coach_action');

          if (savedConversation) localStorage.removeItem('pierre_oauth_conversation');
          if (savedCoachAction) localStorage.removeItem('pierre_pending_coach_action');

          return {
            result,
            savedConversation,
            savedCoachAction: savedCoachAction ? JSON.parse(savedCoachAction) : null,
          };
        } else if (result.timestamp <= fiveMinutesAgo) {
          localStorage.removeItem('pierre_oauth_conversation');
          localStorage.removeItem('pierre_pending_coach_action');
        }
      } catch {
        // Ignore parse errors
      }
      return null;
    };

    const processOAuthData = (data: { result: { provider: string }; savedConversation: string | null; savedCoachAction: { prompt: string; systemPrompt?: string } | null }) => {
      if (isProcessingOAuth) return;
      isProcessingOAuth = true;

      queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
      queryClient.invalidateQueries({ queryKey: ['user-profile'] });

      const providerDisplay = data.result.provider.charAt(0).toUpperCase() + data.result.provider.slice(1);
      setOauthNotification({ provider: providerDisplay, timestamp: Date.now() });
      setConnectingProvider(null);

      if (data.savedConversation) {
        setSelectedConversation(data.savedConversation);
      }

      if (data.savedCoachAction) {
        setPendingPrompt(data.savedCoachAction.prompt);
        if (data.savedCoachAction.systemPrompt) {
          setPendingSystemPrompt(data.savedCoachAction.systemPrompt);
        }
        createConversation.mutate(data.savedCoachAction.systemPrompt);
      }

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

    checkAndProcessOAuthResult();

    return () => {
      window.removeEventListener('message', handleOAuthMessage);
      window.removeEventListener('storage', handleStorageChange);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleFocus);
    };
  }, [queryClient, createConversation]);

  // Handle sending a pending prompt when conversation is ready
  useEffect(() => {
    if (pendingPrompt && selectedConversation && !isStreaming) {
      const promptToSend = pendingPrompt;
      setPendingPrompt(null);
      setNewMessage(promptToSend);
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

    if (connectingProvider) {
      localStorage.setItem('pierre_oauth_conversation', selectedConversation);
    }

    const displayContent = newMessage.trim();
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
        const userMessage = errorData.message || errorData.error || 'Failed to send message';
        throw new Error(userMessage);
      }

      const responseText = await response.text();
      let fullContent = '';
      let responseModel: string | undefined;
      let responseExecutionTimeMs: number | undefined;
      let assistantMessageId: string | undefined;

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

      queryClient.invalidateQueries({ queryKey: ['chat-messages', selectedConversation] });
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });

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
    setSelectedConversation(null);
    setShowConversationDropdown(false);
  };

  const handleSelectPrompt = (prompt: string, coachIdForTracking?: string, systemPrompt?: string) => {
    void coachIdForTracking;

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

  const handleConnectProvider = async (providerName: string) => {
    setConnectingProvider(providerName);
    if (pendingCoachAction) {
      localStorage.setItem('pierre_pending_coach_action', JSON.stringify(pendingCoachAction));
    }
    setShowProviderModal(false);

    try {
      const providerId = providerName.toLowerCase();
      const authUrl = await apiService.getOAuthAuthorizeUrl(providerId);
      window.open(authUrl, '_blank');
      setConnectingProvider(null);
    } catch (error) {
      console.error('Failed to get OAuth authorization URL:', error);
      setConnectingProvider(null);
    }
  };

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

  const handleProviderModalClose = () => {
    setShowProviderModal(false);
    setPendingCoachAction(null);
  };

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

  const handleStartRename = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setEditingConversationId(conv.id);
    setEditedTitle(conv.title);
  };

  const handleSaveRename = () => {
    if (editingConversationId && editedTitle.trim()) {
      updateConversation.mutate({ id: editingConversationId, title: editedTitle.trim() });
    } else {
      setEditingConversationId(null);
      setEditedTitle('');
    }
  };

  const handleCancelRename = () => {
    setEditingConversationId(null);
    setEditedTitle('');
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

  const selectedConversationData = conversationsData?.conversations?.find(c => c.id === selectedConversation);

  return (
    <div className="h-full flex flex-col bg-pierre-dark">
      {/* Header with conversation selector */}
      <div className="flex-shrink-0 border-b border-white/10 px-4 py-3">
        <div className="flex items-center justify-between">
          <div className="relative" ref={dropdownRef}>
            <button
              onClick={() => setShowConversationDropdown(!showConversationDropdown)}
              className="flex items-center gap-2 px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 transition-colors text-white"
            >
              <MessageCircle className="w-4 h-4 text-pierre-violet" />
              <span className="text-sm font-medium truncate max-w-[200px]">
                {selectedConversationData?.title || 'New Chat'}
              </span>
              <ChevronDown className={clsx('w-4 h-4 transition-transform', showConversationDropdown && 'rotate-180')} />
            </button>

            {/* Dropdown menu */}
            {showConversationDropdown && (
              <div className="absolute top-full left-0 mt-1 w-80 bg-pierre-slate border border-white/10 rounded-lg shadow-xl z-50 overflow-hidden">
                <div className="p-2 border-b border-white/10">
                  <button
                    onClick={handleNewChat}
                    className="w-full flex items-center gap-2 px-3 py-2 rounded-lg bg-pierre-violet/20 hover:bg-pierre-violet/30 text-pierre-violet transition-colors"
                  >
                    <Plus className="w-4 h-4" />
                    <span className="text-sm font-medium">New Chat</span>
                  </button>
                </div>
                <div className="max-h-64 overflow-y-auto">
                  {conversationsData?.conversations?.length === 0 ? (
                    <div className="p-4 text-center text-zinc-500 text-sm">No conversations yet</div>
                  ) : (
                    conversationsData?.conversations?.map((conv) => (
                      <div
                        key={conv.id}
                        className={clsx(
                          'group flex items-center gap-2 px-3 py-2 cursor-pointer transition-colors',
                          selectedConversation === conv.id
                            ? 'bg-white/10 text-white'
                            : 'hover:bg-white/5 text-zinc-300'
                        )}
                        onClick={() => {
                          if (editingConversationId !== conv.id) {
                            setSelectedConversation(conv.id);
                            setShowConversationDropdown(false);
                          }
                        }}
                      >
                        <MessageCircle className="w-4 h-4 flex-shrink-0 text-zinc-500" />
                        <div className="flex-1 min-w-0">
                          {editingConversationId === conv.id ? (
                            <div className="flex items-center gap-1" onClick={e => e.stopPropagation()}>
                              <input
                                type="text"
                                value={editedTitle}
                                onChange={(e) => setEditedTitle(e.target.value)}
                                onKeyDown={(e) => {
                                  if (e.key === 'Enter') handleSaveRename();
                                  if (e.key === 'Escape') handleCancelRename();
                                }}
                                className="flex-1 text-sm bg-pierre-dark border border-pierre-violet rounded px-2 py-0.5 text-white focus:outline-none"
                                autoFocus
                              />
                              <button onClick={handleSaveRename} className="p-1 text-pierre-activity hover:text-pierre-activity-light">
                                <Check className="w-3 h-3" />
                              </button>
                              <button onClick={handleCancelRename} className="p-1 text-zinc-500 hover:text-white">
                                <X className="w-3 h-3" />
                              </button>
                            </div>
                          ) : (
                            <p className="text-sm truncate">{conv.title}</p>
                          )}
                        </div>
                        <span className="text-xs text-zinc-500 flex-shrink-0">{formatDate(conv.updated_at)}</span>
                        {editingConversationId !== conv.id && (
                          <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                            <button
                              onClick={(e) => handleStartRename(e, conv)}
                              className="p-1 text-zinc-500 hover:text-pierre-violet"
                            >
                              <Pencil className="w-3 h-3" />
                            </button>
                            <button
                              onClick={(e) => handleDeleteConversation(e, conv)}
                              className="p-1 text-zinc-500 hover:text-pierre-red-500"
                            >
                              <Trash2 className="w-3 h-3" />
                            </button>
                          </div>
                        )}
                      </div>
                    ))
                  )}
                </div>
              </div>
            )}
          </div>

          <button
            onClick={handleNewChat}
            className="p-2 rounded-lg hover:bg-white/10 transition-colors text-zinc-400 hover:text-white"
            title="New Chat"
          >
            <Plus className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Main content area */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {!selectedConversation ? (
          /* Welcome / Onboarding Screen */
          <div className="flex-1 flex flex-col items-center justify-center p-8 overflow-y-auto">
            <div className="max-w-2xl w-full space-y-8">
              {/* Welcome header */}
              <div className="text-center">
                <h1 className="text-3xl font-bold text-white mb-2">Welcome to Pierre</h1>
                <p className="text-zinc-400">Your AI fitness intelligence assistant</p>
              </div>

              {/* OAuth notification */}
              {oauthNotification && (
                <div className="bg-pierre-activity/20 border border-pierre-activity/30 rounded-lg p-4 text-center">
                  <p className="text-pierre-activity font-medium">
                    ✓ Successfully connected to {oauthNotification.provider}!
                  </p>
                </div>
              )}

              {/* Prompt suggestions */}
              <PromptSuggestions
                onSelectPrompt={handleSelectPrompt}
              />
            </div>
          </div>
        ) : (
          /* Chat messages area */
          <>
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              {/* OAuth notification */}
              {oauthNotification && (
                <div className="bg-pierre-activity/20 border border-pierre-activity/30 rounded-lg p-3 flex items-center gap-2">
                  <span className="text-pierre-activity">✓</span>
                  <p className="text-sm text-pierre-activity">
                    Successfully connected to {oauthNotification.provider}!
                  </p>
                </div>
              )}

              {messagesLoading ? (
                <div className="flex justify-center py-8">
                  <div className="pierre-spinner w-6 h-6 border-pierre-violet border-t-transparent"></div>
                </div>
              ) : messagesData?.messages?.length === 0 ? (
                <div className="text-center py-8 text-zinc-500">
                  <p>Start a conversation by typing a message below</p>
                </div>
              ) : (
                messagesData?.messages?.map((msg) => (
                  <div
                    key={msg.id}
                    className={clsx(
                      'flex',
                      msg.role === 'user' ? 'justify-end' : 'justify-start'
                    )}
                  >
                    <div
                      className={clsx(
                        'max-w-[80%] rounded-2xl px-4 py-3',
                        msg.role === 'user'
                          ? 'bg-pierre-violet text-white'
                          : 'bg-white/10 text-zinc-100'
                      )}
                    >
                      {msg.role === 'assistant' ? (
                        <div className="prose prose-invert prose-sm max-w-none">
                          <Markdown
                            components={{
                              a: ({ href, children }) => (
                                <a
                                  href={href}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="text-pierre-cyan hover:underline"
                                >
                                  {children}
                                </a>
                              ),
                            }}
                          >
                            {linkifyUrls(stripContextPrefix(msg.content))}
                          </Markdown>
                          {/* Actions row with model/timing info and share button */}
                          <div className="mt-2 pt-2 border-t border-white/10 flex items-center justify-between">
                            {/* Model/timing info */}
                            {messageMetadata.get(msg.id) ? (
                              <div className="text-xs text-zinc-500">
                                {messageMetadata.get(msg.id)?.model} • {Math.round((messageMetadata.get(msg.id)?.executionTimeMs || 0) / 1000)}s
                              </div>
                            ) : (
                              <div />
                            )}
                            {/* Share button */}
                            <button
                              onClick={() => setShareMessageContent(stripContextPrefix(msg.content))}
                              className="flex items-center gap-1 text-xs text-zinc-400 hover:text-pierre-violet transition-colors"
                              title="Share to social feed"
                            >
                              <Share2 className="w-3.5 h-3.5" />
                              <span>Share</span>
                            </button>
                          </div>
                        </div>
                      ) : (
                        <p className="text-sm whitespace-pre-wrap">{stripContextPrefix(msg.content)}</p>
                      )}
                    </div>
                  </div>
                ))
              )}

              {/* Streaming content */}
              {isStreaming && streamingContent && (
                <div className="flex justify-start">
                  <div className="max-w-[80%] rounded-2xl px-4 py-3 bg-white/10 text-zinc-100">
                    <div className="prose prose-invert prose-sm max-w-none">
                      <Markdown>{linkifyUrls(streamingContent)}</Markdown>
                    </div>
                  </div>
                </div>
              )}

              {/* Streaming indicator */}
              {isStreaming && !streamingContent && (
                <div className="flex justify-start">
                  <div className="rounded-2xl px-4 py-3 bg-white/10">
                    <div className="flex items-center gap-1">
                      <div className="w-2 h-2 rounded-full bg-pierre-violet animate-bounce" style={{ animationDelay: '0ms' }}></div>
                      <div className="w-2 h-2 rounded-full bg-pierre-violet animate-bounce" style={{ animationDelay: '150ms' }}></div>
                      <div className="w-2 h-2 rounded-full bg-pierre-violet animate-bounce" style={{ animationDelay: '300ms' }}></div>
                    </div>
                  </div>
                </div>
              )}

              <div ref={messagesEndRef} />
            </div>

            {/* Error message */}
            {errorMessage && (
              <div className="px-4 pb-2">
                <div className="bg-pierre-red-500/20 border border-pierre-red-500/30 rounded-lg p-3 flex items-center justify-between">
                  <p className="text-sm text-pierre-red-400">
                    {errorCountdown !== null
                      ? errorMessage.replace(/in \d+ seconds/, `in ${errorCountdown} seconds`)
                      : errorMessage}
                  </p>
                  <button
                    onClick={() => setErrorMessage(null)}
                    className="text-pierre-red-400 hover:text-pierre-red-300"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
              </div>
            )}

            {/* Input area */}
            <div className="flex-shrink-0 p-4 border-t border-white/10">
              <div className="flex items-end gap-2">
                <textarea
                  ref={inputRef}
                  value={newMessage}
                  onChange={(e) => setNewMessage(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="Type your message..."
                  disabled={isStreaming}
                  rows={1}
                  className="flex-1 bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white placeholder-zinc-500 resize-none focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent disabled:opacity-50"
                  style={{ minHeight: '48px', maxHeight: '200px' }}
                />
                <button
                  onClick={handleSendMessage}
                  disabled={!newMessage.trim() || isStreaming}
                  aria-label="Send message"
                  className={clsx(
                    'p-3 rounded-xl transition-colors',
                    newMessage.trim() && !isStreaming
                      ? 'bg-pierre-violet hover:bg-pierre-violet-dark text-white'
                      : 'bg-white/10 text-zinc-500 cursor-not-allowed'
                  )}
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                  </svg>
                </button>
              </div>
            </div>
          </>
        )}
      </div>

      {/* Delete confirmation dialog */}
      <ConfirmDialog
        isOpen={!!deleteConfirmation}
        onClose={() => setDeleteConfirmation(null)}
        onConfirm={handleConfirmDelete}
        title="Delete Conversation"
        message={`Are you sure you want to delete "${deleteConfirmation?.title}"? This action cannot be undone.`}
        confirmLabel="Delete"
        variant="danger"
      />

      {/* Provider connection modal */}
      {showProviderModal && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
          <div className="bg-pierre-slate border border-white/10 rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl">
            <h2 className="text-xl font-semibold text-white mb-2">Connect a Fitness Provider</h2>
            <p className="text-zinc-400 mb-6">
              To get personalized insights, connect one of your fitness accounts.
            </p>
            <ProviderConnectionCards onConnectProvider={handleConnectProvider} />
            <div className="mt-6 flex justify-between">
              <button
                onClick={handleProviderModalClose}
                className="px-4 py-2 text-zinc-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleProviderModalSkip}
                className="px-4 py-2 text-pierre-violet hover:text-pierre-violet-light transition-colors"
              >
                Skip for now
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Share message modal */}
      {shareMessageContent && (
        <ShareChatMessageModal
          content={shareMessageContent}
          onClose={() => setShareMessageContent(null)}
          onSuccess={() => {
            setShareMessageContent(null);
            // Could add a toast/notification here
          }}
        />
      )}
    </div>
  );
}
