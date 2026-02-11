// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for managing chat conversations state and mutations
// ABOUTME: Owns chat-conversations query and update/delete mutations

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { chatApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { Conversation } from '../chat/types';

export function useConversationsData(enabled: boolean) {
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [editingConversationId, setEditingConversationId] = useState<string | null>(null);
  const [editedTitleValue, setEditedTitleValue] = useState('');
  const [deleteConfirmation, setDeleteConfirmation] = useState<{ id: string; title: string } | null>(null);
  const queryClient = useQueryClient();

  const { data: conversationsData, isLoading } = useQuery<{ conversations: Conversation[] }>({
    queryKey: QUERY_KEYS.chat.conversations(),
    queryFn: () => chatApi.getConversations(),
    enabled,
  });
  const conversations = conversationsData?.conversations ?? [];

  // Mutations for conversation management
  const updateConversationMutation = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) =>
      chatApi.updateConversation(id, { title }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.chat.conversations() });
      setEditingConversationId(null);
      setEditedTitleValue('');
    },
  });

  const deleteConversationMutation = useMutation({
    mutationFn: (id: string) => chatApi.deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.chat.conversations() });
      if (selectedConversation === deleteConfirmation?.id) {
        setSelectedConversation(null);
      }
      setDeleteConfirmation(null);
    },
  });

  // Action handlers
  const handleStartRename = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setEditingConversationId(conv.id);
    setEditedTitleValue(conv.title || 'Untitled Chat');
  };

  const handleSaveRename = () => {
    if (editingConversationId && editedTitleValue.trim()) {
      updateConversationMutation.mutate({ id: editingConversationId, title: editedTitleValue.trim() });
    } else {
      setEditingConversationId(null);
      setEditedTitleValue('');
    }
  };

  const handleCancelRename = () => {
    setEditingConversationId(null);
    setEditedTitleValue('');
  };

  const handleDeleteClick = (e: React.MouseEvent, conv: Conversation) => {
    e.stopPropagation();
    setDeleteConfirmation({ id: conv.id, title: conv.title || 'Untitled Chat' });
  };

  const handleConfirmDelete = () => {
    if (deleteConfirmation) {
      deleteConversationMutation.mutate(deleteConfirmation.id);
    }
  };

  const handleCancelDelete = () => {
    setDeleteConfirmation(null);
  };

  return {
    conversations,
    isLoading,
    selectedConversation,
    setSelectedConversation,
    editingConversationId,
    editedTitleValue,
    setEditedTitleValue,
    deleteConfirmation,
    handleStartRename,
    handleSaveRename,
    handleCancelRename,
    handleDeleteClick,
    handleConfirmDelete,
    handleCancelDelete,
  };
}
