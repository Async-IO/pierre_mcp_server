// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { User } from '../types/api';
import { Button, Card, Badge } from './ui';
import PendingUsersList from './PendingUsersList';
import UserApprovalModal from './UserApprovalModal';
import UserDetailDrawer from './UserDetailDrawer';

type UserTab = 'pending' | 'active' | 'suspended' | 'all';

export default function UserManagement() {
  const [activeTab, setActiveTab] = useState<UserTab>('pending');
  const [selectedUser, setSelectedUser] = useState<User | null>(null);
  const [modalAction, setModalAction] = useState<'approve' | 'suspend'>('approve');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // Queries for different user types
  const { data: pendingUsers = [], isLoading: pendingLoading } = useQuery<User[]>({
    queryKey: ['pending-users'],
    queryFn: () => apiService.getPendingUsers(),
    refetchInterval: 30000,
  });

  const { data: allUsers = [], isLoading: allUsersLoading } = useQuery<User[]>({
    queryKey: ['all-users'],
    queryFn: () => apiService.getAllUsers(),
    refetchInterval: 60000,
  });

  // Filter users based on active tab and search query
  const filteredUsers = useMemo(() => {
    let users: User[] = [];

    switch (activeTab) {
      case 'pending':
        users = pendingUsers;
        break;
      case 'active':
        users = allUsers.filter(user => user.user_status === 'active');
        break;
      case 'suspended':
        users = allUsers.filter(user => user.user_status === 'suspended');
        break;
      case 'all':
        users = allUsers;
        break;
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      users = users.filter(user =>
        user.email.toLowerCase().includes(query) ||
        (user.display_name?.toLowerCase().includes(query))
      );
    }

    return users;
  }, [activeTab, pendingUsers, allUsers, searchQuery]);

  const tabs: Array<{ id: UserTab; name: string; count: number; icon: React.ReactNode }> = [
    {
      id: 'pending',
      name: 'Pending',
      count: pendingUsers.length,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      )
    },
    {
      id: 'active',
      name: 'Active',
      count: allUsers.filter(u => u.user_status === 'active').length,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      )
    },
    {
      id: 'suspended',
      name: 'Suspended',
      count: allUsers.filter(u => u.user_status === 'suspended').length,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L18.364 5.636M5.636 18.364l12.728-12.728" />
        </svg>
      )
    },
    {
      id: 'all',
      name: 'All Users',
      count: allUsers.length,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
        </svg>
      )
    }
  ];

  const handleUserAction = (user: User, action: 'approve' | 'suspend') => {
    setSelectedUser(user);
    setModalAction(action);
    setIsModalOpen(true);
    setIsDrawerOpen(false);
  };

  const handleCloseModal = () => {
    setIsModalOpen(false);
    setSelectedUser(null);
  };

  const handleOpenDrawer = (user: User) => {
    setSelectedUser(user);
    setIsDrawerOpen(true);
  };

  const handleCloseDrawer = () => {
    setIsDrawerOpen(false);
    setSelectedUser(null);
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  const getStatusBadgeVariant = (status: string) => {
    switch (status) {
      case 'pending': return 'warning';
      case 'active': return 'success';
      case 'suspended': return 'destructive';
      default: return 'secondary';
    }
  };

  // Show pending users component for pending tab
  if (activeTab === 'pending') {
    return (
      <div className="space-y-6">
        <div className="border-b border-pierre-gray-200">
          <nav className="-mb-px flex space-x-8">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`py-2 px-1 border-b-2 font-medium text-sm flex items-center space-x-2 ${
                  activeTab === tab.id
                    ? 'border-pierre-violet text-pierre-violet'
                    : 'border-transparent text-pierre-gray-500 hover:text-pierre-gray-700 hover:border-pierre-gray-300'
                }`}
              >
                {tab.icon}
                <span>{tab.name}</span>
                {tab.count > 0 && (
                  <Badge
                    variant={tab.id === 'pending' ? 'warning' : 'secondary'}
                    className="text-xs"
                  >
                    {tab.count}
                  </Badge>
                )}
              </button>
            ))}
          </nav>
        </div>

        <PendingUsersList />
      </div>
    );
  }

  // For other tabs, show the general user list
  const isLoading = pendingLoading || allUsersLoading;

  return (
    <div className="space-y-6">
      {/* Tabs */}
      <div className="border-b border-pierre-gray-200">
        <nav className="-mb-px flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`py-2 px-1 border-b-2 font-medium text-sm flex items-center space-x-2 ${
                activeTab === tab.id
                  ? 'border-pierre-violet text-pierre-violet'
                  : 'border-transparent text-pierre-gray-500 hover:text-pierre-gray-700 hover:border-pierre-gray-300'
              }`}
            >
              {tab.icon}
              <span>{tab.name}</span>
              {tab.count > 0 && (
                <Badge
                  variant={tab.id === 'pending' ? 'warning' : 'secondary'}
                  className="text-xs"
                >
                  {tab.count}
                </Badge>
              )}
            </button>
          ))}
        </nav>
      </div>

      {/* Search Bar */}
      <div className="flex justify-between items-center">
        <div className="flex-1 max-w-lg">
          <div className="relative">
            <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <svg className="h-5 w-5 text-pierre-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
            </div>
            <input
              type="text"
              className="block w-full pl-10 pr-3 py-2 border border-pierre-gray-300 rounded-md leading-5 bg-white placeholder-pierre-gray-500 focus:outline-none focus:placeholder-pierre-gray-400 focus:ring-1 focus:ring-pierre-violet focus:border-pierre-violet"
              placeholder="Search users by email or name..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
        </div>
        <div className="ml-4 text-sm text-pierre-gray-500">
          {filteredUsers.length} users
        </div>
      </div>

      {/* User List */}
      {isLoading ? (
        <div className="space-y-4">
          {[...Array(5)].map((_, i) => (
            <Card key={i} className="p-4 animate-pulse">
              <div className="flex justify-between items-start">
                <div className="space-y-2">
                  <div className="h-4 bg-pierre-gray-200 rounded w-48"></div>
                  <div className="h-3 bg-pierre-gray-200 rounded w-32"></div>
                  <div className="h-3 bg-pierre-gray-200 rounded w-24"></div>
                </div>
                <div className="space-y-2">
                  <div className="h-6 bg-pierre-gray-200 rounded w-16"></div>
                  <div className="h-8 bg-pierre-gray-200 rounded w-20"></div>
                </div>
              </div>
            </Card>
          ))}
        </div>
      ) : filteredUsers.length === 0 ? (
        <Card className="p-6 text-center">
          <div className="text-pierre-gray-500 mb-4">
            <svg className="w-12 h-12 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
            <p className="text-lg font-medium text-pierre-gray-900">
              {searchQuery ? 'No users found' : `No ${activeTab} users`}
            </p>
            <p className="text-pierre-gray-600">
              {searchQuery ? 'Try adjusting your search terms' : `No users with ${activeTab} status`}
            </p>
          </div>
        </Card>
      ) : (
        <div className="space-y-4">
          {filteredUsers.map((user) => (
            <Card
              key={user.id}
              className="p-4 hover:shadow-md transition-shadow cursor-pointer"
              onClick={() => handleOpenDrawer(user)}
            >
              <div className="flex justify-between items-start">
                <div className="flex-1">
                  <div className="flex items-center space-x-2 mb-1">
                    <h4 className="font-medium text-pierre-gray-900">
                      {user.display_name || 'Unnamed User'}
                    </h4>
                    <Badge variant={getStatusBadgeVariant(user.user_status)} className="text-xs">
                      {user.user_status}
                    </Badge>
                    <span className="text-xs text-pierre-gray-500 capitalize bg-pierre-gray-100 px-2 py-1 rounded">
                      {user.tier}
                    </span>
                  </div>
                  <p className="text-sm text-pierre-gray-600 mb-2">{user.email}</p>
                  <div className="flex items-center space-x-4 text-xs text-pierre-gray-500">
                    <span>Registered: {formatDate(user.created_at)}</span>
                    <span>Last active: {formatDate(user.last_active)}</span>
                    {user.approved_by && (
                      <span>Approved: {formatDate(user.approved_at!)}</span>
                    )}
                  </div>
                </div>

                <div className="flex space-x-2 ml-4" onClick={(e) => e.stopPropagation()}>
                  {user.user_status === 'pending' && (
                    <Button
                      onClick={() => handleUserAction(user, 'approve')}
                      size="sm"
                      className="bg-pierre-green-600 hover:bg-pierre-green-700 text-white"
                    >
                      Approve
                    </Button>
                  )}
                  {user.user_status === 'active' && (
                    <Button
                      onClick={() => handleUserAction(user, 'suspend')}
                      size="sm"
                      variant="outline"
                      className="border-pierre-red-300 text-pierre-red-700 hover:bg-pierre-red-50"
                    >
                      Suspend
                    </Button>
                  )}
                  {user.user_status === 'suspended' && (
                    <Button
                      onClick={() => handleUserAction(user, 'approve')}
                      size="sm"
                      className="bg-pierre-green-600 hover:bg-pierre-green-700 text-white"
                    >
                      Reactivate
                    </Button>
                  )}
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}

      <UserApprovalModal
        user={selectedUser}
        isOpen={isModalOpen}
        onClose={handleCloseModal}
        action={modalAction}
      />

      <UserDetailDrawer
        user={selectedUser}
        isOpen={isDrawerOpen}
        onClose={handleCloseDrawer}
        onAction={handleUserAction}
      />
    </div>
  );
}
