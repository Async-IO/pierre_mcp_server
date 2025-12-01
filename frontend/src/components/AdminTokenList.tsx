// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { format } from 'date-fns';
import { Button, Card, CardHeader, Badge, StatusFilter, ConfirmDialog } from './ui';
import type { StatusFilterValue } from './ui';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import type { AdminToken } from '../types/api';

interface AdminTokenListProps {
  onViewDetails: (tokenId: string) => void;
}

export default function AdminTokenList({ onViewDetails }: AdminTokenListProps) {
  const { isAuthenticated, user } = useAuth();
  const queryClient = useQueryClient();
  const [selectedTokens, setSelectedTokens] = useState<Set<string>>(new Set());
  const [statusFilter, setStatusFilter] = useState<StatusFilterValue>('active');
  const [tokenToRevoke, setTokenToRevoke] = useState<AdminToken | null>(null);
  const [tokensToRevoke, setTokensToRevoke] = useState<Set<string> | null>(null);

  // Always fetch all tokens and filter client-side for accurate counts
  const { data: tokensResponse, isLoading, error } = useQuery({
    queryKey: ['admin-tokens', true],
    queryFn: () => apiService.getAdminTokens({ include_inactive: true }),
    enabled: isAuthenticated && user?.is_admin === true,
  });

  const revokeTokenMutation = useMutation({
    mutationFn: (tokenId: string) => apiService.revokeAdminToken(tokenId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-tokens'] });
      setSelectedTokens(new Set());
      setTokenToRevoke(null);
      setTokensToRevoke(null);
    },
  });

  const allTokens: AdminToken[] = useMemo(
    () => tokensResponse?.admin_tokens || [],
    [tokensResponse?.admin_tokens]
  );

  // Compute counts for the filter
  const activeCount = useMemo(() => allTokens.filter(t => t.is_active).length, [allTokens]);
  const inactiveCount = useMemo(() => allTokens.filter(t => !t.is_active).length, [allTokens]);

  // Filter tokens based on status filter
  const tokens = useMemo(() => {
    switch (statusFilter) {
      case 'active':
        return allTokens.filter(t => t.is_active);
      case 'inactive':
        return allTokens.filter(t => !t.is_active);
      case 'all':
      default:
        return allTokens;
    }
  }, [allTokens, statusFilter]);

  const handleSelectToken = (tokenId: string) => {
    const newSelected = new Set(selectedTokens);
    if (newSelected.has(tokenId)) {
      newSelected.delete(tokenId);
    } else {
      newSelected.add(tokenId);
    }
    setSelectedTokens(newSelected);
  };

  const handleSelectAll = () => {
    if (selectedTokens.size === tokens.length) {
      setSelectedTokens(new Set());
    } else {
      setSelectedTokens(new Set(tokens.map(t => t.id)));
    }
  };

  const handleBulkRevoke = () => {
    if (selectedTokens.size === 0) return;
    setTokensToRevoke(new Set(selectedTokens));
  };

  const confirmBulkRevoke = async () => {
    if (!tokensToRevoke) return;

    for (const tokenId of tokensToRevoke) {
      try {
        await revokeTokenMutation.mutateAsync(tokenId);
      } catch (error) {
        console.error(`Failed to revoke token ${tokenId}:`, error);
      }
    }
  };

  const handleSingleRevoke = (token: AdminToken) => {
    setTokenToRevoke(token);
  };

  const confirmSingleRevoke = () => {
    if (tokenToRevoke) {
      revokeTokenMutation.mutate(tokenToRevoke.id);
    }
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="pierre-spinner w-8 h-8"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-6">
        <div className="flex items-center gap-3">
          <svg className="w-6 h-6 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <div>
            <h3 className="text-lg font-medium text-red-900">Failed to load admin tokens</h3>
            <p className="text-red-700 mt-1">
              {error instanceof Error ? error.message : 'An unknown error occurred'}
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Main Card */}
      <Card>
        <CardHeader
          title="Your Admin Tokens"
          subtitle={`${allTokens.length} total tokens`}
        />

        {/* Status Filter */}
        <div className="px-6 pb-4">
          <div className="flex items-center justify-between">
            <StatusFilter
              value={statusFilter}
              onChange={setStatusFilter}
              activeCount={activeCount}
              inactiveCount={inactiveCount}
              totalCount={allTokens.length}
            />

            {selectedTokens.size > 0 && (
              <div className="flex items-center gap-2">
                <span className="text-sm text-pierre-gray-600">
                  {selectedTokens.size} selected
                </span>
                <Button
                  onClick={handleBulkRevoke}
                  disabled={revokeTokenMutation.isPending}
                  variant="secondary"
                  className="text-red-600 hover:bg-red-50"
                  size="sm"
                >
                  Revoke Selected
                </Button>
              </div>
            )}
          </div>
        </div>

        {/* Token List */}
        {tokens.length === 0 ? (
          <div className="text-center py-8 text-pierre-gray-500 px-6 pb-6">
            <div className="text-4xl mb-4">🔐</div>
            <p className="text-lg mb-2">No admin tokens yet</p>
            <p>Create your first admin token to enable programmatic access</p>
          </div>
        ) : (
          <div className="space-y-4 px-6 pb-6">
            {/* Select All Header */}
            <div className="flex items-center gap-3 p-4 bg-pierre-gray-50 rounded-lg">
              <input
                type="checkbox"
                checked={selectedTokens.size === tokens.length && tokens.length > 0}
                onChange={handleSelectAll}
                className="rounded border-pierre-gray-300 text-pierre-blue-600 focus:ring-pierre-blue-500"
              />
              <span className="text-sm font-medium text-pierre-gray-700">
                Select All ({tokens.length})
              </span>
            </div>

            {/* Token Cards */}
            {tokens.map((token: AdminToken) => (
            <Card key={token.id} className="hover:shadow-md transition-shadow p-4">
              <div className="flex items-start gap-4">
                  <input
                    type="checkbox"
                    checked={selectedTokens.has(token.id)}
                    onChange={() => handleSelectToken(token.id)}
                    className="mt-1 rounded border-pierre-gray-300 text-pierre-blue-600 focus:ring-pierre-blue-500"
                  />
                  
                  <div className="flex-1">
                    <div className="flex items-start justify-between">
                      <div>
                        <h3 className="text-lg font-medium text-pierre-gray-900">
                          {token.service_name}
                        </h3>
                        {/* GitHub-style token prefix display */}
                        {token.token_prefix && (
                          <code className="inline-flex items-center gap-1 mt-1 px-2 py-0.5 bg-pierre-gray-100 text-pierre-gray-700 text-xs font-mono rounded border border-pierre-gray-200">
                            <svg className="w-3 h-3 text-pierre-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
                            </svg>
                            {token.token_prefix}...
                          </code>
                        )}
                        {token.service_description && (
                          <p className="text-sm text-pierre-gray-600 mt-1">
                            {token.service_description}
                          </p>
                        )}
                        <div className="flex items-center gap-2 mt-2">
                          <Badge variant={token.is_active ? 'success' : 'info'}>
                            {token.is_active ? 'Active' : 'Inactive'}
                          </Badge>
                          {token.is_super_admin && (
                            <Badge variant="warning">Super Admin</Badge>
                          )}
                        </div>
                      </div>
                      
                      <div className="flex items-center gap-2">
                        <Button
                          onClick={() => onViewDetails(token.id)}
                          className="btn-secondary text-sm"
                        >
                          View Details
                        </Button>
                        {token.is_active && (
                          <Button
                            onClick={() => handleSingleRevoke(token)}
                            disabled={revokeTokenMutation.isPending}
                            variant="secondary"
                            className="text-red-600 hover:bg-red-50 text-sm"
                          >
                            Revoke
                          </Button>
                        )}
                      </div>
                    </div>
                    
                    <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                      <div>
                        <span className="text-pierre-gray-500">Created:</span>
                        <p className="font-medium">{format(new Date(token.created_at), 'MMM d, yyyy')}</p>
                      </div>
                      <div>
                        <span className="text-pierre-gray-500">Expires:</span>
                        <p className="font-medium">
                          {token.expires_at ? format(new Date(token.expires_at), 'MMM d, yyyy') : 'Never'}
                        </p>
                      </div>
                      <div>
                        <span className="text-pierre-gray-500">Usage:</span>
                        <p className="font-medium">{token.usage_count} requests</p>
                      </div>
                      <div>
                        <span className="text-pierre-gray-500">Last Used:</span>
                        <p className="font-medium">
                          {token.last_used_at ? format(new Date(token.last_used_at), 'MMM d, yyyy') : 'Never'}
                        </p>
                      </div>
                    </div>
                    
                    {token.permissions && token.permissions.length > 0 && (
                      <div className="mt-3">
                        <span className="text-sm text-pierre-gray-500">Permissions:</span>
                        <div className="flex flex-wrap gap-1 mt-1">
                          {token.permissions.map((permission) => (
                            <Badge key={permission} variant="info" className="text-xs">
                              {permission}
                            </Badge>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                </div>
            </Card>
          ))}
        </div>
      )}
      </Card>

      {/* Single Token Revoke Confirmation */}
      <ConfirmDialog
        isOpen={tokenToRevoke !== null}
        onClose={() => setTokenToRevoke(null)}
        onConfirm={confirmSingleRevoke}
        title="Revoke Token"
        message={`Are you sure you want to revoke "${tokenToRevoke?.service_name}"? This action cannot be undone and any services using this token will lose access.`}
        confirmLabel="Revoke Token"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={revokeTokenMutation.isPending}
      />

      {/* Bulk Revoke Confirmation */}
      <ConfirmDialog
        isOpen={tokensToRevoke !== null}
        onClose={() => setTokensToRevoke(null)}
        onConfirm={confirmBulkRevoke}
        title="Revoke Multiple Tokens"
        message={`Are you sure you want to revoke ${tokensToRevoke?.size || 0} token(s)? This action cannot be undone and any services using these tokens will lose access.`}
        confirmLabel={`Revoke ${tokensToRevoke?.size || 0} Token(s)`}
        cancelLabel="Cancel"
        variant="danger"
        isLoading={revokeTokenMutation.isPending}
      />
    </div>
  );
}