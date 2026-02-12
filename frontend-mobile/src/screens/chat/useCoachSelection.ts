// ABOUTME: Hook for managing coach selection and starting coach conversations
// ABOUTME: Handles coach loading, selection logic, and auto-sending initial messages

import React, { useState, useCallback } from 'react';
import { Alert } from 'react-native';
import { chatApi, coachesApi } from '../../services/api';
import type { Coach, Message, Conversation } from '../../types';

export interface CoachSelectionState {
  coaches: Coach[];
  pendingCoachAction: { coach: Coach } | null;
  error: string | null;
}

export interface CoachSelectionActions {
  loadCoaches: () => Promise<void>;
  handleCoachSelect: (
    coach: Coach,
    options: {
      isSending: boolean;
      hasConnectedProvider: () => boolean;
      selectedProvider: string | null;
      connectedProviders: { provider: string; connected: boolean }[];
      setSelectedProvider: (provider: string | null) => void;
      setProviderModalVisible: (visible: boolean) => void;
      startCoachConversation: (coach: Coach) => Promise<void>;
    }
  ) => Promise<void>;
  startCoachConversation: (
    coach: Coach,
    options: {
      createConversation: (params: { title: string; system_prompt?: string }) => Promise<Conversation | null>;
      setMessages: React.Dispatch<React.SetStateAction<Message[]>>;
      setIsSending: (sending: boolean) => void;
      scrollToBottom: () => void;
    }
  ) => Promise<void>;
  setPendingCoachAction: (action: { coach: Coach } | null) => void;
  clearPendingCoachAction: () => void;
}

export function useCoachSelection(): CoachSelectionState & CoachSelectionActions {
  const [coaches, setCoaches] = useState<Coach[]>([]);
  const [pendingCoachAction, setPendingCoachAction] = useState<{ coach: Coach } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadCoaches = useCallback(async () => {
    try {
      setError(null);
      const response = await coachesApi.list();
      const sorted = [...response.coaches].sort((a, b) => {
        if (a.is_favorite !== b.is_favorite) {
          return a.is_favorite ? -1 : 1;
        }
        return b.use_count - a.use_count;
      });
      setCoaches(sorted);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load coaches';
      setError(errorMessage);
      console.error('Failed to load coaches:', err);
    }
  }, []);

  const handleCoachSelect = useCallback(async (
    coach: Coach,
    options: {
      isSending: boolean;
      hasConnectedProvider: () => boolean;
      selectedProvider: string | null;
      connectedProviders: { provider: string; connected: boolean }[];
      setSelectedProvider: (provider: string | null) => void;
      setProviderModalVisible: (visible: boolean) => void;
      startCoachConversation: (coach: Coach) => Promise<void>;
    }
  ) => {
    if (options.isSending) return;

    // Check if we have a cached provider that is still connected
    if (options.selectedProvider) {
      const cachedProvider = options.connectedProviders.find(
        p => p.provider === options.selectedProvider && p.connected
      );
      if (cachedProvider) {
        await options.startCoachConversation(coach);
        return;
      }
      options.setSelectedProvider(null);
    }

    // Check if any provider is connected
    if (options.hasConnectedProvider()) {
      const firstConnected = options.connectedProviders.find(p => p.connected);
      if (firstConnected) {
        options.setSelectedProvider(firstConnected.provider);
      }
      await options.startCoachConversation(coach);
      return;
    }

    // No providers connected - show modal
    setPendingCoachAction({ coach });
    options.setProviderModalVisible(true);
  }, []);

  const startCoachConversation = useCallback(async (
    coach: Coach,
    options: {
      createConversation: (params: { title: string; system_prompt?: string }) => Promise<Conversation | null>;
      setMessages: React.Dispatch<React.SetStateAction<Message[]>>;
      setIsSending: (sending: boolean) => void;
      scrollToBottom: () => void;
    }
  ) => {
    try {
      options.setIsSending(true);
      setError(null);

      // Record usage (fire-and-forget)
      coachesApi.recordUsage(coach.id);

      const conversation = await options.createConversation({
        title: `Chat with ${coach.title}`,
        system_prompt: coach.system_prompt,
      });

      if (!conversation) {
        throw new Error('Failed to create conversation');
      }

      const initialMessage = coach.description || `Let's get started with ${coach.title}!`;

      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        role: 'user',
        content: initialMessage,
        created_at: new Date().toISOString(),
      };
      options.setMessages([userMessage]);

      const response = await chatApi.sendMessage(conversation.id, initialMessage);

      options.setMessages(prev => {
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
      options.scrollToBottom();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to start coach conversation';
      setError(errorMessage);
      console.error('Failed to start coach conversation:', err);
      Alert.alert('Error', 'Failed to start conversation with coach');
    } finally {
      options.setIsSending(false);
    }
  }, []);

  const clearPendingCoachAction = useCallback(() => {
    setPendingCoachAction(null);
  }, []);

  return {
    coaches,
    pendingCoachAction,
    error,
    loadCoaches,
    handleCoachSelect,
    startCoachConversation,
    setPendingCoachAction,
    clearPendingCoachAction,
  };
}
