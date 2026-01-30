// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Chat sidebar component with navigation and conversation list
// ABOUTME: Includes buttons for New Chat, My Coaches, Discover, and user profile

import { MessageCircle, Users, Compass, Plus, Settings } from 'lucide-react';
import { clsx } from 'clsx';
import ConversationItem from './ConversationItem';
import type { Conversation } from './types';

interface ChatSidebarProps {
  conversations: Conversation[];
  conversationsLoading: boolean;
  selectedConversation: string | null;
  showMyCoachesPanel: boolean;
  showStorePanel: boolean;
  editingTitle: string | null;
  editedTitleValue: string;
  user: { display_name?: string; email?: string } | null;
  onNewChat: () => void;
  onSelectConversation: (id: string) => void;
  onShowMyCoaches: () => void;
  onShowStore: () => void;
  onOpenSettings?: () => void;
  onStartRename: (e: React.MouseEvent, conv: Conversation) => void;
  onSaveRename: (id: string) => void;
  onCancelRename: () => void;
  onTitleChange: (value: string) => void;
  onDeleteConversation: (e: React.MouseEvent, conv: Conversation) => void;
  isCreatingConversation: boolean;
}

export default function ChatSidebar({
  conversations,
  conversationsLoading,
  selectedConversation,
  showMyCoachesPanel,
  showStorePanel,
  editingTitle,
  editedTitleValue,
  user,
  onNewChat,
  onSelectConversation,
  onShowMyCoaches,
  onShowStore,
  onOpenSettings,
  onStartRename,
  onSaveRename,
  onCancelRename,
  onTitleChange,
  onDeleteConversation,
  isCreatingConversation,
}: ChatSidebarProps) {
  return (
    <div className="flex flex-col h-full bg-pierre-dark relative overflow-hidden">
      {/* Header - Logo + New Chat button */}
      <div className="p-4 flex items-center justify-between flex-shrink-0">
        {/* Logo */}
        <div className="flex items-center gap-2">
          <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8" />
          <span className="text-lg font-semibold text-white">Pierre</span>
        </div>
        {/* New Chat Button */}
        <button
          onClick={onNewChat}
          disabled={isCreatingConversation}
          className="w-8 h-8 flex items-center justify-center rounded-lg bg-pierre-violet text-white hover:bg-pierre-violet/80 transition-colors disabled:opacity-50 shadow-glow-sm"
          title="New chat"
          aria-label="New chat"
        >
          <Plus className="w-4 h-4" aria-hidden="true" />
        </button>
      </div>

      {/* Navigation Section */}
      <nav className="px-3 space-y-1">
        {/* Chat Button */}
        <button
          onClick={() => {
            onSelectConversation('');
          }}
          title="Chat"
          aria-label="Chat"
          className={clsx(
            'group flex items-center gap-3 px-3 py-2.5 rounded-full transition-all duration-200 w-full',
            !selectedConversation && !showMyCoachesPanel && !showStorePanel
              ? 'bg-pierre-violet/10 border border-pierre-violet/20 text-pierre-violet shadow-[inset_0_0_12px_rgba(124,59,237,0.1)]'
              : 'text-zinc-400 hover:text-white hover:bg-white/5'
          )}
        >
          <MessageCircle className="w-5 h-5" aria-hidden="true" />
          <span className="text-sm font-medium">Chat</span>
          {!selectedConversation && !showMyCoachesPanel && !showStorePanel && (
            <div className="ml-auto w-1.5 h-1.5 rounded-full bg-pierre-violet shadow-glow" />
          )}
        </button>

        {/* My Coaches Button */}
        <button
          onClick={onShowMyCoaches}
          title="My Coaches"
          aria-label="My Coaches"
          className={clsx(
            'group flex items-center gap-3 px-3 py-2.5 rounded-full transition-all duration-200 w-full',
            showMyCoachesPanel
              ? 'bg-pierre-violet/10 border border-pierre-violet/20 text-pierre-violet shadow-[inset_0_0_12px_rgba(124,59,237,0.1)]'
              : 'text-zinc-400 hover:text-white hover:bg-white/5'
          )}
        >
          <Users className="w-5 h-5" aria-hidden="true" />
          <span className="text-sm font-medium">My Coaches</span>
          {showMyCoachesPanel && (
            <div className="ml-auto w-1.5 h-1.5 rounded-full bg-pierre-violet shadow-glow" />
          )}
        </button>

        {/* Discover Coaches Button */}
        <button
          onClick={onShowStore}
          title="Discover Coaches"
          aria-label="Discover Coaches"
          className={clsx(
            'group flex items-center gap-3 px-3 py-2.5 rounded-full transition-all duration-200 w-full',
            showStorePanel
              ? 'bg-pierre-violet/10 border border-pierre-violet/20 text-pierre-violet shadow-[inset_0_0_12px_rgba(124,59,237,0.1)]'
              : 'text-zinc-400 hover:text-white hover:bg-white/5'
          )}
        >
          <Compass className="w-5 h-5" aria-hidden="true" />
          <span className="text-sm font-medium">Discover</span>
          {showStorePanel && (
            <div className="ml-auto w-1.5 h-1.5 rounded-full bg-pierre-violet shadow-glow" />
          )}
        </button>
      </nav>

      {/* Section Divider */}
      <div className="px-6 py-4">
        <div className="h-px w-full bg-gradient-to-r from-transparent via-white/10 to-transparent" />
      </div>

      {/* Recent Conversations Header */}
      <div className="px-6 pb-2">
        <h3 className="text-[11px] font-bold text-zinc-300 tracking-[0.15em] uppercase">Recent Conversations</h3>
      </div>

      {/* Conversation List - Scrollable */}
      <div className="flex-1 overflow-y-auto pb-44 px-3 space-y-0.5 sidebar-scroll">
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
              onSelect={() => onSelectConversation(conv.id)}
              onStartRename={(e) => onStartRename(e, conv)}
              onDelete={(e) => onDeleteConversation(e, conv)}
              onTitleChange={onTitleChange}
              onSaveRename={() => onSaveRename(conv.id)}
              onCancelRename={onCancelRename}
            />
          ))
        )}
      </div>

      {/* Footer Area - Gradient fade + user profile */}
      <div className="absolute bottom-0 left-0 right-0 p-4 bg-gradient-to-t from-pierre-dark via-pierre-dark/95 to-transparent">
        {/* User Profile Pill */}
        <button
          onClick={onOpenSettings}
          className="w-full flex items-center gap-3 p-1.5 pr-3 bg-white/5 border border-white/5 rounded-full hover:bg-white/10 transition-colors cursor-pointer group"
          title="Open settings"
        >
          {/* Avatar */}
          <div className="relative w-9 h-9 rounded-full overflow-hidden border border-white/10 flex-shrink-0 bg-gradient-to-br from-pierre-violet to-pierre-cyan flex items-center justify-center">
            <span className="text-sm font-bold text-white">
              {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
            </span>
          </div>
          {/* Text */}
          <div className="flex flex-col flex-1 min-w-0">
            <p className="text-sm font-medium text-white truncate group-hover:text-pierre-violet transition-colors">
              {user?.display_name || 'User'}
            </p>
            <p className="text-[11px] text-zinc-400 truncate">
              {user?.email || 'Settings'}
            </p>
          </div>
          {/* Settings Icon */}
          <Settings className="w-5 h-5 text-zinc-400 group-hover:text-white transition-all group-hover:rotate-90 duration-500" aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
