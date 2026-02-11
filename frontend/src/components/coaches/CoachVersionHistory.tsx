// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// ABOUTME: Coach version history component for viewing and reverting to previous versions
// ABOUTME: Displays timeline of versions with diff view and revert functionality

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { coachesApi } from '../../services/api';
import { Modal, ModalActions, Button, Card } from '../ui';
import { clsx } from 'clsx';
import { QUERY_KEYS } from '../../constants/queryKeys';

interface CoachVersionHistoryProps {
  coachId: string;
  coachTitle: string;
  isOpen: boolean;
  onClose: () => void;
  onReverted?: () => void;
}

interface VersionItem {
  version: number;
  content_snapshot: Record<string, unknown>;
  change_summary: string | null;
  created_at: string;
  created_by_name: string | null;
}

export function CoachVersionHistory({
  coachId,
  coachTitle,
  isOpen,
  onClose,
  onReverted,
}: CoachVersionHistoryProps) {
  const queryClient = useQueryClient();
  const [selectedVersion, setSelectedVersion] = useState<VersionItem | null>(null);
  const [compareFrom, setCompareFrom] = useState<number | null>(null);
  const [compareTo, setCompareTo] = useState<number | null>(null);
  const [showConfirmRevert, setShowConfirmRevert] = useState(false);

  // Fetch version history
  const { data: versionsData, isLoading } = useQuery({
    queryKey: QUERY_KEYS.coaches.versions(coachId),
    queryFn: () => coachesApi.getVersions(coachId, 50),
    enabled: isOpen,
  });

  // Fetch diff when both versions selected
  const { data: diffData, isLoading: isDiffLoading } = useQuery({
    queryKey: QUERY_KEYS.coaches.versionDiff(coachId, compareFrom ?? undefined, compareTo ?? undefined),
    queryFn: () =>
      compareFrom && compareTo
        ? coachesApi.getVersionDiff(coachId, compareFrom, compareTo)
        : Promise.resolve(null),
    enabled: isOpen && compareFrom !== null && compareTo !== null,
  });

  // Revert mutation
  const revertMutation = useMutation({
    mutationFn: (version: number) => coachesApi.revertToVersion(coachId, version),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.versions(coachId) });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.list() });
      setShowConfirmRevert(false);
      setSelectedVersion(null);
      onReverted?.();
    },
  });

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const handleVersionClick = (version: VersionItem) => {
    if (selectedVersion?.version === version.version) {
      setSelectedVersion(null);
    } else {
      setSelectedVersion(version);
    }
  };

  const handleCompareSelect = (version: number) => {
    if (compareFrom === null) {
      setCompareFrom(version);
    } else if (compareTo === null && version !== compareFrom) {
      setCompareTo(version);
    } else {
      // Reset and start new comparison
      setCompareFrom(version);
      setCompareTo(null);
    }
  };

  const clearComparison = () => {
    setCompareFrom(null);
    setCompareTo(null);
  };

  const handleRevert = () => {
    if (selectedVersion) {
      setShowConfirmRevert(true);
    }
  };

  const confirmRevert = () => {
    if (selectedVersion) {
      revertMutation.mutate(selectedVersion.version);
    }
  };

  const renderSnapshotField = (key: string, value: unknown) => {
    if (value === null || value === undefined) return null;

    const displayValue =
      typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);

    return (
      <div key={key} className="py-2 border-b border-pierre-gray-100 last:border-0">
        <dt className="text-xs font-medium text-pierre-gray-500 uppercase tracking-wide">
          {key.replace(/_/g, ' ')}
        </dt>
        <dd className="mt-1 text-sm text-pierre-gray-900 whitespace-pre-wrap break-words">
          {displayValue}
        </dd>
      </div>
    );
  };

  const renderDiffChange = (change: { field: string; old_value: unknown; new_value: unknown }) => {
    const formatValue = (val: unknown) =>
      val === null || val === undefined
        ? '(empty)'
        : typeof val === 'object'
          ? JSON.stringify(val, null, 2)
          : String(val);

    return (
      <div key={change.field} className="py-3 border-b border-pierre-gray-100 last:border-0">
        <div className="text-sm font-medium text-pierre-gray-700 mb-2">
          {change.field.replace(/_/g, ' ')}
        </div>
        <div className="grid grid-cols-2 gap-4">
          <div className="bg-red-50 rounded-lg p-3">
            <div className="text-xs text-red-600 font-medium mb-1">Before (v{compareFrom})</div>
            <div className="text-sm text-red-800 whitespace-pre-wrap break-words">
              {formatValue(change.old_value)}
            </div>
          </div>
          <div className="bg-green-50 rounded-lg p-3">
            <div className="text-xs text-green-600 font-medium mb-1">After (v{compareTo})</div>
            <div className="text-sm text-green-800 whitespace-pre-wrap break-words">
              {formatValue(change.new_value)}
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <>
      <Modal isOpen={isOpen} onClose={onClose} title={`Version History: ${coachTitle}`} size="xl">
        <div className="space-y-4">
          {/* Stats bar */}
          {versionsData && (
            <div className="flex items-center justify-between px-4 py-2 bg-pierre-gray-50 rounded-lg">
              <span className="text-sm text-pierre-gray-600">
                {versionsData.total} version{versionsData.total !== 1 ? 's' : ''} saved
              </span>
              <span className="text-sm font-medium text-pierre-gray-900">
                Current: v{versionsData.current_version}
              </span>
            </div>
          )}

          {/* Compare mode indicator */}
          {(compareFrom !== null || compareTo !== null) && (
            <div className="flex items-center justify-between px-4 py-2 bg-blue-50 rounded-lg">
              <span className="text-sm text-blue-700">
                {compareFrom !== null && compareTo === null
                  ? `Select second version to compare with v${compareFrom}`
                  : `Comparing v${compareFrom} with v${compareTo}`}
              </span>
              <button
                onClick={clearComparison}
                className="text-sm text-blue-600 hover:text-blue-800 font-medium"
              >
                Clear
              </button>
            </div>
          )}

          {/* Diff view */}
          {diffData && diffData.changes && (
            <Card className="bg-white">
              <div className="p-4">
                <h3 className="text-sm font-semibold text-pierre-gray-900 mb-3">
                  Changes between v{diffData.from_version} and v{diffData.to_version}
                </h3>
                {isDiffLoading ? (
                  <div className="text-sm text-pierre-gray-500">Loading diff...</div>
                ) : diffData.changes.length === 0 ? (
                  <div className="text-sm text-pierre-gray-500">No changes between versions</div>
                ) : (
                  <div className="divide-y divide-pierre-gray-100">
                    {diffData.changes.map(renderDiffChange)}
                  </div>
                )}
              </div>
            </Card>
          )}

          {/* Version list */}
          <div className="max-h-96 overflow-y-auto">
            {isLoading ? (
              <div className="text-center py-8 text-pierre-gray-500">Loading versions...</div>
            ) : !versionsData || versionsData.versions.length === 0 ? (
              <div className="text-center py-8 text-pierre-gray-500">
                No version history yet. Versions are created automatically when you update the
                coach.
              </div>
            ) : (
              <div className="space-y-2">
                {versionsData.versions.map((version) => (
                  <div
                    key={version.version}
                    className={clsx(
                      'border rounded-lg transition-all',
                      selectedVersion?.version === version.version
                        ? 'border-pierre-primary bg-pierre-primary/5'
                        : 'border-pierre-gray-200 hover:border-pierre-gray-300',
                      compareFrom === version.version || compareTo === version.version
                        ? 'ring-2 ring-blue-500 ring-offset-1'
                        : ''
                    )}
                  >
                    {/* Version header */}
                    <button
                      onClick={() => handleVersionClick(version)}
                      className="w-full text-left p-4"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <span className="inline-flex items-center justify-center w-8 h-8 rounded-full bg-pierre-gray-100 text-pierre-gray-700 font-semibold text-sm">
                            v{version.version}
                          </span>
                          <div>
                            <div className="text-sm font-medium text-pierre-gray-900">
                              {version.change_summary || 'Update'}
                            </div>
                            <div className="text-xs text-pierre-gray-500">
                              {formatDate(version.created_at)}
                              {version.created_by_name && ` by ${version.created_by_name}`}
                            </div>
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleCompareSelect(version.version);
                            }}
                            className={clsx(
                              'px-2 py-1 text-xs rounded',
                              compareFrom === version.version || compareTo === version.version
                                ? 'bg-blue-100 text-blue-700'
                                : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
                            )}
                          >
                            Compare
                          </button>
                          <svg
                            className={clsx(
                              'w-5 h-5 text-pierre-gray-400 transition-transform',
                              selectedVersion?.version === version.version ? 'rotate-180' : ''
                            )}
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                          >
                            <path
                              strokeLinecap="round"
                              strokeLinejoin="round"
                              strokeWidth={2}
                              d="M19 9l-7 7-7-7"
                            />
                          </svg>
                        </div>
                      </div>
                    </button>

                    {/* Expanded content */}
                    {selectedVersion?.version === version.version && (
                      <div className="px-4 pb-4 border-t border-pierre-gray-100">
                        <div className="mt-3 bg-pierre-gray-50 rounded-lg p-4">
                          <h4 className="text-xs font-semibold text-pierre-gray-700 uppercase tracking-wide mb-2">
                            Snapshot Content
                          </h4>
                          <dl className="divide-y divide-pierre-gray-100">
                            {Object.entries(version.content_snapshot).map(([key, value]) =>
                              renderSnapshotField(key, value)
                            )}
                          </dl>
                        </div>
                        <div className="mt-3 flex justify-end">
                          <Button
                            variant="secondary"
                            size="sm"
                            onClick={handleRevert}
                            disabled={revertMutation.isPending}
                          >
                            Revert to v{version.version}
                          </Button>
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        <ModalActions className="mt-4">
          <Button variant="secondary" onClick={onClose}>
            Close
          </Button>
        </ModalActions>
      </Modal>

      {/* Confirm Revert Dialog */}
      <Modal
        isOpen={showConfirmRevert}
        onClose={() => setShowConfirmRevert(false)}
        title="Confirm Revert"
        size="sm"
      >
        <p className="text-sm text-pierre-gray-600">
          Are you sure you want to revert to version {selectedVersion?.version}? This will create a
          new version with the reverted content. Your current changes will be preserved in the
          version history.
        </p>
        <ModalActions className="mt-4">
          <Button variant="secondary" onClick={() => setShowConfirmRevert(false)}>
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={confirmRevert}
            disabled={revertMutation.isPending}
          >
            {revertMutation.isPending ? 'Reverting...' : 'Revert'}
          </Button>
        </ModalActions>
      </Modal>
    </>
  );
}

export default CoachVersionHistory;
