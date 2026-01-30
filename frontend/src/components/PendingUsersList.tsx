// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import type { User } from '../types/api';
import { Button, Card, Badge } from './ui';
import UserApprovalModal from './UserApprovalModal';

export default function PendingUsersList() {
  const [selectedUser, setSelectedUser] = useState<User | null>(null);
  const [modalAction, setModalAction] = useState<'approve' | 'suspend'>('approve');
  const [isModalOpen, setIsModalOpen] = useState(false);

  const { 
    data: pendingUsers = [], 
    isLoading, 
    error,
    refetch 
  } = useQuery<User[]>({
    queryKey: ['pending-users'],
    queryFn: () => adminApi.getPendingUsers(),
    refetchInterval: 30000, // Refetch every 30 seconds for real-time updates
  });

  const handleApprove = (user: User) => {
    setSelectedUser(user);
    setModalAction('approve');
    setIsModalOpen(true);
  };

  const handleSuspend = (user: User) => {
    setSelectedUser(user);
    setModalAction('suspend');
    setIsModalOpen(true);
  };

  const handleCloseModal = () => {
    setIsModalOpen(false);
    setSelectedUser(null);
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diffInHours = (now.getTime() - date.getTime()) / (1000 * 60 * 60);

    if (diffInHours < 24) {
      return `${Math.floor(diffInHours)}h ago`;
    } else if (diffInHours < 48) {
      return 'Yesterday';
    } else {
      return date.toLocaleDateString();
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[...Array(3)].map((_, i) => (
          <Card key={i} variant="dark" className="p-4 animate-pulse">
            <div className="flex justify-between items-start">
              <div className="space-y-2">
                <div className="h-4 bg-white/10 rounded w-48"></div>
                <div className="h-3 bg-white/10 rounded w-32"></div>
                <div className="h-3 bg-white/10 rounded w-24"></div>
              </div>
              <div className="space-y-2">
                <div className="h-6 bg-white/10 rounded w-16"></div>
                <div className="h-8 bg-white/10 rounded w-20"></div>
              </div>
            </div>
          </Card>
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <Card variant="dark" className="p-6 text-center">
        <div className="text-pierre-red-400 mb-4">
          <svg className="w-12 h-12 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 18.5c-.77.833.192 2.5 1.732 2.5z" />
          </svg>
          <p className="text-lg font-medium text-white">Failed to load pending users</p>
        </div>
        <Button onClick={() => refetch()} variant="outline">
          <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          Retry
        </Button>
      </Card>
    );
  }

  if (pendingUsers.length === 0) {
    return (
      <Card variant="dark" className="p-6 text-center">
        <div className="text-zinc-400 mb-4">
          <svg className="w-12 h-12 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
          </svg>
          <p className="text-lg font-medium text-white">No pending users</p>
          <p className="text-zinc-400">All users have been processed</p>
        </div>
      </Card>
    );
  }

  return (
    <>
      <div className="space-y-4">
        <div className="flex justify-between items-center">
          <h3 className="text-lg font-medium text-white">
            Pending Users ({pendingUsers.length})
          </h3>
          <Button
            onClick={() => refetch()}
            variant="outline"
            size="sm"
            className="flex items-center space-x-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            <span>Refresh</span>
          </Button>
        </div>

        {pendingUsers.map((user) => (
          <Card key={user.id} variant="dark" className="p-4 hover:border-white/20 transition-colors">
            <div className="flex justify-between items-start">
              <div className="flex-1">
                <div className="flex items-center space-x-2 mb-1">
                  <h4 className="font-medium text-white">
                    {user.display_name || 'Unnamed User'}
                  </h4>
                  <Badge variant="warning" className="text-xs">
                    {user.user_status}
                  </Badge>
                </div>
                <p className="text-sm text-zinc-400 mb-1">{user.email}</p>
                <div className="flex items-center space-x-4 text-xs text-zinc-500">
                  <span>Registered: {formatDate(user.created_at)}</span>
                  <span className="capitalize">Tier: {user.tier}</span>
                </div>
              </div>

              <div className="flex flex-col space-y-2 ml-4">
                <Button
                  onClick={() => handleApprove(user)}
                  size="sm"
                  className="bg-pierre-activity hover:bg-pierre-activity/80 text-white"
                >
                  <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                  </svg>
                  Approve
                </Button>
                <Button
                  onClick={() => handleSuspend(user)}
                  size="sm"
                  variant="outline"
                  className="border-pierre-red-500/30 text-pierre-red-400 hover:bg-pierre-red-500/10"
                >
                  <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L18.364 5.636M5.636 18.364l12.728-12.728" />
                  </svg>
                  Suspend
                </Button>
              </div>
            </div>
          </Card>
        ))}
      </div>

      <UserApprovalModal
        user={selectedUser}
        isOpen={isModalOpen}
        onClose={handleCloseModal}
        action={modalAction}
      />
    </>
  );
}