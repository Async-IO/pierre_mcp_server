import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
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
  const [loading, setLoading] = useState(false);
  const queryClient = useQueryClient();

  const approvalMutation = useMutation({
    mutationFn: async () => {
      if (!user) throw new Error('No user selected');
      
      if (action === 'approve') {
        return apiService.approveUser(user.id, reason || undefined);
      } else {
        return apiService.suspendUser(user.id, reason || undefined);
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
    setLoading(true);
    approvalMutation.mutate();
  };

  const handleClose = () => {
    onClose();
    setReason('');
  };

  if (!isOpen || !user) return null;

  const actionTitle = action === 'approve' ? 'Approve User' : 'Suspend User';
  const actionColor = action === 'approve' ? 'bg-green-600 hover:bg-green-700' : 'bg-red-600 hover:bg-red-700';
  const actionVerb = action === 'approve' ? 'approve' : 'suspend';

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-md w-full m-4">
        <div className="p-6">
          <div className="flex justify-between items-start mb-4">
            <h2 className="text-xl font-semibold text-gray-900">
              {actionTitle}
            </h2>
            <button
              onClick={handleClose}
              className="text-gray-400 hover:text-gray-600"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <Card className="mb-4 p-4 bg-gray-50">
            <div className="flex items-start justify-between">
              <div>
                <h3 className="font-medium text-gray-900">{user.display_name || 'Unnamed User'}</h3>
                <p className="text-sm text-gray-600">{user.email}</p>
                <p className="text-sm text-gray-500 mt-1">
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
                <span className="text-xs text-gray-500 capitalize">{user.tier}</span>
              </div>
            </div>
          </Card>

          <div className="mb-4">
            <label htmlFor="reason" className="block text-sm font-medium text-gray-700 mb-2">
              Reason {action === 'approve' ? '(optional)' : '(recommended)'}
            </label>
            <textarea
              id="reason"
              rows={3}
              className="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder={`Explain why you are ${action === 'approve' ? 'approving' : 'suspending'} this user...`}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
            />
          </div>

          <div className="flex space-x-3">
            <Button
              variant="outline"
              onClick={handleClose}
              disabled={loading || approvalMutation.isPending}
              className="flex-1"
            >
              Cancel
            </Button>
            <Button
              onClick={handleSubmit}
              disabled={loading || approvalMutation.isPending}
              className={`flex-1 text-white ${actionColor}`}
            >
              {approvalMutation.isPending ? (
                <div className="flex items-center justify-center">
                  <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                  </svg>
                  {action === 'approve' ? 'Approving...' : 'Suspending...'}
                </div>
              ) : (
                `${actionVerb.charAt(0).toUpperCase()}${actionVerb.slice(1)} User`
              )}
            </Button>
          </div>

          {approvalMutation.isError && (
            <div className="mt-3 p-3 bg-red-50 border border-red-200 rounded-md">
              <p className="text-sm text-red-700">
                Failed to {actionVerb} user. Please try again.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}