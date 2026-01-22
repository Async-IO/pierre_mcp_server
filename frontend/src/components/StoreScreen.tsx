// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Coach Store browse screen for discovering and installing coaches
// ABOUTME: Lists published coaches with category filters, search, and navigation to detail

import { useState, useEffect, useCallback, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { apiService } from '../services/api';

// Category filter options
const CATEGORY_FILTERS = [
  { key: 'all', label: 'All' },
  { key: 'training', label: 'Training' },
  { key: 'nutrition', label: 'Nutrition' },
  { key: 'recovery', label: 'Recovery' },
  { key: 'recipes', label: 'Recipes' },
  { key: 'mobility', label: 'Mobility' },
  { key: 'custom', label: 'Custom' },
] as const;

type CategoryFilter = typeof CATEGORY_FILTERS[number]['key'];

// Sort options
const SORT_OPTIONS = [
  { key: 'popular', label: 'Popular' },
  { key: 'newest', label: 'Newest' },
  { key: 'title', label: 'A-Z' },
] as const;

type SortOption = typeof SORT_OPTIONS[number]['key'];

// Coach category colors
const COACH_CATEGORY_COLORS: Record<string, string> = {
  training: 'bg-emerald-100 text-emerald-700',
  nutrition: 'bg-amber-100 text-amber-700',
  recovery: 'bg-indigo-100 text-indigo-700',
  recipes: 'bg-orange-100 text-orange-700',
  mobility: 'bg-pink-100 text-pink-700',
  custom: 'bg-violet-100 text-violet-700',
};

interface StoreCoach {
  id: string;
  title: string;
  description: string | null;
  category: string;
  tags: string[];
  sample_prompts: string[];
  token_count: number;
  install_count: number;
  icon_url: string | null;
  published_at: string | null;
  author_id: string | null;
}

interface StoreScreenProps {
  onSelectCoach: (coachId: string) => void;
  onBack: () => void;
}

export default function StoreScreen({ onSelectCoach, onBack }: StoreScreenProps) {
  const [selectedCategory, setSelectedCategory] = useState<CategoryFilter>('all');
  const [selectedSort, setSelectedSort] = useState<SortOption>('popular');
  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');

  // Debounce search query
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Fetch coaches based on filters or search
  const { data: browseData, isLoading: isBrowsing } = useQuery({
    queryKey: ['store-coaches', selectedCategory, selectedSort],
    queryFn: () =>
      apiService.browseStoreCoaches({
        category: selectedCategory === 'all' ? undefined : selectedCategory,
        sort_by: selectedSort,
        limit: 50,
      }),
    enabled: !debouncedSearch,
    staleTime: 30_000,
  });

  const { data: searchData, isLoading: isSearching } = useQuery({
    queryKey: ['store-search', debouncedSearch],
    queryFn: () => apiService.searchStoreCoaches(debouncedSearch, 50),
    enabled: !!debouncedSearch,
    staleTime: 30_000,
  });

  const coaches = useMemo(() => {
    if (debouncedSearch && searchData) {
      return searchData.coaches;
    }
    return browseData?.coaches ?? [];
  }, [debouncedSearch, searchData, browseData]);

  const isLoading = debouncedSearch ? isSearching : isBrowsing;

  const handleClearSearch = useCallback(() => {
    setSearchQuery('');
    setDebouncedSearch('');
  }, []);

  return (
    <div className="h-full flex flex-col bg-white">
      {/* Header */}
      <div className="flex items-center gap-4 px-6 py-4 border-b border-pierre-gray-200">
        <button
          onClick={onBack}
          className="p-2 rounded-lg hover:bg-pierre-gray-100 transition-colors"
          title="Back to Chat"
        >
          <svg className="w-5 h-5 text-pierre-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <div>
          <h1 className="text-xl font-semibold text-pierre-gray-900">Coach Store</h1>
          <p className="text-sm text-pierre-gray-500">Discover AI coaching assistants</p>
        </div>
      </div>

      {/* Search Bar */}
      <div className="px-6 py-4 border-b border-pierre-gray-100">
        <div className="relative">
          <svg
            className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-pierre-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            type="text"
            placeholder="Search coaches..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 pr-10 py-2.5 bg-pierre-gray-50 border border-pierre-gray-200 rounded-lg text-sm text-pierre-gray-900 placeholder-pierre-gray-500 focus:outline-none focus:ring-2 focus:ring-pierre-violet/20 focus:border-pierre-violet transition-colors"
          />
          {searchQuery && (
            <button
              onClick={handleClearSearch}
              className="absolute right-3 top-1/2 transform -translate-y-1/2 text-pierre-gray-400 hover:text-pierre-gray-600"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
          {isSearching && (
            <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
              <div className="w-4 h-4 border-2 border-pierre-violet border-t-transparent rounded-full animate-spin" />
            </div>
          )}
        </div>
      </div>

      {/* Category Filters */}
      <div className="px-6 py-3 border-b border-pierre-gray-100 overflow-x-auto">
        <div className="flex items-center gap-2">
          {CATEGORY_FILTERS.map((filter) => (
            <button
              key={filter.key}
              onClick={() => setSelectedCategory(filter.key)}
              className={clsx(
                'px-4 py-1.5 text-sm font-medium rounded-full whitespace-nowrap transition-colors',
                selectedCategory === filter.key
                  ? 'bg-pierre-violet text-white'
                  : 'bg-pierre-gray-100 text-pierre-gray-600 hover:bg-pierre-gray-200'
              )}
            >
              {filter.label}
            </button>
          ))}
        </div>
      </div>

      {/* Sort Options */}
      <div className="px-6 py-2 bg-pierre-gray-50 border-b border-pierre-gray-100 flex items-center gap-3">
        <span className="text-sm text-pierre-gray-500">Sort by:</span>
        {SORT_OPTIONS.map((option) => (
          <button
            key={option.key}
            onClick={() => setSelectedSort(option.key)}
            className={clsx(
              'px-3 py-1 text-sm rounded transition-colors',
              selectedSort === option.key
                ? 'bg-pierre-violet/10 text-pierre-violet font-medium'
                : 'text-pierre-gray-600 hover:text-pierre-violet'
            )}
          >
            {option.label}
          </button>
        ))}
      </div>

      {/* Coach Grid */}
      <div className="flex-1 overflow-y-auto p-6">
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <div className="text-center">
              <div className="w-8 h-8 border-2 border-pierre-violet border-t-transparent rounded-full animate-spin mx-auto" />
              <p className="mt-3 text-sm text-pierre-gray-500">Loading coaches...</p>
            </div>
          </div>
        ) : coaches.length === 0 ? (
          <div className="text-center py-12">
            <svg
              className="w-12 h-12 text-pierre-gray-300 mx-auto mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <h3 className="text-lg font-medium text-pierre-gray-900">
              {searchQuery ? 'No coaches found' : 'Store is empty'}
            </h3>
            <p className="text-sm text-pierre-gray-500 mt-1">
              {searchQuery
                ? `No coaches match "${searchQuery}"`
                : 'No published coaches available yet'}
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {coaches.map((coach) => (
              <CoachCard key={coach.id} coach={coach} onClick={() => onSelectCoach(coach.id)} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

interface CoachCardProps {
  coach: StoreCoach;
  onClick: () => void;
}

function CoachCard({ coach, onClick }: CoachCardProps) {
  const categoryColors = COACH_CATEGORY_COLORS[coach.category] ?? 'bg-gray-100 text-gray-700';

  return (
    <button
      onClick={onClick}
      className="text-left p-4 bg-white border border-pierre-gray-200 rounded-xl hover:border-pierre-violet/30 hover:shadow-md transition-all duration-200 group"
    >
      {/* Header with category and install count */}
      <div className="flex items-center justify-between mb-2">
        <span className={clsx('px-2.5 py-0.5 text-xs font-medium rounded-full capitalize', categoryColors)}>
          {coach.category}
        </span>
        <span className="text-xs text-pierre-gray-500">
          {coach.install_count} {coach.install_count === 1 ? 'user' : 'users'}
        </span>
      </div>

      {/* Title */}
      <h3 className="font-semibold text-pierre-gray-900 mb-1 line-clamp-1 group-hover:text-pierre-violet transition-colors">
        {coach.title}
      </h3>

      {/* Description */}
      {coach.description && (
        <p className="text-sm text-pierre-gray-600 line-clamp-2 mb-3">{coach.description}</p>
      )}

      {/* Tags */}
      {coach.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {coach.tags.slice(0, 3).map((tag, index) => (
            <span
              key={index}
              className="px-2 py-0.5 text-xs bg-pierre-gray-100 text-pierre-gray-600 rounded"
            >
              {tag}
            </span>
          ))}
          {coach.tags.length > 3 && (
            <span className="text-xs text-pierre-gray-500">+{coach.tags.length - 3}</span>
          )}
        </div>
      )}
    </button>
  );
}
