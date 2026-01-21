// ABOUTME: Admin configuration management UI for runtime parameters
// ABOUTME: Allows admins to view, modify, and reset server configuration values
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useMemo, useCallback, lazy, Suspense } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card, Badge, Input, Button, Modal, Tabs } from './ui';

// Lazy load ToolAvailability to reduce initial bundle size
const ToolAvailability = lazy(() => import('./ToolAvailability'));

// Clipboard copy with fallback for older browsers
const copyToClipboard = async (text: string): Promise<boolean> => {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Fallback for older browsers
    const textArea = document.createElement('textarea');
    textArea.value = text;
    textArea.style.position = 'fixed';
    textArea.style.left = '-999999px';
    document.body.appendChild(textArea);
    textArea.select();
    try {
      document.execCommand('copy');
      return true;
    } catch {
      return false;
    } finally {
      document.body.removeChild(textArea);
    }
  }
};

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

// Category groupings: Server vs Intelligence configuration
const SERVER_CATEGORIES = new Set([
  'rate_limiting',
  'feature_flags',
  'llm_provider',
  'tokio_runtime',
  'sqlx_config',
  'cache_ttl',
  'provider_strava',
  'provider_fitbit',
  'provider_garmin',
  'mcp_network',
  'monitoring',
]);

const INTELLIGENCE_CATEGORIES = new Set([
  'heart_rate_zones',
  'recommendation_engine',
  'sleep_recovery',
  'training_stress',
  'weather_analysis',
  'nutrition',
  'algorithms',
]);

type ConfigGroup = 'server' | 'intelligence';

export default function AdminConfiguration() {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<'parameters' | 'tools' | 'history'>('parameters');
  const [configGroup, setConfigGroup] = useState<ConfigGroup>('server');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [pendingChanges, setPendingChanges] = useState<Record<string, unknown>>({});
  const [changeReason, setChangeReason] = useState('');
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [showResetModal, setShowResetModal] = useState(false);
  const [resetTarget, setResetTarget] = useState<{ category?: string; key?: string } | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [copiedEnvVar, setCopiedEnvVar] = useState<string | null>(null);

  // Handle copy to clipboard with feedback
  const handleCopyEnvVar = useCallback(async (envVar: string) => {
    const success = await copyToClipboard(envVar);
    if (success) {
      setCopiedEnvVar(envVar);
      setTimeout(() => setCopiedEnvVar(null), 2000);
    }
  }, []);

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

  // Filter categories by config group (server vs intelligence), search query, and non-empty
  const filteredCategories = useMemo(() => {
    // First, filter by config group
    const groupSet = configGroup === 'server' ? SERVER_CATEGORIES : INTELLIGENCE_CATEGORIES;
    const groupCategories = categories.filter((cat) => groupSet.has(cat.name));

    // Then, filter out empty categories (those with 0 parameters)
    const nonEmptyCategories = groupCategories.filter((cat) => cat.parameters.length > 0);

    if (!searchQuery.trim()) return nonEmptyCategories;
    const query = searchQuery.toLowerCase();
    return nonEmptyCategories
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
  }, [categories, configGroup, searchQuery]);

  // Get current category parameters (always use filtered to exclude empty categories)
  const currentCategory = useMemo(() => {
    if (!selectedCategory) return filteredCategories[0] || null;
    return filteredCategories.find((c) => c.name === selectedCategory) || filteredCategories[0] || null;
  }, [filteredCategories, selectedCategory]);

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

  // Get original value for a parameter key (for old→new comparison in modal)
  const getOriginalValue = useCallback((key: string): unknown => {
    for (const cat of categories) {
      const param = cat.parameters.find((p: ConfigParameter) => p.key === key);
      if (param) return param.current_value;
    }
    return undefined;
  }, [categories]);

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
        <div className="pierre-spinner w-8 h-8"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card className="border-pierre-red-200">
        <div className="text-center py-8">
          <svg className="w-12 h-12 text-pierre-red-400 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <p className="text-pierre-red-600">Failed to load configuration catalog.</p>
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
            {filteredCategories.reduce((sum, cat) => sum + cat.parameters.length, 0)} parameters &bull;{' '}
            {filteredCategories.length} categories
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
          { id: 'tools', label: 'Tool Availability' },
          { id: 'history', label: 'Change History' },
        ]}
        activeTab={activeTab}
        onChange={(id: string) => setActiveTab(id as 'parameters' | 'tools' | 'history')}
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
                {/* Config group selector */}
                <div className="mb-4">
                  <div className="flex rounded-lg bg-pierre-gray-100 p-1">
                    <button
                      onClick={() => {
                        setConfigGroup('server');
                        setSelectedCategory(null);
                      }}
                      className={`flex-1 px-3 py-2 text-sm font-medium rounded-md transition-colors ${
                        configGroup === 'server'
                          ? 'bg-white text-pierre-violet shadow-sm'
                          : 'text-pierre-gray-600 hover:text-pierre-gray-900'
                      }`}
                    >
                      <div className="flex items-center justify-center gap-1.5">
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
                        </svg>
                        Server
                      </div>
                    </button>
                    <button
                      onClick={() => {
                        setConfigGroup('intelligence');
                        setSelectedCategory(null);
                      }}
                      className={`flex-1 px-3 py-2 text-sm font-medium rounded-md transition-colors ${
                        configGroup === 'intelligence'
                          ? 'bg-white text-pierre-violet shadow-sm'
                          : 'text-pierre-gray-600 hover:text-pierre-gray-900'
                      }`}
                    >
                      <div className="flex items-center justify-center gap-1.5">
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                        </svg>
                        Intelligence
                      </div>
                    </button>
                  </div>
                </div>
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
                    {currentCategory.parameters.map((param: ConfigParameter) => {
                      const hasPendingChange = param.key in pendingChanges;
                      const isModifiedFromDefault = param.is_modified || hasPendingChange;
                      return (
                      <div key={param.key} className={`py-4 ${hasPendingChange ? 'bg-pierre-violet/5 -mx-4 px-4 rounded-lg' : ''}`}>
                        <div className="flex items-start justify-between">
                          <div className="flex-1 mr-4">
                            <div className="flex items-center gap-2">
                              {/* Visual indicator dot for pending changes */}
                              {hasPendingChange && (
                                <span className="w-2 h-2 rounded-full bg-pierre-violet animate-pulse" title="Pending change" />
                              )}
                              <h4 className="font-medium text-pierre-gray-900">
                                {param.display_name}
                              </h4>
                              {param.is_modified && !hasPendingChange && (
                                <Badge variant="warning">Modified</Badge>
                              )}
                              {hasPendingChange && (
                                <Badge variant="info">Unsaved</Badge>
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
                                <span className="flex items-center gap-1">
                                  Env:{' '}
                                  <code className="bg-pierre-gray-100 px-1 rounded">{param.env_variable}</code>
                                  <button
                                    onClick={() => handleCopyEnvVar(param.env_variable!)}
                                    className="p-0.5 hover:bg-pierre-gray-200 rounded transition-colors"
                                    title="Copy to clipboard"
                                  >
                                    {copiedEnvVar === param.env_variable ? (
                                      <svg className="w-3.5 h-3.5 text-pierre-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                                      </svg>
                                    ) : (
                                      <svg className="w-3.5 h-3.5 text-pierre-gray-400 hover:text-pierre-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                      </svg>
                                    )}
                                  </button>
                                </span>
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
                            {/* Show reset button when value differs from default (either saved override or pending change) */}
                            {(isModifiedFromDefault || hasPendingChange) && param.is_runtime_configurable && (
                              <button
                                onClick={() => hasPendingChange
                                  ? handleValueChange(param.key, param.default_value, param.current_value)
                                  : handleReset(undefined, param.key)
                                }
                                className="p-1 text-pierre-gray-400 hover:text-pierre-gray-600 transition-colors"
                                title={hasPendingChange ? "Revert pending change" : "Reset to default"}
                              >
                                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                </svg>
                              </button>
                            )}
                          </div>
                        </div>
                      </div>
                    );})}
                  </div>
                </Card>
              </>
            )}
          </div>
        </div>
          )}
        </>
      ) : activeTab === 'tools' ? (
        /* Tool Availability tab */
        <Suspense
          fallback={
            <div className="flex items-center justify-center py-12">
              <div className="pierre-spinner w-8 h-8" />
            </div>
          }
        >
          <ToolAvailability />
        </Suspense>
      ) : (
        /* History tab */
        <Card>
          <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Change History</h2>
          {auditLoading ? (
            <div className="flex justify-center py-8">
              <div className="pierre-spinner w-6 h-6"></div>
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

          <div className="bg-pierre-gray-50 rounded-lg p-3 max-h-64 overflow-y-auto">
            <div className="space-y-3">
              {Object.entries(pendingChanges).map(([key, newValue]) => {
                const oldValue = getOriginalValue(key);
                return (
                  <div key={key} className="text-sm border-b border-pierre-gray-200 pb-2 last:border-b-0 last:pb-0">
                    <div className="font-medium text-pierre-gray-700 mb-1">{key}</div>
                    <div className="flex items-center gap-2">
                      <span className="text-pierre-gray-500 bg-pierre-gray-100 px-2 py-0.5 rounded line-through">
                        {formatValue(oldValue)}
                      </span>
                      <svg className="w-4 h-4 text-pierre-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7l5 5m0 0l-5 5m5-5H6" />
                      </svg>
                      <span className="font-medium text-pierre-violet bg-pierre-violet/10 px-2 py-0.5 rounded">
                        {formatValue(newValue)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
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
            <div className="p-3 bg-pierre-red-50 text-pierre-red-600 rounded-lg text-sm">
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
            <div className="p-3 bg-pierre-red-50 text-pierre-red-600 rounded-lg text-sm">
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
