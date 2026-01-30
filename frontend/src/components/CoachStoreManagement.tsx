// ABOUTME: Admin Coach Store management main container component
// ABOUTME: Provides stats dashboard, tab navigation for review queue, published, and rejected coaches
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, lazy, Suspense } from 'react';
import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import { clsx } from 'clsx';

// Lazy load tab content components
const CoachReviewQueue = lazy(() => import('./CoachReviewQueue'));
const PublishedCoachesList = lazy(() => import('./PublishedCoachesList'));
const RejectedCoachesList = lazy(() => import('./RejectedCoachesList'));

type TabId = 'review' | 'published' | 'rejected';

interface Tab {
  id: TabId;
  name: string;
  icon: React.ReactNode;
}

const tabs: Tab[] = [
  {
    id: 'review',
    name: 'Review Queue',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
      </svg>
    ),
  },
  {
    id: 'published',
    name: 'Published',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
      </svg>
    ),
  },
  {
    id: 'rejected',
    name: 'Rejected',
    icon: (
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
      </svg>
    ),
  },
];

export default function CoachStoreManagement() {
  const [activeTab, setActiveTab] = useState<TabId>('review');

  // Fetch store stats
  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ['admin-store-stats'],
    queryFn: () => adminApi.getStoreStats(),
    staleTime: 30_000,
  });

  const formatNumber = (num: number | undefined) => {
    if (num === undefined) return '—';
    return num.toLocaleString();
  };

  const formatPercentage = (num: number | undefined) => {
    if (num === undefined) return '—';
    return `${(num * 100).toFixed(1)}%`;
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-2xl font-semibold text-white">Coach Store Management</h2>
        <p className="text-zinc-400 mt-1">
          Review coach submissions, manage published coaches, and monitor store activity.
        </p>
      </div>

      {/* Stats Dashboard */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Pending Reviews */}
        <button
          onClick={() => setActiveTab('review')}
          className={clsx(
            'bg-[rgba(30,30,46,0.6)] backdrop-blur-[16px] border rounded-xl p-5 text-left transition-all',
            activeTab === 'review'
              ? 'border-pierre-violet/50 ring-1 ring-pierre-violet/30'
              : 'border-white/10 hover:border-pierre-violet/30'
          )}
        >
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-pierre-violet/20 flex items-center justify-center">
              <svg className="w-5 h-5 text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            {(stats?.pending_count ?? 0) > 0 && (
              <span className="bg-pierre-violet text-white text-xs font-bold px-2 py-1 rounded-full">
                {stats?.pending_count}
              </span>
            )}
          </div>
          <div className="text-2xl font-bold text-white">
            {statsLoading ? (
              <div className="h-8 w-12 bg-white/10 rounded animate-pulse" />
            ) : (
              formatNumber(stats?.pending_count)
            )}
          </div>
          <div className="text-sm text-zinc-400">Pending Reviews</div>
        </button>

        {/* Published Coaches */}
        <button
          onClick={() => setActiveTab('published')}
          className={clsx(
            'bg-[rgba(30,30,46,0.6)] backdrop-blur-[16px] border rounded-xl p-5 text-left transition-all',
            activeTab === 'published'
              ? 'border-pierre-activity/50 ring-1 ring-pierre-activity/30'
              : 'border-white/10 hover:border-pierre-activity/30'
          )}
        >
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-pierre-activity/20 flex items-center justify-center">
              <svg className="w-5 h-5 text-pierre-activity" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
          </div>
          <div className="text-2xl font-bold text-white">
            {statsLoading ? (
              <div className="h-8 w-12 bg-white/10 rounded animate-pulse" />
            ) : (
              formatNumber(stats?.published_count)
            )}
          </div>
          <div className="text-sm text-zinc-400">Published Coaches</div>
        </button>

        {/* Total Installs */}
        <div className="bg-[rgba(30,30,46,0.6)] backdrop-blur-[16px] border border-white/10 rounded-xl p-5">
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-pierre-cyan/20 flex items-center justify-center">
              <svg className="w-5 h-5 text-pierre-cyan" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
            </div>
          </div>
          <div className="text-2xl font-bold text-white">
            {statsLoading ? (
              <div className="h-8 w-12 bg-white/10 rounded animate-pulse" />
            ) : (
              formatNumber(stats?.total_installs)
            )}
          </div>
          <div className="text-sm text-zinc-400">Total Installs</div>
        </div>

        {/* Rejection Rate */}
        <button
          onClick={() => setActiveTab('rejected')}
          className={clsx(
            'bg-[rgba(30,30,46,0.6)] backdrop-blur-[16px] border rounded-xl p-5 text-left transition-all',
            activeTab === 'rejected'
              ? 'border-pierre-nutrition/50 ring-1 ring-pierre-nutrition/30'
              : 'border-white/10 hover:border-pierre-nutrition/30'
          )}
        >
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-pierre-nutrition/20 flex items-center justify-center">
              <svg className="w-5 h-5 text-pierre-nutrition" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
            </div>
          </div>
          <div className="text-2xl font-bold text-white">
            {statsLoading ? (
              <div className="h-8 w-12 bg-white/10 rounded animate-pulse" />
            ) : (
              formatPercentage(stats?.rejection_rate)
            )}
          </div>
          <div className="text-sm text-zinc-400">Rejection Rate</div>
        </button>
      </div>

      {/* Tab Navigation */}
      <div className="border-b border-white/10">
        <nav className="flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={clsx(
                'flex items-center gap-2 px-1 py-4 text-sm font-medium border-b-2 transition-colors',
                activeTab === tab.id
                  ? 'border-pierre-violet text-pierre-violet-light'
                  : 'border-transparent text-zinc-400 hover:text-white hover:border-white/30'
              )}
            >
              {tab.icon}
              {tab.name}
              {tab.id === 'review' && (stats?.pending_count ?? 0) > 0 && (
                <span className="bg-pierre-violet/20 text-pierre-violet-light text-xs font-bold px-2 py-0.5 rounded-full">
                  {stats?.pending_count}
                </span>
              )}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab Content */}
      <Suspense
        fallback={
          <div className="flex justify-center py-12">
            <div className="pierre-spinner w-8 h-8"></div>
          </div>
        }
      >
        {activeTab === 'review' && <CoachReviewQueue />}
        {activeTab === 'published' && <PublishedCoachesList />}
        {activeTab === 'rejected' && <RejectedCoachesList />}
      </Suspense>
    </div>
  );
}
