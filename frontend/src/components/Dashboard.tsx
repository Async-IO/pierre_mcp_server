// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, lazy, Suspense, useEffect, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAuth } from '../hooks/useAuth';
import { dashboardApi, adminApi, a2aApi, chatApi } from '../services/api';
import type { DashboardOverview, RateLimitOverview, User, AdminToken } from '../types/api';
import type { Conversation } from './chat/types';
import type { AnalyticsData } from '../types/chart';
import { useWebSocketContext } from '../hooks/useWebSocketContext';
import { Card, ConfirmDialog } from './ui';
import { clsx } from 'clsx';
import ConversationItem from './chat/ConversationItem';

// Lazy load heavy components to reduce initial bundle size
const OverviewTab = lazy(() => import('./OverviewTab'));
const UsageAnalytics = lazy(() => import('./UsageAnalytics'));
const RequestMonitor = lazy(() => import('./RequestMonitor'));
const ToolUsageBreakdown = lazy(() => import('./ToolUsageBreakdown'));
const UnifiedConnections = lazy(() => import('./UnifiedConnections'));
const UserManagement = lazy(() => import('./UserManagement'));
const UserSettings = lazy(() => import('./UserSettings'));
const AdminSettings = lazy(() => import('./AdminSettings'));
const ApiKeyList = lazy(() => import('./ApiKeyList'));
const ApiKeyDetails = lazy(() => import('./ApiKeyDetails'));
const ChatTab = lazy(() => import('./ChatTab'));
const AdminConfiguration = lazy(() => import('./AdminConfiguration'));
const SystemCoachesTab = lazy(() => import('./SystemCoachesTab'));
const CoachStoreManagement = lazy(() => import('./CoachStoreManagement'));
const CoachLibraryTab = lazy(() => import('./CoachLibraryTab'));
const StoreScreen = lazy(() => import('./StoreScreen'));
const FriendsTab = lazy(() => import('./social/FriendsTab'));
const SocialFeedTab = lazy(() => import('./social/SocialFeedTab'));
const SocialSettingsTab = lazy(() => import('./social/SocialSettingsTab'));

// Tab definition type with optional badge for notification counts
interface TabDefinition {
  id: string;
  name: string;
  icon: React.ReactNode;
  badge?: number;
}

const PierreLogo = () => (
  <svg width="48" height="48" viewBox="0 0 120 120" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <defs>
      <linearGradient id="pg" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#8B5CF6"/><stop offset="100%" stopColor="#22D3EE"/></linearGradient>
      <linearGradient id="ag" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#4ADE80"/><stop offset="100%" stopColor="#22C55E"/></linearGradient>
      <linearGradient id="ng" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#F59E0B"/><stop offset="100%" stopColor="#D97706"/></linearGradient>
      <linearGradient id="rg" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#818CF8"/><stop offset="100%" stopColor="#6366F1"/></linearGradient>
    </defs>
    <g strokeWidth="2" opacity="0.5" strokeLinecap="round">
      <line x1="40" y1="30" x2="52" y2="42" stroke="url(#ag)"/><line x1="52" y1="42" x2="70" y2="35" stroke="url(#ag)"/>
      <line x1="52" y1="42" x2="48" y2="55" stroke="url(#pg)"/><line x1="48" y1="55" x2="75" y2="52" stroke="url(#ng)"/>
      <line x1="48" y1="55" x2="55" y2="72" stroke="url(#pg)"/><line x1="55" y1="72" x2="35" y2="85" stroke="url(#rg)"/><line x1="55" y1="72" x2="72" y2="82" stroke="url(#rg)"/>
    </g>
    <circle cx="40" cy="30" r="7" fill="url(#ag)"/><circle cx="52" cy="42" r="5" fill="url(#ag)"/><circle cx="70" cy="35" r="3.5" fill="url(#ag)"/>
    <circle cx="48" cy="55" r="6" fill="url(#pg)"/><circle cx="48" cy="55" r="3" fill="#fff" opacity="0.9"/>
    <circle cx="75" cy="52" r="4.5" fill="url(#ng)"/><circle cx="88" cy="60" r="3.5" fill="url(#ng)"/>
    <circle cx="55" cy="72" r="5" fill="url(#rg)"/><circle cx="35" cy="85" r="4" fill="url(#rg)"/><circle cx="72" cy="82" r="4" fill="url(#rg)"/>
  </svg>
);

export default function Dashboard() {
  const { user, logout } = useAuth();
  // Default tab depends on user role: admin sees 'overview', regular users see 'chat'
  const isAdminUser = user?.role === 'admin' || user?.role === 'super_admin';
  const isSuperAdmin = user?.role === 'super_admin';
  const [activeTab, setActiveTab] = useState(isAdminUser ? 'overview' : 'chat');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [selectedAdminToken, setSelectedAdminToken] = useState<AdminToken | null>(null);
  const [showUserMenu, setShowUserMenu] = useState(false);
  const { lastMessage } = useWebSocketContext();

  const { data: overview, isLoading: overviewLoading, refetch: refetchOverview } = useQuery<DashboardOverview>({
    queryKey: ['dashboard-overview'],
    queryFn: () => dashboardApi.getDashboardOverview(),
    enabled: isAdminUser,
  });

  const { data: rateLimits } = useQuery<RateLimitOverview[]>({
    queryKey: ['rate-limits'],
    queryFn: () => dashboardApi.getRateLimitOverview(),
    enabled: isAdminUser,
  });

  const { data: weeklyUsage } = useQuery<AnalyticsData>({
    queryKey: ['usage-analytics', 7],
    queryFn: () => dashboardApi.getUsageAnalytics(7),
    enabled: isAdminUser,
  });

  const { data: a2aOverview } = useQuery({
    queryKey: ['a2a-dashboard-overview'],
    queryFn: () => a2aApi.getA2ADashboardOverview(),
    enabled: isAdminUser,
  });

  // Pending users badge - only fetch for admin users
  const { data: pendingUsers = [] } = useQuery<User[]>({
    queryKey: ['pending-users'],
    queryFn: () => adminApi.getPendingUsers(),
    staleTime: 30_000,
    retry: false,
    enabled: isAdminUser,
  });

  // Coach store stats for pending review badge
  const { data: storeStats } = useQuery({
    queryKey: ['admin-store-stats'],
    queryFn: () => adminApi.getStoreStats(),
    staleTime: 30_000,
    retry: false,
    enabled: isAdminUser,
  });

  // Chat conversations - fetch for all users when Chat tab is active
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [editingConversationId, setEditingConversationId] = useState<string | null>(null);
  const [editedTitleValue, setEditedTitleValue] = useState('');
  const [deleteConfirmation, setDeleteConfirmation] = useState<{ id: string; title: string } | null>(null);
  const queryClient = useQueryClient();

  const { data: conversationsData, isLoading: conversationsLoading } = useQuery<{ conversations: Conversation[] }>({
    queryKey: ['chat-conversations'],
    queryFn: () => chatApi.getConversations(),
    enabled: activeTab === 'chat',
  });
  const conversations = conversationsData?.conversations ?? [];

  // Mutations for conversation management
  const updateConversationMutation = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) =>
      chatApi.updateConversation(id, { title }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      setEditingConversationId(null);
      setEditedTitleValue('');
    },
  });

  const deleteConversationMutation = useMutation({
    mutationFn: (id: string) => chatApi.deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chat-conversations'] });
      if (selectedConversation === deleteConfirmation?.id) {
        setSelectedConversation(null);
      }
      setDeleteConfirmation(null);
    },
  });

  // Conversation action handlers
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

  // Refresh data when WebSocket updates are received
  useEffect(() => {
    if (lastMessage && isAdminUser) {
      if (lastMessage.type === 'usage_update' || lastMessage.type === 'system_stats') {
        refetchOverview();
      }
    }
  }, [lastMessage, refetchOverview, isAdminUser]);

  // Close user menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (showUserMenu && !(e.target as Element).closest('.user-menu-container')) {
        setShowUserMenu(false);
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, [showUserMenu]);

  // Tab definitions for admin users
  const adminTabs: TabDefinition[] = useMemo(() => [
    { id: 'overview', name: 'Overview', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    ) },
    { id: 'connections', name: 'Connections', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
      </svg>
    ) },
    { id: 'analytics', name: 'Analytics', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    ) },
    { id: 'monitor', name: 'Monitor', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
      </svg>
    ) },
    { id: 'tools', name: 'Tools', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
    { id: 'users', name: 'Users', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
      </svg>
    ), badge: pendingUsers.length > 0 ? pendingUsers.length : undefined },
    { id: 'configuration', name: 'Configuration', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
      </svg>
    ) },
    { id: 'coaches', name: 'Coaches', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.121 17.804A13.937 13.937 0 0112 16c2.5 0 4.847.655 6.879 1.804M15 10a3 3 0 11-6 0 3 3 0 016 0zm6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    ) },
    { id: 'coach-store', name: 'Coach Store', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z" />
      </svg>
    ), badge: (storeStats?.pending_count ?? 0) > 0 ? storeStats?.pending_count : undefined },
    { id: 'social-friends', name: 'Friends', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
      </svg>
    ) },
    { id: 'insights', name: 'Insights', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
      </svg>
    ) },
    { id: 'social-settings', name: 'Social Settings', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
    { id: 'admin-settings', name: 'Settings', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
  ], [pendingUsers.length, storeStats?.pending_count]);

  // Super admin tabs extend admin tabs with admin token management
  const superAdminTabs: TabDefinition[] = useMemo(() => [
    ...adminTabs,
    { id: 'admin-tokens', name: 'Admin Tokens', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
      </svg>
    ) },
  ], [adminTabs]);

  // Regular user tabs - Chat, My Coaches, Discover, Insights, Friends, Settings
  const regularTabs: TabDefinition[] = useMemo(() => [
    { id: 'chat', name: 'Chat', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
      </svg>
    ) },
    { id: 'my-coaches', name: 'My Coaches', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
      </svg>
    ) },
    { id: 'discover', name: 'Discover', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
    ) },
    { id: 'insights', name: 'Insights', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
      </svg>
    ) },
    { id: 'social-friends', name: 'Friends', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
      </svg>
    ) },
    { id: 'settings', name: 'Settings', icon: (
      <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
  ], []);

  // For admin users, use sidebar tabs
  const tabs = isSuperAdmin ? superAdminTabs : (isAdminUser ? adminTabs : regularTabs);

  // Admin user view: Full sidebar with tabs - Dark Theme
  return (
    <div className="min-h-screen bg-pierre-dark flex">
      {/* Vertical Sidebar - Dark */}
      <aside
        className={clsx(
          'fixed left-0 top-0 h-screen bg-pierre-slate border-r border-white/10 flex flex-col z-40 transition-all duration-300 ease-in-out overflow-hidden',
          sidebarCollapsed ? 'w-[72px]' : 'w-[260px]'
        )}
      >
        {/* Sidebar accent bar */}
        <div className="absolute top-0 left-0 bottom-0 w-1 bg-gradient-to-b from-pierre-violet via-pierre-cyan to-pierre-activity"></div>

        {/* Logo Section */}
        <div className={clsx(
          'flex items-center border-b border-white/10 transition-all duration-300',
          sidebarCollapsed ? 'px-3 py-4 justify-center' : 'px-5 py-5 gap-3'
        )}>
          <PierreLogo />
          {!sidebarCollapsed && (
            <div className="flex flex-col">
              <span className="text-lg font-semibold bg-gradient-to-r from-pierre-violet to-pierre-cyan bg-clip-text text-transparent">
                Pierre
              </span>
              <span className="text-[10px] text-zinc-300 tracking-wide uppercase">
                Fitness Intelligence
              </span>
            </div>
          )}
        </div>

        {/* Navigation Items */}
        <nav className="flex-1 py-4 overflow-y-auto overflow-x-hidden">
          <ul className="space-y-1 px-3">
            {tabs.map((tab) => (
              <li key={tab.id}>
                <button
                  onClick={() => {
                    setActiveTab(tab.id);
                    // Reset conversation selection when clicking Chat tab to show coach selection
                    if (tab.id === 'chat') {
                      setSelectedConversation(null);
                    }
                  }}
                  className={clsx(
                    'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-200 group relative min-h-[44px]',
                    {
                      'bg-gradient-to-r from-pierre-violet/20 to-pierre-cyan/10 text-pierre-violet-light shadow-sm': activeTab === tab.id,
                      'text-zinc-400 hover:bg-white/5 hover:text-white': activeTab !== tab.id,
                    },
                    sidebarCollapsed && 'justify-center'
                  )}
                  title={sidebarCollapsed ? tab.name : undefined}
                >
                  {/* Active indicator */}
                  {activeTab === tab.id && (
                    <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-pierre-violet rounded-r-full" />
                  )}
                  <div className="relative flex-shrink-0">
                    {tab.icon}
                    {tab.badge && (
                      <span
                        data-testid="pending-users-badge"
                        className="absolute -top-1 -right-1 bg-pierre-red-500 text-white text-xs rounded-full h-4 w-4 flex items-center justify-center font-bold text-[10px]"
                      >
                        {tab.badge}
                      </span>
                    )}
                  </div>
                  {!sidebarCollapsed && <span>{tab.name}</span>}
                  {/* Tooltip for collapsed state */}
                  {sidebarCollapsed && (
                    <div className="absolute left-full ml-2 px-2 py-1 bg-white/10 backdrop-blur-sm text-white text-xs rounded opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap transition-opacity z-50">
                      {tab.name}
                    </div>
                  )}
                </button>
              </li>
            ))}
          </ul>

          {/* Recent Conversations - shown when Chat tab is active */}
          {activeTab === 'chat' && !sidebarCollapsed && (
            <div className="mt-4 px-3">
              <div className="border-t border-white/10 pt-4">
                <h3 className="text-[11px] font-bold text-zinc-400 tracking-wider uppercase px-3 mb-2">
                  Recent Chats
                </h3>
                <div className="space-y-0.5 max-h-64 overflow-y-auto">
                  {conversationsLoading ? (
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
                        onSelect={() => setSelectedConversation(conv.id)}
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
            </div>
          )}
        </nav>

        {/* User Profile Section - Bottom of sidebar */}
        <div className={clsx(
          'border-t border-white/10',
          sidebarCollapsed ? 'p-1.5' : 'px-2 py-1.5'
        )}>
          <div className={clsx(
            'flex items-center',
            sidebarCollapsed ? 'flex-col gap-1' : 'gap-2'
          )}>
            {/* User Avatar with online indicator */}
            <div className="relative flex-shrink-0">
              <div className="w-5 h-5 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-full flex items-center justify-center">
                <span className="text-[9px] font-bold text-white">
                  {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
                </span>
              </div>
              {/* Online status dot */}
              <div className="absolute -bottom-0.5 -right-0.5 w-2 h-2 bg-pierre-activity rounded-full border border-pierre-slate" />
            </div>

            {!sidebarCollapsed && (
              <div className="flex-1 min-w-0">
                <p className="text-[10px] font-medium text-white truncate leading-none">
                  {user?.display_name || user?.email}
                </p>
                <span className="text-[8px] text-zinc-300 uppercase">
                  {user?.role === 'super_admin' ? 'Super Admin' : user?.role === 'admin' ? 'Admin' : 'User'}
                </span>
              </div>
            )}

            {!sidebarCollapsed && (
              <button
                onClick={logout}
                className="text-zinc-500 hover:text-pierre-violet transition-colors flex-shrink-0 min-w-[44px] min-h-[44px] flex items-center justify-center"
                title="Sign out"
                aria-label="Sign out"
              >
                <svg className="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
                </svg>
              </button>
            )}

            {sidebarCollapsed && (
              <button
                onClick={logout}
                className="text-zinc-500 hover:text-pierre-violet transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center"
                title="Sign out"
                aria-label="Sign out"
              >
                <svg className="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
                </svg>
              </button>
            )}
          </div>
        </div>

        {/* Collapse Toggle Button */}
        <button
          onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
          className="absolute -right-5 top-20 w-11 h-11 bg-pierre-slate border border-white/20 rounded-full flex items-center justify-center shadow-sm hover:bg-white/10 hover:border-pierre-violet transition-all duration-200 z-50"
          title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <svg
            className={clsx(
              'w-4 h-4 text-zinc-400 transition-transform duration-300',
              sidebarCollapsed && 'rotate-180'
            )}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </aside>

      {/* Main Content Area */}
      <main
        className={clsx(
          'flex-1 h-screen flex flex-col transition-all duration-300 ease-in-out',
          sidebarCollapsed ? 'ml-[72px]' : 'ml-[260px]'
        )}
      >
        {/* Top Header Bar - Dark with glassmorphism */}
        <header className="bg-pierre-slate/80 backdrop-blur-lg shadow-sm border-b border-white/10 sticky top-0 z-30 flex-shrink-0">
          <div className="px-6 py-4 flex items-center justify-between">
            <div>
              <h1 className="text-xl font-medium text-white">
                {tabs.find(t => t.id === activeTab)?.name}
              </h1>
            </div>
          </div>
        </header>

        {/* Content Area - Full height for chat, padded for other tabs */}
        <div className={clsx(
          'flex-1 overflow-auto',
          activeTab === 'chat' ? '' : 'p-6'
        )}>

          {/* Content */}
        {activeTab === 'overview' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <OverviewTab
              overview={overview}
              overviewLoading={overviewLoading}
              rateLimits={rateLimits}
              weeklyUsage={weeklyUsage}
              a2aOverview={a2aOverview}
              pendingUsersCount={pendingUsers.length}
              onNavigate={setActiveTab}
            />
          </Suspense>
        )}

        {activeTab === 'connections' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <UnifiedConnections />
          </Suspense>
        )}
        {activeTab === 'analytics' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <UsageAnalytics />
          </Suspense>
        )}
        {activeTab === 'monitor' && (
          <div className="space-y-6">
            <Card variant="dark">
              <h2 className="text-xl font-semibold mb-4 text-white">Real-time Request Monitor</h2>
              <p className="text-zinc-400 mb-4">
                Monitor API requests in real-time across all your connections. See request status, response times, and error details as they happen.
              </p>
            </Card>
            <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
              <RequestMonitor showAllKeys={true} />
            </Suspense>
          </div>
        )}
        {activeTab === 'tools' && (
          <div className="space-y-6">
            <Card variant="dark">
              <h2 className="text-xl font-semibold mb-4 text-white">Tool Usage Analysis</h2>
              <p className="text-zinc-400 mb-4">
                Analyze which fitness tools are being used most frequently, their performance metrics, and success rates.
              </p>
            </Card>
            <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
              <ToolUsageBreakdown />
            </Suspense>
          </div>
        )}
        {activeTab === 'users' && (
          <div className="space-y-6">
            <Card variant="dark">
              <h2 className="text-xl font-semibold mb-4 text-white">User Management</h2>
              <p className="text-zinc-400 mb-4">
                Manage user registrations, approve pending users, and monitor user activity across the platform.
              </p>
            </Card>
            <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
              <UserManagement />
            </Suspense>
          </div>
        )}
        {activeTab === 'admin-settings' && (
          <div className="space-y-6">
            <Card variant="dark">
              <h2 className="text-xl font-semibold mb-4 text-white">System Settings</h2>
              <p className="text-zinc-400 mb-4">
                Configure system-wide settings for user registration, security, and platform behavior.
              </p>
            </Card>
            <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
              <AdminSettings />
            </Suspense>
          </div>
        )}
        {activeTab === 'configuration' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <AdminConfiguration />
          </Suspense>
        )}
        {activeTab === 'coaches' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <SystemCoachesTab />
          </Suspense>
        )}
        {activeTab === 'coach-store' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <CoachStoreManagement />
          </Suspense>
        )}
        {activeTab === 'social-friends' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <FriendsTab />
          </Suspense>
        )}
        {activeTab === 'insights' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <SocialFeedTab />
          </Suspense>
        )}
        {activeTab === 'social-settings' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <SocialSettingsTab />
          </Suspense>
        )}
        {activeTab === 'chat' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <ChatTab
              selectedConversation={selectedConversation}
              onSelectConversation={setSelectedConversation}
              onNavigateToInsights={() => setActiveTab('insights')}
            />
          </Suspense>
        )}
        {activeTab === 'my-coaches' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <CoachLibraryTab />
          </Suspense>
        )}
        {activeTab === 'discover' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <StoreScreen />
          </Suspense>
        )}
        {activeTab === 'settings' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <UserSettings />
          </Suspense>
        )}
        {activeTab === 'admin-tokens' && (
          <div className="space-y-6">
            {selectedAdminToken ? (
              <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
                <ApiKeyDetails
                  token={selectedAdminToken}
                  onBack={() => setSelectedAdminToken(null)}
                  onTokenUpdated={() => setSelectedAdminToken(null)}
                />
              </Suspense>
            ) : (
              <>
                <Card variant="dark">
                  <h2 className="text-xl font-semibold mb-4 text-white">API Key Management</h2>
                  <p className="text-zinc-400 mb-4">
                    Manage API keys for MCP clients and programmatic access. Only super admins can create, rotate, and revoke API keys.
                  </p>
                </Card>
                <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
                  <ApiKeyList onViewDetails={setSelectedAdminToken} />
                </Suspense>
              </>
            )}
          </div>
        )}
        </div>
      </main>

      {/* Delete Confirmation Dialog */}
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
    </div>
  );
}
