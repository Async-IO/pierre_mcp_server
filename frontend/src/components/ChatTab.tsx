// ABOUTME: AI Chat tab component for users to interact with fitness AI assistant
// ABOUTME: Features conversation list, message history, and streaming responses
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, Button, Input } from './ui';
import { clsx } from 'clsx';
import { apiService } from '../services/api';

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
  limit: number;
  offset: number;
}

export default function ChatTab() {
  const queryClient = useQueryClient();
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [newMessage, setNewMessage] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const [showNewChat, setShowNewChat] = useState(false);
  const [newChatTitle, setNewChatTitle] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

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

  // Create conversation mutation
  const createConversation = useMutation({
    mutationFn: (title: string) => apiService.createConversation({ title }),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setSelectedConversation(data.id);
      setShowNewChat(false);
      setNewChatTitle('');
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

  const handleSendMessage = useCallback(async () => {
    if (!newMessage.trim() || !selectedConversation || isStreaming) return;

    const messageContent = newMessage.trim();
    setNewMessage('');
    setIsStreaming(true);
    setStreamingContent('');

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
        const error = await response.json();
        throw new Error(error.message || 'Failed to send message');
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
      // Show error in streaming content
      setStreamingContent(`Error: ${error instanceof Error ? error.message : 'Failed to send message'}`);
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
    if (!newChatTitle.trim()) return;
    createConversation.mutate(newChatTitle.trim());
  };

  const formatTime = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
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
    <div className="flex h-[calc(100vh-12rem)] gap-4">
      {/* Conversation List Sidebar */}
      <div className="w-72 flex-shrink-0 flex flex-col">
        <Card className="flex-1 flex flex-col overflow-hidden">
          <div className="p-4 border-b border-pierre-gray-100">
            <Button
              onClick={() => setShowNewChat(true)}
              className="w-full"
              variant="primary"
            >
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
              </svg>
              New Chat
            </Button>
          </div>

          {/* New Chat Dialog */}
          {showNewChat && (
            <div className="p-4 border-b border-pierre-gray-100 bg-pierre-gray-50">
              <Input
                value={newChatTitle}
                onChange={(e) => setNewChatTitle(e.target.value)}
                placeholder="Chat title..."
                className="mb-2"
                onKeyDown={(e) => e.key === 'Enter' && handleNewChat()}
              />
              <div className="flex gap-2">
                <Button size="sm" onClick={handleNewChat} disabled={!newChatTitle.trim()}>
                  Create
                </Button>
                <Button size="sm" variant="secondary" onClick={() => { setShowNewChat(false); setNewChatTitle(''); }}>
                  Cancel
                </Button>
              </div>
            </div>
          )}

          {/* Conversation List */}
          <div className="flex-1 overflow-y-auto">
            {conversationsLoading ? (
              <div className="p-4 text-center text-pierre-gray-500">Loading...</div>
            ) : !conversationsData?.conversations.length ? (
              <div className="p-4 text-center text-pierre-gray-500">
                <p className="text-sm">No conversations yet</p>
                <p className="text-xs mt-1">Start a new chat to begin</p>
              </div>
            ) : (
              <div className="divide-y divide-pierre-gray-100">
                {conversationsData.conversations.map((conv) => (
                  <button
                    key={conv.id}
                    onClick={() => setSelectedConversation(conv.id)}
                    className={clsx(
                      'w-full p-3 text-left hover:bg-pierre-gray-50 transition-colors group',
                      selectedConversation === conv.id && 'bg-pierre-violet/5 border-l-2 border-pierre-violet'
                    )}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex-1 min-w-0">
                        <p className="font-medium text-pierre-gray-900 truncate text-sm">
                          {conv.title}
                        </p>
                        <p className="text-xs text-pierre-gray-500 mt-0.5">
                          {conv.message_count} messages · {formatDate(conv.updated_at)}
                        </p>
                      </div>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          if (confirm('Delete this conversation?')) {
                            deleteConversation.mutate(conv.id);
                          }
                        }}
                        className="opacity-0 group-hover:opacity-100 text-pierre-gray-400 hover:text-red-500 transition-all"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                      </button>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </Card>
      </div>

      {/* Chat Area */}
      <div className="flex-1 flex flex-col">
        <Card className="flex-1 flex flex-col overflow-hidden">
          {!selectedConversation ? (
            // Empty state
            <div className="flex-1 flex items-center justify-center">
              <div className="text-center">
                <div className="w-16 h-16 mx-auto mb-4 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-full flex items-center justify-center">
                  <svg className="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-pierre-gray-900">Pierre AI Assistant</h3>
                <p className="text-sm text-pierre-gray-600 mt-1 max-w-sm">
                  Ask about your fitness data, get training insights, or explore your activities.
                </p>
              </div>
            </div>
          ) : (
            <>
              {/* Messages Area */}
              <div className="flex-1 overflow-y-auto p-4 space-y-4">
                {messagesLoading ? (
                  <div className="text-center text-pierre-gray-500">Loading messages...</div>
                ) : (
                  <>
                    {messagesData?.messages.map((msg) => (
                      <div
                        key={msg.id}
                        className={clsx(
                          'flex',
                          msg.role === 'user' ? 'justify-end' : 'justify-start'
                        )}
                      >
                        <div
                          className={clsx(
                            'max-w-[80%] rounded-2xl px-4 py-2',
                            msg.role === 'user'
                              ? 'bg-gradient-to-r from-pierre-violet to-pierre-cyan text-white'
                              : 'bg-pierre-gray-100 text-pierre-gray-900'
                          )}
                        >
                          <p className="whitespace-pre-wrap text-sm">{msg.content}</p>
                          <p className={clsx(
                            'text-xs mt-1',
                            msg.role === 'user' ? 'text-white/70' : 'text-pierre-gray-500'
                          )}>
                            {formatTime(msg.created_at)}
                          </p>
                        </div>
                      </div>
                    ))}

                    {/* Streaming response */}
                    {isStreaming && streamingContent && (
                      <div className="flex justify-start">
                        <div className="max-w-[80%] rounded-2xl px-4 py-2 bg-pierre-gray-100 text-pierre-gray-900">
                          <p className="whitespace-pre-wrap text-sm">{streamingContent}</p>
                          <div className="flex items-center gap-1 mt-1">
                            <div className="w-1.5 h-1.5 bg-pierre-violet rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                            <div className="w-1.5 h-1.5 bg-pierre-violet rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                            <div className="w-1.5 h-1.5 bg-pierre-violet rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                          </div>
                        </div>
                      </div>
                    )}

                    {/* Typing indicator when streaming but no content yet */}
                    {isStreaming && !streamingContent && (
                      <div className="flex justify-start">
                        <div className="rounded-2xl px-4 py-3 bg-pierre-gray-100">
                          <div className="flex items-center gap-1">
                            <div className="w-2 h-2 bg-pierre-violet rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                            <div className="w-2 h-2 bg-pierre-violet rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                            <div className="w-2 h-2 bg-pierre-violet rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                          </div>
                        </div>
                      </div>
                    )}
                  </>
                )}
                <div ref={messagesEndRef} />
              </div>

              {/* Input Area */}
              <div className="border-t border-pierre-gray-100 p-4">
                <div className="flex gap-2">
                  <textarea
                    ref={inputRef}
                    value={newMessage}
                    onChange={(e) => setNewMessage(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Ask about your fitness data..."
                    className="flex-1 resize-none rounded-xl border border-pierre-gray-200 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-pierre-violet/50 focus:border-pierre-violet text-sm"
                    rows={1}
                    disabled={isStreaming}
                  />
                  <Button
                    onClick={handleSendMessage}
                    disabled={!newMessage.trim() || isStreaming}
                    className="self-end"
                  >
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                    </svg>
                  </Button>
                </div>
                <p className="text-xs text-pierre-gray-500 mt-2">
                  Press Enter to send, Shift+Enter for new line
                </p>
              </div>
            </>
          )}
        </Card>
      </div>
    </div>
  );
}
