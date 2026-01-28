// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: AI Chat tab component for users to interact with fitness AI assistant
// ABOUTME: Features Claude.ai-style two-column layout with sidebar and chat area

import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Panel, Group as PanelGroup, Separator as PanelResizeHandle, usePanelRef } from 'react-resizable-panels';
import { ConfirmDialog } from './ui';
import { clsx } from 'clsx';
import { apiService } from '../services/api';
import PromptSuggestions from './PromptSuggestions';
import StoreScreen from './StoreScreen';
import StoreCoachDetail from './StoreCoachDetail';
import { useAuth } from '../hooks/useAuth';
import { MessageCircle, Users, Plus, ChevronRight, Compass, Sparkles } from 'lucide-react';
import { ShareChatMessageModal } from './social';
import {
  ChatSidebar,
  MessageList,
  MessageInput,
  MyCoachCard,
  CategoryFilterButton,
  ProviderConnectionModal,
  CoachFormModal,
  CreateCoachFromConversationModal,
  stripContextPrefix,
  getCategoryBadgeClass,
  getCategoryIcon,
  COACH_CATEGORIES,
  DEFAULT_COACH_FORM_DATA,
} from './chat';
import type {
  Message,
  ConversationListResponse,
  Conversation,
  Coach,
  MessageMetadata,
  OAuthNotification,
  DeleteConfirmation,
  CoachDeleteConfirmation,
  PendingCoachAction,
  CoachFormData,
} from './chat';

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
  const [oauthNotification, setOauthNotification] = useState<OAuthNotification | null>(null);
  const [deleteConfirmation, setDeleteConfirmation] = useState<DeleteConfirmation | null>(null);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);
  const [pendingSystemPrompt, setPendingSystemPrompt] = useState<string | null>(null);
  const [showIdeas, setShowIdeas] = useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [connectingProvider, setConnectingProvider] = useState<string | null>(null);
  const [showProviderModal, setShowProviderModal] = useState(false);
  const [pendingCoachAction, setPendingCoachAction] = useState<PendingCoachAction | null>(null);
  const [messageMetadata, setMessageMetadata] = useState<Map<string, MessageMetadata>>(new Map());
  const [showCoachModal, setShowCoachModal] = useState(false);
  const [showMyCoachesPanel, setShowMyCoachesPanel] = useState(false);
  const [showStorePanel, setShowStorePanel] = useState(false);
  const [selectedStoreCoach, setSelectedStoreCoach] = useState<string | null>(null);
  const [coachesCategoryFilter, setCoachesCategoryFilter] = useState<string | null>(null);
  const [coachesSearchQuery, setCoachesSearchQuery] = useState('');
  const [editingCoachId, setEditingCoachId] = useState<string | null>(null);
  const [coachFormData, setCoachFormData] = useState<CoachFormData>(DEFAULT_COACH_FORM_DATA);
  const [coachDeleteConfirmation, setCoachDeleteConfirmation] = useState<CoachDeleteConfirmation | null>(null);
  const [shareMessageContent, setShareMessageContent] = useState<string | null>(null);
  const [showCreateCoachFromConversation, setShowCreateCoachFromConversation] = useState(false);

  const sidebarPanelRef = usePanelRef();
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Fetch conversations
  const { data: conversationsData, isLoading: conversationsLoading } = useQuery<ConversationListResponse>({
    queryKey: ['chat-conversations'],
    queryFn: () => apiService.getConversations(),
  });

  // Fetch OAuth status
  const { data: oauthStatus } = useQuery({
    queryKey: ['oauth-status'],
    queryFn: () => apiService.getOAuthStatus(),
  });

  const hasConnectedProvider = oauthStatus?.providers?.some(p => p.connected) ?? false;

  // Fetch coaches for My Coaches panel
  const { data: coachesData, isLoading: coachesLoading } = useQuery({
    queryKey: ['user-coaches'],
    queryFn: () => apiService.getCoaches(),
    staleTime: 5 * 60 * 1000,
    enabled: showMyCoachesPanel,
  });

  // Fetch hidden coaches
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

  // Mutations
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

  const updateConversation = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) =>
      apiService.updateConversation(id, { title }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setEditingTitle(null);
      setEditedTitleValue('');
    },
  });

  const deleteConversationMutation = useMutation({
    mutationFn: (id: string) => apiService.deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      if (selectedConversation) {
        setSelectedConversation(null);
      }
    },
  });

  const createCoach = useMutation({
    mutationFn: (data: CoachFormData) => apiService.createCoach(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowCoachModal(false);
      setCoachFormData(DEFAULT_COACH_FORM_DATA);
    },
  });

  const updateCoach = useMutation({
    mutationFn: ({ id, data }: { id: string; data: CoachFormData }) => apiService.updateCoach(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowCoachModal(false);
      setEditingCoachId(null);
      setCoachFormData(DEFAULT_COACH_FORM_DATA);
    },
  });

  const deleteCoach = useMutation({
    mutationFn: (id: string) => apiService.deleteCoach(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setCoachDeleteConfirmation(null);
    },
  });

  const hideCoach = useMutation({
    mutationFn: (coachId: string) => apiService.hideCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      queryClient.invalidateQueries({ queryKey: ['hidden-coaches'] });
    },
  });

  const showCoachMutation = useMutation({
    mutationFn: (coachId: string) => apiService.showCoach(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      queryClient.invalidateQueries({ queryKey: ['hidden-coaches'] });
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
      const connectedProviders = oauthStatus?.providers?.filter(p => p.connected).map(p =>
        p.provider.charAt(0).toUpperCase() + p.provider.slice(1)
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
      const response = await fetch(`/api/conversations/${selectedConversation}/messages/stream`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${localStorage.getItem('auth_token')}`,
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
  }, [newMessage, selectedConversation, isStreaming, connectingProvider, oauthNotification, hasConnectedProvider, messagesData?.messages, oauthStatus?.providers, queryClient]);

  // Sidebar handlers
  const handleStartRename = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setEditingTitle(conv.id);
    setEditedTitleValue(conv.title || '');
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

  const handleConfirmDelete = () => {
    if (deleteConfirmation) {
      deleteConversationMutation.mutate(deleteConfirmation.id);
      setDeleteConfirmation(null);
    }
  };

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

  const handleConnectProvider = (provider: string) => {
    setConnectingProvider(provider);
    if (selectedConversation) {
      localStorage.setItem('pierre_oauth_conversation', selectedConversation);
    }
    window.open(`/api/oauth/${provider}/connect`, '_blank');
  };

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

  const toggleSidebar = () => {
    if (sidebarCollapsed) {
      sidebarPanelRef.current?.resize(18);
    } else {
      sidebarPanelRef.current?.resize(0);
    }
    setSidebarCollapsed(!sidebarCollapsed);
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

  // Filter coaches
  const getFilteredCoaches = () => {
    const coaches: Coach[] = coachesData?.coaches || [];
    let filtered = coaches;

    if (coachesSearchQuery) {
      const query = coachesSearchQuery.toLowerCase();
      filtered = filtered.filter(c =>
        c.title.toLowerCase().includes(query) ||
        c.description?.toLowerCase().includes(query)
      );
    }

    if (coachesCategoryFilter && coachesCategoryFilter !== '__hidden__') {
      filtered = filtered.filter(c => c.category === coachesCategoryFilter);
    }

    return filtered;
  };

  // Render My Coaches panel content
  const renderMyCoachesContent = () => {
    if (coachesCategoryFilter === '__hidden__') {
      const hiddenCoaches: Coach[] = hiddenCoachesData?.coaches || [];
      if (hiddenCoaches.length === 0) {
        return (
          <div className="text-center py-12 text-zinc-500">
            <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-white/5 flex items-center justify-center">
              <svg className="w-8 h-8 text-zinc-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
              </svg>
            </div>
            <p className="text-sm">No hidden coaches</p>
          </div>
        );
      }

      return (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {hiddenCoaches.map((coach) => (
            <div
              key={coach.id}
              className="relative text-left text-sm rounded-xl border border-white/5 px-4 py-3 opacity-60 hover:opacity-100 transition-all group bg-white/5"
            >
              <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={() => showCoachMutation.mutate(coach.id)}
                  disabled={showCoachMutation.isPending}
                  className="p-1.5 text-zinc-400 hover:text-pierre-activity hover:bg-pierre-activity/10 rounded-lg transition-colors disabled:opacity-50"
                  title="Show coach"
                  aria-label="Show coach"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                  </svg>
                </button>
              </div>
              <div className="flex items-center justify-between">
                <span className="font-medium text-zinc-400">{coach.title}</span>
                <span className={clsx('text-xs px-1.5 py-0.5 rounded', getCategoryBadgeClass(coach.category))}>
                  {getCategoryIcon(coach.category)}
                </span>
              </div>
              {coach.description && (
                <p className="text-zinc-500 text-xs mt-1 line-clamp-2">{coach.description}</p>
              )}
            </div>
          ))}
        </div>
      );
    }

    const filteredCoaches = getFilteredCoaches();
    const userCoaches = filteredCoaches.filter(c => !c.is_system);
    const systemCoaches = filteredCoaches.filter(c => c.is_system);

    if (filteredCoaches.length === 0) {
      return (
        <div className="text-center py-12 text-zinc-500">
          <p className="text-sm">No coaches found</p>
        </div>
      );
    }

    return (
      <div className="space-y-6">
        {userCoaches.length > 0 && (
          <div>
            <h4 className="text-xs font-bold text-zinc-500 tracking-wider uppercase mb-3">Your Coaches</h4>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {userCoaches.map((coach) => (
                <MyCoachCard
                  key={coach.id}
                  coach={coach}
                  onSelect={() => handleSelectPrompt(`Hello ${coach.title}!`, coach.system_prompt)}
                  onEdit={() => handleEditCoach(coach)}
                  onDelete={() => handleDeleteCoach(coach)}
                  onHide={() => hideCoach.mutate(coach.id)}
                  isHiding={hideCoach.isPending}
                />
              ))}
            </div>
          </div>
        )}
        {systemCoaches.length > 0 && (
          <div>
            <h4 className="text-xs font-bold text-zinc-500 tracking-wider uppercase mb-3">System Coaches</h4>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {systemCoaches.map((coach) => (
                <MyCoachCard
                  key={coach.id}
                  coach={coach}
                  onSelect={() => handleSelectPrompt(`Hello ${coach.title}!`, coach.system_prompt)}
                  onEdit={() => handleEditCoach(coach)}
                  onDelete={() => handleDeleteCoach(coach)}
                  onHide={() => hideCoach.mutate(coach.id)}
                  isHiding={hideCoach.isPending}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    );
  };

  return (
    <PanelGroup orientation="horizontal" className="h-full">
      {/* Sidebar */}
      <Panel
        panelRef={sidebarPanelRef}
        defaultSize="18%"
        minSize="12%"
        maxSize="30%"
        collapsible
        collapsedSize="0%"
        onResize={(size) => setSidebarCollapsed(size.asPercentage === 0)}
        className="bg-pierre-dark flex flex-col relative border-r border-white/5"
      >
        <ChatSidebar
          conversations={conversationsData?.conversations || []}
          conversationsLoading={conversationsLoading}
          selectedConversation={selectedConversation}
          showMyCoachesPanel={showMyCoachesPanel}
          showStorePanel={showStorePanel}
          editingTitle={editingTitle}
          editedTitleValue={editedTitleValue}
          user={user}
          onNewChat={() => {
            setShowMyCoachesPanel(false);
            setShowStorePanel(false);
            createConversation.mutate();
          }}
          onSelectConversation={(id) => {
            if (id) {
              setShowMyCoachesPanel(false);
              setShowStorePanel(false);
              setSelectedConversation(id);
            } else {
              setSelectedConversation(null);
              setShowMyCoachesPanel(false);
              setShowStorePanel(false);
            }
          }}
          onShowMyCoaches={() => {
            setSelectedConversation(null);
            setShowStorePanel(false);
            setShowMyCoachesPanel(true);
          }}
          onShowStore={() => {
            setSelectedConversation(null);
            setShowMyCoachesPanel(false);
            setShowStorePanel(true);
            setSelectedStoreCoach(null);
          }}
          onOpenSettings={onOpenSettings}
          onStartRename={handleStartRename}
          onSaveRename={handleSaveRename}
          onCancelRename={handleCancelRename}
          onTitleChange={setEditedTitleValue}
          onDeleteConversation={handleDeleteConversation}
          isCreatingConversation={createConversation.isPending}
        />
      </Panel>

      {/* Resize Handle */}
      <PanelResizeHandle className="w-1 bg-pierre-dark/50 hover:bg-pierre-violet/30 transition-colors relative group">
        <button
          onClick={toggleSidebar}
          className={clsx(
            'absolute top-3 -left-3 w-6 h-6 rounded-full bg-pierre-slate border border-white/10 shadow-sm flex items-center justify-center text-zinc-400 hover:text-pierre-violet hover:border-pierre-violet transition-all z-10',
            sidebarCollapsed ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
          )}
          title={sidebarCollapsed ? 'Show sidebar' : 'Hide sidebar'}
        >
          <ChevronRight className={clsx('w-3 h-3', !sidebarCollapsed && 'rotate-180')} />
        </button>
      </PanelResizeHandle>

      {/* Main Chat Area */}
      <Panel defaultSize="82%" className="flex flex-col bg-pierre-dark">
        {/* Coach Store View */}
        {showStorePanel && !selectedConversation ? (
          selectedStoreCoach ? (
            <StoreCoachDetail
              coachId={selectedStoreCoach}
              onBack={() => setSelectedStoreCoach(null)}
              onNavigateToLibrary={() => {
                setShowStorePanel(false);
                setSelectedStoreCoach(null);
                setShowMyCoachesPanel(true);
              }}
            />
          ) : (
            <StoreScreen
              onNavigateToCoaches={() => {
                setShowStorePanel(false);
                setSelectedStoreCoach(null);
                setShowMyCoachesPanel(true);
              }}
            />
          )
        ) : showMyCoachesPanel && !selectedConversation ? (
          /* My Coaches Panel */
          <div className="flex-1 flex flex-col overflow-hidden">
            <div className="p-6 border-b border-white/5 flex items-center justify-between flex-shrink-0">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 flex items-center justify-center rounded-xl bg-gradient-to-br from-pierre-violet to-pierre-recovery-dark text-white shadow-glow-sm">
                  <Users className="w-5 h-5" />
                </div>
                <div>
                  <h2 className="text-xl font-semibold text-white">My Coaches</h2>
                  <p className="text-sm text-zinc-400">Select a coach to start chatting</p>
                </div>
              </div>
              <button
                onClick={() => {
                  setShowMyCoachesPanel(false);
                  setEditingCoachId(null);
                  setCoachFormData(DEFAULT_COACH_FORM_DATA);
                  setShowCoachModal(true);
                }}
                className="flex items-center gap-1.5 px-4 py-2 text-sm font-medium text-white bg-pierre-violet rounded-lg hover:bg-pierre-violet-dark transition-colors shadow-glow-sm hover:shadow-glow"
              >
                <Plus className="w-4 h-4" />
                Add Coach
              </button>
            </div>

            {/* Search Bar */}
            <div className="px-6 py-4 border-b border-white/10">
              <div className="relative">
                <svg className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
                <input
                  type="text"
                  placeholder="Search coaches..."
                  value={coachesSearchQuery}
                  onChange={(e) => setCoachesSearchQuery(e.target.value)}
                  className="w-full pl-10 pr-10 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-pierre-violet/30 focus:border-pierre-violet transition-colors"
                />
                {coachesSearchQuery && (
                  <button onClick={() => setCoachesSearchQuery('')} className="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-gray-300">
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                )}
              </div>
            </div>

            {/* Category Filters */}
            <div className="px-6 py-4 border-b border-white/5 flex-shrink-0">
              <div className="flex items-center gap-2 overflow-x-auto pb-1">
                <CategoryFilterButton
                  category={null}
                  label="All"
                  isSelected={coachesCategoryFilter === null}
                  onClick={() => setCoachesCategoryFilter(null)}
                  showIcon={false}
                />
                {COACH_CATEGORIES.map((category) => (
                  <CategoryFilterButton
                    key={category}
                    category={category}
                    label={category}
                    isSelected={coachesCategoryFilter === category}
                    onClick={() => setCoachesCategoryFilter(category)}
                  />
                ))}
                {(hiddenCoachesData?.coaches?.length ?? 0) > 0 && (
                  <button
                    onClick={() => setCoachesCategoryFilter(coachesCategoryFilter === '__hidden__' ? null : '__hidden__')}
                    className={clsx(
                      'px-4 py-2 text-sm font-medium rounded-full whitespace-nowrap transition-colors flex items-center gap-1.5',
                      coachesCategoryFilter === '__hidden__'
                        ? 'bg-zinc-600 text-white shadow-sm'
                        : 'bg-white/5 text-zinc-500 hover:bg-white/10 hover:text-zinc-300 border border-dashed border-zinc-600'
                    )}
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                    </svg>
                    Hidden ({hiddenCoachesData?.coaches?.length})
                  </button>
                )}
              </div>
            </div>

            {/* Coaches List */}
            <div className="flex-1 overflow-y-auto p-6">
              {coachesLoading ? (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {[1, 2, 3, 4, 5, 6].map((i) => (
                    <div key={i} className="h-24 bg-white/5 rounded-xl animate-pulse" />
                  ))}
                </div>
              ) : (
                renderMyCoachesContent()
              )}
            </div>
          </div>
        ) : !selectedConversation ? (
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
                      ? oauthStatus?.providers?.filter(p => p.connected).map(p =>
                          p.provider.charAt(0).toUpperCase() + p.provider.slice(1)
                        ).join(', ') + ' connected'
                      : 'No provider connected'}
                  </p>
                </div>
              </div>
              <button
                onClick={() => {
                  setShowMyCoachesPanel(false);
                  setShowStorePanel(false);
                  createConversation.mutate();
                }}
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

                <div className="text-center mt-6">
                  <button
                    onClick={() => {
                      setShowMyCoachesPanel(false);
                      setShowStorePanel(true);
                    }}
                    className="inline-flex items-center gap-2 px-4 py-2 text-sm text-zinc-400 hover:text-white hover:bg-white/5 rounded-lg transition-colors"
                  >
                    <Compass className="w-4 h-4" />
                    Discover more coaches
                  </button>
                </div>
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
                  isLoading={messagesLoading}
                  isStreaming={isStreaming}
                  streamingContent={streamingContent}
                  errorMessage={errorMessage}
                  errorCountdown={errorCountdown}
                  oauthNotification={oauthNotification}
                  onDismissError={() => { setErrorMessage(null); setErrorCountdown(null); }}
                  onDismissOAuthNotification={() => setOauthNotification(null)}
                  onShareMessage={(content) => setShareMessageContent(stripContextPrefix(content))}
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
      </Panel>

      {/* Modals and Dialogs */}
      <ConfirmDialog
        isOpen={!!deleteConfirmation}
        onClose={() => setDeleteConfirmation(null)}
        onConfirm={handleConfirmDelete}
        title="Delete Conversation"
        message={`Are you sure you want to delete "${deleteConfirmation?.title || 'this conversation'}"? This action cannot be undone.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={deleteConversationMutation.isPending}
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
            setShowMyCoachesPanel(true);
            setSelectedConversation(null);
          }}
        />
      )}
    </PanelGroup>
  );
}
