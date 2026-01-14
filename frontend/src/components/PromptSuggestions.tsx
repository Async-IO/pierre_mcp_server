// ABOUTME: Coach selector component for the chat interface
// ABOUTME: Fetches user's available coaches from API and displays them grouped by category
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card } from './ui';

type Category = 'Training' | 'Nutrition' | 'Recovery' | 'Recipes' | 'Custom';

// Map categories to their gradient background classes
const CATEGORY_GRADIENTS: Record<Category, string> = {
  Training: 'bg-gradient-activity',
  Nutrition: 'bg-gradient-nutrition',
  Recovery: 'bg-gradient-recovery',
  Recipes: 'bg-gradient-nutrition',
  Custom: 'bg-pierre-violet',
};

// Map categories to icons
const CATEGORY_ICONS: Record<Category, string> = {
  Training: '🏃',
  Nutrition: '🥗',
  Recovery: '😴',
  Recipes: '👨‍🍳',
  Custom: '⚙️',
};

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
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl mx-auto mt-6">
        {[1, 2, 3, 4].map((i) => (
          <Card key={i} className="p-4 animate-pulse">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 rounded-lg bg-pierre-gray-200" />
              <div className="h-5 w-24 bg-pierre-gray-200 rounded" />
            </div>
            <div className="space-y-2">
              <div className="h-8 bg-pierre-gray-100 rounded-lg" />
              <div className="h-8 bg-pierre-gray-100 rounded-lg" />
            </div>
          </Card>
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-2xl mx-auto mt-6 text-center">
        <Card className="p-6 border-red-200 bg-red-50">
          <div className="text-red-600 mb-2">
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
            <p className="text-sm text-red-500 mt-1">
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
      <div className="max-w-2xl mx-auto mt-6 text-center text-pierre-gray-500">
        <p>No coaches available yet</p>
        <p className="text-sm mt-2">Ask your admin to assign some coaching personas to get started.</p>
      </div>
    );
  }

  // Group coaches by category
  const coachesByCategory = coaches.reduce<Record<string, Coach[]>>((acc, coach) => {
    const category = coach.category || 'Custom';
    if (!acc[category]) {
      acc[category] = [];
    }
    acc[category].push(coach);
    return acc;
  }, {});

  // Sort categories in preferred order
  const categoryOrder: Category[] = ['Training', 'Nutrition', 'Recovery', 'Recipes', 'Custom'];
  const sortedCategories = Object.keys(coachesByCategory).sort((a, b) => {
    const aIndex = categoryOrder.indexOf(a as Category);
    const bIndex = categoryOrder.indexOf(b as Category);
    if (aIndex === -1 && bIndex === -1) return a.localeCompare(b);
    if (aIndex === -1) return 1;
    if (bIndex === -1) return -1;
    return aIndex - bIndex;
  });

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl mx-auto mt-6">
      {sortedCategories.map((category) => (
        <Card key={category} className="p-4 hover:shadow-md transition-shadow">
          <div className="flex items-center gap-2 mb-3">
            <div
              className={`w-8 h-8 rounded-lg ${CATEGORY_GRADIENTS[category as Category] || 'bg-pierre-gray-200'} flex items-center justify-center text-lg`}
              role="img"
              aria-label={`${category} category`}
            >
              {CATEGORY_ICONS[category as Category] || '📌'}
            </div>
            <h3 className="font-medium text-pierre-gray-900">{category}</h3>
          </div>
          <div className="space-y-2">
            {coachesByCategory[category].map((coach) => (
              <button
                key={coach.id}
                type="button"
                onClick={() => {
                  // Record usage and start conversation with this coach
                  apiService.recordCoachUsage(coach.id).catch(() => {
                    // Silently ignore usage tracking errors
                  });
                  onSelectPrompt(
                    coach.description || `Chat with ${coach.title}`,
                    coach.id,
                    coach.system_prompt
                  );
                }}
                className="w-full text-left text-sm hover:bg-pierre-gray-50 rounded-lg px-3 py-2 transition-colors focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 group"
              >
                <div className="flex items-center justify-between">
                  <span className="font-medium text-pierre-gray-800 group-hover:text-pierre-violet">
                    {coach.title}
                  </span>
                  {coach.is_favorite && (
                    <span className="text-yellow-500">★</span>
                  )}
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
      ))}
    </div>
  );
}

// Hook to get coaches data for use in other components
export function useCoaches() {
  const { data: coachesData, isLoading, error } = useQuery({
    queryKey: ['user-coaches'],
    queryFn: () => apiService.getCoaches(),
    staleTime: 5 * 60 * 1000,
    retry: 2,
  });

  return {
    coaches: coachesData?.coaches ?? [],
    total: coachesData?.total ?? 0,
    isLoading,
    error,
  };
}

// Legacy hook for backwards compatibility - returns a default welcome prompt
export function useWelcomePrompt() {
  return {
    welcomePrompt: 'Ready to analyze your fitness',
    isLoading: false,
    error: null,
  };
}
