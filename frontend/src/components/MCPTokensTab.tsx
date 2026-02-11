// ABOUTME: MCP Tokens management tab for user dashboard
// ABOUTME: Allows users to create/revoke tokens and view setup instructions for AI clients
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { format } from 'date-fns';
import { Button, Card, CardHeader, Badge, ConfirmDialog } from './ui';
import { useAuth } from '../hooks/useAuth';
import { userApi } from '../services/api';
import A2AClientList from './A2AClientList';
import CreateA2AClient from './CreateA2AClient';
import { QUERY_KEYS } from '../constants/queryKeys';

interface McpToken {
  id: string;
  name: string;
  token_prefix: string;
  expires_at: string | null;
  last_used_at: string | null;
  usage_count: number;
  is_revoked: boolean;
  created_at: string;
}

export default function MCPTokensTab() {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [tokenToRevoke, setTokenToRevoke] = useState<McpToken | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newTokenName, setNewTokenName] = useState('');
  const [expiresInDays, setExpiresInDays] = useState<number | undefined>(undefined);
  const [createdToken, setCreatedToken] = useState<{
    token_value: string;
    name: string;
  } | null>(null);
  const [copied, setCopied] = useState(false);
  const [showCreateA2AClient, setShowCreateA2AClient] = useState(false);
  const [showSetupInstructions, setShowSetupInstructions] = useState(false);

  const { data: tokensResponse, isLoading, error } = useQuery({
    queryKey: QUERY_KEYS.mcpTokens.list(),
    queryFn: () => userApi.getMcpTokens(),
    enabled: isAuthenticated,
  });

  const createTokenMutation = useMutation({
    mutationFn: (data: { name: string; expires_in_days?: number }) =>
      userApi.createMcpToken(data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.mcpTokens.list() });
      setCreatedToken({ token_value: data.token_value ?? '', name: data.name });
      setShowCreateForm(false);
      setNewTokenName('');
      setExpiresInDays(undefined);
    },
  });

  const revokeTokenMutation = useMutation({
    mutationFn: (tokenId: string) => userApi.revokeMcpToken(tokenId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.mcpTokens.list() });
      setTokenToRevoke(null);
    },
  });

  const tokens: McpToken[] = tokensResponse?.tokens || [];
  const activeTokens = tokens.filter((t) => !t.is_revoked);

  const handleCreateToken = () => {
    if (!newTokenName.trim()) return;
    createTokenMutation.mutate({
      name: newTokenName.trim(),
      expires_in_days: expiresInDays,
    });
  };

  const copyToClipboard = async (text: string) => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
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
      <div className="bg-pierre-red-50 border border-pierre-red-200 rounded-lg p-6">
        <div className="flex items-center gap-3">
          <svg className="w-6 h-6 text-pierre-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <div>
            <h3 className="text-lg font-medium text-pierre-red-900">Failed to load tokens</h3>
            <p className="text-pierre-red-700 mt-1">
              {error instanceof Error ? error.message : 'An unknown error occurred'}
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Created Token Display */}
      {createdToken && (
        <div className="bg-pierre-green-50 border border-pierre-green-200 rounded-lg p-6">
          <div className="flex items-start gap-3">
            <svg className="w-6 h-6 text-pierre-green-600 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <div className="flex-1">
              <h3 className="text-lg font-medium text-pierre-green-900">Token Created: {createdToken.name}</h3>
              <p className="text-pierre-green-700 mt-1 mb-3">
                Copy this token now. You won&apos;t be able to see it again!
              </p>
              <div className="flex items-center gap-2">
                <code className="flex-1 px-3 py-2 bg-white border border-pierre-green-300 rounded font-mono text-sm break-all">
                  {createdToken.token_value}
                </code>
                <Button
                  onClick={() => copyToClipboard(createdToken.token_value)}
                  variant="secondary"
                  size="sm"
                >
                  {copied ? 'Copied!' : 'Copy'}
                </Button>
              </div>
              <Button
                onClick={() => setCreatedToken(null)}
                variant="secondary"
                size="sm"
                className="mt-3"
              >
                Dismiss
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Main Card */}
      <Card>
        <CardHeader
          title="Tokens"
          subtitle={`${activeTokens.length} active tokens for AI client connections`}
        />

        {/* Create Token Section */}
        <div className="px-6 pb-4">
          {!showCreateForm ? (
            <Button onClick={() => setShowCreateForm(true)} variant="primary">
              Create New Token
            </Button>
          ) : (
            <div className="bg-pierre-gray-50 rounded-lg p-4 space-y-4">
              <h4 className="font-medium text-pierre-gray-900">Create Token</h4>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Token Name
                  </label>
                  <input
                    type="text"
                    value={newTokenName}
                    onChange={(e) => setNewTokenName(e.target.value)}
                    placeholder="e.g., Claude Desktop, Cursor IDE"
                    className="w-full px-3 py-2 border border-pierre-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-pierre-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Expires In (days)
                  </label>
                  <select
                    value={expiresInDays || ''}
                    onChange={(e) => setExpiresInDays(e.target.value ? Number(e.target.value) : undefined)}
                    className="w-full px-3 py-2 border border-pierre-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-pierre-blue-500"
                  >
                    <option value="">Never expires</option>
                    <option value="30">30 days</option>
                    <option value="90">90 days</option>
                    <option value="180">180 days</option>
                    <option value="365">1 year</option>
                  </select>
                </div>
              </div>
              <div className="flex gap-2">
                <Button
                  onClick={handleCreateToken}
                  disabled={!newTokenName.trim() || createTokenMutation.isPending}
                  variant="primary"
                >
                  {createTokenMutation.isPending ? 'Creating...' : 'Create Token'}
                </Button>
                <Button onClick={() => setShowCreateForm(false)} variant="secondary">
                  Cancel
                </Button>
              </div>
            </div>
          )}
        </div>

        {/* Token List */}
        {tokens.length === 0 ? (
          <div className="text-center py-8 text-pierre-gray-500 px-6 pb-6">
            <div className="text-4xl mb-4">🔑</div>
            <p className="text-lg mb-2">No tokens yet</p>
            <p>Create a token to connect AI clients like Claude Desktop or Cursor to Pierre</p>
          </div>
        ) : (
          <div className="space-y-4 px-6 pb-6">
            {tokens.map((token) => (
              <Card key={token.id} className="hover:shadow-md transition-shadow p-4">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <h3 className="text-lg font-medium text-pierre-gray-900">{token.name}</h3>
                      <Badge variant={token.is_revoked ? 'info' : 'success'}>
                        {token.is_revoked ? 'Revoked' : 'Active'}
                      </Badge>
                    </div>
                    <code className="inline-flex items-center gap-1 mt-1 px-2 py-0.5 bg-pierre-gray-100 text-pierre-gray-700 text-xs font-mono rounded border border-pierre-gray-200">
                      <svg className="w-3 h-3 text-pierre-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
                      </svg>
                      {token.token_prefix}...
                    </code>

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
                  </div>

                  {!token.is_revoked && (
                    <Button
                      onClick={() => setTokenToRevoke(token)}
                      disabled={revokeTokenMutation.isPending}
                      variant="secondary"
                      className="text-pierre-red-600 hover:bg-pierre-red-50"
                      size="sm"
                    >
                      Revoke
                    </Button>
                  )}
                </div>
              </Card>
            ))}
          </div>
        )}

        {/* Setup Instructions - Collapsible */}
        <div className="border-t border-pierre-gray-200 px-6 py-4">
          <button
            onClick={() => setShowSetupInstructions(!showSetupInstructions)}
            className="flex items-center justify-between w-full text-left"
          >
            <div className="flex items-center gap-2">
              <svg className="w-5 h-5 text-pierre-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span className="font-medium text-pierre-gray-900">Setup Instructions</span>
              <span className="text-sm text-pierre-gray-500">for Claude & ChatGPT</span>
            </div>
            <svg
              className={`w-5 h-5 text-pierre-gray-400 transition-transform ${showSetupInstructions ? 'rotate-180' : ''}`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </button>

          {showSetupInstructions && (
            <div className="mt-4 space-y-4">
              <div className="bg-pierre-gray-50 rounded-lg p-4">
                <h4 className="font-medium text-pierre-gray-900 mb-2">Claude Desktop</h4>
                <p className="text-sm text-pierre-gray-600 mb-3">
                  Add the following to your Claude Desktop config file:
                </p>
                <pre className="text-xs bg-pierre-gray-800 text-pierre-gray-100 p-3 rounded overflow-x-auto">
{`{
  "mcpServers": {
    "pierre": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-client"],
      "env": {
        "MCP_SERVER_URL": "${window.location.origin}/mcp",
        "MCP_TOKEN": "<your-token-here>"
      }
    }
  }
}`}
                </pre>
              </div>

              <div className="bg-pierre-gray-50 rounded-lg p-4">
                <h4 className="font-medium text-pierre-gray-900 mb-2">ChatGPT</h4>
                <p className="text-sm text-pierre-gray-600 mb-3">
                  Configure in ChatGPT MCP settings:
                </p>
                <pre className="text-xs bg-pierre-gray-800 text-pierre-gray-100 p-3 rounded overflow-x-auto">
{`Server URL: ${window.location.origin}/mcp
Authorization: Bearer <your-token-here>`}
                </pre>
              </div>
            </div>
          )}
        </div>
      </Card>

      {/* Connected Apps Section */}
      <Card>
        <CardHeader
          title="Connected Apps"
          subtitle="Third-party applications authorized to access your fitness data via OAuth"
        />
        <div className="px-6 pb-6">
          {showCreateA2AClient ? (
            <CreateA2AClient
              onSuccess={() => setShowCreateA2AClient(false)}
              onCancel={() => setShowCreateA2AClient(false)}
            />
          ) : (
            <A2AClientList onCreateClient={() => setShowCreateA2AClient(true)} />
          )}
        </div>
      </Card>

      {/* Revoke Confirmation */}
      <ConfirmDialog
        isOpen={tokenToRevoke !== null}
        onClose={() => setTokenToRevoke(null)}
        onConfirm={() => tokenToRevoke && revokeTokenMutation.mutate(tokenToRevoke.id)}
        title="Revoke Token"
        message={`Are you sure you want to revoke "${tokenToRevoke?.name}"? Any AI clients using this token will lose access immediately.`}
        confirmLabel="Revoke Token"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={revokeTokenMutation.isPending}
      />
    </div>
  );
}
