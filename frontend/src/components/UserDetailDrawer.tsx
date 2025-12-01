import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { User } from '../types/api';
import { Button, Card } from './ui';
import { Badge } from './ui/Badge';
import PasswordResetModal from './PasswordResetModal';

interface UserDetailDrawerProps {
  user: User | null;
  isOpen: boolean;
  onClose: () => void;
  onAction: (user: User, action: 'approve' | 'suspend') => void;
}

export default function UserDetailDrawer({
  user,
  isOpen,
  onClose,
  onAction
}: UserDetailDrawerProps) {
  const [isResetModalOpen, setIsResetModalOpen] = useState(false);

  const { data: rateLimit, isLoading: rateLimitLoading } = useQuery({
    queryKey: ['user-rate-limit', user?.id],
    queryFn: () => user ? apiService.getUserRateLimit(user.id) : null,
    enabled: !!user && isOpen,
  });

  const { data: activity, isLoading: activityLoading } = useQuery({
    queryKey: ['user-activity', user?.id],
    queryFn: () => user ? apiService.getUserActivity(user.id, 30) : null,
    enabled: !!user && isOpen,
  });

  if (!isOpen || !user) return null;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  const formatLimit = (limit: number | null) => {
    if (limit === null) return 'Unlimited';
    return limit.toLocaleString();
  };

  const getUsagePercentage = (used: number, limit: number | null) => {
    if (limit === null || limit === 0) return 0;
    return Math.min(100, (used / limit) * 100);
  };

  const getUsageColor = (percentage: number) => {
    if (percentage >= 90) return 'bg-red-500';
    if (percentage >= 75) return 'bg-amber-500';
    return 'bg-green-500';
  };

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black bg-opacity-50 z-40"
        onClick={onClose}
      />

      {/* Drawer */}
      <div className="fixed inset-y-0 right-0 w-full max-w-md bg-white shadow-xl z-50 overflow-y-auto">
        {/* Header */}
        <div className="sticky top-0 bg-white border-b border-gray-200 px-6 py-4 flex justify-between items-center">
          <h2 className="text-xl font-semibold text-gray-900">User Details</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="p-6 space-y-6">
          {/* User Info Card */}
          <Card className="p-4">
            <div className="flex items-start justify-between mb-4">
              <div>
                <h3 className="text-lg font-semibold text-gray-900">
                  {user.display_name || 'Unnamed User'}
                </h3>
                <p className="text-sm text-gray-600">{user.email}</p>
              </div>
              <div className="flex flex-col items-end space-y-2">
                <Badge
                  variant={
                    user.user_status === 'pending' ? 'warning' :
                    user.user_status === 'active' ? 'success' : 'destructive'
                  }
                >
                  {user.user_status}
                </Badge>
                <span className="text-xs text-gray-500 capitalize bg-gray-100 px-2 py-1 rounded">
                  {user.tier}
                </span>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-gray-500">Registered</span>
                <p className="font-medium text-gray-900">{formatDate(user.created_at)}</p>
              </div>
              <div>
                <span className="text-gray-500">Last Active</span>
                <p className="font-medium text-gray-900">{formatDate(user.last_active)}</p>
              </div>
              {user.approved_at && (
                <div>
                  <span className="text-gray-500">Approved</span>
                  <p className="font-medium text-gray-900">{formatDate(user.approved_at)}</p>
                </div>
              )}
              {user.approved_by && (
                <div>
                  <span className="text-gray-500">Approved By</span>
                  <p className="font-medium text-gray-900">{user.approved_by}</p>
                </div>
              )}
            </div>
          </Card>

          {/* Rate Limits Card */}
          <Card className="p-4">
            <h4 className="text-sm font-semibold text-gray-900 mb-4 flex items-center">
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
              Rate Limits
            </h4>

            {rateLimitLoading ? (
              <div className="space-y-3 animate-pulse">
                <div className="h-4 bg-gray-200 rounded w-3/4"></div>
                <div className="h-2 bg-gray-200 rounded w-full"></div>
                <div className="h-4 bg-gray-200 rounded w-3/4"></div>
                <div className="h-2 bg-gray-200 rounded w-full"></div>
              </div>
            ) : rateLimit ? (
              <div className="space-y-4">
                {/* Daily Usage */}
                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-gray-600">Daily Usage</span>
                    <span className="font-medium">
                      {rateLimit.rate_limits.daily.used.toLocaleString()} / {formatLimit(rateLimit.rate_limits.daily.limit)}
                    </span>
                  </div>
                  <div className="w-full bg-gray-200 rounded-full h-2">
                    <div
                      className={`h-2 rounded-full transition-all ${getUsageColor(getUsagePercentage(rateLimit.rate_limits.daily.used, rateLimit.rate_limits.daily.limit))}`}
                      style={{ width: `${getUsagePercentage(rateLimit.rate_limits.daily.used, rateLimit.rate_limits.daily.limit)}%` }}
                    />
                  </div>
                  <p className="text-xs text-gray-500 mt-1">
                    Resets: {formatDate(rateLimit.reset_times.daily_reset)}
                  </p>
                </div>

                {/* Monthly Usage */}
                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-gray-600">Monthly Usage</span>
                    <span className="font-medium">
                      {rateLimit.rate_limits.monthly.used.toLocaleString()} / {formatLimit(rateLimit.rate_limits.monthly.limit)}
                    </span>
                  </div>
                  <div className="w-full bg-gray-200 rounded-full h-2">
                    <div
                      className={`h-2 rounded-full transition-all ${getUsageColor(getUsagePercentage(rateLimit.rate_limits.monthly.used, rateLimit.rate_limits.monthly.limit))}`}
                      style={{ width: `${getUsagePercentage(rateLimit.rate_limits.monthly.used, rateLimit.rate_limits.monthly.limit)}%` }}
                    />
                  </div>
                  <p className="text-xs text-gray-500 mt-1">
                    Resets: {formatDate(rateLimit.reset_times.monthly_reset)}
                  </p>
                </div>
              </div>
            ) : (
              <p className="text-sm text-gray-500">Unable to load rate limit data</p>
            )}
          </Card>

          {/* Activity Card */}
          <Card className="p-4">
            <h4 className="text-sm font-semibold text-gray-900 mb-4 flex items-center">
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
              Activity (Last 30 Days)
            </h4>

            {activityLoading ? (
              <div className="space-y-3 animate-pulse">
                <div className="h-4 bg-gray-200 rounded w-1/2"></div>
                <div className="h-4 bg-gray-200 rounded w-3/4"></div>
                <div className="h-4 bg-gray-200 rounded w-2/3"></div>
              </div>
            ) : activity ? (
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <span className="text-gray-600">Total Requests</span>
                  <span className="text-2xl font-bold text-gray-900">
                    {activity.total_requests.toLocaleString()}
                  </span>
                </div>

                {activity.top_tools.length > 0 ? (
                  <div>
                    <span className="text-sm text-gray-600">Top Tools</span>
                    <div className="mt-2 space-y-2">
                      {activity.top_tools.slice(0, 5).map((tool) => (
                        <div key={tool.tool_name} className="flex items-center justify-between">
                          <span className="text-sm font-medium text-gray-900">{tool.tool_name}</span>
                          <div className="flex items-center space-x-2">
                            <span className="text-sm text-gray-600">{tool.call_count.toLocaleString()}</span>
                            <span className="text-xs text-gray-400">({tool.percentage.toFixed(1)}%)</span>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : (
                  <p className="text-sm text-gray-500">No tool usage in the last 30 days</p>
                )}
              </div>
            ) : (
              <p className="text-sm text-gray-500">Unable to load activity data</p>
            )}
          </Card>

          {/* Actions */}
          <Card className="p-4">
            <h4 className="text-sm font-semibold text-gray-900 mb-4">Actions</h4>
            <div className="space-y-2">
              <Button
                onClick={() => setIsResetModalOpen(true)}
                variant="outline"
                className="w-full justify-start"
              >
                <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
                </svg>
                Reset Password
              </Button>

              {user.user_status === 'pending' && (
                <Button
                  onClick={() => onAction(user, 'approve')}
                  className="w-full justify-start bg-green-600 hover:bg-green-700 text-white"
                >
                  <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  Approve User
                </Button>
              )}

              {user.user_status === 'active' && (
                <Button
                  onClick={() => onAction(user, 'suspend')}
                  variant="outline"
                  className="w-full justify-start border-red-300 text-red-700 hover:bg-red-50"
                >
                  <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L18.364 5.636M5.636 18.364l12.728-12.728" />
                  </svg>
                  Suspend User
                </Button>
              )}

              {user.user_status === 'suspended' && (
                <Button
                  onClick={() => onAction(user, 'approve')}
                  className="w-full justify-start bg-green-600 hover:bg-green-700 text-white"
                >
                  <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  Reactivate User
                </Button>
              )}
            </div>
          </Card>
        </div>
      </div>

      {/* Password Reset Modal */}
      <PasswordResetModal
        user={user}
        isOpen={isResetModalOpen}
        onClose={() => setIsResetModalOpen(false)}
      />
    </>
  );
}
