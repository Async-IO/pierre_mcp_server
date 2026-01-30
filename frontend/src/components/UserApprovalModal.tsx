// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import type { User, UserManagementResponse } from '../types/api';
import { Button, Card } from './ui';
import { Badge } from './ui/Badge';

interface UserApprovalModalProps {
  user: User | null;
  isOpen: boolean;
  onClose: () => void;
  action: 'approve' | 'suspend';
}

export default function UserApprovalModal({ 
  user, 
  isOpen, 
  onClose, 
  action 
}: UserApprovalModalProps) {
  const [reason, setReason] = useState('');
  const queryClient = useQueryClient();

  const approvalMutation = useMutation({
    mutationFn: async () => {
      if (!user) throw new Error('No user selected');
      
      if (action === 'approve') {
        return adminApi.approveUser(user.id, reason || undefined);
      } else {
        return adminApi.suspendUser(user.id, reason || undefined);
      }
    },
    onSuccess: (response: UserManagementResponse) => {
      // Refresh user lists
      queryClient.invalidateQueries({ queryKey: ['pending-users'] });
      queryClient.invalidateQueries({ queryKey: ['all-users'] });
      
      // Show success message
      console.log(`User ${action}d successfully:`, response.message);
      
      // Close modal
      onClose();
      setReason('');
    },
    onError: (error) => {
      console.error(`Failed to ${action} user:`, error);
    }
  });

  const handleSubmit = () => {
    approvalMutation.mutate();
  };

  const handleClose = () => {
    onClose();
    setReason('');
  };

  if (!isOpen || !user) return null;

  const actionTitle = action === 'approve' ? 'Approve User' : 'Suspend User';
  const actionColor = action === 'approve' ? 'bg-pierre-activity hover:bg-pierre-activity/80' : 'bg-pierre-red-500 hover:bg-pierre-red-600';
  const actionVerb = action === 'approve' ? 'approve' : 'suspend';

  return (
    <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
      <div className="bg-pierre-slate rounded-lg shadow-xl max-w-md w-full m-4 border border-white/10">
        <div className="p-6">
          <div className="flex justify-between items-start mb-4">
            <h2 className="text-xl font-semibold text-white">
              {actionTitle}
            </h2>
            <button
              onClick={handleClose}
              aria-label="Close modal"
              className="text-zinc-400 hover:text-white"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <Card variant="dark" className="mb-4 p-4 bg-white/5">
            <div className="flex items-start justify-between">
              <div>
                <h3 className="font-medium text-white">{user.display_name || 'Unnamed User'}</h3>
                <p className="text-sm text-zinc-400">{user.email}</p>
                <p className="text-sm text-zinc-500 mt-1">
                  Registered: {new Date(user.created_at).toLocaleDateString()}
                </p>
              </div>
              <div className="flex flex-col items-end space-y-1">
                <Badge
                  variant={
                    user.user_status === 'pending' ? 'warning' :
                    user.user_status === 'active' ? 'success' : 'destructive'
                  }
                  className="text-xs"
                >
                  {user.user_status}
                </Badge>
                <span className="text-xs text-zinc-400 capitalize">{user.tier}</span>
              </div>
            </div>
          </Card>

          <div className="mb-4">
            <label htmlFor="reason" className="block text-sm font-medium text-zinc-300 mb-2">
              Reason {action === 'approve' ? '(optional)' : '(recommended)'}
            </label>
            <textarea
              id="reason"
              rows={3}
              className="input-dark"
              placeholder={`Explain why you are ${action === 'approve' ? 'approving' : 'suspending'} this user...`}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
            />
          </div>

          <div className="flex space-x-3">
            <Button
              variant="outline"
              onClick={handleClose}
              disabled={approvalMutation.isPending}
              className="flex-1"
            >
              Cancel
            </Button>
            <Button
              onClick={handleSubmit}
              disabled={approvalMutation.isPending}
              className={`flex-1 text-white ${actionColor}`}
            >
              {approvalMutation.isPending ? (
                <div className="flex items-center justify-center">
                  <div className="pierre-spinner w-4 h-4 mr-2 border-white border-t-transparent" />
                  {action === 'approve' ? 'Approving...' : 'Suspending...'}
                </div>
              ) : (
                `${actionVerb.charAt(0).toUpperCase()}${actionVerb.slice(1)} User`
              )}
            </Button>
          </div>

          {approvalMutation.isError && (
            <div className="mt-3 p-3 bg-pierre-red-500/15 border border-pierre-red-500/30 rounded-md">
              <p className="text-sm text-pierre-red-400">
                Failed to {actionVerb} user. Please try again.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}