import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { format } from 'date-fns';
import { Button, Card, CardHeader, Badge } from './ui';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import type { AdminToken, AdminTokenAudit, AdminTokenUsageStats, ProvisionedKey } from '../types/api';

interface AdminTokenDetailsProps {
  token: AdminToken;
  onBack: () => void;
  onTokenUpdated: () => void;
}

interface TokenSuccessModalProps {
  isOpen: boolean;
  onClose: () => void;
  newToken: string;
  tokenInfo: AdminToken;
}

const TokenSuccessModal: React.FC<TokenSuccessModalProps> = ({ 
  isOpen, 
  onClose, 
  newToken, 
  tokenInfo 
}) => {
  const [copied, setCopied] = useState(false);

  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText(newToken);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy token:', err);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card className="max-w-2xl mx-4 w-full">
        <CardHeader 
          title="🔄 Token Rotated Successfully" 
          subtitle="Your new admin token is ready"
        />
        
        <div className="space-y-6">
          <div className="bg-pierre-yellow-50 border border-pierre-yellow-200 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <svg className="w-6 h-6 text-pierre-yellow-600 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.5 0L4.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
              <div>
                <h4 className="font-medium text-pierre-yellow-800">Important Security Notice</h4>
                <p className="text-sm text-pierre-yellow-700 mt-1">
                  This is the only time the full token will be displayed. Please copy it now and store it securely. 
                  The old token has been invalidated and will no longer work.
                </p>
              </div>
            </div>
          </div>

          <div>
            <label className="label">New JWT Token</label>
            <div className="relative">
              <textarea
                className="input-field font-mono text-xs resize-none"
                value={newToken}
                readOnly
                rows={8}
                onClick={(e) => e.currentTarget.select()}
              />
              <Button
                variant="secondary"
                size="sm"
                className="absolute top-2 right-2"
                onClick={copyToClipboard}
              >
                {copied ? (
                  <>
                    <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                    </svg>
                    Copied!
                  </>
                ) : (
                  <>
                    <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                    Copy
                  </>
                )}
              </Button>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-pierre-gray-500">Service:</span>
              <span className="ml-2 font-medium">{tokenInfo.service_name}</span>
            </div>
            <div>
              <span className="text-pierre-gray-500">Prefix:</span>
              <span className="ml-2 font-mono">{tokenInfo.token_prefix}...</span>
            </div>
          </div>

          <div className="flex gap-3 pt-4 border-t border-pierre-gray-200">
            <Button onClick={onClose} className="flex-1">
              I've Saved the Token Securely
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
};

export default function AdminTokenDetails({ token, onBack, onTokenUpdated }: AdminTokenDetailsProps) {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [showRotateModal, setShowRotateModal] = useState(false);
  const [rotatedToken, setRotatedToken] = useState<string>('');

  const { data: auditData, isLoading: auditLoading } = useQuery({
    queryKey: ['admin-token-audit', token.id],
    queryFn: () => apiService.getAdminTokenAudit(token.id),
    enabled: isAuthenticated,
  });

  const { data: usageStats, isLoading: statsLoading } = useQuery({
    queryKey: ['admin-token-usage-stats', token.id],
    queryFn: () => apiService.getAdminTokenUsageStats(token.id),
    enabled: isAuthenticated,
  });

  const { data: provisionedKeys } = useQuery({
    queryKey: ['admin-token-provisioned-keys', token.id],
    queryFn: () => apiService.getAdminTokenProvisionedKeys(token.id),
    enabled: isAuthenticated,
  });

  const revokeTokenMutation = useMutation({
    mutationFn: () => apiService.revokeAdminToken(token.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-tokens'] });
      onTokenUpdated();
      onBack();
    },
  });

  const rotateTokenMutation = useMutation({
    mutationFn: () => apiService.rotateAdminToken(token.id),
    onSuccess: (data) => {
      setRotatedToken(data.jwt_token);
      setShowRotateModal(true);
      queryClient.invalidateQueries({ queryKey: ['admin-tokens'] });
      queryClient.invalidateQueries({ queryKey: ['admin-token-audit', token.id] });
      onTokenUpdated();
    },
  });

  const handleRevoke = () => {
    const confirmed = confirm(
      `Are you sure you want to revoke the admin token for "${token.service_name}"? This action cannot be undone and will immediately disable all access using this token.`
    );
    if (confirmed) {
      revokeTokenMutation.mutate();
    }
  };

  const handleRotate = () => {
    const confirmed = confirm(
      `Are you sure you want to rotate the admin token for "${token.service_name}"? The current token will be invalidated and a new one will be generated.`
    );
    if (confirmed) {
      rotateTokenMutation.mutate();
    }
  };

  const getStatusBadge = () => {
    if (!token.is_active) {
      return <Badge variant="error">Revoked</Badge>;
    }
    
    if (token.expires_at) {
      const expiresAt = new Date(token.expires_at);
      const now = new Date();
      const daysUntilExpiry = Math.ceil((expiresAt.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
      
      if (expiresAt < now) {
        return <Badge variant="error">Expired</Badge>;
      } else if (daysUntilExpiry <= 7) {
        return <Badge variant="warning">Expires Soon</Badge>;
      }
    }
    
    if (token.is_super_admin) {
      return <Badge variant="enterprise">Super Admin</Badge>;
    }
    
    return <Badge variant="success">Active</Badge>;
  };

  const auditEntries = auditData?.audit_entries || [];
  const stats = usageStats as AdminTokenUsageStats;
  const provisionedKeysData = provisionedKeys?.provisioned_keys || [];

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <TokenSuccessModal
        isOpen={showRotateModal}
        onClose={() => setShowRotateModal(false)}
        newToken={rotatedToken}
        tokenInfo={token}
      />

      {/* Header */}
      <Card>
        <CardHeader 
          title={token.service_name} 
          subtitle={`Admin Token • ${token.token_prefix}...`}
        >
          <div className="flex gap-3">
            <Button variant="secondary" onClick={onBack}>
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
              </svg>
              Back
            </Button>
            {token.is_active && (
              <>
                <Button
                  variant="secondary"
                  onClick={handleRotate}
                  loading={rotateTokenMutation.isPending}
                >
                  🔄 Rotate Token
                </Button>
                <Button
                  variant="danger"
                  onClick={handleRevoke}
                  loading={revokeTokenMutation.isPending}
                >
                  Revoke Token
                </Button>
              </>
            )}
          </div>
        </CardHeader>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <div>
            <span className="text-sm text-pierre-gray-500">Status</span>
            <div className="mt-1">{getStatusBadge()}</div>
          </div>
          <div>
            <span className="text-sm text-pierre-gray-500">Usage Count</span>
            <div className="text-xl font-semibold text-pierre-gray-900 mt-1">
              {token.usage_count.toLocaleString()}
            </div>
          </div>
          <div>
            <span className="text-sm text-pierre-gray-500">Created</span>
            <div className="text-sm text-pierre-gray-900 mt-1">
              {format(new Date(token.created_at), 'MMM d, yyyy')}
            </div>
          </div>
          <div>
            <span className="text-sm text-pierre-gray-500">Last Used</span>
            <div className="text-sm text-pierre-gray-900 mt-1">
              {token.last_used_at 
                ? format(new Date(token.last_used_at), 'MMM d, yyyy')
                : 'Never'
              }
            </div>
          </div>
        </div>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Token Information */}
        <Card>
          <CardHeader title="Token Information" />
          <div className="space-y-4">
            <div>
              <label className="label">Service Name</label>
              <div className="text-sm text-pierre-gray-900">{token.service_name}</div>
            </div>
            
            {token.service_description && (
              <div>
                <label className="label">Description</label>
                <div className="text-sm text-pierre-gray-900">{token.service_description}</div>
              </div>
            )}

            <div>
              <label className="label">Token Prefix</label>
              <div className="text-sm font-mono text-pierre-gray-900">{token.token_prefix}...</div>
            </div>

            <div>
              <label className="label">Permissions</label>
              <div className="flex flex-wrap gap-2">
                {token.is_super_admin ? (
                  <Badge variant="enterprise">All Permissions (Super Admin)</Badge>
                ) : (
                  token.permissions.map(permission => (
                    <Badge key={permission} variant="info">
                      {permission.replace(/_/g, ' ')}
                    </Badge>
                  ))
                )}
              </div>
            </div>

            {token.expires_at && (
              <div>
                <label className="label">Expires</label>
                <div className="text-sm text-pierre-gray-900">
                  {format(new Date(token.expires_at), 'MMM d, yyyy \'at\' h:mm a')}
                </div>
              </div>
            )}
          </div>
        </Card>

        {/* Usage Statistics */}
        <Card>
          <CardHeader title="Usage Statistics" />
          {statsLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="pierre-spinner w-6 h-6" />
            </div>
          ) : stats ? (
            <div className="space-y-4">
              <div className="grid grid-cols-3 gap-4">
                <div className="text-center">
                  <div className="text-2xl font-semibold text-pierre-blue-600">
                    {stats.total_actions.toLocaleString()}
                  </div>
                  <div className="text-xs text-pierre-gray-500">Total Actions</div>
                </div>
                <div className="text-center">
                  <div className="text-2xl font-semibold text-pierre-green-600">
                    {stats.actions_last_24h.toLocaleString()}
                  </div>
                  <div className="text-xs text-pierre-gray-500">Last 24h</div>
                </div>
                <div className="text-center">
                  <div className="text-2xl font-semibold text-pierre-purple-600">
                    {stats.actions_last_7d.toLocaleString()}
                  </div>
                  <div className="text-xs text-pierre-gray-500">Last 7 days</div>
                </div>
              </div>

              {stats.most_common_actions.length > 0 && (
                <div>
                  <label className="label">Most Common Actions</label>
                  <div className="space-y-2">
                    {stats.most_common_actions.slice(0, 5).map((action, index) => (
                      <div key={index} className="flex justify-between text-sm">
                        <span className="text-pierre-gray-900">{action.action}</span>
                        <span className="text-pierre-gray-500">{action.count.toLocaleString()}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="text-center py-8 text-pierre-gray-500">
              No usage statistics available
            </div>
          )}
        </Card>
      </div>

      {/* Provisioned API Keys */}
      <Card>
        <CardHeader title={`Provisioned API Keys (${provisionedKeysData.length})`} />
        {provisionedKeysData.length === 0 ? (
          <div className="text-center py-8 text-pierre-gray-500">
            No API keys have been provisioned using this token yet.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-pierre-gray-200">
                  <th className="text-left py-3 px-4 font-medium text-pierre-gray-700">User Email</th>
                  <th className="text-left py-3 px-4 font-medium text-pierre-gray-700">Tier</th>
                  <th className="text-left py-3 px-4 font-medium text-pierre-gray-700">Status</th>
                  <th className="text-left py-3 px-4 font-medium text-pierre-gray-700">Provisioned</th>
                </tr>
              </thead>
              <tbody>
                {provisionedKeysData.slice(0, 10).map((key: ProvisionedKey) => (
                  <tr key={key.api_key_id} className="border-b border-pierre-gray-100">
                    <td className="py-3 px-4 text-sm">{key.user_email}</td>
                    <td className="py-3 px-4">
                      <Badge variant={key.requested_tier as 'trial' | 'starter' | 'professional' | 'enterprise' | 'info'}>{key.requested_tier}</Badge>
                    </td>
                    <td className="py-3 px-4">
                      <Badge variant={key.key_status === 'active' ? 'success' : 'error'}>
                        {key.key_status}
                      </Badge>
                    </td>
                    <td className="py-3 px-4 text-sm text-pierre-gray-600">
                      {format(new Date(key.created_at), 'MMM d, yyyy')}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      {/* Recent Activity */}
      <Card>
        <CardHeader title="Recent Activity" />
        {auditLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="pierre-spinner w-6 h-6" />
          </div>
        ) : auditEntries.length === 0 ? (
          <div className="text-center py-8 text-pierre-gray-500">
            No recent activity found for this token.
          </div>
        ) : (
          <div className="space-y-3">
            {auditEntries.slice(0, 20).map((entry: AdminTokenAudit) => (
              <div key={entry.id} className="flex items-start gap-3 p-3 rounded-lg bg-pierre-gray-50">
                <div className={`w-2 h-2 rounded-full mt-2 ${
                  entry.success ? 'bg-pierre-green-500' : 'bg-pierre-red-500'
                }`} />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 text-sm">
                    <span className="font-medium text-pierre-gray-900">{entry.action}</span>
                    <span className="text-pierre-gray-500">
                      {format(new Date(entry.timestamp), 'MMM d, h:mm a')}
                    </span>
                  </div>
                  {entry.target_resource && (
                    <div className="text-xs text-pierre-gray-600 mt-1">
                      Target: {entry.target_resource}
                    </div>
                  )}
                  {entry.error_message && (
                    <div className="text-xs text-pierre-red-600 mt-1">
                      Error: {entry.error_message}
                    </div>
                  )}
                  {entry.ip_address && (
                    <div className="text-xs text-pierre-gray-500 mt-1">
                      IP: {entry.ip_address}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}