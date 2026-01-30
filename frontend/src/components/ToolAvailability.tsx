// ABOUTME: Tool availability configuration UI for per-tenant MCP tool management
// ABOUTME: Allows admins to enable/disable tools, view global restrictions, and manage overrides
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useMemo, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import { Card, Badge, Input, Button, Modal } from './ui';
import { useAuth } from '../hooks/useAuth';

interface ToolEntry {
  tool_name: string;
  display_name: string;
  description: string;
  category: string;
  is_enabled: boolean;
  source: string;
  min_plan: string;
}

interface CatalogEntry {
  tool_name: string;
  display_name: string;
  description: string;
  category: string;
  default_enabled: boolean;
  is_globally_disabled: boolean;
  available_in_tiers: string[];
}

// Extract unique categories from tools
const extractCategories = (tools: ToolEntry[] | CatalogEntry[]): string[] => {
  const categories = new Set<string>();
  tools.forEach((tool) => categories.add(tool.category));
  return Array.from(categories).sort();
};

interface ToolAvailabilityProps {
  tenantId?: string;
}

export default function ToolAvailability({ tenantId }: ToolAvailabilityProps) {
  const queryClient = useQueryClient();
  const { user } = useAuth();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedTools, setSelectedTools] = useState<Set<string>>(new Set());
  const [showDisableModal, setShowDisableModal] = useState(false);
  const [showEnableModal, setShowEnableModal] = useState(false);
  const [pendingAction, setPendingAction] = useState<{
    toolName: string;
    action: 'enable' | 'disable';
  } | null>(null);
  const [bulkAction, setBulkAction] = useState<'enable' | 'disable' | null>(null);
  const [overrideReason, setOverrideReason] = useState('');

  // Use provided tenantId, or fall back to the user's user_id (which is often their tenant_id)
  const effectiveTenantId = tenantId || user?.user_id || '';

  // Fetch global disabled tools
  const { data: globalDisabled } = useQuery({
    queryKey: ['global-disabled-tools'],
    queryFn: () => adminApi.getGlobalDisabledTools(),
    retry: 1,
  });

  // Fetch tenant tools
  const {
    data: tenantToolsData,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['tenant-tools', effectiveTenantId],
    queryFn: () => adminApi.getTenantTools(effectiveTenantId),
    retry: 1,
    enabled: !!effectiveTenantId,
  });

  // Fetch availability summary
  const { data: summaryData } = useQuery({
    queryKey: ['tool-availability-summary', effectiveTenantId],
    queryFn: () => adminApi.getToolAvailabilitySummary(effectiveTenantId),
    retry: 1,
    enabled: !!effectiveTenantId,
  });

  // Set tool override mutation
  const setOverrideMutation = useMutation({
    mutationFn: ({
      toolName,
      isEnabled,
      reason,
    }: {
      toolName: string;
      isEnabled: boolean;
      reason?: string;
    }) => adminApi.setToolOverride(effectiveTenantId, toolName, isEnabled, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tenant-tools', effectiveTenantId] });
      queryClient.invalidateQueries({ queryKey: ['tool-availability-summary', effectiveTenantId] });
      setShowDisableModal(false);
      setShowEnableModal(false);
      setPendingAction(null);
      setBulkAction(null);
      setOverrideReason('');
      setSelectedTools(new Set());
    },
  });

  // Remove override mutation
  const removeOverrideMutation = useMutation({
    mutationFn: (toolName: string) => adminApi.removeToolOverride(effectiveTenantId, toolName),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tenant-tools', effectiveTenantId] });
      queryClient.invalidateQueries({ queryKey: ['tool-availability-summary', effectiveTenantId] });
    },
  });

  // Get categories from tools
  const categories = useMemo(() => {
    if (!tenantToolsData?.data) return [];
    return extractCategories(tenantToolsData.data);
  }, [tenantToolsData]);

  // Filter tools by search and category
  const filteredTools = useMemo(() => {
    if (!tenantToolsData?.data) return [];
    let tools = tenantToolsData.data;

    // Filter by category
    if (selectedCategory) {
      tools = tools.filter((tool) => tool.category === selectedCategory);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      tools = tools.filter(
        (tool) =>
          tool.tool_name.toLowerCase().includes(query) ||
          tool.display_name.toLowerCase().includes(query) ||
          tool.description.toLowerCase().includes(query)
      );
    }

    return tools;
  }, [tenantToolsData, selectedCategory, searchQuery]);

  // Check if a tool is globally disabled
  const isGloballyDisabled = useCallback(
    (toolName: string): boolean => {
      return globalDisabled?.data?.disabled_tools?.includes(toolName) ?? false;
    },
    [globalDisabled]
  );

  // Handle individual tool toggle
  const handleToggleTool = (tool: ToolEntry) => {
    if (isGloballyDisabled(tool.tool_name)) {
      return; // Can't toggle globally disabled tools
    }

    setPendingAction({
      toolName: tool.tool_name,
      action: tool.is_enabled ? 'disable' : 'enable',
    });

    if (tool.is_enabled) {
      setShowDisableModal(true);
    } else {
      setShowEnableModal(true);
    }
  };

  // Handle bulk action
  const handleBulkAction = (action: 'enable' | 'disable') => {
    if (selectedTools.size === 0) return;
    setBulkAction(action);
    if (action === 'disable') {
      setShowDisableModal(true);
    } else {
      setShowEnableModal(true);
    }
  };

  // Confirm single action
  const confirmAction = () => {
    if (pendingAction) {
      setOverrideMutation.mutate({
        toolName: pendingAction.toolName,
        isEnabled: pendingAction.action === 'enable',
        reason: overrideReason || undefined,
      });
    }
  };

  // Confirm bulk action
  const confirmBulkAction = async () => {
    if (!bulkAction) return;

    const toolsArray = Array.from(selectedTools);
    for (const toolName of toolsArray) {
      await adminApi.setToolOverride(
        effectiveTenantId,
        toolName,
        bulkAction === 'enable',
        overrideReason || undefined
      );
    }

    queryClient.invalidateQueries({ queryKey: ['tenant-tools', effectiveTenantId] });
    queryClient.invalidateQueries({ queryKey: ['tool-availability-summary', effectiveTenantId] });
    setShowDisableModal(false);
    setShowEnableModal(false);
    setBulkAction(null);
    setOverrideReason('');
    setSelectedTools(new Set());
  };

  // Handle checkbox selection
  const handleSelectTool = (toolName: string, checked: boolean) => {
    const newSelected = new Set(selectedTools);
    if (checked) {
      newSelected.add(toolName);
    } else {
      newSelected.delete(toolName);
    }
    setSelectedTools(newSelected);
  };

  // Handle select all
  const handleSelectAll = (checked: boolean) => {
    if (checked) {
      const selectableTools = filteredTools
        .filter((tool) => !isGloballyDisabled(tool.tool_name))
        .map((tool) => tool.tool_name);
      setSelectedTools(new Set(selectableTools));
    } else {
      setSelectedTools(new Set());
    }
  };

  // Check if all filtered tools are selected
  const allSelected = useMemo(() => {
    const selectableTools = filteredTools.filter((tool) => !isGloballyDisabled(tool.tool_name));
    return selectableTools.length > 0 && selectableTools.every((tool) => selectedTools.has(tool.tool_name));
  }, [filteredTools, selectedTools, isGloballyDisabled]);

  // Get source badge color based on enablement source
  const getSourceBadge = (source: string) => {
    switch (source.toLowerCase()) {
      case 'globaldisabled':
        return <Badge variant="destructive">Global Block</Badge>;
      case 'planrestriction':
        return <Badge variant="warning">Plan Restricted</Badge>;
      case 'tenantoverride':
        return <Badge variant="info">Override</Badge>;
      default:
        return <Badge variant="secondary">Default</Badge>;
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="pierre-spinner w-8 h-8" />
      </div>
    );
  }

  if (error) {
    return (
      <Card className="border-red-200">
        <div className="text-center py-8">
          <svg
            className="w-12 h-12 text-red-400 mx-auto mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <p className="text-red-600">Failed to load tool availability.</p>
          <p className="text-sm text-pierre-gray-500 mt-2">Please check your permissions and try again.</p>
        </div>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Global Disabled Banner */}
      {globalDisabled?.data && globalDisabled.data.count > 0 && (
        <Card className="bg-red-50 border-red-200">
          <div className="flex items-start gap-3">
            <svg
              className="w-5 h-5 text-red-500 mt-0.5 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            <div>
              <h3 className="font-medium text-red-800">Globally Disabled Tools</h3>
              <p className="text-sm text-red-600 mt-1">
                {globalDisabled.data.count} tool(s) are disabled via <code className="bg-red-100 px-1 rounded">PIERRE_DISABLED_TOOLS</code> environment variable.
                These cannot be enabled through the admin UI.
              </p>
              <div className="flex flex-wrap gap-2 mt-2">
                {globalDisabled.data.disabled_tools.map((tool) => (
                  <Badge key={tool} variant="destructive">
                    {tool}
                  </Badge>
                ))}
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* Summary Stats */}
      {summaryData?.data && (
        <div className="grid grid-cols-5 gap-4">
          <Card className="text-center py-4">
            <div className="text-2xl font-bold text-pierre-gray-900">{summaryData.data.total_tools}</div>
            <div className="text-sm text-pierre-gray-500">Total Tools</div>
          </Card>
          <Card className="text-center py-4">
            <div className="text-2xl font-bold text-green-600">{summaryData.data.enabled_tools}</div>
            <div className="text-sm text-pierre-gray-500">Enabled</div>
          </Card>
          <Card className="text-center py-4">
            <div className="text-2xl font-bold text-red-600">{summaryData.data.disabled_tools}</div>
            <div className="text-sm text-pierre-gray-500">Disabled</div>
          </Card>
          <Card className="text-center py-4">
            <div className="text-2xl font-bold text-pierre-violet">{summaryData.data.overridden_tools}</div>
            <div className="text-sm text-pierre-gray-500">Overrides</div>
          </Card>
          <Card className="text-center py-4">
            <div className="text-2xl font-bold text-orange-600">{summaryData.data.globally_disabled_count}</div>
            <div className="text-sm text-pierre-gray-500">Global Blocks</div>
          </Card>
        </div>
      )}

      {/* Filters and Search */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2">
          {/* Category filter chips */}
          <button
            onClick={() => setSelectedCategory(null)}
            className={`px-3 py-1.5 rounded-full text-sm font-medium transition-colors ${
              selectedCategory === null
                ? 'bg-pierre-violet text-white'
                : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
            }`}
          >
            All
          </button>
          {categories.map((category) => (
            <button
              key={category}
              onClick={() => setSelectedCategory(category)}
              className={`px-3 py-1.5 rounded-full text-sm font-medium transition-colors ${
                selectedCategory === category
                  ? 'bg-pierre-violet text-white'
                  : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
              }`}
            >
              {category}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-3">
          {/* Search */}
          <div className="relative">
            <Input
              type="text"
              placeholder="Search tools..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-64"
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

          {/* Bulk actions */}
          {selectedTools.size > 0 && (
            <div className="flex items-center gap-2">
              <span className="text-sm text-pierre-gray-500">{selectedTools.size} selected</span>
              <Button variant="outline" size="sm" onClick={() => handleBulkAction('enable')}>
                Enable Selected
              </Button>
              <Button variant="danger" size="sm" onClick={() => handleBulkAction('disable')}>
                Disable Selected
              </Button>
            </div>
          )}
        </div>
      </div>

      {/* Tools Table */}
      <Card>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-pierre-gray-200">
                <th className="text-left py-3 px-3 w-10">
                  <input
                    type="checkbox"
                    checked={allSelected}
                    onChange={(e) => handleSelectAll(e.target.checked)}
                    className="rounded border-pierre-gray-300 text-pierre-violet focus:ring-pierre-violet"
                  />
                </th>
                <th className="text-left py-3 px-3 font-medium text-pierre-gray-600">Tool</th>
                <th className="text-left py-3 px-3 font-medium text-pierre-gray-600">Category</th>
                <th className="text-left py-3 px-3 font-medium text-pierre-gray-600">Status</th>
                <th className="text-left py-3 px-3 font-medium text-pierre-gray-600">Source</th>
                <th className="text-left py-3 px-3 font-medium text-pierre-gray-600">Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredTools.map((tool) => {
                const globallyDisabled = isGloballyDisabled(tool.tool_name);
                return (
                  <tr
                    key={tool.tool_name}
                    className={`border-b border-pierre-gray-100 hover:bg-pierre-gray-50 ${
                      globallyDisabled ? 'opacity-60' : ''
                    }`}
                  >
                    <td className="py-3 px-3">
                      <input
                        type="checkbox"
                        checked={selectedTools.has(tool.tool_name)}
                        onChange={(e) => handleSelectTool(tool.tool_name, e.target.checked)}
                        disabled={globallyDisabled}
                        className="rounded border-pierre-gray-300 text-pierre-violet focus:ring-pierre-violet disabled:opacity-50"
                      />
                    </td>
                    <td className="py-3 px-3">
                      <div className="font-medium text-pierre-gray-900">{tool.display_name}</div>
                      <div className="text-xs text-pierre-gray-500 mt-0.5">
                        <code>{tool.tool_name}</code>
                      </div>
                      <div className="text-xs text-pierre-gray-400 mt-1 max-w-md truncate" title={tool.description}>
                        {tool.description}
                      </div>
                    </td>
                    <td className="py-3 px-3">
                      <Badge variant="secondary">{tool.category}</Badge>
                    </td>
                    <td className="py-3 px-3">
                      {tool.is_enabled ? (
                        <span className="inline-flex items-center gap-1 text-green-600">
                          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                            <path
                              fillRule="evenodd"
                              d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                              clipRule="evenodd"
                            />
                          </svg>
                          Enabled
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1 text-red-600">
                          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                            <path
                              fillRule="evenodd"
                              d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                              clipRule="evenodd"
                            />
                          </svg>
                          Disabled
                        </span>
                      )}
                    </td>
                    <td className="py-3 px-3">{getSourceBadge(tool.source)}</td>
                    <td className="py-3 px-3">
                      <div className="flex items-center gap-2">
                        {/* Toggle switch */}
                        <button
                          onClick={() => handleToggleTool(tool)}
                          disabled={globallyDisabled}
                          className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-offset-2 ${
                            tool.is_enabled ? 'bg-green-500' : 'bg-pierre-gray-300'
                          } ${globallyDisabled ? 'opacity-50 cursor-not-allowed' : ''}`}
                          role="switch"
                          aria-checked={tool.is_enabled}
                          title={globallyDisabled ? 'Cannot toggle globally disabled tools' : undefined}
                        >
                          <span
                            className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow-sm ${
                              tool.is_enabled ? 'translate-x-6' : 'translate-x-1'
                            }`}
                          />
                        </button>

                        {/* Remove override button (only if source is tenant override) */}
                        {tool.source.toLowerCase() === 'tenantoverride' && (
                          <button
                            onClick={() => removeOverrideMutation.mutate(tool.tool_name)}
                            className="p-1 text-pierre-gray-400 hover:text-pierre-gray-600 transition-colors"
                            title="Remove override (revert to default)"
                          >
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                              />
                            </svg>
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>

          {filteredTools.length === 0 && (
            <div className="text-center py-8 text-pierre-gray-500">No tools found matching your criteria.</div>
          )}
        </div>
      </Card>

      {/* Disable Confirmation Modal */}
      <Modal isOpen={showDisableModal} onClose={() => setShowDisableModal(false)} title="Confirm Disable Tool(s)">
        <div className="space-y-4">
          <p className="text-pierre-gray-600">
            {bulkAction
              ? `You are about to disable ${selectedTools.size} tool(s). Users will no longer be able to use these tools.`
              : `You are about to disable "${pendingAction?.toolName}". Users will no longer be able to use this tool.`}
          </p>

          <Input
            label="Reason for disabling (optional)"
            value={overrideReason}
            onChange={(e) => setOverrideReason(e.target.value)}
            placeholder="e.g., Security concern, Maintenance, Feature deprecation..."
          />

          <div className="p-3 bg-orange-50 text-orange-700 rounded-lg text-sm">
            <strong>Note:</strong> This override can be removed later to restore the tool to its default state.
          </div>

          {setOverrideMutation.isError && (
            <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
              Failed to update tool settings. Please try again.
            </div>
          )}

          <div className="flex justify-end gap-3">
            <Button
              variant="outline"
              onClick={() => {
                setShowDisableModal(false);
                setPendingAction(null);
                setBulkAction(null);
                setOverrideReason('');
              }}
            >
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={bulkAction ? confirmBulkAction : confirmAction}
              disabled={setOverrideMutation.isPending}
            >
              {setOverrideMutation.isPending ? 'Disabling...' : 'Disable'}
            </Button>
          </div>
        </div>
      </Modal>

      {/* Enable Confirmation Modal */}
      <Modal isOpen={showEnableModal} onClose={() => setShowEnableModal(false)} title="Confirm Enable Tool(s)">
        <div className="space-y-4">
          <p className="text-pierre-gray-600">
            {bulkAction
              ? `You are about to enable ${selectedTools.size} tool(s). Users will be able to use these tools.`
              : `You are about to enable "${pendingAction?.toolName}". Users will be able to use this tool.`}
          </p>

          <Input
            label="Reason for enabling (optional)"
            value={overrideReason}
            onChange={(e) => setOverrideReason(e.target.value)}
            placeholder="e.g., Feature release, Customer request..."
          />

          {setOverrideMutation.isError && (
            <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
              Failed to update tool settings. Please try again.
            </div>
          )}

          <div className="flex justify-end gap-3">
            <Button
              variant="outline"
              onClick={() => {
                setShowEnableModal(false);
                setPendingAction(null);
                setBulkAction(null);
                setOverrideReason('');
              }}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={bulkAction ? confirmBulkAction : confirmAction}
              disabled={setOverrideMutation.isPending}
            >
              {setOverrideMutation.isPending ? 'Enabling...' : 'Enable'}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
