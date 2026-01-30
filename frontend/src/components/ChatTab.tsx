// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: AI Chat tab component for users to interact with fitness AI assistant
// ABOUTME: Renders chat interface with collapsible conversations panel

import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { ConfirmDialog } from './ui';
import { chatApi, providersApi, coachesApi } from '../services/api';
import PromptSuggestions from './PromptSuggestions';
import { MessageCircle, Plus, Sparkles, PanelLeftClose, PanelLeft, History } from 'lucide-react';
import { ShareChatMessageModal } from './social';
import { clsx } from 'clsx';
import {
  MessageList,
  MessageInput,
  ProviderConnectionModal,
  CoachFormModal,
  CreateCoachFromConversationModal,
  ConversationItem,
  stripContextPrefix,
  DEFAULT_COACH_FORM_DATA,
} from './chat';
import type {
  Message,
  Conversation,
  Coach,
  MessageMetadata,
  MessageFeedback,
  OAuthNotification,
  CoachDeleteConfirmation,
  PendingCoachAction,
  CoachFormData,
  DeleteConfirmation,
} from './chat';

export default function ChatTab() {
  const queryClient = useQueryClient();
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [newMessage, setNewMessage] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [errorCountdown, setErrorCountdown] = useState<number | null>(null);
  const [oauthNotification, setOauthNotification] = useState<OAuthNotification | null>(null);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);
  const [pendingSystemPrompt, setPendingSystemPrompt] = useState<string | null>(null);
  const [showIdeas, setShowIdeas] = useState(false);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);
  const [showProviderModal, setShowProviderModal] = useState(false);
  const [pendingCoachAction, setPendingCoachAction] = useState<PendingCoachAction | null>(null);
  const [messageMetadata, setMessageMetadata] = useState<Map<string, MessageMetadata>>(new Map());
  const [messageFeedback, setMessageFeedback] = useState<Map<string, MessageFeedback>>(new Map());
  const [showCoachModal, setShowCoachModal] = useState(false);
  const [editingCoachId, setEditingCoachId] = useState<string | null>(null);
  const [coachFormData, setCoachFormData] = useState<CoachFormData>(DEFAULT_COACH_FORM_DATA);
  const [coachDeleteConfirmation, setCoachDeleteConfirmation] = useState<CoachDeleteConfirmation | null>(null);
  const [shareMessageContent, setShareMessageContent] = useState<string | null>(null);
  const [showCreateCoachFromConversation, setShowCreateCoachFromConversation] = useState(false);

  // Conversations panel state
  const [conversationsPanelOpen, setConversationsPanelOpen] = useState(true);
  const [editingTitle, setEditingTitle] = useState<string | null>(null);
  const [editedTitleValue, setEditedTitleValue] = useState('');
  const [deleteConfirmation, setDeleteConfirmation] = useState<DeleteConfirmation | null>(null);

  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Fetch provider status (includes both OAuth and non-OAuth providers like synthetic)
  const { data: providersData } = useQuery({
    queryKey: ['providers-status'],
    queryFn: () => providersApi.getProvidersStatus(),
  });

  const hasConnectedProvider = providersData?.providers?.some(p => p.connected) ?? false;

  // Fetch conversations list
  const { data: conversationsData, isLoading: conversationsLoading } = useQuery<{ conversations: Conversation[] }>({
    queryKey: ['chat-conversations'],
    queryFn: () => chatApi.getConversations(),
  });

  const conversations = conversationsData?.conversations ?? [];

  // Fetch messages for selected conversation
  const { data: messagesData, isLoading: messagesLoading } = useQuery<{ messages: Message[] }>({
    queryKey: ['chat-messages', selectedConversation],
    queryFn: () => chatApi.getConversationMessages(selectedConversation!),
    enabled: !!selectedConversation,
  });

  // Mutations
  const createConversation = useMutation<{ id: string }, Error, string | void>({
    mutationFn: (systemPrompt) => {
      const now = new Date();
      const defaultTitle = `Chat ${now.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })} ${now.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })}`;
      return chatApi.createConversation({
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

  const updateConversation = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) =>
      chatApi.updateConversation(id, { title }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setEditingTitle(null);
      setEditedTitleValue('');
    },
  });

  const deleteConversation = useMutation({
    mutationFn: (id: string) => chatApi.deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setDeleteConfirmation(null);
      if (selectedConversation === deleteConfirmation?.id) {
        setSelectedConversation(null);
      }
    },
  });

  const createCoach = useMutation({
    mutationFn: (data: CoachFormData) => coachesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowCoachModal(false);
      setCoachFormData(DEFAULT_COACH_FORM_DATA);
    },
  });

  const updateCoach = useMutation({
    mutationFn: ({ id, data }: { id: string; data: CoachFormData }) => coachesApi.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowCoachModal(false);
      setEditingCoachId(null);
      setCoachFormData(DEFAULT_COACH_FORM_DATA);
    },
  });

  const deleteCoach = useMutation({
    mutationFn: (id: string) => coachesApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setCoachDeleteConfirmation(null);
    },
  });

  // Focus input when conversation is selected
  useEffect(() => {
    if (selectedConversation) {
      inputRef.current?.focus();
    }
  }, [selectedConversation]);

  // OAuth completion listener
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

    const processOAuthData = (data: { result: { provider: string }; savedConversation: string | null; savedCoachAction: PendingCoachAction | null }) => {
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

  // Handle sending a pending prompt
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

  // Error countdown timer
  useEffect(() => {
    if (!errorMessage) {
      setErrorCountdown(null);
      return;
    }
    const match = errorMessage.match(/in (\d+) seconds/);
    if (match) {
      setErrorCountdown(parseInt(match[1], 10));
    }
  }, [errorMessage]);

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

  // Send message handler
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
      const connectedProviders = providersData?.providers?.filter(p => p.connected).map(p =>
        p.display_name
      ).join(', ');
      messageContent = `[Context: I have connected ${connectedProviders}] ${displayContent}`;
    }

    setNewMessage('');
    setIsStreaming(true);
    setStreamingContent('');
    setErrorMessage(null);

    const userMessageId = `user-${Date.now()}`;
    const tempUserMessage: Message = {
      id: userMessageId,
      role: 'user',
      content: displayContent,
      created_at: new Date().toISOString(),
    };

    queryClient.setQueryData(['chat-messages', selectedConversation], (old: { messages: Message[] } | undefined) => ({
      messages: [...(old?.messages || []), tempUserMessage],
    }));

    try {
      const response = await fetch(`/api/chat/conversations/${selectedConversation}/stream`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${localStorage.getItem('jwt_token')}`,
        },
        body: JSON.stringify({ content: messageContent }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP error! status: ${response.status}`);
      }

      const reader = response.body?.getReader();
      if (!reader) throw new Error('No response body');

      const decoder = new TextDecoder();
      let assistantContent = '';
      let assistantMessageId = '';
      let model = '';
      let executionTimeMs = 0;

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
              if (parsed.content) {
                assistantContent += parsed.content;
                setStreamingContent(assistantContent);
              }
              if (parsed.message_id) {
                assistantMessageId = parsed.message_id;
              }
              if (parsed.model) {
                model = parsed.model;
              }
              if (parsed.execution_time_ms) {
                executionTimeMs = parsed.execution_time_ms;
              }
            } catch {
              // Skip invalid JSON
            }
          }
        }
      }

      if (assistantMessageId && model) {
        setMessageMetadata(prev => {
          const newMap = new Map(prev);
          newMap.set(assistantMessageId, { model, executionTimeMs });
          return newMap;
        });
      }

      queryClient.invalidateQueries({ queryKey: ['chat-messages', selectedConversation] });
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to send message';
      setErrorMessage(message);
      queryClient.invalidateQueries({ queryKey: ['chat-messages', selectedConversation] });
    } finally {
      setIsStreaming(false);
      setStreamingContent('');
    }
  }, [newMessage, selectedConversation, isStreaming, connectingProvider, oauthNotification, hasConnectedProvider, messagesData?.messages, providersData?.providers, queryClient]);

  // Coach handlers
  const handleSelectPrompt = (prompt: string, systemPrompt?: string) => {
    if (!hasConnectedProvider) {
      setPendingCoachAction({ prompt, systemPrompt });
      localStorage.setItem('pierre_pending_coach_action', JSON.stringify({ prompt, systemPrompt }));
      setShowProviderModal(true);
      return;
    }

    setPendingPrompt(prompt);
    if (systemPrompt) {
      setPendingSystemPrompt(systemPrompt);
    }
    createConversation.mutate(systemPrompt);
  };

  const handleFillPrompt = (prompt: string) => {
    setNewMessage(prompt);
    setShowIdeas(false);
    inputRef.current?.focus();
  };

  const handleEditCoach = (coach: Coach) => {
    setEditingCoachId(coach.id);
    setCoachFormData({
      title: coach.title,
      description: coach.description || '',
      system_prompt: coach.system_prompt,
      category: coach.category,
    });
    setShowCoachModal(true);
  };

  const handleDeleteCoach = (coach: Coach) => {
    setCoachDeleteConfirmation({ id: coach.id, title: coach.title });
  };

  const handleConfirmCoachDelete = () => {
    if (coachDeleteConfirmation) {
      deleteCoach.mutate(coachDeleteConfirmation.id);
    }
  };

  // Conversation management handlers
  const handleStartRename = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setEditingTitle(conv.id);
    setEditedTitleValue(conv.title ?? 'Untitled Chat');
  };

  const handleSaveRename = (id: string) => {
    if (editedTitleValue.trim()) {
      updateConversation.mutate({ id, title: editedTitleValue.trim() });
    } else {
      setEditingTitle(null);
      setEditedTitleValue('');
    }
  };

  const handleCancelRename = () => {
    setEditingTitle(null);
    setEditedTitleValue('');
  };

  const handleDeleteConversation = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setDeleteConfirmation({ id: conv.id, title: conv.title });
  };

  const handleConfirmDeleteConversation = () => {
    if (deleteConfirmation) {
      deleteConversation.mutate(deleteConfirmation.id);
    }
  };

  const handleConnectProvider = (provider: string) => {
    setConnectingProvider(provider);
    if (selectedConversation) {
      localStorage.setItem('pierre_oauth_conversation', selectedConversation);
    }
    window.open(`/api/oauth/${provider}/connect`, '_blank');
  };

  // Message action handlers
  const handleCopyMessage = useCallback((content: string) => {
    navigator.clipboard.writeText(stripContextPrefix(content));
  }, []);

  const handleShareMessage = useCallback((content: string) => {
    // Use native Web Share API if available, otherwise copy to clipboard
    const strippedContent = stripContextPrefix(content);
    if (navigator.share) {
      navigator.share({
        title: 'Pierre AI Insight',
        text: strippedContent,
      }).catch(() => {
        // User cancelled share, ignore
      });
    } else {
      navigator.clipboard.writeText(strippedContent);
    }
  }, []);

  const handleThumbsUp = useCallback((messageId: string) => {
    setMessageFeedback(prev => {
      const newMap = new Map(prev);
      const current = newMap.get(messageId);
      // Toggle: if already up, remove; otherwise set to up
      newMap.set(messageId, current === 'up' ? null : 'up');
      return newMap;
    });
  }, []);

  const handleThumbsDown = useCallback((messageId: string) => {
    setMessageFeedback(prev => {
      const newMap = new Map(prev);
      const current = newMap.get(messageId);
      // Toggle: if already down, remove; otherwise set to down
      newMap.set(messageId, current === 'down' ? null : 'down');
      return newMap;
    });
  }, []);

  const handleRetryMessage = useCallback(async (messageId: string) => {
    if (!selectedConversation || isStreaming) return;

    // Find the message to retry and get the preceding user message
    const messages = messagesData?.messages || [];
    const messageIndex = messages.findIndex(m => m.id === messageId);
    if (messageIndex <= 0) return;

    // Find the user message that preceded this assistant message
    let userMessageIndex = messageIndex - 1;
    while (userMessageIndex >= 0 && messages[userMessageIndex].role !== 'user') {
      userMessageIndex--;
    }
    if (userMessageIndex < 0) return;

    const userMessage = messages[userMessageIndex];
    setNewMessage(userMessage.content);

    // Use setTimeout to allow state update before triggering send
    setTimeout(() => {
      const sendButton = document.querySelector('[aria-label="Send message"]') as HTMLButtonElement;
      sendButton?.click();
    }, 100);
  }, [selectedConversation, isStreaming, messagesData?.messages]);

  const handleProviderModalClose = () => {
    setShowProviderModal(false);
    setPendingCoachAction(null);
    localStorage.removeItem('pierre_pending_coach_action');
  };

  const handleProviderModalSkip = () => {
    setShowProviderModal(false);
    if (pendingCoachAction) {
      setPendingPrompt(pendingCoachAction.prompt);
      if (pendingCoachAction.systemPrompt) {
        setPendingSystemPrompt(pendingCoachAction.systemPrompt);
      }
      createConversation.mutate(pendingCoachAction.systemPrompt);
    }
    setPendingCoachAction(null);
    localStorage.removeItem('pierre_pending_coach_action');
  };

  // Coach form handlers
  const handleCoachFormSubmit = () => {
    if (editingCoachId) {
      updateCoach.mutate({ id: editingCoachId, data: coachFormData });
    } else {
      createCoach.mutate(coachFormData);
    }
  };

  const handleCoachFormClose = () => {
    setShowCoachModal(false);
    setEditingCoachId(null);
    setCoachFormData(DEFAULT_COACH_FORM_DATA);
  };

  return (
    <div className="h-full flex bg-pierre-dark relative">
      {/* Conversations Panel */}
      <div
        className={clsx(
          'flex-shrink-0 border-r border-white/5 bg-pierre-slate/50 transition-all duration-300 flex flex-col',
          conversationsPanelOpen ? 'w-72' : 'w-0 overflow-hidden'
        )}
      >
        {/* Panel Header */}
        <div className="p-4 border-b border-white/5 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <History className="w-4 h-4 text-zinc-400" />
            <span className="text-sm font-medium text-zinc-300">Recent Chats</span>
          </div>
          <button
            onClick={() => createConversation.mutate()}
            disabled={createConversation.isPending}
            className="p-1.5 text-zinc-400 hover:text-white hover:bg-white/5 rounded-lg transition-colors"
            title="New chat"
          >
            <Plus className="w-4 h-4" />
          </button>
        </div>

        {/* Conversations List */}
        <div className="flex-1 overflow-y-auto py-2 px-2 space-y-0.5">
          {conversationsLoading ? (
            <div className="p-4 text-center text-zinc-500 text-sm">Loading...</div>
          ) : conversations.length === 0 ? (
            <div className="p-4 text-center text-zinc-500 text-sm">No conversations yet</div>
          ) : (
            conversations.map((conv) => (
              <ConversationItem
                key={conv.id}
                conversation={conv}
                isSelected={selectedConversation === conv.id}
                isEditing={editingTitle === conv.id}
                editedTitleValue={editedTitleValue}
                onSelect={() => setSelectedConversation(conv.id)}
                onStartRename={(e) => handleStartRename(e, conv)}
                onDelete={(e) => handleDeleteConversation(e, conv)}
                onTitleChange={setEditedTitleValue}
                onSaveRename={() => handleSaveRename(conv.id)}
                onCancelRename={handleCancelRename}
              />
            ))
          )}
        </div>
      </div>

      {/* Panel Toggle Button */}
      <button
        onClick={() => setConversationsPanelOpen(!conversationsPanelOpen)}
        className="absolute left-0 top-1/2 -translate-y-1/2 z-10 p-1.5 bg-pierre-slate border border-white/10 rounded-r-lg text-zinc-400 hover:text-white hover:bg-white/5 transition-colors"
        style={{ left: conversationsPanelOpen ? '286px' : '0px' }}
        title={conversationsPanelOpen ? 'Hide conversations' : 'Show conversations'}
      >
        {conversationsPanelOpen ? (
          <PanelLeftClose className="w-4 h-4" />
        ) : (
          <PanelLeft className="w-4 h-4" />
        )}
      </button>

      {/* Main Content Area */}
      <div className="flex-1 flex flex-col min-w-0">
        {!selectedConversation ? (
          /* Welcome View */
          <div className="flex-1 flex flex-col overflow-hidden">
            <div className="p-6 border-b border-white/5 flex items-center justify-between flex-shrink-0">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 flex items-center justify-center rounded-xl bg-gradient-to-br from-pierre-violet to-pierre-cyan text-white shadow-glow-sm">
                  <MessageCircle className="w-5 h-5" />
                </div>
                <div>
                  <h2 className="text-xl font-semibold text-white">Chat</h2>
                  <p className="text-sm text-zinc-400">
                    {hasConnectedProvider
                      ? providersData?.providers?.filter(p => p.connected).map(p =>
                          p.display_name
                        ).join(', ') + ' connected'
                      : 'No provider connected'}
                  </p>
                </div>
              </div>
              <button
                onClick={() => createConversation.mutate()}
                disabled={createConversation.isPending}
                className="flex items-center gap-1.5 px-4 py-2 text-sm font-medium text-white bg-pierre-violet rounded-lg hover:bg-pierre-violet-dark transition-colors shadow-glow-sm hover:shadow-glow"
              >
                <Plus className="w-4 h-4" />
                New Chat
              </button>
            </div>

            <div className="flex-1 overflow-y-auto">
              <div className="w-full max-w-5xl mx-auto px-6 py-8">
                <div className="text-center mb-8">
                  <h2 className="text-2xl font-semibold text-white mb-2">Ready to analyze your fitness</h2>
                  <p className="text-zinc-400 text-sm">
                    {hasConnectedProvider
                      ? 'Get personalized insights from your activity data'
                    : 'Select a coach to get started - connect your data anytime'}
                </p>

                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (newMessage.trim()) {
                      setPendingPrompt(newMessage.trim());
                      createConversation.mutate();
                    }
                  }}
                  className="relative mt-6 max-w-2xl mx-auto"
                >
                  <input
                    type="text"
                    value={newMessage}
                    onChange={(e) => setNewMessage(e.target.value)}
                    placeholder="Message Pierre..."
                    className="w-full rounded-xl border border-white/10 bg-[#151520] text-white placeholder-zinc-500 pl-4 pr-24 py-3.5 focus:outline-none focus:ring-2 focus:ring-pierre-violet/30 focus:border-pierre-violet text-sm transition-colors"
                    disabled={createConversation.isPending}
                  />
                  <button
                    type="submit"
                    disabled={!newMessage.trim() || createConversation.isPending}
                    className="absolute right-2 top-1/2 -translate-y-1/2 px-4 py-1.5 bg-pierre-violet text-white text-sm font-medium rounded-lg hover:bg-pierre-violet-dark transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1.5"
                  >
                    {createConversation.isPending ? (
                      <div className="pierre-spinner w-4 h-4 border-white border-t-transparent" />
                    ) : (
                      <>
                        Send
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 5l7 7m0 0l-7 7m7-7H3" />
                        </svg>
                      </>
                    )}
                  </button>
                </form>
              </div>

              <PromptSuggestions
                onSelectPrompt={handleSelectPrompt}
                onEditCoach={handleEditCoach}
                onDeleteCoach={handleDeleteCoach}
              />
            </div>
          </div>
        </div>
      ) : (
        /* Active Conversation View */
        <div className="h-full flex flex-col">
          {/* Conversation Header with Create Coach button */}
          {(messagesData?.messages?.length ?? 0) >= 2 && (
            <div className="border-b border-white/5 px-6 py-3 flex items-center justify-end">
              <button
                onClick={() => setShowCreateCoachFromConversation(true)}
                disabled={isStreaming}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-pierre-violet bg-pierre-violet/10 hover:bg-pierre-violet/20 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                title="Create a coach based on this conversation"
              >
                <Sparkles className="w-3.5 h-3.5" />
                Create Coach
              </button>
            </div>
          )}
          <div className="flex-1 overflow-y-auto min-h-0">
            <div className="max-w-3xl mx-auto py-6 px-6">
              <MessageList
                messages={messagesData?.messages || []}
                messageMetadata={messageMetadata}
                messageFeedback={messageFeedback}
                isLoading={messagesLoading}
                isStreaming={isStreaming}
                streamingContent={streamingContent}
                errorMessage={errorMessage}
                errorCountdown={errorCountdown}
                oauthNotification={oauthNotification}
                onDismissError={() => { setErrorMessage(null); setErrorCountdown(null); }}
                onDismissOAuthNotification={() => setOauthNotification(null)}
                onCopyMessage={handleCopyMessage}
                onShareMessage={handleShareMessage}
                onShareToFeed={(content) => setShareMessageContent(stripContextPrefix(content))}
                onThumbsUp={handleThumbsUp}
                onThumbsDown={handleThumbsDown}
                onRetryMessage={handleRetryMessage}
              />
            </div>
          </div>

          <MessageInput
            value={newMessage}
            onChange={setNewMessage}
            onSend={handleSendMessage}
            isStreaming={isStreaming}
            showIdeas={showIdeas}
            onToggleIdeas={() => setShowIdeas(!showIdeas)}
            onSelectPrompt={handleFillPrompt}
          />
        </div>
      )}
      </div>

      {/* Modals and Dialogs */}
      <ConfirmDialog
        isOpen={!!deleteConfirmation}
        onClose={() => setDeleteConfirmation(null)}
        onConfirm={handleConfirmDeleteConversation}
        title="Delete Conversation"
        message={`Are you sure you want to delete "${deleteConfirmation?.title || 'this conversation'}"? This action cannot be undone.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={deleteConversation.isPending}
      />
      <ProviderConnectionModal
        isOpen={showProviderModal}
        onClose={handleProviderModalClose}
        onConnectProvider={handleConnectProvider}
        connectingProvider={connectingProvider}
        onSkip={handleProviderModalSkip}
        isSkipPending={createConversation.isPending}
      />

      <CoachFormModal
        isOpen={showCoachModal}
        isEditing={!!editingCoachId}
        formData={coachFormData}
        onFormDataChange={setCoachFormData}
        onSubmit={handleCoachFormSubmit}
        onClose={handleCoachFormClose}
        isSubmitting={editingCoachId ? updateCoach.isPending : createCoach.isPending}
        submitError={createCoach.isError || updateCoach.isError}
      />

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

      {shareMessageContent && (
        <ShareChatMessageModal
          content={shareMessageContent}
          onClose={() => setShareMessageContent(null)}
          onSuccess={() => setShareMessageContent(null)}
        />
      )}

      {selectedConversation && (
        <CreateCoachFromConversationModal
          isOpen={showCreateCoachFromConversation}
          conversationId={selectedConversation}
          messageCount={messagesData?.messages?.length ?? 0}
          onClose={() => setShowCreateCoachFromConversation(false)}
          onSuccess={() => {
            setShowCreateCoachFromConversation(false);
            queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
          }}
        />
      )}
    </div>
  );
}
