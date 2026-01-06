// ABOUTME: Pre-configured prompt suggestions component for the chat interface
// ABOUTME: Displays categorized prompt cards using Pierre's Three Pillars design system
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { Card } from './ui';

type Pillar = 'activity' | 'nutrition' | 'recovery';

interface PromptCategory {
  icon: string;
  title: string;
  prompts: string[];
  pillar: Pillar;
}

// Map pillars to their gradient background classes (from tailwind.config.cjs)
const PILLAR_GRADIENTS: Record<Pillar, string> = {
  activity: 'bg-gradient-activity',
  nutrition: 'bg-gradient-nutrition',
  recovery: 'bg-gradient-recovery',
};

const PROMPT_CATEGORIES: PromptCategory[] = [
  {
    icon: '🏃',
    title: 'Training',
    prompts: [
      'Am I ready for a hard workout today?',
      "What's my predicted marathon time?",
    ],
    pillar: 'activity', // Activity pillar (Emerald)
  },
  {
    icon: '🥗',
    title: 'Nutrition',
    prompts: [
      'How many calories should I eat today?',
      'What should I eat before my morning run?',
    ],
    pillar: 'nutrition', // Nutrition pillar (Amber)
  },
  {
    icon: '😴',
    title: 'Recovery',
    prompts: [
      'Do I need a rest day?',
      'Analyze my sleep quality',
    ],
    pillar: 'recovery', // Recovery pillar (Indigo)
  },
  {
    icon: '🍳',
    title: 'Recipes',
    prompts: [
      'Create a high-protein post-workout meal',
      'Show my saved recipes',
    ],
    pillar: 'nutrition', // Recipes fall under Nutrition pillar (Amber)
  },
];

// Featured prompt for first-time connected users - analyzes recent activities
export const WELCOME_ANALYSIS_PROMPT = 'List my last 20 activities with their dates, distances, and durations. Then give me a fitness summary with insights and recommendations';

interface PromptSuggestionsProps {
  onSelectPrompt: (prompt: string) => void;
}

export default function PromptSuggestions({ onSelectPrompt }: PromptSuggestionsProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl mx-auto mt-6">
      {PROMPT_CATEGORIES.map((category) => (
        <Card key={category.title} className="p-4 hover:shadow-md transition-shadow">
          <div className="flex items-center gap-2 mb-3">
            <div
              className={`w-8 h-8 rounded-lg ${PILLAR_GRADIENTS[category.pillar]} flex items-center justify-center text-lg`}
              role="img"
              aria-label={`${category.title} category`}
            >
              {category.icon}
            </div>
            <h3 className="font-medium text-pierre-gray-900">{category.title}</h3>
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
