// ABOUTME: Coach selector component for the chat interface
// ABOUTME: Fetches user's available coaches from API and displays them with help tooltip
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card } from './ui';

interface Coach {
  id: string;
  title: string;
  description?: string;
  system_prompt: string;
  category: string;
  tags: string[];
  token_count: number;
  is_favorite: boolean;
  use_count: number;
  last_used_at?: string;
  is_system: boolean;
  is_assigned: boolean;
}

interface PromptSuggestionsProps {
  onSelectPrompt: (prompt: string, coachId?: string, systemPrompt?: string) => void;
}

export default function PromptSuggestions({ onSelectPrompt }: PromptSuggestionsProps) {
  const {
    data: coachesData,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['user-coaches'],
    queryFn: () => apiService.getCoaches(),
    staleTime: 5 * 60 * 1000, // Cache for 5 minutes
    retry: 2,
  });

  if (isLoading) {
    return (
      <Card className="p-3 mt-4 animate-pulse">
        <div className="flex items-center gap-2 mb-3">
          <div className="w-8 h-8 rounded-lg bg-pierre-gray-200" />
          <div className="h-5 w-20 bg-pierre-gray-200 rounded" />
          <div className="w-5 h-5 rounded-full bg-pierre-gray-200" />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i} className="h-20 bg-pierre-gray-100 rounded-xl" />
          ))}
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <div className="mt-4 text-center">
        <Card className="p-6 border-pierre-red-200 bg-pierre-red-50">
          <div className="text-pierre-red-600 mb-2">
            <svg
              className="w-8 h-8 mx-auto mb-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <p className="font-medium">Failed to load coaches</p>
            <p className="text-sm text-pierre-red-500 mt-1">
              {error instanceof Error ? error.message : 'Please try refreshing the page'}
            </p>
          </div>
        </Card>
      </div>
    );
  }

  const coaches = coachesData?.coaches || [];

  if (coaches.length === 0) {
    return (
      <div className="mt-4 text-center text-pierre-gray-500">
        <p>No coaches available yet</p>
        <p className="text-sm mt-2">Ask your admin to assign some coaching personas to get started.</p>
      </div>
    );
  }

  // Sort coaches: favorites first, then by use_count
  const sortedCoaches = [...coaches].sort((a, b) => {
    if (a.is_favorite !== b.is_favorite) return a.is_favorite ? -1 : 1;
    return b.use_count - a.use_count;
  });

  return (
    <CoachesSection coaches={sortedCoaches} onSelectPrompt={onSelectPrompt} />
  );
}

// Help tooltip popover component
function HelpTooltip({ isVisible, onClose }: { isVisible: boolean; onClose: () => void }) {
  if (!isVisible) return null;

  return (
    <div className="absolute top-full left-0 mt-2 z-50">
      <div className="bg-white rounded-lg shadow-lg border border-pierre-gray-200 p-4 max-w-sm">
        <div className="flex items-start justify-between gap-2">
          <div>
            <p className="text-sm text-pierre-gray-700 font-medium mb-2">
              AI Coaching Personas
            </p>
            <p className="text-xs text-pierre-gray-500">
              Coaches are specialized AI assistants trained to help with specific aspects of your fitness journey.
              Select a coach to start a conversation focused on their expertise area.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="text-pierre-gray-400 hover:text-pierre-gray-600 flex-shrink-0"
            aria-label="Close help"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}

// Coaches section with header, help button, and add coach functionality
function CoachesSection({
  coaches,
  onSelectPrompt
}: {
  coaches: Coach[];
  onSelectPrompt: (prompt: string, coachId?: string, systemPrompt?: string) => void;
}) {
  const [showHelp, setShowHelp] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [formData, setFormData] = useState({
    title: '',
    description: '',
    system_prompt: '',
    category: 'Training',
  });
  const queryClient = useQueryClient();

  const createMutation = useMutation({
    mutationFn: (data: typeof formData) => apiService.createCoach(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setShowCreateForm(false);
      setFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
    },
  });

  const handleCreateSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData.title.trim() || !formData.system_prompt.trim()) return;
    createMutation.mutate(formData);
  };

  // Token estimation for display
  const estimatedTokens = Math.ceil(formData.system_prompt.length / 4);
  const contextPercentage = ((estimatedTokens / 128000) * 100).toFixed(1);

  return (
    <Card className="p-3 mt-4">
      {/* Header with help button */}
      <div className="flex items-center gap-2 mb-3 relative">
        <div
          className="w-8 h-8 rounded-lg bg-gradient-to-br from-pierre-violet to-purple-600 flex items-center justify-center"
          role="img"
          aria-label="Coaches"
        >
          <svg className="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
          </svg>
        </div>
        <h3 className="font-medium text-pierre-gray-900">Coaches</h3>
        <button
          type="button"
          onClick={() => setShowHelp(!showHelp)}
          className="w-5 h-5 rounded-full bg-pierre-gray-100 hover:bg-pierre-gray-200 flex items-center justify-center text-pierre-gray-500 hover:text-pierre-gray-700 transition-colors"
          aria-label="What are coaches?"
        >
          <span className="text-xs font-medium">?</span>
        </button>
        <HelpTooltip isVisible={showHelp} onClose={() => setShowHelp(false)} />
      </div>

      {/* Create Coach Form (when visible) */}
      {showCreateForm && (
        <form onSubmit={handleCreateSubmit} className="mb-4 p-4 border border-pierre-violet/30 rounded-xl bg-pierre-violet/5">
          <h4 className="font-medium text-pierre-gray-900 mb-3">Create New Coach</h4>
          <div className="space-y-3">
            <div>
              <input
                type="text"
                placeholder="Coach name (e.g., Marathon Training Coach)"
                value={formData.title}
                onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                required
              />
            </div>
            <div>
              <input
                type="text"
                placeholder="Brief description (optional)"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
              />
            </div>
            <div>
              <textarea
                placeholder="System prompt - Define your coach's personality and expertise..."
                value={formData.system_prompt}
                onChange={(e) => setFormData({ ...formData, system_prompt: e.target.value })}
                rows={3}
                className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent resize-none"
                required
              />
              {formData.system_prompt && (
                <p className="text-xs text-pierre-gray-500 mt-1">
                  ~{estimatedTokens} tokens ({contextPercentage}% of context)
                </p>
              )}
            </div>
            <div>
              <select
                value={formData.category}
                onChange={(e) => setFormData({ ...formData, category: e.target.value })}
                className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
              >
                <option value="Training">Training</option>
                <option value="Nutrition">Nutrition</option>
                <option value="Recovery">Recovery</option>
                <option value="Recipes">Recipes</option>
                <option value="Analysis">Analysis</option>
                <option value="Custom">Custom</option>
              </select>
            </div>
          </div>
          <div className="flex gap-2 mt-4">
            <button
              type="submit"
              disabled={createMutation.isPending || !formData.title.trim() || !formData.system_prompt.trim()}
              className="flex-1 px-4 py-2 text-sm font-medium text-white bg-pierre-violet rounded-lg hover:bg-pierre-violet/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {createMutation.isPending ? 'Creating...' : 'Create Coach'}
            </button>
            <button
              type="button"
              onClick={() => {
                setShowCreateForm(false);
                setFormData({ title: '', description: '', system_prompt: '', category: 'Training' });
              }}
              className="px-4 py-2 text-sm font-medium text-pierre-gray-600 bg-pierre-gray-100 rounded-lg hover:bg-pierre-gray-200 transition-colors"
            >
              Cancel
            </button>
          </div>
          {createMutation.isError && (
            <p className="text-xs text-pierre-red-500 mt-2">
              Failed to create coach. Please try again.
            </p>
          )}
        </form>
      )}

      {/* Coach list - responsive grid: 1 col mobile, 2 col tablet, 3 col desktop */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {/* Add Coach Card */}
        {!showCreateForm && (
          <button
            type="button"
            onClick={() => setShowCreateForm(true)}
            className="text-left text-sm rounded-xl border-2 border-dashed border-pierre-gray-300 hover:border-pierre-violet hover:bg-pierre-violet/5 px-4 py-3 transition-all focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 group"
          >
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-lg bg-pierre-gray-100 group-hover:bg-pierre-violet/10 flex items-center justify-center transition-colors">
                <svg className="w-5 h-5 text-pierre-gray-400 group-hover:text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
              </div>
              <span className="font-medium text-pierre-gray-500 group-hover:text-pierre-violet">
                Add Coach
              </span>
            </div>
            <p className="text-pierre-gray-400 text-xs mt-1">
              Create a custom coaching persona
            </p>
          </button>
        )}

        {/* Existing Coaches */}
        {coaches.map((coach) => (
          <button
            key={coach.id}
            type="button"
            onClick={() => {
              apiService.recordCoachUsage(coach.id).catch(() => {
                // Silently ignore usage tracking errors
              });
              onSelectPrompt(
                coach.description || `Chat with ${coach.title}`,
                coach.id,
                coach.system_prompt
              );
            }}
            className="text-left text-sm rounded-xl border border-pierre-gray-200 hover:border-pierre-violet hover:bg-pierre-violet/5 px-4 py-3 transition-all focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 group hover:shadow-sm"
          >
            <div className="flex items-center justify-between">
              <span className="font-medium text-pierre-gray-800 group-hover:text-pierre-violet">
                {coach.title}
              </span>
              <div className="flex items-center gap-1">
                {coach.is_favorite && (
                  <span className="text-pierre-yellow-500">★</span>
                )}
                <span className={`text-xs px-1.5 py-0.5 rounded ${getCategoryBadgeClass(coach.category)}`}>
                  {getCategoryIcon(coach.category)}
                </span>
              </div>
            </div>
            {coach.description && (
              <p className="text-pierre-gray-500 text-xs mt-0.5 line-clamp-2">
                {coach.description}
              </p>
            )}
            <div className="flex items-center gap-2 mt-1 text-xs text-pierre-gray-400">
              {coach.is_system && (
                <span className="bg-pierre-violet bg-opacity-10 text-pierre-violet px-1.5 py-0.5 rounded">
                  System
                </span>
              )}
              {coach.use_count > 0 && (
                <span>Used {coach.use_count}x</span>
              )}
            </div>
          </button>
        ))}
      </div>
    </Card>
  );
}

// Helper functions for category styling
function getCategoryBadgeClass(category: string): string {
  const classes: Record<string, string> = {
    training: 'bg-pierre-green-100 text-pierre-green-700',
    nutrition: 'bg-pierre-nutrition/10 text-pierre-nutrition',
    recovery: 'bg-pierre-blue-100 text-pierre-blue-700',
    recipes: 'bg-pierre-yellow-100 text-pierre-yellow-700',
    analysis: 'bg-pierre-violet/10 text-pierre-violet',
    custom: 'bg-pierre-gray-100 text-pierre-gray-600',
  };
  return classes[category.toLowerCase()] || classes.custom;
}

function getCategoryIcon(category: string): string {
  const icons: Record<string, string> = {
    training: '🏃',
    nutrition: '🥗',
    recovery: '😴',
    recipes: '👨‍🍳',
    analysis: '📊',
    custom: '⚙️',
  };
  return icons[category.toLowerCase()] || icons.custom;
}
