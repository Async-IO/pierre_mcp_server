// ABOUTME: Pre-configured prompt suggestions component for the chat interface
// ABOUTME: Fetches categorized prompt cards from API using Pierre's Three Pillars design system
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useQuery } from '@tanstack/react-query';
import { apiService } from '../services/api';
import { Card } from './ui';

type Pillar = 'activity' | 'nutrition' | 'recovery';

// Map pillars to their gradient background classes (from tailwind.config.cjs)
const PILLAR_GRADIENTS: Record<Pillar, string> = {
  activity: 'bg-gradient-activity',
  nutrition: 'bg-gradient-nutrition',
  recovery: 'bg-gradient-recovery',
};

interface PromptSuggestionsProps {
  onSelectPrompt: (prompt: string) => void;
}

export default function PromptSuggestions({ onSelectPrompt }: PromptSuggestionsProps) {
  const {
    data: promptsData,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['prompt-suggestions'],
    queryFn: () => apiService.getPromptSuggestions(),
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
            <p className="font-medium">Failed to load prompt suggestions</p>
            <p className="text-sm text-red-500 mt-1">
              {error instanceof Error ? error.message : 'Please try refreshing the page'}
            </p>
          </div>
        </Card>
      </div>
    );
  }

  if (!promptsData?.categories?.length) {
    return (
      <div className="max-w-2xl mx-auto mt-6 text-center text-pierre-gray-500">
        <p>No prompt suggestions available</p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl mx-auto mt-6">
      {promptsData.categories.map((category) => (
        <Card key={category.category_key} className="p-4 hover:shadow-md transition-shadow">
          <div className="flex items-center gap-2 mb-3">
            <div
              className={`w-8 h-8 rounded-lg ${PILLAR_GRADIENTS[category.pillar]} flex items-center justify-center text-lg`}
              role="img"
              aria-label={`${category.category_title} category`}
            >
              {category.category_icon}
            </div>
            <h3 className="font-medium text-pierre-gray-900">{category.category_title}</h3>
          </div>
          <div className="space-y-2">
            {category.prompts.map((prompt) => (
              <button
                key={prompt}
                type="button"
                onClick={() => onSelectPrompt(prompt)}
                className="w-full text-left text-sm text-pierre-gray-600 hover:text-pierre-violet hover:bg-pierre-gray-50 rounded-lg px-3 py-2 transition-colors focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50"
              >
                "{prompt}"
              </button>
            ))}
          </div>
        </Card>
      ))}
    </div>
  );
}

// Hook to get the welcome prompt for use in other components
export function useWelcomePrompt() {
  const { data: promptsData, isLoading, error } = useQuery({
    queryKey: ['prompt-suggestions'],
    queryFn: () => apiService.getPromptSuggestions(),
    staleTime: 5 * 60 * 1000,
    retry: 2,
  });

  return {
    welcomePrompt: promptsData?.welcome_prompt ?? null,
    isLoading,
    error,
  };
}
