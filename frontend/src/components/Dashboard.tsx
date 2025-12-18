// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, lazy, Suspense, useEffect, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import type { DashboardOverview, RateLimitOverview, User, AdminToken } from '../types/api';
import type { AnalyticsData } from '../types/chart';
import { useWebSocketContext } from '../hooks/useWebSocketContext';
import { Card } from './ui';
import { clsx } from 'clsx';

// Lazy load heavy components to reduce initial bundle size
const OverviewTab = lazy(() => import('./OverviewTab'));
const UsageAnalytics = lazy(() => import('./UsageAnalytics'));
const RequestMonitor = lazy(() => import('./RequestMonitor'));
const ToolUsageBreakdown = lazy(() => import('./ToolUsageBreakdown'));
const UnifiedConnections = lazy(() => import('./UnifiedConnections'));
const UserManagement = lazy(() => import('./UserManagement'));
const UserHome = lazy(() => import('./UserHome'));
const UserSettings = lazy(() => import('./UserSettings'));
const AdminSettings = lazy(() => import('./AdminSettings'));
const ApiKeyList = lazy(() => import('./ApiKeyList'));
const ApiKeyDetails = lazy(() => import('./ApiKeyDetails'));
const MCPTokensTab = lazy(() => import('./MCPTokensTab'));
const ChatTab = lazy(() => import('./ChatTab'));
const AdminConfiguration = lazy(() => import('./AdminConfiguration'));

// Tab definition type with optional badge for notification counts
interface TabDefinition {
  id: string;
  name: string;
  icon: React.ReactNode;
  badge?: number;
}

const PierreLogo = () => (
  <svg width="48" height="48" viewBox="0 0 120 120" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <linearGradient id="pg" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#7C3AED"/><stop offset="100%" stopColor="#06B6D4"/></linearGradient>
      <linearGradient id="ag" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#10B981"/><stop offset="100%" stopColor="#059669"/></linearGradient>
      <linearGradient id="ng" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#F59E0B"/><stop offset="100%" stopColor="#D97706"/></linearGradient>
      <linearGradient id="rg" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#6366F1"/><stop offset="100%" stopColor="#4F46E5"/></linearGradient>
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
  // Default tab depends on user role: admin sees 'overview', regular users see 'home'
  const isAdminUser = user?.role === 'admin' || user?.role === 'super_admin';
  const isSuperAdmin = user?.role === 'super_admin';
  const [activeTab, setActiveTab] = useState(isAdminUser ? 'overview' : 'home');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [selectedAdminToken, setSelectedAdminToken] = useState<AdminToken | null>(null);
  const { lastMessage } = useWebSocketContext();

  const { data: overview, isLoading: overviewLoading, refetch: refetchOverview } = useQuery<DashboardOverview>({
    queryKey: ['dashboard-overview'],
    queryFn: () => apiService.getDashboardOverview(),
  });

  const { data: rateLimits } = useQuery<RateLimitOverview[]>({
    queryKey: ['rate-limits'],
    queryFn: () => apiService.getRateLimitOverview(),
  });

  const { data: weeklyUsage } = useQuery<AnalyticsData>({
    queryKey: ['usage-analytics', 7],
    queryFn: () => apiService.getUsageAnalytics(7),
  });

  const { data: a2aOverview } = useQuery({
    queryKey: ['a2a-dashboard-overview'],
    queryFn: () => apiService.getA2ADashboardOverview(),
  });

  // Pending users badge - only fetch for admin users
  const { data: pendingUsers = [] } = useQuery<User[]>({
    queryKey: ['pending-users'],
    queryFn: () => apiService.getPendingUsers(),
    staleTime: 30_000,
    retry: false,
    enabled: isAdminUser,
  });

  // Refresh data when WebSocket updates are received
  useEffect(() => {
    if (lastMessage) {
      if (lastMessage.type === 'usage_update' || lastMessage.type === 'system_stats') {
        refetchOverview();
      }
    }
  }, [lastMessage, refetchOverview]);

  // Tab definitions
  const adminTabs: TabDefinition[] = useMemo(() => [
    { id: 'overview', name: 'Overview', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    ) },
    { id: 'connections', name: 'Connections', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
      </svg>
    ) },
    { id: 'analytics', name: 'Analytics', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    ) },
    { id: 'monitor', name: 'Monitor', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
      </svg>
    ) },
    { id: 'tools', name: 'Tools', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
    { id: 'users', name: 'Users', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
      </svg>
    ), badge: pendingUsers.length > 0 ? pendingUsers.length : undefined },
    { id: 'configuration', name: 'Configuration', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
      </svg>
    ) },
    { id: 'admin-settings', name: 'Settings', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
  ], [pendingUsers.length]);

  // Super admin tabs extend admin tabs with admin token management
  const superAdminTabs: TabDefinition[] = useMemo(() => [
    ...adminTabs,
    { id: 'admin-tokens', name: 'Admin Tokens', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
      </svg>
    ) },
  ], [adminTabs]);

  const userTabs: TabDefinition[] = useMemo(() => [
    { id: 'home', name: 'Home', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
      </svg>
    ) },
    { id: 'chat', name: 'Chat with Pierre', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
      </svg>
    ) },
    { id: 'connections', name: 'Connections', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
      </svg>
    ) },
    { id: 'mcp-tokens', name: 'Tokens', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
      </svg>
    ) },
    { id: 'settings', name: 'Settings', icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ) },
  ], []);

  // Select tabs based on user role: super_admin gets extra tabs, admin gets admin tabs, users get user tabs
  const tabs = isSuperAdmin ? superAdminTabs : isAdminUser ? adminTabs : userTabs;

  return (
    <div className="min-h-screen bg-pierre-gray-50 flex">
      {/* Vertical Sidebar */}
      <aside
        className={clsx(
          'fixed left-0 top-0 h-screen bg-white border-r border-pierre-gray-200 flex flex-col z-40 transition-all duration-300 ease-in-out overflow-hidden',
          sidebarCollapsed ? 'w-[72px]' : 'w-[260px]'
        )}
      >
        {/* Sidebar accent bar */}
        <div className="absolute top-0 left-0 bottom-0 w-1 bg-gradient-to-b from-pierre-violet via-pierre-cyan to-pierre-activity"></div>

        {/* Logo Section */}
        <div className={clsx(
          'flex items-center border-b border-pierre-gray-100 transition-all duration-300',
          sidebarCollapsed ? 'px-3 py-4 justify-center' : 'px-5 py-5 gap-3'
        )}>
          <PierreLogo />
          {!sidebarCollapsed && (
            <div className="flex flex-col">
              <span className="text-lg font-semibold bg-gradient-to-r from-pierre-violet to-pierre-cyan bg-clip-text text-transparent">
                Pierre
              </span>
              <span className="text-[10px] text-pierre-gray-500 tracking-wide uppercase">
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
                  onClick={() => setActiveTab(tab.id)}
                  className={clsx(
                    'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-200 group relative',
                    {
                      'bg-gradient-to-r from-pierre-violet/10 to-pierre-cyan/5 text-pierre-violet shadow-sm': activeTab === tab.id,
                      'text-pierre-gray-600 hover:bg-pierre-gray-50 hover:text-pierre-violet': activeTab !== tab.id,
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
                        className="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full h-4 w-4 flex items-center justify-center font-bold text-[10px]"
                      >
                        {tab.badge}
                      </span>
                    )}
                  </div>
                  {!sidebarCollapsed && <span>{tab.name}</span>}
                  {/* Tooltip for collapsed state */}
                  {sidebarCollapsed && (
                    <div className="absolute left-full ml-2 px-2 py-1 bg-pierre-gray-900 text-white text-xs rounded opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap transition-opacity z-50">
                      {tab.name}
                    </div>
                  )}
                </button>
              </li>
            ))}
          </ul>
        </nav>

        {/* User Profile Section - Bottom of sidebar */}
        <div className={clsx(
          'border-t border-pierre-gray-100',
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
              <div className="absolute -bottom-0.5 -right-0.5 w-2 h-2 bg-green-500 rounded-full border border-white" />
            </div>

            {!sidebarCollapsed && (
              <div className="flex-1 min-w-0">
                <p className="text-[10px] font-medium text-pierre-gray-900 truncate leading-none">
                  {user?.display_name || user?.email}
                </p>
                <span className="text-[8px] text-pierre-gray-500 uppercase">
                  {user?.role === 'super_admin' ? 'Super Admin' : user?.role === 'admin' ? 'Admin' : 'User'}
                </span>
              </div>
            )}

            {!sidebarCollapsed && (
              <button
                onClick={logout}
                className="text-pierre-gray-400 hover:text-pierre-violet transition-colors flex-shrink-0"
                title="Sign out"
              >
                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
                </svg>
              </button>
            )}

            {sidebarCollapsed && (
              <button
                onClick={logout}
                className="text-pierre-gray-500 hover:text-pierre-violet transition-colors"
                title="Sign out"
              >
                <svg className="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
                </svg>
              </button>
            )}
          </div>
        </div>

        {/* Collapse Toggle Button */}
        <button
          onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
          className="absolute -right-3 top-20 w-6 h-6 bg-white border border-pierre-gray-200 rounded-full flex items-center justify-center shadow-sm hover:bg-pierre-gray-50 hover:border-pierre-violet transition-all duration-200 z-50"
          title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <svg
            className={clsx(
              'w-3 h-3 text-pierre-gray-500 transition-transform duration-300',
              sidebarCollapsed && 'rotate-180'
            )}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </aside>

      {/* Main Content Area */}
      <main
        className={clsx(
          'flex-1 min-h-screen transition-all duration-300 ease-in-out',
          sidebarCollapsed ? 'ml-[72px]' : 'ml-[260px]'
        )}
      >
        {/* Top Header Bar */}
        <header className="bg-white shadow-sm border-b border-pierre-gray-200 sticky top-0 z-30">
          <div className="px-6 py-4 flex items-center justify-between">
            <div>
              <h1 className="text-xl font-medium text-pierre-gray-800">
                {tabs.find(t => t.id === activeTab)?.name}
              </h1>
            </div>
          </div>
        </header>

        {/* Content Area */}
        <div className="p-6 overflow-auto">

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
            <Card>
              <h2 className="text-xl font-semibold mb-4">Real-time Request Monitor</h2>
              <p className="text-pierre-gray-600 mb-4">
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
            <Card>
              <h2 className="text-xl font-semibold mb-4">Tool Usage Analysis</h2>
              <p className="text-pierre-gray-600 mb-4">
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
            <Card>
              <h2 className="text-xl font-semibold mb-4">User Management</h2>
              <p className="text-pierre-gray-600 mb-4">
                Manage user registrations, approve pending users, and monitor user activity across the platform.
              </p>
            </Card>
            <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
              <UserManagement />
            </Suspense>
          </div>
        )}
        {activeTab === 'home' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <UserHome onNavigate={setActiveTab} />
          </Suspense>
        )}
        {activeTab === 'chat' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <ChatTab />
          </Suspense>
        )}
        {activeTab === 'settings' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <UserSettings />
          </Suspense>
        )}
        {activeTab === 'mcp-tokens' && (
          <Suspense fallback={<div className="flex justify-center py-8"><div className="pierre-spinner"></div></div>}>
            <MCPTokensTab />
          </Suspense>
        )}
        {activeTab === 'admin-settings' && (
          <div className="space-y-6">
            <Card>
              <h2 className="text-xl font-semibold mb-4">System Settings</h2>
              <p className="text-pierre-gray-600 mb-4">
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
                <Card>
                  <h2 className="text-xl font-semibold mb-4">API Key Management</h2>
                  <p className="text-pierre-gray-600 mb-4">
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
    </div>
  );
}