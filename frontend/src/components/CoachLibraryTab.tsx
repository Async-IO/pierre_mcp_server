// ABOUTME: User Coach Library UI component for managing personal AI coaching personas
// ABOUTME: Provides CRUD operations for user-created coaches with category filtering and favorites
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import type { Coach } from '../types/api';
import { Card, Button } from './ui';
import { clsx } from 'clsx';

// Coach category options
const COACH_CATEGORIES = ['Training', 'Nutrition', 'Recovery', 'Recipes', 'Mobility', 'Custom'];

// Category colors for visual differentiation (matching ASY-35 specs)
const CATEGORY_COLORS: Record<string, string> = {
  Training: 'bg-pierre-activity/10 text-pierre-activity border-pierre-activity/20',
  Nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition border-pierre-nutrition/20',
  Recovery: 'bg-pierre-recovery/10 text-pierre-recovery border-pierre-recovery/20',
  Recipes: 'bg-pierre-yellow-500/10 text-pierre-yellow-600 border-pierre-yellow-500/20',
  Mobility: 'bg-pink-500/10 text-pink-600 border-pink-500/20',
  Custom: 'bg-pierre-violet/10 text-pierre-violet border-pierre-violet/20',
};

// Category border colors for left accent (matching mobile design)
const CATEGORY_BORDER_COLORS: Record<string, string> = {
  Training: '#10B981',
  Nutrition: '#F59E0B',
  Recovery: '#6366F1',
  Recipes: '#F97316',
  Mobility: '#EC4899',
  Custom: '#7C3AED',
};

// LLM context window size for percentage calculation
const CONTEXT_WINDOW_SIZE = 128000;

interface CoachFormData {
  title: string;
  description: string;
  system_prompt: string;
  category: string;
  tags: string;
}

const defaultFormData: CoachFormData = {
  title: '',
  description: '',
  system_prompt: '',
  category: 'Training',
  tags: '',
};

interface CoachLibraryTabProps {
  onBack?: () => void;
}

export default function CoachLibraryTab({ onBack }: CoachLibraryTabProps) {
  const queryClient = useQueryClient();
  const [selectedCoach, setSelectedCoach] = useState<Coach | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [formData, setFormData] = useState<CoachFormData>(defaultFormData);
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [favoritesOnly, setFavoritesOnly] = useState(false);

  // Fetch user's coaches (personal coaches only, not system)
  const { data: coachesData, isLoading: coachesLoading } = useQuery({
    queryKey: ['user-coaches', categoryFilter, favoritesOnly],
    queryFn: () => apiService.getCoaches({
      category: categoryFilter || undefined,
      favorites_only: favoritesOnly || undefined,
    }),
  });

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: typeof formData) => apiService.createCoach({
      title: data.title,
      description: data.description || undefined,
      system_prompt: data.system_prompt,
      category: data.category,
      tags: data.tags.split(',').map(t => t.trim()).filter(Boolean),
    }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setIsCreating(false);
      setFormData(defaultFormData);
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: typeof formData }) => apiService.updateCoach(id, {
      title: data.title,
      description: data.description || undefined,
      system_prompt: data.system_prompt,
      category: data.category,
      tags: data.tags.split(',').map(t => t.trim()).filter(Boolean),
    }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setIsEditing(false);
      setSelectedCoach(null);
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (id: string) => apiService.deleteCoach(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setSelectedCoach(null);
    },
  });

  // Toggle favorite mutation
  const favoriteMutation = useMutation({
    mutationFn: (id: string) => apiService.toggleCoachFavorite(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
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

  const handleToggleFavorite = (e: React.MouseEvent, coachId: string) => {
    e.stopPropagation();
    favoriteMutation.mutate(coachId);
  };

  // Filter to only personal coaches (not system coaches)
  const personalCoaches = (coachesData?.coaches || []).filter(coach => !coach.is_system);

  // Token count estimation (same formula as mobile: text.length / 4)
  const estimateTokenCount = (text: string): number => {
    return Math.ceil(text.length / 4);
  };

  // Context percentage calculation (tokens / 128000 * 100)
  const getContextPercentage = (tokens: number): string => {
    return ((tokens / CONTEXT_WINDOW_SIZE) * 100).toFixed(1);
  };

  // Coach list view
  if (!selectedCoach && !isCreating) {
    return (
      <div className="space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            {onBack && (
              <button
                onClick={onBack}
                className="flex items-center gap-2 text-pierre-gray-600 hover:text-pierre-violet transition-colors"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
                Back
              </button>
            )}
            <div>
              <h2 className="text-2xl font-semibold text-pierre-gray-900">My Coaches</h2>
              <p className="text-pierre-gray-600 mt-1">
                Create custom AI personas to get specialized fitness coaching.
              </p>
            </div>
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

        {/* Filters */}
        <div className="flex flex-wrap items-center gap-4">
          {/* Category filter */}
          <div className="flex items-center gap-2 overflow-x-auto pb-2">
            <button
              onClick={() => setCategoryFilter(null)}
              className={clsx(
                'px-3 py-1.5 text-sm font-medium rounded-full transition-colors whitespace-nowrap',
                categoryFilter === null
                  ? 'bg-pierre-violet text-white'
                  : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
              )}
            >
              All
            </button>
            {COACH_CATEGORIES.map((cat) => (
              <button
                key={cat}
                onClick={() => setCategoryFilter(cat)}
                className={clsx(
                  'px-3 py-1.5 text-sm font-medium rounded-full transition-colors whitespace-nowrap',
                  categoryFilter === cat
                    ? 'bg-pierre-violet text-white'
                    : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
                )}
              >
                {cat}
              </button>
            ))}
          </div>

          {/* Favorites toggle */}
          <button
            onClick={() => setFavoritesOnly(!favoritesOnly)}
            className={clsx(
              'flex items-center gap-1 px-3 py-1.5 text-sm font-medium rounded-full transition-colors',
              favoritesOnly
                ? 'bg-pierre-yellow-100 text-pierre-yellow-700'
                : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
            )}
          >
            <svg
              className={clsx('w-4 h-4', favoritesOnly ? 'fill-pierre-yellow-500' : 'fill-none')}
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"
              />
            </svg>
            Favorites
          </button>
        </div>

        {/* Coaches Grid */}
        {coachesLoading ? (
          <div className="flex justify-center py-12">
            <div className="pierre-spinner w-8 h-8"></div>
          </div>
        ) : personalCoaches.length === 0 ? (
          <Card className="text-center py-12">
            <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-pierre-gray-100 flex items-center justify-center">
              <svg className="w-8 h-8 text-pierre-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.121 17.804A13.937 13.937 0 0112 16c2.5 0 4.847.655 6.879 1.804M15 10a3 3 0 11-6 0 3 3 0 016 0zm6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <h3 className="text-lg font-medium text-pierre-gray-900 mb-2">
              {favoritesOnly ? 'No Favorite Coaches' : 'No Coaches Yet'}
            </h3>
            <p className="text-pierre-gray-600 mb-4">
              {favoritesOnly
                ? 'Star some coaches to see them here.'
                : 'Create your first coach to customize how Pierre helps you.'}
            </p>
            {!favoritesOnly && (
              <Button onClick={() => setIsCreating(true)}>Create Your First Coach</Button>
            )}
          </Card>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {personalCoaches.map((coach) => (
              <div
                key={coach.id}
                className="cursor-pointer hover:shadow-md transition-shadow border-l-4 card"
                style={{ borderLeftColor: CATEGORY_BORDER_COLORS[coach.category] || CATEGORY_BORDER_COLORS.Custom }}
                onClick={() => setSelectedCoach(coach)}
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1 min-w-0">
                    <h3 className="font-semibold text-pierre-gray-900 truncate">{coach.title}</h3>
                    <span className={clsx(
                      'inline-block mt-1 px-2 py-0.5 text-xs font-medium rounded-full border',
                      CATEGORY_COLORS[coach.category] || CATEGORY_COLORS.Custom
                    )}>
                      {coach.category}
                    </span>
                  </div>
                  <button
                    onClick={(e) => handleToggleFavorite(e, coach.id)}
                    className="text-pierre-gray-400 hover:text-pierre-yellow-500 transition-colors p-1"
                    title={coach.is_favorite ? 'Remove from favorites' : 'Add to favorites'}
                  >
                    <svg
                      className={clsx('w-5 h-5', coach.is_favorite ? 'fill-pierre-yellow-400 text-pierre-yellow-400' : 'fill-none')}
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"
                      />
                    </svg>
                  </button>
                </div>
                {coach.description && (
                  <p className="text-sm text-pierre-gray-600 line-clamp-2 mb-3">{coach.description}</p>
                )}
                <div className="flex items-center gap-4 text-xs text-pierre-gray-500">
                  <span className="flex items-center gap-1" title="Token count and context usage">
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                    </svg>
                    ~{coach.token_count.toLocaleString()} tokens ({getContextPercentage(coach.token_count)}%)
                  </span>
                  <span className="flex items-center gap-1" title="Times used">
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
                      <span key={tag} className="px-2 py-0.5 text-xs bg-pierre-gray-100 text-pierre-gray-600 rounded">
                        {tag}
                      </span>
                    ))}
                    {coach.tags.length > 3 && (
                      <span className="px-2 py-0.5 text-xs bg-pierre-gray-100 text-pierre-gray-500 rounded">
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
    const tokenCount = estimateTokenCount(formData.system_prompt);

    return (
      <div className="max-w-2xl mx-auto">
        <Card>
          {/* Card header with integrated back button - industry standard pattern */}
          <div className="flex items-center gap-3 pb-4 mb-6 border-b border-pierre-gray-100">
            <button
              onClick={() => {
                setIsCreating(false);
                setIsEditing(false);
                setFormData(defaultFormData);
                setSelectedCoach(null);
              }}
              className="p-1.5 rounded-lg text-pierre-gray-500 hover:text-pierre-violet hover:bg-pierre-gray-100 transition-colors"
              title="Back to coaches"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
            </button>
            <h2 className="text-xl font-semibold text-pierre-gray-900">
              {isCreating ? 'Create Coach' : `Edit "${selectedCoach?.title}"`}
            </h2>
          </div>

          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Title */}
            <div>
              <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                Title <span className="text-pierre-red-500">*</span>
              </label>
              <input
                type="text"
                value={formData.title}
                onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                className="w-full px-3 py-2 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                placeholder="e.g., Marathon Training Coach"
                maxLength={100}
                required
              />
            </div>

            {/* Category */}
            <div>
              <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                Category
              </label>
              <select
                value={formData.category}
                onChange={(e) => setFormData({ ...formData, category: e.target.value })}
                className="w-full px-3 py-2 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
              >
                {COACH_CATEGORIES.map((cat) => (
                  <option key={cat} value={cat}>{cat}</option>
                ))}
              </select>
            </div>

            {/* Description */}
            <div>
              <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                Description
              </label>
              <textarea
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                className="w-full px-3 py-2 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                rows={2}
                maxLength={500}
                placeholder="Brief description of the coach's specialty..."
              />
              <p className="mt-1 text-xs text-pierre-gray-500 text-right">
                {formData.description.length}/500
              </p>
            </div>

            {/* System Prompt */}
            <div>
              <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                System Prompt <span className="text-pierre-red-500">*</span>
              </label>
              <textarea
                value={formData.system_prompt}
                onChange={(e) => setFormData({ ...formData, system_prompt: e.target.value })}
                className="w-full px-3 py-2 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent font-mono text-sm"
                rows={8}
                maxLength={4000}
                placeholder="You are Pierre, an expert coach with deep knowledge of..."
                required
              />
              <div className="mt-1 flex items-center justify-between text-xs text-pierre-gray-500">
                <span>
                  📊 ~{tokenCount.toLocaleString()} tokens ({getContextPercentage(tokenCount)}% context)
                </span>
                <span>{formData.system_prompt.length}/4000</span>
              </div>
            </div>

            {/* Tags */}
            <div>
              <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                Tags
              </label>
              <input
                type="text"
                value={formData.tags}
                onChange={(e) => setFormData({ ...formData, tags: e.target.value })}
                className="w-full px-3 py-2 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                placeholder="marathon, endurance, beginner (comma-separated)"
              />
            </div>

            {/* Actions */}
            <div className="flex items-center gap-3 pt-4 border-t">
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
                  setSelectedCoach(null);
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
    <div className="max-w-3xl mx-auto">
      {/* Coach Details Card */}
      <Card>
        {/* Card header with integrated back button - industry standard pattern */}
        <div className="flex items-center justify-between pb-4 mb-6 border-b border-pierre-gray-100">
          <div className="flex items-center gap-3">
            <button
              onClick={() => setSelectedCoach(null)}
              className="p-1.5 rounded-lg text-pierre-gray-500 hover:text-pierre-violet hover:bg-pierre-gray-100 transition-colors"
              title="Back to coaches"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
            </button>
            <h2 className="text-2xl font-semibold text-pierre-gray-900">{selectedCoach.title}</h2>
            <span className={clsx(
              'px-2 py-1 text-xs font-medium rounded-full border',
              CATEGORY_COLORS[selectedCoach.category] || CATEGORY_COLORS.Custom
            )}>
              {selectedCoach.category}
            </span>
            <button
              onClick={(e) => handleToggleFavorite(e, selectedCoach.id)}
              className="text-pierre-gray-400 hover:text-pierre-yellow-500 transition-colors"
              title={selectedCoach.is_favorite ? 'Remove from favorites' : 'Add to favorites'}
            >
              <svg
                className={clsx('w-6 h-6', selectedCoach.is_favorite ? 'fill-pierre-yellow-400 text-pierre-yellow-400' : 'fill-none')}
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"
                />
              </svg>
            </button>
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

        {/* Description */}
        {selectedCoach.description && (
          <p className="text-pierre-gray-600 mb-6">{selectedCoach.description}</p>
        )}

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4 mb-6 p-4 bg-pierre-gray-50 rounded-lg">
          <div className="text-center">
            <div className="text-2xl font-bold text-pierre-violet">
              ~{selectedCoach.token_count.toLocaleString()}
            </div>
            <div className="text-xs text-pierre-gray-500">
              Tokens ({getContextPercentage(selectedCoach.token_count)}% context)
            </div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-pierre-activity">{selectedCoach.use_count}</div>
            <div className="text-xs text-pierre-gray-500">Uses</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-pierre-nutrition">
              {selectedCoach.is_favorite ? '★' : '☆'}
            </div>
            <div className="text-xs text-pierre-gray-500">
              {selectedCoach.is_favorite ? 'Favorite' : 'Not Favorite'}
            </div>
          </div>
        </div>

        {/* System Prompt */}
        <div className="mb-6">
          <h3 className="text-sm font-medium text-pierre-gray-700 mb-2">System Prompt</h3>
          <div className="p-4 bg-pierre-gray-50 rounded-lg font-mono text-sm text-pierre-gray-800 whitespace-pre-wrap max-h-48 overflow-y-auto">
            {selectedCoach.system_prompt}
          </div>
        </div>

        {/* Tags */}
        {selectedCoach.tags.length > 0 && (
          <div className="mb-6">
            <h3 className="text-sm font-medium text-pierre-gray-700 mb-2">Tags</h3>
            <div className="flex flex-wrap gap-2">
              {selectedCoach.tags.map((tag) => (
                <span key={tag} className="px-3 py-1 text-sm bg-pierre-gray-100 text-pierre-gray-700 rounded-full">
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Timestamps */}
        <div className="grid grid-cols-2 gap-4 text-sm text-pierre-gray-500 pt-4 border-t">
          <div>
            <span className="font-medium">Created:</span>{' '}
            {new Date(selectedCoach.created_at).toLocaleString()}
          </div>
          <div>
            <span className="font-medium">Last Updated:</span>{' '}
            {new Date(selectedCoach.updated_at).toLocaleString()}
          </div>
        </div>
      </Card>
    </div>
  );
}
