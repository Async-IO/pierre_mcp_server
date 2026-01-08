// ABOUTME: Admin interface for managing prompt suggestions and welcome prompts
// ABOUTME: Provides CRUD operations for prompt categories with pillar-based organization
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card, Badge, Input, Button, Modal, Tabs } from './ui';

type Pillar = 'activity' | 'nutrition' | 'recovery';

interface PromptCategory {
  id: string;
  category_key: string;
  category_title: string;
  category_icon: string;
  pillar: string;
  prompts: string[];
  display_order: number;
  is_active: boolean;
}

// Map pillars to their gradient background classes
const PILLAR_GRADIENTS: Record<Pillar, string> = {
  activity: 'bg-gradient-to-r from-pierre-activity to-pierre-activity-light',
  nutrition: 'bg-gradient-to-r from-pierre-nutrition to-pierre-nutrition-light',
  recovery: 'bg-gradient-to-r from-pierre-recovery to-pierre-recovery-light',
};

const PILLAR_LABELS: Record<Pillar, string> = {
  activity: 'Activity',
  nutrition: 'Nutrition',
  recovery: 'Recovery',
};

export default function PromptsAdminTab() {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<'categories' | 'welcome' | 'system'>('categories');
  const [editingCategory, setEditingCategory] = useState<PromptCategory | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showDeleteModal, setShowDeleteModal] = useState<PromptCategory | null>(null);
  const [showResetModal, setShowResetModal] = useState(false);

  // Form state for create/edit
  const [formData, setFormData] = useState({
    category_key: '',
    category_title: '',
    category_icon: '',
    pillar: 'activity' as Pillar,
    prompts: [''],
    display_order: 0,
    is_active: true,
  });

  // Welcome prompt state
  const [welcomePromptText, setWelcomePromptText] = useState('');
  const [welcomePromptModified, setWelcomePromptModified] = useState(false);

  // System prompt state
  const [systemPromptText, setSystemPromptText] = useState('');
  const [systemPromptModified, setSystemPromptModified] = useState(false);

  // Fetch categories
  const { data: categories = [], isLoading, error } = useQuery({
    queryKey: ['admin-prompt-categories'],
    queryFn: () => apiService.getAdminPromptCategories(),
  });

  // Fetch welcome prompt
  const { data: welcomeData, isLoading: welcomeLoading } = useQuery({
    queryKey: ['admin-welcome-prompt'],
    queryFn: () => apiService.getAdminWelcomePrompt(),
    enabled: activeTab === 'welcome',
  });

  // Fetch system prompt
  const { data: systemData, isLoading: systemLoading } = useQuery({
    queryKey: ['admin-system-prompt'],
    queryFn: () => apiService.getAdminSystemPrompt(),
    enabled: activeTab === 'system',
  });

  // Initialize welcome prompt text when data loads
  const handleWelcomeDataLoad = useCallback(() => {
    if (welcomeData && !welcomePromptModified) {
      setWelcomePromptText(welcomeData.prompt_text);
    }
  }, [welcomeData, welcomePromptModified]);

  // Apply welcome data when it changes
  if (welcomeData && welcomePromptText === '' && !welcomePromptModified) {
    handleWelcomeDataLoad();
  }

  // Initialize system prompt text when data loads
  const handleSystemDataLoad = useCallback(() => {
    if (systemData && !systemPromptModified) {
      setSystemPromptText(systemData.prompt_text);
    }
  }, [systemData, systemPromptModified]);

  // Apply system data when it changes
  if (systemData && systemPromptText === '' && !systemPromptModified) {
    handleSystemDataLoad();
  }

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: {
      category_key: string;
      category_title: string;
      category_icon: string;
      pillar: Pillar;
      prompts: string[];
      display_order?: number;
    }) => apiService.createPromptCategory(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-prompt-categories'] });
      queryClient.invalidateQueries({ queryKey: ['prompt-suggestions'] });
      setShowCreateModal(false);
      resetForm();
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: {
      category_title?: string;
      category_icon?: string;
      pillar?: Pillar;
      prompts?: string[];
      display_order?: number;
      is_active?: boolean;
    } }) =>
      apiService.updatePromptCategory(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-prompt-categories'] });
      queryClient.invalidateQueries({ queryKey: ['prompt-suggestions'] });
      setEditingCategory(null);
      resetForm();
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (id: string) => apiService.deletePromptCategory(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-prompt-categories'] });
      queryClient.invalidateQueries({ queryKey: ['prompt-suggestions'] });
      setShowDeleteModal(null);
    },
  });

  // Welcome prompt mutation
  const welcomeMutation = useMutation({
    mutationFn: (text: string) => apiService.updateWelcomePrompt(text),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-welcome-prompt'] });
      queryClient.invalidateQueries({ queryKey: ['prompt-suggestions'] });
      setWelcomePromptModified(false);
    },
  });

  // System prompt mutation
  const systemMutation = useMutation({
    mutationFn: (text: string) => apiService.updateSystemPrompt(text),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-system-prompt'] });
      setSystemPromptModified(false);
    },
  });

  // Reset to defaults mutation
  const resetMutation = useMutation({
    mutationFn: () => apiService.resetPromptsToDefaults(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-prompt-categories'] });
      queryClient.invalidateQueries({ queryKey: ['admin-welcome-prompt'] });
      queryClient.invalidateQueries({ queryKey: ['admin-system-prompt'] });
      queryClient.invalidateQueries({ queryKey: ['prompt-suggestions'] });
      setShowResetModal(false);
      setWelcomePromptModified(false);
      setSystemPromptModified(false);
    },
  });

  const resetForm = () => {
    setFormData({
      category_key: '',
      category_title: '',
      category_icon: '',
      pillar: 'activity',
      prompts: [''],
      display_order: 0,
      is_active: true,
    });
  };

  const openEditModal = (category: PromptCategory) => {
    setEditingCategory(category);
    setFormData({
      category_key: category.category_key,
      category_title: category.category_title,
      category_icon: category.category_icon,
      pillar: category.pillar as Pillar,
      prompts: category.prompts.length > 0 ? category.prompts : [''],
      display_order: category.display_order,
      is_active: category.is_active,
    });
  };

  const handlePromptChange = (index: number, value: string) => {
    const newPrompts = [...formData.prompts];
    newPrompts[index] = value;
    setFormData({ ...formData, prompts: newPrompts });
  };

  const addPrompt = () => {
    setFormData({ ...formData, prompts: [...formData.prompts, ''] });
  };

  const removePrompt = (index: number) => {
    if (formData.prompts.length > 1) {
      const newPrompts = formData.prompts.filter((_, i) => i !== index);
      setFormData({ ...formData, prompts: newPrompts });
    }
  };

  const handleSubmit = () => {
    const cleanedPrompts = formData.prompts.filter((p) => p.trim() !== '');
    if (cleanedPrompts.length === 0) return;

    if (editingCategory) {
      updateMutation.mutate({
        id: editingCategory.id,
        data: {
          category_title: formData.category_title,
          category_icon: formData.category_icon,
          pillar: formData.pillar,
          prompts: cleanedPrompts,
          display_order: formData.display_order,
          is_active: formData.is_active,
        },
      });
    } else {
      createMutation.mutate({
        category_key: formData.category_key,
        category_title: formData.category_title,
        category_icon: formData.category_icon,
        pillar: formData.pillar,
        prompts: cleanedPrompts,
        display_order: formData.display_order,
      });
    }
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
          <p className="text-red-600">Failed to load prompt categories.</p>
          <p className="text-sm text-pierre-gray-500 mt-2">Please check your permissions and try again.</p>
        </div>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-pierre-gray-900">Prompt Management</h1>
          <p className="text-sm text-pierre-gray-500 mt-1">
            {categories.length} categories &bull; {categories.reduce((sum, cat) => sum + cat.prompts.length, 0)} prompts
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Button variant="outline" size="sm" onClick={() => setShowResetModal(true)}>
            Reset to Defaults
          </Button>
          <Button variant="primary" size="sm" onClick={() => setShowCreateModal(true)}>
            Add Category
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <Tabs
        tabs={[
          { id: 'categories', label: 'Prompt Categories' },
          { id: 'welcome', label: 'Welcome Prompt' },
          { id: 'system', label: 'System Prompt' },
        ]}
        activeTab={activeTab}
        onChange={(id: string) => setActiveTab(id as 'categories' | 'welcome' | 'system')}
      />

      {activeTab === 'categories' ? (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {categories
            .sort((a, b) => a.display_order - b.display_order)
            .map((category) => (
              <Card key={category.id} className={`relative ${!category.is_active ? 'opacity-60' : ''}`}>
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <div
                      className={`w-10 h-10 rounded-lg ${PILLAR_GRADIENTS[category.pillar as Pillar] || 'bg-pierre-gray-200'} flex items-center justify-center text-xl`}
                    >
                      {category.category_icon}
                    </div>
                    <div>
                      <h3 className="font-medium text-pierre-gray-900">{category.category_title}</h3>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge variant="secondary">
                          {PILLAR_LABELS[category.pillar as Pillar] || category.pillar}
                        </Badge>
                        {!category.is_active && <Badge variant="warning">Inactive</Badge>}
                        <span className="text-xs text-pierre-gray-400">Order: {category.display_order}</span>
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => openEditModal(category)}
                      className="p-1.5 text-pierre-gray-400 hover:text-pierre-violet rounded-lg hover:bg-pierre-gray-50 transition-colors"
                      title="Edit category"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                      </svg>
                    </button>
                    <button
                      onClick={() => setShowDeleteModal(category)}
                      className="p-1.5 text-pierre-gray-400 hover:text-red-500 rounded-lg hover:bg-red-50 transition-colors"
                      title="Delete category"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </button>
                  </div>
                </div>
                <div className="space-y-1.5">
                  {category.prompts.map((prompt, index) => (
                    <div
                      key={index}
                      className="text-sm text-pierre-gray-600 bg-pierre-gray-50 rounded-lg px-3 py-2"
                    >
                      &quot;{prompt}&quot;
                    </div>
                  ))}
                </div>
                <div className="mt-3 text-xs text-pierre-gray-400">
                  Key: <code className="bg-pierre-gray-100 px-1 rounded">{category.category_key}</code>
                </div>
              </Card>
            ))}
        </div>
      ) : activeTab === 'welcome' ? (
        <Card>
          <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Welcome Prompt</h2>
          <p className="text-sm text-pierre-gray-600 mb-4">
            This prompt is used when users first connect a fitness provider and want to see an analysis of their recent activity.
          </p>
          {welcomeLoading ? (
            <div className="flex justify-center py-8">
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-pierre-violet" />
            </div>
          ) : (
            <div className="space-y-4">
              <textarea
                value={welcomePromptText}
                onChange={(e) => {
                  setWelcomePromptText(e.target.value);
                  setWelcomePromptModified(true);
                }}
                rows={6}
                className="w-full px-4 py-3 border border-pierre-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent resize-none"
                placeholder="Enter the welcome prompt text..."
              />
              <div className="flex justify-end gap-3">
                {welcomePromptModified && (
                  <Button
                    variant="outline"
                    onClick={() => {
                      setWelcomePromptText(welcomeData?.prompt_text || '');
                      setWelcomePromptModified(false);
                    }}
                  >
                    Discard Changes
                  </Button>
                )}
                <Button
                  variant="primary"
                  onClick={() => welcomeMutation.mutate(welcomePromptText)}
                  disabled={!welcomePromptModified || welcomeMutation.isPending}
                >
                  {welcomeMutation.isPending ? 'Saving...' : 'Save Welcome Prompt'}
                </Button>
              </div>
              {welcomeMutation.isError && (
                <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
                  Failed to update welcome prompt. Please try again.
                </div>
              )}
              {welcomeMutation.isSuccess && (
                <div className="p-3 bg-green-50 text-green-600 rounded-lg text-sm">
                  Welcome prompt updated successfully.
                </div>
              )}
            </div>
          )}
        </Card>
      ) : (
        <Card>
          <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">System Prompt</h2>
          <p className="text-sm text-pierre-gray-600 mb-4">
            This is the system prompt that defines Pierre&apos;s behavior and personality when responding to users.
            It supports Markdown formatting.
          </p>
          {systemLoading ? (
            <div className="flex justify-center py-8">
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-pierre-violet" />
            </div>
          ) : (
            <div className="space-y-4">
              <textarea
                value={systemPromptText}
                onChange={(e) => {
                  setSystemPromptText(e.target.value);
                  setSystemPromptModified(true);
                }}
                rows={20}
                className="w-full px-4 py-3 border border-pierre-gray-200 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent resize-y"
                placeholder="Enter the system prompt text..."
              />
              <div className="flex justify-end gap-3">
                {systemPromptModified && (
                  <Button
                    variant="outline"
                    onClick={() => {
                      setSystemPromptText(systemData?.prompt_text || '');
                      setSystemPromptModified(false);
                    }}
                  >
                    Discard Changes
                  </Button>
                )}
                <Button
                  variant="primary"
                  onClick={() => systemMutation.mutate(systemPromptText)}
                  disabled={!systemPromptModified || systemMutation.isPending}
                >
                  {systemMutation.isPending ? 'Saving...' : 'Save System Prompt'}
                </Button>
              </div>
              {systemMutation.isError && (
                <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
                  Failed to update system prompt. Please try again.
                </div>
              )}
              {systemMutation.isSuccess && (
                <div className="p-3 bg-green-50 text-green-600 rounded-lg text-sm">
                  System prompt updated successfully.
                </div>
              )}
            </div>
          )}
        </Card>
      )}

      {/* Create/Edit Modal */}
      <Modal
        isOpen={showCreateModal || editingCategory !== null}
        onClose={() => {
          setShowCreateModal(false);
          setEditingCategory(null);
          resetForm();
        }}
        title={editingCategory ? 'Edit Prompt Category' : 'Create Prompt Category'}
      >
        <div className="space-y-4">
          {!editingCategory && (
            <Input
              label="Category Key"
              value={formData.category_key}
              onChange={(e) => setFormData({ ...formData, category_key: e.target.value })}
              placeholder="e.g., training_insights"
              className="w-full"
            />
          )}

          <Input
            label="Title"
            value={formData.category_title}
            onChange={(e) => setFormData({ ...formData, category_title: e.target.value })}
            placeholder="e.g., Training Insights"
            className="w-full"
          />

          <Input
            label="Icon (Emoji)"
            value={formData.category_icon}
            onChange={(e) => setFormData({ ...formData, category_icon: e.target.value })}
            placeholder="e.g., 🏃"
            className="w-24"
          />

          <div>
            <label className="block text-sm font-medium text-pierre-gray-700 mb-1">Pillar</label>
            <select
              value={formData.pillar}
              onChange={(e) => setFormData({ ...formData, pillar: e.target.value as Pillar })}
              className="w-full px-3 py-2 border border-pierre-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-pierre-violet"
            >
              <option value="activity">Activity</option>
              <option value="nutrition">Nutrition</option>
              <option value="recovery">Recovery</option>
            </select>
          </div>

          <div className="flex items-center gap-2">
            <Input
              label="Display Order"
              type="number"
              value={String(formData.display_order)}
              onChange={(e) => setFormData({ ...formData, display_order: parseInt(e.target.value, 10) || 0 })}
              className="w-24"
            />
            {editingCategory && (
              <div className="flex items-center gap-2 mt-6">
                <button
                  onClick={() => setFormData({ ...formData, is_active: !formData.is_active })}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-offset-2 ${
                    formData.is_active ? 'bg-pierre-activity' : 'bg-pierre-gray-300'
                  }`}
                  role="switch"
                  aria-checked={formData.is_active}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow-sm ${
                      formData.is_active ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
                <span className="text-sm text-pierre-gray-600">Active</span>
              </div>
            )}
          </div>

          <div>
            <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Prompts</label>
            <div className="space-y-2">
              {formData.prompts.map((prompt, index) => (
                <div key={index} className="flex items-center gap-2">
                  <Input
                    value={prompt}
                    onChange={(e) => handlePromptChange(index, e.target.value)}
                    placeholder="Enter a prompt..."
                    className="flex-1"
                  />
                  <button
                    onClick={() => removePrompt(index)}
                    className="p-2 text-pierre-gray-400 hover:text-red-500 transition-colors"
                    title="Remove prompt"
                    disabled={formData.prompts.length <= 1}
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              ))}
            </div>
            <Button variant="outline" size="sm" onClick={addPrompt} className="mt-2">
              Add Prompt
            </Button>
          </div>

          {(createMutation.isError || updateMutation.isError) && (
            <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
              Failed to save category. Please try again.
            </div>
          )}

          <div className="flex justify-end gap-3 pt-4">
            <Button
              variant="outline"
              onClick={() => {
                setShowCreateModal(false);
                setEditingCategory(null);
                resetForm();
              }}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleSubmit}
              disabled={createMutation.isPending || updateMutation.isPending}
            >
              {createMutation.isPending || updateMutation.isPending
                ? 'Saving...'
                : editingCategory
                ? 'Save Changes'
                : 'Create Category'}
            </Button>
          </div>
        </div>
      </Modal>

      {/* Delete Confirmation Modal */}
      <Modal
        isOpen={showDeleteModal !== null}
        onClose={() => setShowDeleteModal(null)}
        title="Delete Category"
      >
        <div className="space-y-4">
          <p className="text-pierre-gray-600">
            Are you sure you want to delete the category &quot;{showDeleteModal?.category_title}&quot;?
            This action cannot be undone.
          </p>

          {deleteMutation.isError && (
            <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
              Failed to delete category. Please try again.
            </div>
          )}

          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={() => setShowDeleteModal(null)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={() => showDeleteModal && deleteMutation.mutate(showDeleteModal.id)}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? 'Deleting...' : 'Delete Category'}
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
            Are you sure you want to reset all prompt categories, the welcome prompt, and the system prompt to their default values?
            This will remove any custom prompts you have created.
          </p>

          {resetMutation.isError && (
            <div className="p-3 bg-red-50 text-red-600 rounded-lg text-sm">
              Failed to reset prompts. Please try again.
            </div>
          )}

          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={() => setShowResetModal(false)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={() => resetMutation.mutate()}
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
