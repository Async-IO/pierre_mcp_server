// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import React, { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { AdminPermission, CreateAdminTokenResponse } from '../types/api';

interface CreateApiKeyProps {
  onBack: () => void;
  onTokenCreated: (response: CreateAdminTokenResponse) => void;
}

const PERMISSION_DESCRIPTIONS: Record<AdminPermission, { label: string; description: string; danger?: boolean }> = {
  provision_keys: {
    label: 'Provision API Keys',
    description: 'Create new API keys for users and applications',
  },
  revoke_keys: {
    label: 'Revoke API Keys',
    description: 'Revoke existing API keys',
    danger: true,
  },
  list_keys: {
    label: 'List API Keys',
    description: 'View and list existing API keys',
  },
  manage_admin_tokens: {
    label: 'Manage Admin Tokens',
    description: 'Create, update, and manage admin tokens',
    danger: true,
  },
  view_audit_logs: {
    label: 'View Audit Logs',
    description: 'Access audit logs and activity history',
  },
  super_admin: {
    label: 'System Administration',
    description: 'Full system administration access',
    danger: true,
  },
};

export default function CreateApiKey({ onBack, onTokenCreated }: CreateApiKeyProps) {
  const [serviceName, setServiceName] = useState('');
  const [serviceDescription, setServiceDescription] = useState('');
  const [selectedPermissions, setSelectedPermissions] = useState<Set<AdminPermission>>(new Set(['provision_keys']));
  const [isSuperAdmin, setIsSuperAdmin] = useState(false);
  const [expiresInDays, setExpiresInDays] = useState<number | null>(365);
  const [neverExpires, setNeverExpires] = useState(false);

  const createTokenMutation = useMutation({
    mutationFn: (data: {
      service_name: string;
      service_description?: string;
      permissions?: string[];
      is_super_admin?: boolean;
      expires_in_days?: number;
    }) => apiService.createAdminToken(data),
    onSuccess: (response) => {
      onTokenCreated(response);
    },
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!serviceName.trim()) {
      return;
    }

    const tokenData = {
      service_name: serviceName.trim(),
      service_description: serviceDescription.trim() || undefined,
      permissions: isSuperAdmin ? undefined : Array.from(selectedPermissions),
      is_super_admin: isSuperAdmin,
      expires_in_days: neverExpires ? 0 : expiresInDays || undefined,
    };

    createTokenMutation.mutate(tokenData);
  };

  const handlePermissionToggle = (permission: AdminPermission) => {
    const newPermissions = new Set(selectedPermissions);
    if (newPermissions.has(permission)) {
      newPermissions.delete(permission);
    } else {
      newPermissions.add(permission);
    }
    setSelectedPermissions(newPermissions);
  };

  const handleSuperAdminToggle = (checked: boolean) => {
    setIsSuperAdmin(checked);
    if (checked) {
      setNeverExpires(true);
      setExpiresInDays(null);
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <button onClick={onBack} className="btn-secondary">
          ← Back
        </button>
        <div>
          <h2 className="text-xl font-semibold text-pierre-gray-900">Create API Token</h2>
          <p className="text-sm text-pierre-gray-600 mt-1">
            Generate a new API token for MCP clients and programmatic access
          </p>
        </div>
      </div>

      {/* Form */}
      <div className="bg-white border border-pierre-gray-200 rounded-lg p-6">
        <form onSubmit={handleSubmit} className="space-y-6">
          {createTokenMutation.error && (
            <div className="bg-pierre-red-50 border border-pierre-red-200 text-pierre-red-600 px-4 py-3 rounded">
              {createTokenMutation.error instanceof Error
                ? createTokenMutation.error.message
                : 'Failed to create API token'}
            </div>
          )}

          {/* Service Details */}
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-pierre-gray-900">Service Details</h3>
            
            <div>
              <label htmlFor="serviceName" className="block text-sm font-medium text-pierre-gray-700 mb-2">
                Service Name *
              </label>
              <input
                id="serviceName"
                type="text"
                required
                className="input-field"
                placeholder="e.g., pierre_admin_service, api_gateway"
                value={serviceName}
                onChange={(e) => setServiceName(e.target.value)}
              />
              <p className="text-xs text-pierre-gray-500 mt-1">
                A unique identifier for the service that will use this token
              </p>
            </div>

            <div>
              <label htmlFor="serviceDescription" className="block text-sm font-medium text-pierre-gray-700 mb-2">
                Description
              </label>
              <textarea
                id="serviceDescription"
                className="input-field"
                rows={3}
                placeholder="Brief description of the service and its purpose"
                value={serviceDescription}
                onChange={(e) => setServiceDescription(e.target.value)}
              />
            </div>
          </div>

          {/* Admin Level */}
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-pierre-gray-900">Admin Level</h3>
            
            <div className="space-y-3">
              <label className="flex items-start gap-3">
                <input
                  type="checkbox"
                  checked={isSuperAdmin}
                  onChange={(e) => handleSuperAdminToggle(e.target.checked)}
                  className="mt-1 rounded border-pierre-gray-300 text-pierre-blue-600 focus:ring-pierre-blue-500"
                />
                <div>
                  <div className="font-medium text-pierre-gray-900">Super Admin API Token</div>
                  <p className="text-sm text-pierre-gray-600">
                    Grants all permissions and never expires. Use with extreme caution.
                  </p>
                </div>
              </label>

              {isSuperAdmin && (
                <div className="bg-pierre-red-50 border border-pierre-red-200 rounded-lg p-4">
                  <div className="flex items-start gap-3">
                    <svg className="w-6 h-6 text-pierre-red-600 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.5 0L4.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
                    </svg>
                    <div>
                      <h4 className="font-medium text-pierre-red-800">Danger Zone</h4>
                      <p className="text-sm text-pierre-red-700 mt-1">
                        Super admin API tokens have unrestricted access to all system operations.
                        Only create these for trusted, critical services.
                      </p>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Permissions */}
          {!isSuperAdmin && (
            <div className="space-y-4">
              <h3 className="text-lg font-medium text-pierre-gray-900">Permissions</h3>
              <p className="text-sm text-pierre-gray-600">
                Select the specific permissions this token should have
              </p>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {Object.entries(PERMISSION_DESCRIPTIONS).map(([permission, info]) => (
                  <label key={permission} className="flex items-start gap-3 p-3 border border-pierre-gray-200 rounded-lg hover:bg-pierre-gray-50">
                    <input
                      type="checkbox"
                      checked={selectedPermissions.has(permission as AdminPermission)}
                      onChange={() => handlePermissionToggle(permission as AdminPermission)}
                      className="mt-1 rounded border-pierre-gray-300 text-pierre-blue-600 focus:ring-pierre-blue-500"
                    />
                    <div className="flex-1">
                      <div className={`font-medium ${info.danger ? 'text-pierre-red-800' : 'text-pierre-gray-900'}`}>
                        {info.label}
                        {info.danger && (
                          <span className="ml-2 text-xs bg-pierre-red-100 text-pierre-red-700 px-1.5 py-0.5 rounded">
                            High Risk
                          </span>
                        )}
                      </div>
                      <p className="text-sm text-pierre-gray-600 mt-1">
                        {info.description}
                      </p>
                    </div>
                  </label>
                ))}
              </div>

              {selectedPermissions.size === 0 && (
                <div className="bg-pierre-yellow-50 border border-pierre-yellow-200 rounded-lg p-3">
                  <p className="text-sm text-pierre-yellow-800">
                    ⚠️ At least one permission must be selected for the token to be useful.
                  </p>
                </div>
              )}
            </div>
          )}

          {/* Expiration */}
          {!isSuperAdmin && (
            <div className="space-y-4">
              <h3 className="text-lg font-medium text-pierre-gray-900">Expiration</h3>
              
              <div className="space-y-3">
                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={neverExpires}
                    onChange={(e) => {
                      setNeverExpires(e.target.checked);
                      if (e.target.checked) {
                        setExpiresInDays(null);
                      } else {
                        setExpiresInDays(365);
                      }
                    }}
                    className="rounded border-pierre-gray-300 text-pierre-blue-600 focus:ring-pierre-blue-500"
                  />
                  <span className="font-medium text-pierre-gray-900">Never expires</span>
                </label>

                {!neverExpires && (
                  <div>
                    <label htmlFor="expiresInDays" className="block text-sm font-medium text-pierre-gray-700 mb-2">
                      Expires in (days)
                    </label>
                    <input
                      id="expiresInDays"
                      type="number"
                      min="1"
                      max="3650"
                      className="input-field w-32"
                      value={expiresInDays || ''}
                      onChange={(e) => setExpiresInDays(e.target.value ? parseInt(e.target.value) : null)}
                    />
                    <p className="text-xs text-pierre-gray-500 mt-1">
                      Recommended: 365 days (1 year) for production services
                    </p>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="flex items-center gap-3 pt-4 border-t border-pierre-gray-200">
            <button
              type="submit"
              disabled={
                createTokenMutation.isPending || 
                !serviceName.trim() || 
                (!isSuperAdmin && selectedPermissions.size === 0)
              }
              className="btn-primary disabled:opacity-50"
            >
              {createTokenMutation.isPending ? 'Creating API Token...' : 'Create API Token'}
            </button>
            <button
              type="button"
              onClick={onBack}
              disabled={createTokenMutation.isPending}
              className="btn-secondary"
            >
              Cancel
            </button>
          </div>
        </form>
      </div>

      {/* Security Reminder */}
      <div className="bg-pierre-blue-50 border border-pierre-blue-200 rounded-lg p-4">
        <h4 className="font-medium text-pierre-blue-900 mb-2">🔒 Security Reminder</h4>
        <ul className="text-sm text-pierre-blue-800 space-y-1">
          <li>• The API token will be shown only once after creation</li>
          <li>• Store the token securely in your environment</li>
          <li>• Never commit API tokens to version control</li>
          <li>• Use HTTPS when transmitting API tokens</li>
          <li>• Regularly rotate tokens for better security</li>
        </ul>
      </div>
    </div>
  );
}