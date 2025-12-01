import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { User } from '../types/api';
import { Button, Card } from './ui';
import { Badge } from './ui/Badge';

interface PasswordResetModalProps {
  user: User | null;
  isOpen: boolean;
  onClose: () => void;
}

interface PasswordResetResponse {
  success: boolean;
  temporary_password: string;
  expires_at: string;
  user_email: string;
}

export default function PasswordResetModal({
  user,
  isOpen,
  onClose
}: PasswordResetModalProps) {
  const [resetResult, setResetResult] = useState<PasswordResetResponse | null>(null);
  const [copied, setCopied] = useState(false);

  const resetMutation = useMutation({
    mutationFn: async () => {
      if (!user) throw new Error('No user selected');
      return apiService.resetUserPassword(user.id);
    },
    onSuccess: (response: PasswordResetResponse) => {
      setResetResult(response);
    },
    onError: (error) => {
      console.error('Failed to reset password:', error);
    }
  });

  const handleReset = () => {
    resetMutation.mutate();
  };

  const handleCopyPassword = async () => {
    if (resetResult?.temporary_password) {
      await navigator.clipboard.writeText(resetResult.temporary_password);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleClose = () => {
    onClose();
    setResetResult(null);
    setCopied(false);
    resetMutation.reset();
  };

  if (!isOpen || !user) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-md w-full m-4">
        <div className="p-6">
          <div className="flex justify-between items-start mb-4">
            <h2 className="text-xl font-semibold text-gray-900">
              Reset User Password
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
              </div>
              <Badge
                variant={
                  user.user_status === 'pending' ? 'warning' :
                  user.user_status === 'active' ? 'success' : 'destructive'
                }
                className="text-xs"
              >
                {user.user_status}
              </Badge>
            </div>
          </Card>

          {!resetResult ? (
            <>
              <div className="mb-4 p-3 bg-amber-50 border border-amber-200 rounded-md">
                <div className="flex items-start">
                  <svg className="w-5 h-5 text-amber-600 mr-2 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                  </svg>
                  <div>
                    <p className="text-sm font-medium text-amber-800">Warning</p>
                    <p className="text-sm text-amber-700">
                      This will generate a temporary password for the user. They must change it on their next login.
                    </p>
                  </div>
                </div>
              </div>

              <div className="flex space-x-3">
                <Button
                  variant="outline"
                  onClick={handleClose}
                  disabled={resetMutation.isPending}
                  className="flex-1"
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleReset}
                  disabled={resetMutation.isPending}
                  className="flex-1 bg-blue-600 hover:bg-blue-700 text-white"
                >
                  {resetMutation.isPending ? (
                    <div className="flex items-center justify-center">
                      <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                      </svg>
                      Resetting...
                    </div>
                  ) : (
                    'Reset Password'
                  )}
                </Button>
              </div>
            </>
          ) : (
            <>
              <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-md">
                <div className="flex items-center mb-2">
                  <svg className="w-5 h-5 text-green-600 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <p className="text-sm font-medium text-green-800">Password Reset Successful</p>
                </div>
                <p className="text-sm text-green-700 mb-3">
                  A temporary password has been generated for <strong>{resetResult.user_email}</strong>.
                </p>
              </div>

              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Temporary Password
                </label>
                <div className="flex items-center space-x-2">
                  <code className="flex-1 px-3 py-2 bg-gray-100 border border-gray-300 rounded-md font-mono text-sm">
                    {resetResult.temporary_password}
                  </code>
                  <Button
                    variant="outline"
                    onClick={handleCopyPassword}
                    className="px-3"
                  >
                    {copied ? (
                      <svg className="w-5 h-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                      </svg>
                    ) : (
                      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                      </svg>
                    )}
                  </Button>
                </div>
                <p className="mt-2 text-xs text-gray-500">
                  Expires: {new Date(resetResult.expires_at).toLocaleString()}
                </p>
              </div>

              <div className="mb-4 p-3 bg-blue-50 border border-blue-200 rounded-md">
                <p className="text-sm text-blue-700">
                  Please securely share this temporary password with the user. They will be required to change it upon their next login.
                </p>
              </div>

              <Button
                onClick={handleClose}
                className="w-full bg-gray-600 hover:bg-gray-700 text-white"
              >
                Done
              </Button>
            </>
          )}

          {resetMutation.isError && !resetResult && (
            <div className="mt-3 p-3 bg-red-50 border border-red-200 rounded-md">
              <p className="text-sm text-red-700">
                Failed to reset password. Please try again.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
