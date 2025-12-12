// ABOUTME: Admin configuration management UI for runtime parameters
// ABOUTME: Allows admins to view, modify, and reset server configuration values
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card, Badge, Input, Button, Modal, Tabs } from './ui';

interface ConfigParameter {
  key: string;
  display_name: string;
  description: string;
  category: string;
  data_type: string;
  current_value: unknown;
  default_value: unknown;
  is_modified: boolean;
  valid_range?: { min?: number; max?: number; step?: number };
  enum_options?: string[];
  units?: string;
  scientific_basis?: string;
  env_variable?: string;
  is_runtime_configurable: boolean;
  requires_restart: boolean;
}

interface ConfigCategory {
  id: string;
  name: string;
  display_name: string;
  description: string;
  display_order: number;
  icon?: string;
  is_active: boolean;
  parameters: ConfigParameter[];
}

interface AuditEntry {
  id: string;
  timestamp: string;
  admin_user_id: string;
  admin_email: string;
  category: string;
  config_key: string;
  old_value?: unknown;
  new_value: unknown;
  data_type: string;
  reason?: string;
}

export default function AdminConfiguration() {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<'parameters' | 'history'>('parameters');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [pendingChanges, setPendingChanges] = useState<Record<string, unknown>>({});
  const [changeReason, setChangeReason] = useState('');
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [showResetModal, setShowResetModal] = useState(false);
  const [resetTarget, setResetTarget] = useState<{ category?: string; key?: string } | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // Fetch configuration catalog
  const { data: catalogData, isLoading, error } = useQuery({
    queryKey: ['admin-config-catalog'],
    queryFn: () => apiService.getConfigCatalog(),
    retry: 1,
  });

  // Fetch audit history
  const { data: auditData, isLoading: auditLoading } = useQuery({
    queryKey: ['admin-config-audit'],
    queryFn: () => apiService.getConfigAuditLog({ limit: 50 }),
    enabled: activeTab === 'history',
  });

  // Update configuration mutation
  const updateMutation = useMutation({
    mutationFn: ({ parameters, reason }: { parameters: Record<string, unknown>; reason?: string }) =>
      apiService.updateConfig({ parameters, reason }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-config-catalog'] });
      queryClient.invalidateQueries({ queryKey: ['admin-config-audit'] });
      setPendingChanges({});
      setChangeReason('');
      setShowConfirmModal(false);
    },
  });

  // Reset configuration mutation
  const resetMutation = useMutation({
    mutationFn: ({ category, keys }: { category?: string; keys?: string[] }) =>
      apiService.resetConfig({ category, parameters: keys }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-config-catalog'] });
      queryClient.invalidateQueries({ queryKey: ['admin-config-audit'] });
      setShowResetModal(false);
      setResetTarget(null);
    },
  });

  // Get categories from catalog data
  const categories = useMemo(() => {
    if (!catalogData?.data?.categories) return [];
    return [...catalogData.data.categories].sort((a, b) => a.display_order - b.display_order);
  }, [catalogData]);

  // Filter categories and parameters based on search query
  const filteredCategories = useMemo(() => {
    if (!searchQuery.trim()) return categories;
    const query = searchQuery.toLowerCase();
    return categories
      .map((cat) => ({
        ...cat,
        parameters: cat.parameters.filter(
          (p: ConfigParameter) =>
            p.display_name.toLowerCase().includes(query) ||
            p.key.toLowerCase().includes(query) ||
            p.description.toLowerCase().includes(query)
        ),
      }))
      .filter((cat) => cat.parameters.length > 0);
  }, [categories, searchQuery]);

  // Get current category parameters
  const currentCategory = useMemo(() => {
    const cats = searchQuery ? filteredCategories : categories;
    if (!selectedCategory) return cats[0] || null;
    return cats.find((c) => c.name === selectedCategory) || cats[0] || null;
  }, [categories, filteredCategories, searchQuery, selectedCategory]);

  // Check if there are pending changes
  const hasPendingChanges = Object.keys(pendingChanges).length > 0;

  // Handle parameter value change
  const handleValueChange = (key: string, value: unknown, originalValue: unknown) => {
    if (JSON.stringify(value) === JSON.stringify(originalValue)) {
      // Remove from pending if value is reset to original
      const newPending = { ...pendingChanges };
      delete newPending[key];
      setPendingChanges(newPending);
    } else {
      setPendingChanges({ ...pendingChanges, [key]: value });
    }
  };

  // Handle save changes
  const handleSaveChanges = () => {
    if (hasPendingChanges) {
      setShowConfirmModal(true);
    }
  };

  // Confirm and apply changes
  const confirmChanges = () => {
    updateMutation.mutate({
      parameters: pendingChanges,
      reason: changeReason || undefined,
    });
  };

  // Handle reset
  const handleReset = (category?: string, key?: string) => {
    setResetTarget({ category, key });
    setShowResetModal(true);
  };

  // Confirm reset
  const confirmReset = () => {
    if (resetTarget) {
      resetMutation.mutate({
        category: resetTarget.category,
        keys: resetTarget.key ? [resetTarget.key] : undefined,
      });
    }
  };

  // Get effective value (pending change or current)
  const getEffectiveValue = (param: ConfigParameter) => {
    if (param.key in pendingChanges) {
      return pendingChanges[param.key];
    }
    return param.current_value;
  };

  // Render parameter input based on data type
  const renderParameterInput = (param: ConfigParameter) => {
    const effectiveValue = getEffectiveValue(param);
    const isModified = param.key in pendingChanges;

    switch (param.data_type) {
      case 'boolean':
        return (
          <button
            onClick={() => handleValueChange(param.key, !effectiveValue, param.current_value)}
            disabled={!param.is_runtime_configurable}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-offset-2 ${
              effectiveValue ? 'bg-pierre-activity' : 'bg-pierre-gray-300'
            } ${!param.is_runtime_configurable ? 'opacity-50 cursor-not-allowed' : ''}`}
            role="switch"
            aria-checked={Boolean(effectiveValue)}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow-sm ${
                effectiveValue ? 'translate-x-6' : 'translate-x-1'
              }`}
            />
          </button>
        );

      case 'integer':
      case 'float':
        return (
          <div className="flex items-center gap-2">
            <Input
              type="number"
              value={String(effectiveValue ?? '')}
              onChange={(e) => {
                const val = param.data_type === 'integer'
                  ? parseInt(e.target.value, 10)
                  : parseFloat(e.target.value);
                if (!isNaN(val)) {
                  handleValueChange(param.key, val, param.current_value);
                }
              }}
              min={param.valid_range?.min as number}
              max={param.valid_range?.max as number}
              step={param.valid_range?.step || (param.data_type === 'integer' ? 1 : 0.1)}
              disabled={!param.is_runtime_configurable}
              className={`w-32 ${isModified ? 'border-pierre-violet ring-1 ring-pierre-violet' : ''}`}
            />
            {param.units && (
              <span className="text-sm text-pierre-gray-500">{param.units}</span>
            )}
          </div>
        );

      case 'enum':
        return (
          <select
            value={String(effectiveValue)}
            onChange={(e) => handleValueChange(param.key, e.target.value, param.current_value)}
            disabled={!param.is_runtime_configurable}
            className={`px-3 py-2 border border-pierre-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-pierre-violet ${
              isModified ? 'border-pierre-violet ring-1 ring-pierre-violet' : ''
            } ${!param.is_runtime_configurable ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            {param.enum_options?.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        );

      case 'string':
      default:
        return (
          <Input
            type="text"
            value={String(effectiveValue ?? '')}
            onChange={(e) => handleValueChange(param.key, e.target.value, param.current_value)}
            disabled={!param.is_runtime_configurable}
            className={`w-64 ${isModified ? 'border-pierre-violet ring-1 ring-pierre-violet' : ''}`}
          />
        );
    }
  };

  // Format value for display
  const formatValue = (value: unknown): string => {
    if (value === null || value === undefined) return 'null';
    if (typeof value === 'boolean') return value ? 'true' : 'false';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-pierre-violet" />
      </div>
    );
  }

  if (error) {
    return (
      <Card className="border-red-200">
        <div className="text-center py-8">
          <svg className="w-12 h-12 text-red-400 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <p className="text-red-600">Failed to load configuration catalog.</p>
          <p className="text-sm text-pierre-gray-500 mt-2">Please check your permissions and try again.</p>
        </div>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with stats */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-pierre-gray-900">Configuration Management</h1>
          <p className="text-sm text-pierre-gray-500 mt-1">
            {catalogData?.data?.total_parameters ?? 0} parameters &bull;{' '}
            {categories.length} categories
          </p>
        </div>
        {hasPendingChanges && (
          <div className="flex items-center gap-3">
            <Badge variant="warning">{Object.keys(pendingChanges).length} unsaved changes</Badge>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPendingChanges({})}
            >
              Discard All
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={handleSaveChanges}
            >
              Review &amp; Save Changes
            </Button>
          </div>
        )}
      </div>

      {/* Tabs */}
      <Tabs
        tabs={[
          { id: 'parameters', label: 'Parameters' },
          { id: 'history', label: 'Change History' },
        ]}
        activeTab={activeTab}
        onChange={(id: string) => setActiveTab(id as 'parameters' | 'history')}
      />

      {activeTab === 'parameters' ? (
        <>
          {/* Search input */}
          <div className="relative">
            <Input
              type="text"
              placeholder="Search parameters"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full max-w-md"
            />
            {searchQuery && (
              <button
                aria-label="Clear search"
                onClick={() => setSearchQuery('')}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-pierre-gray-400 hover:text-pierre-gray-600"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            )}
          </div>

          {filteredCategories.length === 0 ? (
            <Card className="text-center py-8">
              <p className="text-pierre-gray-500">No parameters found</p>
            </Card>
          ) : (
          <div className="grid grid-cols-12 gap-6">
            {/* Category sidebar */}
            <div className="col-span-3">
              <Card className="sticky top-4">
                <h3 className="font-semibold text-pierre-gray-900 mb-3">Categories</h3>
                <nav className="space-y-1">
                  {filteredCategories.map((cat: ConfigCategory) => (
                    <button
                      key={cat.name}
                      onClick={() => setSelectedCategory(cat.name)}
                      className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors ${
                        (currentCategory?.name === cat.name)
                          ? 'bg-pierre-violet text-white'
                          : 'text-pierre-gray-600 hover:bg-pierre-gray-100'
                      }`}
                    >
                      <div className="font-medium">{cat.display_name}</div>
                      <div className={`text-xs ${currentCategory?.name === cat.name ? 'text-pierre-gray-200' : 'text-pierre-gray-400'}`}>
                        {cat.parameters.length} parameters
                      </div>
                    </button>
                  ))}
                </nav>
              </Card>
            </div>

          {/* Parameters list */}
          <div className="col-span-9 space-y-4">
            {currentCategory && (
              <>
                <Card>
                  <div className="flex items-center justify-between mb-4">
                    <div>
                      <h2 className="text-lg font-semibold text-pierre-gray-900">
                        {currentCategory.display_name}
                      </h2>
                      <p className="text-sm text-pierre-gray-500">{currentCategory.description}</p>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleReset(currentCategory.name)}
                    >
                      Reset Category
                    </Button>
                  </div>

                  <div className="divide-y divide-pierre-gray-100">
                    {currentCategory.parameters.map((param: ConfigParameter) => (
                      <div key={param.key} className="py-4">
                        <div className="flex items-start justify-between">
                          <div className="flex-1 mr-4">
                            <div className="flex items-center gap-2">
                              <h4 className="font-medium text-pierre-gray-900">
                                {param.display_name}
                              </h4>
                              {param.is_modified && (
                                <Badge variant="warning">Modified</Badge>
                              )}
                              {param.requires_restart && (
                                <Badge variant="destructive">Requires Restart</Badge>
                              )}
                              {!param.is_runtime_configurable && (
                                <Badge variant="secondary">Static</Badge>
                              )}
                            </div>
                            <p className="text-sm text-pierre-gray-600 mt-1">
                              {param.description}
                            </p>
                            <div className="flex items-center gap-4 mt-2 text-xs text-pierre-gray-400">
                              <span>Key: <code className="bg-pierre-gray-100 px-1 rounded">{param.key}</code></span>
                              <span>Default: <code className="bg-pierre-gray-100 px-1 rounded">{formatValue(param.default_value)}</code></span>
                              {param.valid_range && (
                                <span>Range: {param.valid_range.min} - {param.valid_range.max}</span>
                              )}
                              {param.env_variable && (
                                <span>Env: <code className="bg-pierre-gray-100 px-1 rounded">{param.env_variable}</code></span>
                              )}
                            </div>
                            {param.scientific_basis && (
                              <p className="text-xs text-pierre-gray-400 mt-1 italic">
                                Basis: {param.scientific_basis}
                              </p>
                            )}
                          </div>
                          <div className="flex items-center gap-2">
                            {renderParameterInput(param)}
                            {param.is_modified && (
                              <button
                                onClick={() => handleReset(undefined, param.key)}
                                className="p-1 text-pierre-gray-400 hover:text-pierre-gray-600"
                                title="Reset to default"
                              >
                                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                </svg>
                              </button>
                            )}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </Card>
              </>
            )}
          </div>
        </div>
          )}
        </>
      ) : (
        /* History tab */
        <Card>
          <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Change History</h2>
          {auditLoading ? (
            <div className="flex justify-center py-8">
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-pierre-violet" />
            </div>
          ) : auditData?.data?.entries?.length === 0 ? (
            <p className="text-center text-pierre-gray-500 py-8">No configuration changes recorded yet.</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-pierre-gray-200">
                    <th className="text-left py-2 px-3 font-medium text-pierre-gray-600">Timestamp</th>
                    <th className="text-left py-2 px-3 font-medium text-pierre-gray-600">Admin</th>
                    <th className="text-left py-2 px-3 font-medium text-pierre-gray-600">Parameter</th>
                    <th className="text-left py-2 px-3 font-medium text-pierre-gray-600">Old Value</th>
                    <th className="text-left py-2 px-3 font-medium text-pierre-gray-600">New Value</th>
                    <th className="text-left py-2 px-3 font-medium text-pierre-gray-600">Reason</th>
                  </tr>
                </thead>
                <tbody>
                  {auditData?.data?.entries?.map((entry: AuditEntry) => (
                    <tr key={entry.id} className="border-b border-pierre-gray-100 hover:bg-pierre-gray-50">
                      <td className="py-2 px-3 text-pierre-gray-500">
                        {new Date(entry.timestamp).toLocaleString()}
                      </td>
                      <td className="py-2 px-3">{entry.admin_email}</td>
                      <td className="py-2 px-3">
                        <code className="bg-pierre-gray-100 px-1 rounded text-xs">{entry.config_key}</code>
                      </td>
                      <td className="py-2 px-3 text-pierre-gray-500">
                        {entry.old_value !== undefined ? formatValue(entry.old_value) : '-'}
                      </td>
                      <td className="py-2 px-3 font-medium">{formatValue(entry.new_value)}</td>
                      <td className="py-2 px-3 text-pierre-gray-500">{entry.reason || '-'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>
      )}

      {/* Confirm Changes Modal */}
      <Modal
        isOpen={showConfirmModal}
        onClose={() => setShowConfirmModal(false)}
        title="Confirm Configuration Changes"
      >
        <div className="space-y-4">
          <p className="text-pierre-gray-600">
            You are about to update {Object.keys(pendingChanges).length} configuration parameter(s).
          </p>

          <div className="bg-pierre-gray-50 rounded-lg p-3 max-h-48 overflow-y-auto">
            {Object.entries(pendingChanges).map(([key, value]) => (
              <div key={key} className="flex justify-between text-sm py-1">
                <span className="text-pierre-gray-600">{key}</span>
                <span className="font-medium">{formatValue(value)}</span>
              </div>
            ))}
          </div>

          <Input
            label="Reason for changes (optional)"
            value={changeReason}
            onChange={(e) => setChangeReason(e.target.value)}
            placeholder="Describe why these changes are being made..."
          />

          {updateMutation.data?.data?.requires_restart && (
            <div className="p-3 bg-pierre-nutrition-light/20 text-pierre-nutrition rounded-lg text-sm">
              Some changes require a server restart to take effect.
            </div>
          )}

          {updateMutation.isError && (
            <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
              Failed to update configuration. Please try again.
            </div>
          )}

          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={() => setShowConfirmModal(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={confirmChanges}
              disabled={updateMutation.isPending}
            >
              {updateMutation.isPending ? 'Saving...' : 'Confirm Changes'}
            </Button>
          </div>
        </div>
      </Modal>

      {/* Reset Confirmation Modal */}
      <Modal
        isOpen={showResetModal}
        onClose={() => setShowResetModal(false)}
        title="Reset to Defaults"
      >
        <div className="space-y-4">
          <p className="text-pierre-gray-600">
            {resetTarget?.key
              ? `Are you sure you want to reset "${resetTarget.key}" to its default value?`
              : resetTarget?.category
              ? `Are you sure you want to reset all parameters in "${resetTarget.category}" to their defaults?`
              : 'Are you sure you want to reset all configuration to defaults?'}
          </p>

          {resetMutation.isError && (
            <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
              Failed to reset configuration. Please try again.
            </div>
          )}

          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={() => setShowResetModal(false)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={confirmReset}
              disabled={resetMutation.isPending}
            >
              {resetMutation.isPending ? 'Resetting...' : 'Reset to Defaults'}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
