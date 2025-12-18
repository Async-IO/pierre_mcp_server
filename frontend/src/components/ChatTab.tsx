// ABOUTME: AI Chat tab component for users to interact with fitness AI assistant
// ABOUTME: Features Claude.ai-style two-column layout with sidebar and chat area
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from './ui';
import { clsx } from 'clsx';
import { apiService } from '../services/api';
import Markdown from 'react-markdown';

// Convert plain URLs to markdown links for clickability
// Matches http/https URLs that aren't already in markdown link format
const urlRegex = /(?<!\]\()(?<!\[)(https?:\/\/[^\s<>[\]()]+)/g;
const linkifyUrls = (text: string): string => {
  return text.replace(urlRegex, (url) => `[${url}](${url})`);
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
  const [editingTitle, setEditingTitle] = useState<string | null>(null);
  const [editedTitleValue, setEditedTitleValue] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const titleInputRef = useRef<HTMLInputElement>(null);

  // Fetch conversations
  const { data: conversationsData, isLoading: conversationsLoading } = useQuery<ConversationListResponse>({
    queryKey: ['chat-conversations'],
    queryFn: () => apiService.getConversations(),
  });

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
        if (stored) {
          const result = JSON.parse(stored);
          // Only process if it's recent (within last 5 minutes)
          const fiveMinutesAgo = Date.now() - 5 * 60 * 1000;
          if (result.type === 'oauth_completed' && result.success && result.timestamp > fiveMinutesAgo) {
            console.log(`OAuth completed (detected on focus): ${result.provider} connected successfully`);
            queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
            queryClient.invalidateQueries({ queryKey: ['user-profile'] });
            // Clean up the storage item
            localStorage.removeItem('pierre_oauth_result');
          } else if (result.timestamp <= fiveMinutesAgo) {
            // Clean up stale entries
            localStorage.removeItem('pierre_oauth_result');
          }
        }
      } catch {
        // Ignore parse errors
      }
    };

    const handleOAuthMessage = (event: MessageEvent) => {
      // Validate message structure
      if (event.data?.type === 'oauth_completed') {
        const { provider, success } = event.data;
        if (success) {
          // Show success notification in console (toast could be added)
          console.log(`OAuth completed: ${provider} connected successfully`);
          // Invalidate any queries that depend on connection status
          queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
          queryClient.invalidateQueries({ queryKey: ['user-profile'] });
        }
      }
    };

    const handleStorageChange = (event: StorageEvent) => {
      if (event.key === 'pierre_oauth_result' && event.newValue) {
        try {
          const result = JSON.parse(event.newValue);
          if (result.type === 'oauth_completed' && result.success) {
            console.log(`OAuth completed (via storage): ${result.provider} connected successfully`);
            queryClient.invalidateQueries({ queryKey: ['oauth-status'] });
            queryClient.invalidateQueries({ queryKey: ['user-profile'] });
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

  const handleSendMessage = useCallback(async () => {
    if (!newMessage.trim() || !selectedConversation || isStreaming) return;

    const messageContent = newMessage.trim();
    setNewMessage('');
    setIsStreaming(true);
    setStreamingContent('');
    setErrorMessage(null);

    try {
      // Optimistically add user message to UI
      queryClient.setQueryData(['chat-messages', selectedConversation], (old: { messages: Message[] } | undefined) => ({
        messages: [
          ...(old?.messages || []),
          {
            id: `temp-${Date.now()}`,
            role: 'user' as const,
            content: messageContent,
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
        // Parse user-friendly error messages
        let userMessage = errorData.message || 'Failed to send message';
        if (userMessage.includes('quota') || userMessage.includes('429') || userMessage.includes('rate limit')) {
          userMessage = 'AI service is temporarily unavailable due to rate limiting. Please try again in a few seconds.';
        } else if (response.status === 500) {
          userMessage = 'The AI service encountered an error. Please try again.';
        }
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
    } catch (error) {
      console.error('Failed to send message:', error);
      const errorMsg = error instanceof Error ? error.message : 'Failed to send message';
      setErrorMessage(errorMsg);
    } finally {
      setIsStreaming(false);
      setStreamingContent('');
    }
  }, [newMessage, selectedConversation, isStreaming, queryClient]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  const handleNewChat = () => {
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

  const handleDeleteConversation = (e: React.MouseEvent, convId: string) => {
    e.stopPropagation();
    if (confirm('Delete this conversation?')) {
      deleteConversation.mutate(convId);
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
    <div className="flex h-[calc(100vh-8rem)] -mx-6 -mt-6">
      {/* Left Sidebar - Conversation List */}
      <div className="w-72 flex-shrink-0 border-r border-pierre-gray-200 bg-pierre-gray-50 flex flex-col">
        {/* Header with New Chat Button */}
        <div className="p-3 flex items-center justify-between">
          <button
            onClick={handleNewChat}
            disabled={createConversation.isPending}
            title="New conversation"
            aria-label="New conversation"
            className="w-8 h-8 flex items-center justify-center rounded-lg bg-pierre-violet text-white hover:bg-pierre-violet/90 transition-colors disabled:opacity-50 shadow-sm"
          >
            {createConversation.isPending ? (
              <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24" fill="none">
                <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" strokeOpacity="0.25" />
                <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
              </svg>
            ) : (
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
              </svg>
            )}
          </button>
          {conversationsData?.conversations && conversationsData.conversations.length > 0 && (
            <span className="text-xs text-pierre-gray-500">
              {conversationsData.conversations.length} chats
            </span>
          )}
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
                        onClick={(e) => handleDeleteConversation(e, conv.id)}
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
      </div>

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col bg-white">
        {!selectedConversation ? (
          // Empty state - centered hero
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center max-w-md px-6">
              <div className="w-16 h-16 mx-auto mb-4 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-2xl flex items-center justify-center shadow-lg">
                <svg className="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
                </svg>
              </div>
              <h2 className="text-xl font-semibold text-pierre-gray-900 mb-2">Pierre Fitness Intelligence Assistant</h2>
              <p className="text-pierre-gray-600 text-sm mb-4">
                Ask about your fitness data, get training insights, analyze activities, or explore personalized recommendations.
              </p>
              <Button onClick={handleNewChat} variant="primary" disabled={createConversation.isPending}>
                {createConversation.isPending ? 'Creating...' : 'Start a conversation'}
              </Button>
            </div>
          </div>
        ) : (
          <>
            {/* Messages Area */}
            <div className="flex-1 overflow-y-auto">
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
                              {linkifyUrls(msg.content)}
                            </Markdown>
                          </div>
                        </div>
                      </div>
                    ))}

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
                            <p className="text-red-700 text-sm">{errorMessage}</p>
                            <button
                              onClick={() => setErrorMessage(null)}
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
                <p className="text-xs text-pierre-gray-400 mt-2 text-center">
                  Press Enter to send, Shift+Enter for new line
                </p>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
