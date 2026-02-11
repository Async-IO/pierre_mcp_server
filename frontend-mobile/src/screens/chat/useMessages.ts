// ABOUTME: Hook for managing chat messages state and operations
// ABOUTME: Handles loading, sending, insights, feedback, and message rendering logic

import React, { useState, useCallback, useRef } from 'react';
import { FlatList } from 'react-native';
import { chatApi } from '../../services/api';
import { isInsightPrompt, detectInsightMessages, createInsightPrompt } from '@pierre/chat-utils';
import type { Message } from '../../types';

export interface MessagesState {
  messages: Message[];
  isSending: boolean;
  error: string | null;
  messageFeedback: Record<string, 'up' | 'down' | null>;
  insightMessages: Set<string>;
}

export interface MessagesActions {
  loadMessages: (conversationId: string) => Promise<void>;
  sendMessage: (
    conversationId: string,
    messageText: string,
    onConversationNeeded?: () => Promise<string | null>
  ) => Promise<void>;
  createInsight: (
    content: string,
    conversationId: string | undefined,
    onConversationNeeded?: () => Promise<string | null>
  ) => Promise<void>;
  retryMessage: (messageId: string, conversationId: string) => Promise<void>;
  handleThumbsUp: (messageId: string) => void;
  handleThumbsDown: (messageId: string) => void;
  clearMessages: () => void;
  scrollToBottom: () => void;
  flatListRef: React.RefObject<FlatList | null>;
}

export function useMessages(): MessagesState & MessagesActions {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [messageFeedback, setMessageFeedback] = useState<Record<string, 'up' | 'down' | null>>({});
  const [insightMessages, setInsightMessages] = useState<Set<string>>(new Set());
  const flatListRef = useRef<FlatList>(null);

  const scrollToBottom = useCallback(() => {
    if (flatListRef.current && messages.length > 0) {
      flatListRef.current.scrollToEnd({ animated: true });
    }
  }, [messages.length]);

  const loadMessages = useCallback(async (conversationId: string) => {
    try {
      setError(null);
      const response = await chatApi.getConversationMessages(conversationId);
      const allMessages = response.messages || [];

      const detectedInsights = detectInsightMessages(allMessages);
      if (detectedInsights.size > 0) {
        setInsightMessages(prev => {
          const merged = new Set(prev);
          detectedInsights.forEach(id => merged.add(id));
          return merged;
        });
      }

      const filteredMessages = allMessages.filter(
        (msg: Message) => !(msg.role === 'user' && isInsightPrompt(msg.content))
      );
      setMessages(filteredMessages);
      setTimeout(() => scrollToBottom(), 100);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load messages';
      setError(errorMessage);
      console.error('Failed to load messages:', err);
    }
  }, [scrollToBottom]);

  const sendMessage = useCallback(async (
    conversationId: string,
    messageText: string,
  ) => {
    if (!messageText.trim() || isSending) return;

    setIsSending(true);
    setError(null);

    const userMessage: Message = {
      id: `temp-${Date.now()}`,
      role: 'user',
      content: messageText,
      created_at: new Date().toISOString(),
    };
    setMessages(prev => [...prev, userMessage]);
    setTimeout(() => scrollToBottom(), 50);

    try {
      const response = await chatApi.sendMessage(conversationId, messageText);
      setMessages(prev => {
        const filtered = prev.filter(m => m.id !== userMessage.id);
        const newMessages: Message[] = [];
        if (response.user_message?.id) {
          newMessages.push(response.user_message);
        }
        if (response.assistant_message?.id) {
          newMessages.push({
            ...response.assistant_message,
            model: response.model,
            execution_time_ms: response.execution_time_ms,
          });
        }
        return [...filtered, ...newMessages];
      });
      setTimeout(() => scrollToBottom(), 50);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to send message';
      setError(errorMsg);
      const errorResponse: Message = {
        id: `error-${Date.now()}`,
        role: 'assistant',
        content: `⚠️ ${errorMsg}\n\nPlease try again.`,
        created_at: new Date().toISOString(),
        isError: true,
      };
      setMessages(prev => {
        const updated = prev.map(m =>
          m.id === userMessage.id ? { ...m, id: `user-${Date.now()}` } : m
        );
        return [...updated, errorResponse];
      });
      setTimeout(() => scrollToBottom(), 50);
    } finally {
      setIsSending(false);
    }
  }, [isSending, scrollToBottom]);

  const createInsight = useCallback(async (
    content: string,
    conversationId: string | undefined,
    onConversationNeeded?: () => Promise<string | null>
  ) => {
    if (isSending) return;

    let resolvedConversationId = conversationId;
    if (!resolvedConversationId && onConversationNeeded) {
      resolvedConversationId = (await onConversationNeeded()) ?? undefined;
      if (!resolvedConversationId) return;
    }
    if (!resolvedConversationId) return;

    setIsSending(true);
    setError(null);
    const insightPrompt = createInsightPrompt(content);
    setTimeout(() => scrollToBottom(), 50);

    try {
      const response = await chatApi.sendMessage(resolvedConversationId, insightPrompt);

      if (response.assistant_message?.id) {
        setInsightMessages(prev => {
          const updated = new Set(prev);
          updated.add(response.assistant_message.id);
          return updated;
        });

        setMessages(prev => [...prev, {
          ...response.assistant_message,
          model: response.model,
          execution_time_ms: response.execution_time_ms,
        }]);
      }
      setTimeout(() => scrollToBottom(), 50);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to generate insight';
      setError(errorMsg);
      console.error('Failed to create insight:', err);
    } finally {
      setIsSending(false);
    }
  }, [isSending, scrollToBottom]);

  const retryMessage = useCallback(async (messageId: string, conversationId: string) => {
    const messageIndex = messages.findIndex(m => m.id === messageId);
    if (messageIndex <= 0) return;

    const userMessage = messages[messageIndex - 1];
    if (userMessage.role !== 'user') return;

    setMessages(prev => prev.filter(m => m.id !== messageId));
    setIsSending(true);
    setError(null);

    try {
      const response = await chatApi.sendMessage(conversationId, userMessage.content);

      setMessages(prev => {
        if (response.assistant_message?.id) {
          return [...prev, {
            ...response.assistant_message,
            model: response.model,
            execution_time_ms: response.execution_time_ms,
          }];
        }
        return prev;
      });
      setTimeout(() => scrollToBottom(), 50);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to get response';
      setError(errorMsg);
      const errorMessage: Message = {
        id: `error-${Date.now()}`,
        role: 'assistant',
        content: `⚠️ ${errorMsg}\n\nPlease try again.`,
        created_at: new Date().toISOString(),
        isError: true,
      };
      setMessages(prev => [...prev, errorMessage]);
      setTimeout(() => scrollToBottom(), 50);
    } finally {
      setIsSending(false);
    }
  }, [messages, scrollToBottom]);

  const handleThumbsUp = useCallback((messageId: string) => {
    setMessageFeedback(prev => ({
      ...prev,
      [messageId]: prev[messageId] === 'up' ? null : 'up',
    }));
  }, []);

  const handleThumbsDown = useCallback((messageId: string) => {
    setMessageFeedback(prev => ({
      ...prev,
      [messageId]: prev[messageId] === 'down' ? null : 'down',
    }));
  }, []);

  const clearMessages = useCallback(() => {
    setMessages([]);
    setError(null);
  }, []);

  return {
    messages,
    isSending,
    error,
    messageFeedback,
    insightMessages,
    loadMessages,
    sendMessage,
    createInsight,
    retryMessage,
    handleThumbsUp,
    handleThumbsDown,
    clearMessages,
    scrollToBottom,
    flatListRef,
  };
}
