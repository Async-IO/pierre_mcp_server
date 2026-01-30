// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for managing chat conversations - CRUD operations and queries
// ABOUTME: Extracted from ChatTab to improve separation of concerns

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { chatApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { Conversation } from '@pierre/shared-types';

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

interface UseConversationsOptions {
  onConversationCreated?: (id: string) => void;
  onConversationDeleted?: () => void;
}

export function useConversations(options: UseConversationsOptions = {}) {
  const queryClient = useQueryClient();
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState<string | null>(null);
  const [editedTitleValue, setEditedTitleValue] = useState('');
  const [deleteConfirmation, setDeleteConfirmation] = useState<{ id: string; title: string | null } | null>(null);
  const [pendingSystemPrompt, setPendingSystemPrompt] = useState<string | null>(null);

  // Fetch conversations
  const {
    data: conversationsData,
    isLoading: conversationsLoading,
  } = useQuery<ConversationListResponse>({
    queryKey: QUERY_KEYS.chat.conversations(),
    queryFn: () => chatApi.getConversations(),
  });

  // Fetch messages for selected conversation
  const {
    data: messagesData,
    isLoading: messagesLoading,
  } = useQuery<{ messages: Message[] }>({
    queryKey: QUERY_KEYS.chat.messages(selectedConversation),
    queryFn: () => chatApi.getConversationMessages(selectedConversation!),
    enabled: !!selectedConversation,
  });

  // Create conversation mutation
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
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.chat.conversations() });
      setSelectedConversation(data.id);
      setPendingSystemPrompt(null);
      options.onConversationCreated?.(data.id);
    },
  });

  // Update conversation mutation for renaming
  const updateConversation = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) =>
      chatApi.updateConversation(id, { title }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.chat.conversations() });
      setEditingTitle(null);
      setEditedTitleValue('');
    },
  });

  // Delete conversation mutation
  const deleteConversation = useMutation({
    mutationFn: (id: string) => chatApi.deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.chat.conversations() });
      if (selectedConversation) {
        setSelectedConversation(null);
        options.onConversationDeleted?.();
      }
    },
  });

  // Handlers
  const handleSelectConversation = (id: string | null) => {
    setSelectedConversation(id);
  };

  const handleNewChat = () => {
    setSelectedConversation(null);
  };

  const handleStartRename = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setEditingTitle(conv.id);
    setEditedTitleValue(conv.title ?? '');
  };

  const handleSaveRename = (convId: string) => {
    const currentTitle = conversationsData?.conversations.find(c => c.id === convId)?.title;
    if (editedTitleValue.trim() && editedTitleValue.trim() !== currentTitle) {
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

  const handleCancelDelete = () => {
    setDeleteConfirmation(null);
  };

  return {
    // State
    selectedConversation,
    editingTitle,
    editedTitleValue,
    deleteConfirmation,
    pendingSystemPrompt,

    // Setters
    setSelectedConversation: handleSelectConversation,
    setEditedTitleValue,
    setPendingSystemPrompt,

    // Query data
    conversations: conversationsData?.conversations ?? [],
    conversationsTotal: conversationsData?.total ?? 0,
    conversationsLoading,
    messages: messagesData?.messages ?? [],
    messagesLoading,

    // Mutations
    createConversation,
    updateConversation,
    deleteConversation,

    // Handlers
    handleNewChat,
    handleStartRename,
    handleSaveRename,
    handleCancelRename,
    handleDeleteConversation,
    handleConfirmDelete,
    handleCancelDelete,
  };
}

export type { Conversation, Message, ConversationListResponse };
