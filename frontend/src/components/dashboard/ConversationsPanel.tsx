// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Conversations panel for sidebar chat history display
// ABOUTME: Owns its own useQuery and mutation calls for conversation management

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { chatApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';
import type { Conversation } from '../chat/types';
import ConversationItem from '../chat/ConversationItem';
import { ConfirmDialog } from '../ui';

interface ConversationsPanelProps {
  selectedConversation: string | null;
  onSelectConversation: (id: string | null) => void;
}

export default function ConversationsPanel({
  selectedConversation,
  onSelectConversation,
}: ConversationsPanelProps) {
  const [editingConversationId, setEditingConversationId] = useState<string | null>(null);
  const [editedTitleValue, setEditedTitleValue] = useState('');
  const [deleteConfirmation, setDeleteConfirmation] = useState<{ id: string; title: string } | null>(null);
  const queryClient = useQueryClient();

  const { data: conversationsData, isLoading } = useQuery<{ conversations: Conversation[] }>({
    queryKey: QUERY_KEYS.chat.conversations(),
    queryFn: () => chatApi.getConversations(),
  });
  const conversations = conversationsData?.conversations ?? [];

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
        onSelectConversation(null);
      }
      setDeleteConfirmation(null);
    },
  });

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

  return (
    <>
      <div className="border-t border-white/10 pt-4">
        <h3 className="text-[11px] font-bold text-zinc-400 tracking-wider uppercase px-3 mb-2">
          Recent Chats
        </h3>
        <div className="space-y-0.5 max-h-64 overflow-y-auto">
          {isLoading ? (
            <div className="px-3 py-2 text-zinc-500 text-sm">Loading...</div>
          ) : conversations.length === 0 ? (
            <div className="px-3 py-2 text-zinc-500 text-sm">No conversations yet</div>
          ) : (
            conversations.slice(0, 10).map((conv) => (
              <ConversationItem
                key={conv.id}
                conversation={conv}
                isSelected={selectedConversation === conv.id}
                isEditing={editingConversationId === conv.id}
                editedTitleValue={editedTitleValue}
                onSelect={() => onSelectConversation(conv.id)}
                onStartRename={(e) => handleStartRename(e, conv)}
                onDelete={(e) => handleDeleteClick(e, conv)}
                onTitleChange={setEditedTitleValue}
                onSaveRename={handleSaveRename}
                onCancelRename={handleCancelRename}
              />
            ))
          )}
        </div>
      </div>

      <ConfirmDialog
        isOpen={!!deleteConfirmation}
        title="Delete Conversation"
        message={`Are you sure you want to delete "${deleteConfirmation?.title}"? This action cannot be undone.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
        onConfirm={handleConfirmDelete}
        onClose={handleCancelDelete}
        variant="danger"
      />
    </>
  );
}
