// ABOUTME: Hook for managing chat conversations CRUD operations and state
// ABOUTME: Handles loading, creating, updating, deleting, and selecting conversations

import React, { useState, useCallback, useRef } from 'react';
import { Alert } from 'react-native';
import { chatApi } from '../../services/api';
import type { Conversation } from '../../types';

export interface ConversationsState {
  conversations: Conversation[];
  currentConversation: Conversation | null;
  isLoading: boolean;
  error: string | null;
}

export interface ConversationsActions {
  loadConversations: () => Promise<void>;
  setCurrentConversation: (conversation: Conversation | null) => void;
  createConversation: (params: { title: string; system_prompt?: string }) => Promise<Conversation | null>;
  deleteConversation: (conversationId: string) => Promise<void>;
  renameConversation: (conversationId: string, newTitle: string) => Promise<void>;
  handleNewChat: () => void;
  updateConversationInList: (conversation: Conversation) => void;
  addConversationToTop: (conversation: Conversation) => void;
  justCreatedConversationRef: React.MutableRefObject<string | null>;
}

export function useConversations(): ConversationsState & ConversationsActions {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversation, setCurrentConversation] = useState<Conversation | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const justCreatedConversationRef = useRef<string | null>(null);

  const loadConversations = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await chatApi.getConversations();
      const seen = new Set<string>();
      const deduplicated = (response.conversations || []).filter((conv: { id: string }) => {
        if (seen.has(conv.id)) return false;
        seen.add(conv.id);
        return true;
      });
      const sorted = deduplicated.sort((a: { updated_at?: string }, b: { updated_at?: string }) => {
        const dateA = a.updated_at ? new Date(a.updated_at).getTime() : 0;
        const dateB = b.updated_at ? new Date(b.updated_at).getTime() : 0;
        return dateB - dateA;
      });
      setConversations(sorted);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load conversations';
      setError(errorMessage);
      console.error('Failed to load conversations:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const createConversation = useCallback(async (params: { title: string; system_prompt?: string }): Promise<Conversation | null> => {
    try {
      setError(null);
      const conversation = await chatApi.createConversation(params);
      if (!conversation || !conversation.id) {
        throw new Error('Invalid conversation response');
      }
      setConversations(prev => [conversation, ...prev]);
      justCreatedConversationRef.current = conversation.id;
      setCurrentConversation(conversation);
      return conversation;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to create conversation';
      setError(errorMessage);
      console.error('Failed to create conversation:', err);
      Alert.alert('Error', 'Failed to create conversation');
      return null;
    }
  }, []);

  const deleteConversation = useCallback(async (conversationId: string) => {
    try {
      setError(null);
      await chatApi.deleteConversation(conversationId);
      setConversations(prev => prev.filter(c => c.id !== conversationId));
      if (currentConversation?.id === conversationId) {
        setCurrentConversation(null);
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to delete conversation';
      setError(errorMessage);
      Alert.alert('Error', 'Failed to delete conversation');
    }
  }, [currentConversation?.id]);

  const renameConversation = useCallback(async (conversationId: string, newTitle: string) => {
    try {
      setError(null);
      const updated = await chatApi.updateConversation(conversationId, { title: newTitle });
      setConversations(prev => {
        const updatedConv = prev.find(c => c.id === conversationId);
        if (!updatedConv) return prev;
        const others = prev.filter(c => c.id !== conversationId);
        return [
          { ...updatedConv, title: updated.title, updated_at: updated.updated_at },
          ...others,
        ];
      });
      setCurrentConversation(prev => {
        if (prev?.id === conversationId) {
          return { ...prev, title: updated.title, updated_at: updated.updated_at };
        }
        return prev;
      });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to rename conversation';
      setError(errorMessage);
      console.error('Failed to rename conversation:', err);
      Alert.alert('Error', 'Failed to rename conversation');
    }
  }, []);

  const handleNewChat = useCallback(() => {
    setCurrentConversation(null);
  }, []);

  const updateConversationInList = useCallback((conversation: Conversation) => {
    setConversations(prev => {
      const others = prev.filter(c => c.id !== conversation.id);
      return [conversation, ...others];
    });
  }, []);

  const addConversationToTop = useCallback((conversation: Conversation) => {
    setConversations(prev => [conversation, ...prev]);
    justCreatedConversationRef.current = conversation.id;
    setCurrentConversation(conversation);
  }, []);

  return {
    conversations,
    currentConversation,
    isLoading,
    error,
    loadConversations,
    setCurrentConversation,
    createConversation,
    deleteConversation,
    renameConversation,
    handleNewChat,
    updateConversationInList,
    addConversationToTop,
    justCreatedConversationRef,
  };
}
