// ABOUTME: User home dashboard tab for regular users
// ABOUTME: Shows welcome message, quick stats, provider connections, and quick actions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useQuery } from '@tanstack/react-query';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import { Card } from './ui';

interface UserHomeProps {
  onNavigate: (tab: string) => void;
}

export default function UserHome({ onNavigate }: UserHomeProps) {
  const { user } = useAuth();

  // Fetch user stats for dashboard
  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ['userStats'],
    queryFn: () => apiService.getUserStats(),
    staleTime: 30000, // Cache for 30 seconds
  });

  return (
    <div className="space-y-6">
      {/* Welcome Card */}
      <Card>
        <div className="flex items-center gap-4">
          <div className="w-16 h-16 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-full flex items-center justify-center flex-shrink-0">
            <span className="text-2xl font-bold text-white">
              {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
            </span>
          </div>
          <div>
            <h1 className="text-2xl font-semibold text-pierre-gray-900">
              Welcome!
            </h1>
            <p className="text-pierre-gray-600 mt-1">
              Manage your fitness connections and explore your data with AI.
            </p>
          </div>
        </div>
      </Card>

      {/* Quick Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-pierre-violet">
              {statsLoading ? '...' : (stats?.connected_providers ?? 0)}
            </div>
            <div className="text-sm text-pierre-gray-600 mt-1">Connected Providers</div>
          </div>
        </Card>
        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-pierre-activity">
              {statsLoading ? '...' : (stats?.activities_synced ?? 0)}
            </div>
            <div className="text-sm text-pierre-gray-600 mt-1">Activities Synced</div>
          </div>
        </Card>
        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-pierre-nutrition">
              {statsLoading ? '...' : (stats?.days_active ?? 0)}
            </div>
            <div className="text-sm text-pierre-gray-600 mt-1">Days Active</div>
          </div>
        </Card>
      </div>

      {/* Quick Actions */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card className="hover:shadow-md transition-shadow cursor-pointer" onClick={() => onNavigate('connections')}>
          <div className="flex items-start gap-4">
            <div className="w-12 h-12 bg-pierre-activity-light rounded-lg flex items-center justify-center flex-shrink-0">
              <svg className="w-6 h-6 text-pierre-activity" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
              </svg>
            </div>
            <div>
              <h3 className="font-semibold text-pierre-gray-900">Connect Providers</h3>
              <p className="text-sm text-pierre-gray-600 mt-1">
                Link your Strava, Garmin, or other fitness accounts to sync your data.
              </p>
            </div>
          </div>
        </Card>

        <Card className="hover:shadow-md transition-shadow cursor-pointer" onClick={() => onNavigate('settings')}>
          <div className="flex items-start gap-4">
            <div className="w-12 h-12 bg-pierre-violet/10 rounded-lg flex items-center justify-center flex-shrink-0">
              <svg className="w-6 h-6 text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            </div>
            <div>
              <h3 className="font-semibold text-pierre-gray-900">Account Settings</h3>
              <p className="text-sm text-pierre-gray-600 mt-1">
                Update your profile, preferences, and manage your account.
              </p>
            </div>
          </div>
        </Card>
      </div>

      {/* Getting Started Section */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Getting Started</h2>
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <div className="w-6 h-6 rounded-full bg-pierre-activity-light flex items-center justify-center flex-shrink-0">
              <span className="text-xs font-bold text-pierre-activity">1</span>
            </div>
            <p className="text-sm text-pierre-gray-700">Connect your fitness providers (Strava, Garmin, etc.)</p>
          </div>
          <div className="flex items-center gap-3">
            <div className="w-6 h-6 rounded-full bg-pierre-gray-100 flex items-center justify-center flex-shrink-0">
              <span className="text-xs font-bold text-pierre-gray-500">2</span>
            </div>
            <p className="text-sm text-pierre-gray-500">Sync your activity data</p>
          </div>
          <div className="flex items-center gap-3">
            <div className="w-6 h-6 rounded-full bg-pierre-gray-100 flex items-center justify-center flex-shrink-0">
              <span className="text-xs font-bold text-pierre-gray-500">3</span>
            </div>
            <p className="text-sm text-pierre-gray-500">Explore your fitness insights with AI</p>
          </div>
        </div>
      </Card>
    </div>
  );
}
