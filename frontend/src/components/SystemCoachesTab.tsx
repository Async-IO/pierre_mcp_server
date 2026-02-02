// ABOUTME: Admin System Coaches management UI component
// ABOUTME: Provides CRUD operations for system coaches and user assignments
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { adminApi } from '../services/api';
import type { Coach, User } from '../types/api';
import { Card, Button } from './ui';
import { clsx } from 'clsx';

// Coach category options
const COACH_CATEGORIES = ['Training', 'Nutrition', 'Recovery', 'Recipes', 'Mobility', 'Custom'];

// Category colors for visual differentiation
const CATEGORY_COLORS: Record<string, string> = {
  Training: 'bg-pierre-activity/10 text-pierre-activity border-pierre-activity/20',
  Nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition border-pierre-nutrition/20',
  Recovery: 'bg-pierre-recovery/10 text-pierre-recovery border-pierre-recovery/20',
  Recipes: 'bg-pierre-yellow-500/10 text-pierre-yellow-600 border-pierre-yellow-500/20',
  Mobility: 'bg-pierre-mobility/10 text-pierre-mobility border-pierre-mobility/20',
  Custom: 'bg-pierre-violet/10 text-pierre-violet-light border-pierre-violet/20',
};

// Helper to get category color class with case-insensitive lookup
function getCategoryColorClass(category: string): string {
  const normalized = category.charAt(0).toUpperCase() + category.slice(1).toLowerCase();
  return CATEGORY_COLORS[normalized] || CATEGORY_COLORS.Custom;
}

interface CoachFormData {
  title: string;
  description: string;
  system_prompt: string;
  category: string;
  tags: string;
  visibility: string;
}

const defaultFormData: CoachFormData = {
  title: '',
  description: '',
  system_prompt: '',
  category: 'Training',
  tags: '',
  visibility: 'tenant',
};

export default function SystemCoachesTab() {
  const queryClient = useQueryClient();
  const [selectedCoach, setSelectedCoach] = useState<Coach | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [formData, setFormData] = useState<CoachFormData>(defaultFormData);
  const [showAssignModal, setShowAssignModal] = useState(false);
  const [selectedUserIds, setSelectedUserIds] = useState<string[]>([]);

  // Fetch system coaches
  const { data: coachesData, isLoading: coachesLoading } = useQuery({
    queryKey: ['admin-system-coaches'],
    queryFn: () => adminApi.getSystemCoaches(),
  });

  // Fetch all users for assignment
  const { data: usersData } = useQuery({
    queryKey: ['admin-all-users'],
    queryFn: () => adminApi.getAllUsers({ limit: 200 }),
    enabled: showAssignModal,
  });

  // Fetch assignments for selected coach
  const { data: assignmentsData, refetch: refetchAssignments } = useQuery({
    queryKey: ['coach-assignments', selectedCoach?.id],
    queryFn: () => selectedCoach ? adminApi.getCoachAssignments(selectedCoach.id) : null,
    enabled: !!selectedCoach,
  });

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: typeof formData) => adminApi.createSystemCoach({
      title: data.title,
      description: data.description || undefined,
      system_prompt: data.system_prompt,
      category: data.category,
      tags: data.tags.split(',').map(t => t.trim()).filter(Boolean),
      visibility: data.visibility,
    }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-system-coaches'] });
      setIsCreating(false);
      setFormData(defaultFormData);
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: typeof formData }) => adminApi.updateSystemCoach(id, {
      title: data.title,
      description: data.description || undefined,
      system_prompt: data.system_prompt,
      category: data.category,
      tags: data.tags.split(',').map(t => t.trim()).filter(Boolean),
    }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-system-coaches'] });
      setIsEditing(false);
      if (selectedCoach) {
        adminApi.getSystemCoach(selectedCoach.id).then(setSelectedCoach);
      }
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (id: string) => adminApi.deleteSystemCoach(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-system-coaches'] });
      setSelectedCoach(null);
    },
  });

  // Assign mutation
  const assignMutation = useMutation({
    mutationFn: ({ coachId, userIds }: { coachId: string; userIds: string[] }) =>
      adminApi.assignCoachToUsers(coachId, userIds),
    onSuccess: () => {
      refetchAssignments();
      setShowAssignModal(false);
      setSelectedUserIds([]);
    },
  });

  // Unassign mutation
  const unassignMutation = useMutation({
    mutationFn: ({ coachId, userIds }: { coachId: string; userIds: string[] }) =>
      adminApi.unassignCoachFromUsers(coachId, userIds),
    onSuccess: () => {
      refetchAssignments();
    },
  });

  // Load form data when editing
  useEffect(() => {
    if (isEditing && selectedCoach) {
      setFormData({
        title: selectedCoach.title,
        description: selectedCoach.description || '',
        system_prompt: selectedCoach.system_prompt,
        category: selectedCoach.category,
        tags: selectedCoach.tags.join(', '),
        visibility: selectedCoach.visibility || 'private',
      });
    }
  }, [isEditing, selectedCoach]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (isCreating) {
      createMutation.mutate(formData);
    } else if (isEditing && selectedCoach) {
      updateMutation.mutate({ id: selectedCoach.id, data: formData });
    }
  };

  const handleDelete = () => {
    if (selectedCoach && confirm(`Delete coach "${selectedCoach.title}"? This cannot be undone.`)) {
      deleteMutation.mutate(selectedCoach.id);
    }
  };

  const handleAssign = () => {
    if (selectedCoach && selectedUserIds.length > 0) {
      assignMutation.mutate({ coachId: selectedCoach.id, userIds: selectedUserIds });
    }
  };

  const handleUnassign = (userId: string) => {
    if (selectedCoach && confirm('Remove this user\'s access to the coach?')) {
      unassignMutation.mutate({ coachId: selectedCoach.id, userIds: [userId] });
    }
  };

  const coaches = coachesData?.coaches || [];
  const users = usersData || [];
  const assignments = assignmentsData?.assignments || [];
  const assignedUserIds = new Set(assignments.map(a => a.user_id));

  // Coach list view
  if (!selectedCoach && !isCreating) {
    return (
      <div className="space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-semibold text-white">System Coaches</h2>
            <p className="text-zinc-400 mt-1">
              Manage AI coaching personas available to all users in your organization.
            </p>
          </div>
          <Button
            onClick={() => {
              setFormData(defaultFormData);
              setIsCreating(true);
            }}
            className="flex items-center gap-2"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            Create Coach
          </Button>
        </div>

        {/* Coaches Grid */}
        {coachesLoading ? (
          <div className="flex justify-center py-12">
            <div className="pierre-spinner w-8 h-8"></div>
          </div>
        ) : coaches.length === 0 ? (
          <Card variant="dark" className="text-center py-12">
            <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-white/10 flex items-center justify-center">
              <svg className="w-8 h-8 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            </div>
            <h3 className="text-lg font-medium text-white mb-2">No System Coaches</h3>
            <p className="text-zinc-400 mb-4">
              Create your first system coach to provide AI coaching personas to your users.
            </p>
            <Button onClick={() => setIsCreating(true)}>Create Your First Coach</Button>
          </Card>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {coaches.map((coach) => (
              <div
                key={coach.id}
                className="cursor-pointer hover:border-white/30 transition-all border-l-4 card-dark"
                style={{ borderLeftColor: getCategoryColor(coach.category) }}
                onClick={() => setSelectedCoach(coach)}
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1 min-w-0">
                    <h3 className="font-semibold text-white truncate">{coach.title}</h3>
                    <span className={clsx(
                      'inline-block mt-1 px-2 py-0.5 text-xs font-medium rounded-full border',
                      getCategoryColorClass(coach.category)
                    )}>
                      {coach.category}
                    </span>
                  </div>
                  <div className="flex items-center gap-1 text-zinc-500">
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                    </svg>
                  </div>
                </div>
                {coach.description && (
                  <p className="text-sm text-zinc-400 line-clamp-2 mb-3">{coach.description}</p>
                )}
                <div className="flex items-center gap-4 text-xs text-zinc-500">
                  <span className="flex items-center gap-1">
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                    </svg>
                    {coach.token_count.toLocaleString()} tokens
                  </span>
                  <span className="flex items-center gap-1">
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                    </svg>
                    {coach.use_count} uses
                  </span>
                </div>
                {coach.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1 mt-3">
                    {coach.tags.slice(0, 3).map((tag) => (
                      <span key={tag} className="px-2 py-0.5 text-xs bg-white/10 text-zinc-400 rounded">
                        {tag}
                      </span>
                    ))}
                    {coach.tags.length > 3 && (
                      <span className="px-2 py-0.5 text-xs bg-white/10 text-zinc-500 rounded">
                        +{coach.tags.length - 3}
                      </span>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  }

  // Create/Edit form view
  if (isCreating || isEditing) {
    return (
      <div className="space-y-6">
        {/* Back button */}
        <button
          onClick={() => {
            setIsCreating(false);
            setIsEditing(false);
            setFormData(defaultFormData);
          }}
          className="flex items-center gap-2 text-zinc-400 hover:text-pierre-violet-light transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Coaches
        </button>

        <Card variant="dark">
          <h2 className="text-xl font-semibold text-white mb-6">
            {isCreating ? 'Create System Coach' : `Edit "${selectedCoach?.title}"`}
          </h2>

          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Title */}
            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Title <span className="text-pierre-red-400">*</span>
              </label>
              <input
                type="text"
                value={formData.title}
                onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                className="input-dark"
                placeholder="e.g., Marathon Training Coach"
                required
              />
            </div>

            {/* Description */}
            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Description
              </label>
              <textarea
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                className="input-dark"
                rows={2}
                placeholder="Brief description of the coach's specialty..."
              />
            </div>

            {/* System Prompt */}
            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                System Prompt <span className="text-pierre-red-400">*</span>
              </label>
              <textarea
                value={formData.system_prompt}
                onChange={(e) => setFormData({ ...formData, system_prompt: e.target.value })}
                className="input-dark font-mono text-sm"
                rows={8}
                placeholder="You are a professional marathon coach with expertise in..."
                required
              />
              <p className="mt-1 text-xs text-zinc-500">
                Estimated tokens: {estimateTokenCount(formData.system_prompt).toLocaleString()}
              </p>
            </div>

            {/* Category and Visibility */}
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Category
                </label>
                <select
                  value={formData.category}
                  onChange={(e) => setFormData({ ...formData, category: e.target.value })}
                  className="select-dark"
                >
                  {COACH_CATEGORIES.map((cat) => (
                    <option key={cat} value={cat}>{cat}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Visibility
                </label>
                <select
                  value={formData.visibility}
                  onChange={(e) => setFormData({ ...formData, visibility: e.target.value })}
                  className="select-dark"
                  disabled={isEditing}
                >
                  <option value="tenant">Tenant Only</option>
                  <option value="global">Global (All Tenants)</option>
                </select>
              </div>
            </div>

            {/* Tags */}
            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Tags
              </label>
              <input
                type="text"
                value={formData.tags}
                onChange={(e) => setFormData({ ...formData, tags: e.target.value })}
                className="input-dark"
                placeholder="marathon, endurance, beginner (comma-separated)"
              />
            </div>

            {/* Actions */}
            <div className="flex items-center gap-3 pt-4 border-t border-white/10">
              <Button
                type="submit"
                disabled={createMutation.isPending || updateMutation.isPending}
              >
                {createMutation.isPending || updateMutation.isPending ? (
                  <span className="flex items-center gap-2">
                    <div className="pierre-spinner w-4 h-4"></div>
                    Saving...
                  </span>
                ) : (
                  isCreating ? 'Create Coach' : 'Save Changes'
                )}
              </Button>
              <Button
                type="button"
                variant="secondary"
                onClick={() => {
                  setIsCreating(false);
                  setIsEditing(false);
                  setFormData(defaultFormData);
                }}
              >
                Cancel
              </Button>
            </div>
          </form>
        </Card>
      </div>
    );
  }

  // Coach detail view - TypeScript guard for selectedCoach
  if (!selectedCoach) {
    return null;
  }

  return (
    <div className="space-y-6">
      {/* Back button */}
      <button
        onClick={() => setSelectedCoach(null)}
        className="flex items-center gap-2 text-zinc-400 hover:text-pierre-violet-light transition-colors"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        Back to Coaches
      </button>

      {/* Coach Details Card */}
      <Card variant="dark">
        <div className="flex items-start justify-between mb-6">
          <div>
            <div className="flex items-center gap-3">
              <h2 className="text-2xl font-semibold text-white">{selectedCoach.title}</h2>
              <span className={clsx(
                'px-2 py-1 text-xs font-medium rounded-full border',
                getCategoryColorClass(selectedCoach.category)
              )}>
                {selectedCoach.category}
              </span>
            </div>
            {selectedCoach.description && (
              <p className="text-zinc-400 mt-2">{selectedCoach.description}</p>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              onClick={() => setIsEditing(true)}
            >
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
              Edit
            </Button>
            <Button
              variant="danger"
              onClick={handleDelete}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? 'Deleting...' : 'Delete'}
            </Button>
          </div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-4 gap-4 mb-6 p-4 bg-white/5 rounded-lg border border-white/10">
          <div className="text-center">
            <div className="text-2xl font-bold text-pierre-violet-light">{selectedCoach.token_count.toLocaleString()}</div>
            <div className="text-xs text-zinc-500">Tokens</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-pierre-activity">{selectedCoach.use_count}</div>
            <div className="text-xs text-zinc-500">Uses</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-pierre-nutrition">{assignments.length}</div>
            <div className="text-xs text-zinc-500">Assigned Users</div>
          </div>
          <div className="text-center">
            <div className="text-sm font-medium text-zinc-300">
              {selectedCoach.visibility === 'global' ? 'Global' : 'Tenant'}
            </div>
            <div className="text-xs text-zinc-500">Visibility</div>
          </div>
        </div>

        {/* System Prompt */}
        <div className="mb-6">
          <h3 className="text-sm font-medium text-zinc-300 mb-2">System Prompt</h3>
          <div className="p-4 bg-white/5 rounded-lg font-mono text-sm text-zinc-300 whitespace-pre-wrap max-h-48 overflow-y-auto border border-white/10 scrollbar-dark">
            {selectedCoach.system_prompt}
          </div>
        </div>

        {/* Tags */}
        {selectedCoach.tags.length > 0 && (
          <div className="mb-6">
            <h3 className="text-sm font-medium text-zinc-300 mb-2">Tags</h3>
            <div className="flex flex-wrap gap-2">
              {selectedCoach.tags.map((tag) => (
                <span key={tag} className="px-3 py-1 text-sm bg-white/10 text-zinc-300 rounded-full">
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Timestamps */}
        <div className="grid grid-cols-2 gap-4 text-sm text-zinc-500 pt-4 border-t border-white/10">
          <div>
            <span className="font-medium text-zinc-400">Created:</span>{' '}
            {new Date(selectedCoach.created_at).toLocaleString()}
          </div>
          <div>
            <span className="font-medium text-zinc-400">Last Updated:</span>{' '}
            {new Date(selectedCoach.updated_at).toLocaleString()}
          </div>
        </div>
      </Card>

      {/* Assignments Card */}
      <Card variant="dark">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white">User Assignments</h3>
          <Button onClick={() => setShowAssignModal(true)}>
            <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
            </svg>
            Assign Users
          </Button>
        </div>

        {assignments.length === 0 ? (
          <p className="text-zinc-500 text-center py-8">
            No users assigned to this coach yet. Click "Assign Users" to add access.
          </p>
        ) : (
          <div className="divide-y divide-white/10">
            {assignments.map((assignment) => (
              <div key={assignment.user_id} className="flex items-center justify-between py-3">
                <div>
                  <div className="font-medium text-white">
                    {assignment.user_email || assignment.user_id}
                  </div>
                  <div className="text-xs text-zinc-500">
                    Assigned {new Date(assignment.assigned_at).toLocaleDateString()}
                    {assignment.assigned_by && ` by ${assignment.assigned_by}`}
                  </div>
                </div>
                <button
                  onClick={() => handleUnassign(assignment.user_id)}
                  className="text-pierre-red-400 hover:text-pierre-red-300 transition-colors p-2"
                  title="Remove assignment"
                  disabled={unassignMutation.isPending}
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Assign Users Modal */}
      {showAssignModal && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
          <div className="bg-pierre-slate rounded-xl shadow-xl max-w-lg w-full mx-4 max-h-[80vh] flex flex-col border border-white/10">
            <div className="p-6 border-b border-white/10">
              <h3 className="text-lg font-semibold text-white">Assign Users to Coach</h3>
              <p className="text-sm text-zinc-400 mt-1">
                Select users to give access to "{selectedCoach.title}"
              </p>
            </div>

            <div className="flex-1 overflow-y-auto p-6 scrollbar-dark">
              {users.length === 0 ? (
                <p className="text-center text-zinc-500">Loading users...</p>
              ) : (
                <div className="space-y-2">
                  {users
                    .filter((user: User) => !assignedUserIds.has(user.id))
                    .map((user: User) => (
                      <label
                        key={user.id}
                        className={clsx(
                          'flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors',
                          selectedUserIds.includes(user.id)
                            ? 'bg-pierre-violet/20 border-2 border-pierre-violet'
                            : 'bg-white/5 border-2 border-transparent hover:bg-white/10'
                        )}
                      >
                        <input
                          type="checkbox"
                          checked={selectedUserIds.includes(user.id)}
                          onChange={(e) => {
                            if (e.target.checked) {
                              setSelectedUserIds([...selectedUserIds, user.id]);
                            } else {
                              setSelectedUserIds(selectedUserIds.filter(id => id !== user.id));
                            }
                          }}
                          className="w-4 h-4 text-pierre-violet focus:ring-pierre-violet rounded bg-white/10 border-white/20"
                        />
                        <div className="flex-1">
                          <div className="font-medium text-white">{user.email}</div>
                          {user.display_name && (
                            <div className="text-sm text-zinc-400">{user.display_name}</div>
                          )}
                        </div>
                        <span className={clsx(
                          'px-2 py-0.5 text-xs rounded-full',
                          user.user_status === 'active' ? 'bg-pierre-activity/20 text-pierre-activity' : 'bg-white/10 text-zinc-400'
                        )}>
                          {user.user_status}
                        </span>
                      </label>
                    ))}
                </div>
              )}
            </div>

            <div className="p-6 border-t border-white/10 flex items-center justify-between">
              <span className="text-sm text-zinc-400">
                {selectedUserIds.length} user{selectedUserIds.length !== 1 ? 's' : ''} selected
              </span>
              <div className="flex items-center gap-3">
                <Button
                  variant="secondary"
                  onClick={() => {
                    setShowAssignModal(false);
                    setSelectedUserIds([]);
                  }}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleAssign}
                  disabled={selectedUserIds.length === 0 || assignMutation.isPending}
                >
                  {assignMutation.isPending ? 'Assigning...' : 'Assign Selected'}
                </Button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// Helper function to get category accent color
function getCategoryColor(category: string): string {
  const colors: Record<string, string> = {
    Training: '#4ADE80',
    Nutrition: '#F59E0B',
    Recovery: '#6366F1',
    Recipes: '#F97316',
    Mobility: '#EC4899',
    Custom: '#8B5CF6',
  };
  // Normalize category to title case for lookup
  const normalized = category.charAt(0).toUpperCase() + category.slice(1).toLowerCase();
  return colors[normalized] || colors.Custom;
}

// Simple token count estimation (roughly 4 chars per token)
function estimateTokenCount(text: string): number {
  return Math.ceil(text.length / 4);
}
