// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Friends management tab for the web dashboard
// ABOUTME: Displays friend list, search, and pending requests with Pierre dark theme

import { useState, useEffect, useCallback } from 'react';
import { clsx } from 'clsx';
import { socialApi } from '../../services/api';
import { Card, Button } from '../ui';
import type {
  FriendWithInfo,
  DiscoverableUser,
  PendingRequestWithInfo,
} from '@pierre/shared-types';

type TabView = 'friends' | 'search' | 'pending';

interface FriendsTabProps {
  onBack?: () => void;
}

export default function FriendsTab({ onBack }: FriendsTabProps) {
  const [activeTab, setActiveTab] = useState<TabView>('friends');
  const [friends, setFriends] = useState<FriendWithInfo[]>([]);
  const [searchResults, setSearchResults] = useState<DiscoverableUser[]>([]);
  const [pendingReceived, setPendingReceived] = useState<PendingRequestWithInfo[]>([]);
  const [pendingSent, setPendingSent] = useState<PendingRequestWithInfo[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const loadFriends = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await socialApi.listFriends();
      setFriends(response.friends);
    } catch (error) {
      console.error('Failed to load friends:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const loadPendingRequests = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await socialApi.getPendingFriendRequests();
      setPendingReceived(response.received);
      setPendingSent(response.sent);
    } catch (error) {
      console.error('Failed to load pending requests:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (activeTab === 'friends') {
      loadFriends();
    } else if (activeTab === 'pending') {
      loadPendingRequests();
    }
  }, [activeTab, loadFriends, loadPendingRequests]);

  const handleSearch = async () => {
    if (!searchQuery.trim()) return;

    try {
      setIsSearching(true);
      const response = await socialApi.searchUsers(searchQuery.trim());
      setSearchResults(response.users);
    } catch (error) {
      console.error('Search failed:', error);
    } finally {
      setIsSearching(false);
    }
  };

  const handleSendRequest = async (userId: string) => {
    try {
      setActionLoading(userId);
      await socialApi.sendFriendRequest(userId);
      // Update search results to show pending
      setSearchResults(prev =>
        prev.map(u => u.id === userId ? { ...u, has_pending_request: true } : u)
      );
    } catch (error) {
      console.error('Failed to send friend request:', error);
    } finally {
      setActionLoading(null);
    }
  };

  const handleAcceptRequest = async (connectionId: string) => {
    try {
      setActionLoading(connectionId);
      await socialApi.acceptFriendRequest(connectionId);
      await loadPendingRequests();
    } catch (error) {
      console.error('Failed to accept request:', error);
    } finally {
      setActionLoading(null);
    }
  };

  const handleRejectRequest = async (connectionId: string) => {
    try {
      setActionLoading(connectionId);
      await socialApi.rejectFriendRequest(connectionId);
      await loadPendingRequests();
    } catch (error) {
      console.error('Failed to reject request:', error);
    } finally {
      setActionLoading(null);
    }
  };

  const handleRemoveFriend = async (userId: string) => {
    try {
      setActionLoading(userId);
      await socialApi.removeFriend(userId);
      await loadFriends();
    } catch (error) {
      console.error('Failed to remove friend:', error);
    } finally {
      setActionLoading(null);
    }
  };

  const getInitials = (name: string | null, email?: string): string => {
    if (name) {
      const parts = name.split(' ');
      if (parts.length >= 2) {
        return (parts[0][0] + parts[1][0]).toUpperCase();
      }
      return name.substring(0, 2).toUpperCase();
    }
    if (email) {
      return email.substring(0, 2).toUpperCase();
    }
    return '??';
  };

  const getAvatarColor = (str: string): string => {
    const hash = str.split('').reduce((acc, char) => {
      return char.charCodeAt(0) + ((acc << 5) - acc);
    }, 0);
    const hue = Math.abs(hash % 360);
    return `hsl(${hue}, 70%, 50%)`;
  };

  const formatRelativeTime = (dateStr: string): string => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  return (
    <div className="space-y-6">
      {/* Toolbar */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {onBack && (
            <button
              onClick={onBack}
              className="flex items-center gap-1 text-zinc-400 hover:text-pierre-violet transition-colors"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
              Insights
            </button>
          )}
          <p className="text-sm text-zinc-400">
            Connect with other athletes and share coach insights
          </p>
        </div>
        {pendingReceived.length > 0 && activeTab !== 'pending' && (
          <span className="px-2 py-1 text-xs font-medium bg-pierre-violet/20 text-pierre-violet-light rounded-full border border-pierre-violet/30">
            {pendingReceived.length} pending
          </span>
        )}
      </div>

      {/* Tab navigation */}
      <div className="flex gap-2">
        <button
          onClick={() => setActiveTab('friends')}
          className={clsx(
            'px-4 py-2 rounded-lg text-sm font-medium transition-colors',
            activeTab === 'friends'
              ? 'bg-pierre-violet text-white'
              : 'bg-white/5 text-zinc-400 hover:text-white hover:bg-white/10'
          )}
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
            </svg>
            Friends
          </span>
        </button>
        <button
          onClick={() => setActiveTab('search')}
          className={clsx(
            'px-4 py-2 rounded-lg text-sm font-medium transition-colors',
            activeTab === 'search'
              ? 'bg-pierre-violet text-white'
              : 'bg-white/5 text-zinc-400 hover:text-white hover:bg-white/10'
          )}
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            Find Friends
          </span>
        </button>
        <button
          onClick={() => setActiveTab('pending')}
          className={clsx(
            'px-4 py-2 rounded-lg text-sm font-medium transition-colors relative',
            activeTab === 'pending'
              ? 'bg-pierre-violet text-white'
              : 'bg-white/5 text-zinc-400 hover:text-white hover:bg-white/10'
          )}
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            Pending
            {pendingReceived.length > 0 && (
              <span className="absolute -top-1 -right-1 w-5 h-5 bg-pierre-nutrition text-white text-xs font-bold rounded-full flex items-center justify-center">
                {pendingReceived.length}
              </span>
            )}
          </span>
        </button>
      </div>

      {/* Friends List */}
      {activeTab === 'friends' && (
        <Card variant="dark" className="!p-5">
          {isLoading ? (
            <div className="flex justify-center py-8">
              <div className="pierre-spinner"></div>
            </div>
          ) : friends.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-pierre-violet/20 flex items-center justify-center">
                <svg className="w-8 h-8 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
                </svg>
              </div>
              <h3 className="text-lg font-semibold text-white mb-2">No friends yet</h3>
              <p className="text-zinc-400 mb-4">Find and connect with other athletes</p>
              <Button variant="primary" onClick={() => setActiveTab('search')}>
                Find Friends
              </Button>
            </div>
          ) : (
            <div className="space-y-3">
              {friends.map((friend) => (
                <div
                  key={friend.id}
                  className="flex items-center justify-between p-4 rounded-lg bg-white/5 hover:bg-white/10 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    <div
                      className="w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold text-white"
                      style={{ backgroundColor: getAvatarColor(friend.friend_email) }}
                    >
                      {getInitials(friend.friend_display_name, friend.friend_email)}
                    </div>
                    <div>
                      <p className="font-medium text-white">
                        {friend.friend_display_name || friend.friend_email.split('@')[0]}
                      </p>
                      <p className="text-sm text-zinc-500">
                        Friends since {formatRelativeTime(friend.accepted_at || friend.created_at)}
                      </p>
                    </div>
                  </div>
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() => handleRemoveFriend(friend.friend_user_id)}
                    loading={actionLoading === friend.friend_user_id}
                  >
                    Remove
                  </Button>
                </div>
              ))}
            </div>
          )}
        </Card>
      )}

      {/* Search */}
      {activeTab === 'search' && (
        <div className="space-y-4">
          <Card variant="dark" className="!p-5">
            <div className="flex gap-3">
              <input
                type="search"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                placeholder="Search by name or email..."
                aria-label="Search friends by name or email"
                className="flex-1 px-4 py-2 bg-white/5 border border-white/10 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-pierre-violet/50"
              />
              <Button variant="primary" onClick={handleSearch} loading={isSearching}>
                Search
              </Button>
            </div>
          </Card>

          {searchResults.length > 0 && (
            <Card variant="dark" className="!p-5">
              <h3 className="text-sm font-semibold text-zinc-400 mb-4">Search Results</h3>
              <div className="space-y-3">
                {searchResults.map((user) => (
                  <div
                    key={user.id}
                    className="flex items-center justify-between p-4 rounded-lg bg-white/5"
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold text-white"
                        style={{ backgroundColor: getAvatarColor(user.email ?? user.id) }}
                      >
                        {getInitials(user.display_name, user.email)}
                      </div>
                      <div>
                        <p className="font-medium text-white">
                          {user.display_name || user.email?.split('@')[0] || 'Unknown User'}
                        </p>
                      </div>
                    </div>
                    {user.is_friend ? (
                      <span className="px-3 py-1 text-sm text-pierre-activity bg-pierre-activity/20 rounded-full">
                        Friends
                      </span>
                    ) : user.has_pending_request ? (
                      <span className="px-3 py-1 text-sm text-pierre-nutrition bg-pierre-nutrition/20 rounded-full">
                        Pending
                      </span>
                    ) : (
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={() => handleSendRequest(user.id)}
                        loading={actionLoading === user.id}
                      >
                        Add Friend
                      </Button>
                    )}
                  </div>
                ))}
              </div>
            </Card>
          )}
        </div>
      )}

      {/* Pending Requests */}
      {activeTab === 'pending' && (
        <div className="space-y-4">
          {/* Received Requests */}
          <Card variant="dark" className="!p-5">
            <h3 className="text-sm font-semibold text-zinc-400 mb-4">
              Received Requests ({pendingReceived.length})
            </h3>
            {isLoading ? (
              <div className="flex justify-center py-8">
                <div className="pierre-spinner"></div>
              </div>
            ) : pendingReceived.length === 0 ? (
              <p className="text-center py-8 text-zinc-500">No pending requests</p>
            ) : (
              <div className="space-y-3">
                {pendingReceived.map((request) => {
                  const displayName = request.user_display_name || request.user_email.split('@')[0];
                  const initials = displayName.substring(0, 2).toUpperCase();
                  return (
                  <div
                    key={request.id}
                    className="flex items-center justify-between p-4 rounded-lg bg-white/5"
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold text-white"
                        style={{ backgroundColor: getAvatarColor(request.user_id) }}
                      >
                        {initials}
                      </div>
                      <div>
                        <p className="font-medium text-white">{displayName}</p>
                        <p className="text-sm text-zinc-500">
                          Sent {formatRelativeTime(request.created_at)}
                        </p>
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={() => handleAcceptRequest(request.id)}
                        loading={actionLoading === request.id}
                      >
                        Accept
                      </Button>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => handleRejectRequest(request.id)}
                        loading={actionLoading === request.id}
                      >
                        Decline
                      </Button>
                    </div>
                  </div>
                  );
                })}
              </div>
            )}
          </Card>

          {/* Sent Requests */}
          <Card variant="dark" className="!p-5">
            <h3 className="text-sm font-semibold text-zinc-400 mb-4">
              Sent Requests ({pendingSent.length})
            </h3>
            {pendingSent.length === 0 ? (
              <p className="text-center py-8 text-zinc-500">No sent requests</p>
            ) : (
              <div className="space-y-3">
                {pendingSent.map((request) => {
                  const displayName = request.user_display_name || request.user_email.split('@')[0];
                  const initials = displayName.substring(0, 2).toUpperCase();
                  return (
                  <div
                    key={request.id}
                    className="flex items-center justify-between p-4 rounded-lg bg-white/5"
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold text-white"
                        style={{ backgroundColor: getAvatarColor(request.user_id) }}
                      >
                        {initials}
                      </div>
                      <div>
                        <p className="font-medium text-white">{displayName}</p>
                        <p className="text-sm text-zinc-500">
                          Sent {formatRelativeTime(request.created_at)}
                        </p>
                      </div>
                    </div>
                    <span className="px-3 py-1 text-sm text-pierre-nutrition bg-pierre-nutrition/20 rounded-full">
                      Awaiting
                    </span>
                  </div>
                  );
                })}
              </div>
            )}
          </Card>
        </div>
      )}
    </div>
  );
}
