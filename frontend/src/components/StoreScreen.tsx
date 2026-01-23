// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Coach Store browse screen for discovering and installing coaches
// ABOUTME: Lists published coaches with category filters, search, and navigation to detail

import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { useQuery, useInfiniteQuery } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { Compass } from 'lucide-react';
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

// Coach category colors (dark theme)
const COACH_CATEGORY_COLORS: Record<string, string> = {
  training: 'bg-emerald-500/20 text-emerald-400',
  nutrition: 'bg-amber-500/20 text-amber-400',
  recovery: 'bg-indigo-500/20 text-indigo-400',
  recipes: 'bg-orange-500/20 text-orange-400',
  mobility: 'bg-pink-500/20 text-pink-400',
  custom: 'bg-violet-500/20 text-violet-400',
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
  onBack?: () => void;
}

export default function StoreScreen({ onSelectCoach }: StoreScreenProps) {
  const [selectedCategory, setSelectedCategory] = useState<CategoryFilter>('all');
  const [selectedSort, setSelectedSort] = useState<SortOption>('popular');
  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const loadMoreRef = useRef<HTMLDivElement>(null);

  // Debounce search query
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Infinite query for cursor-based pagination
  const {
    data: browseData,
    isLoading: isBrowsing,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
  } = useInfiniteQuery({
    queryKey: ['store-coaches', selectedCategory, selectedSort],
    queryFn: ({ pageParam }) =>
      apiService.browseStoreCoaches({
        category: selectedCategory === 'all' ? undefined : selectedCategory,
        sort_by: selectedSort,
        limit: 20,
        cursor: pageParam,
      }),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (lastPage) => lastPage.has_more ? lastPage.next_cursor ?? undefined : undefined,
    enabled: !debouncedSearch,
    staleTime: 30_000,
  });

  const { data: searchData, isLoading: isSearching } = useQuery({
    queryKey: ['store-search', debouncedSearch],
    queryFn: () => apiService.searchStoreCoaches(debouncedSearch, 50),
    enabled: !!debouncedSearch,
    staleTime: 30_000,
  });

  // Flatten pages for rendering
  const coaches = useMemo(() => {
    if (debouncedSearch && searchData) {
      return searchData.coaches;
    }
    return browseData?.pages.flatMap(page => page.coaches) ?? [];
  }, [debouncedSearch, searchData, browseData]);

  const isLoading = debouncedSearch ? isSearching : isBrowsing;

  // Intersection Observer for infinite scroll
  useEffect(() => {
    if (debouncedSearch) return; // Don't infinite scroll for search results

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasNextPage && !isFetchingNextPage) {
          fetchNextPage();
        }
      },
      { threshold: 0.1 }
    );

    if (loadMoreRef.current) {
      observer.observe(loadMoreRef.current);
    }

    return () => observer.disconnect();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage, debouncedSearch]);

  const handleClearSearch = useCallback(() => {
    setSearchQuery('');
    setDebouncedSearch('');
  }, []);

  return (
    <div className="h-full flex flex-col bg-pierre-dark">
      {/* Header - matches Chat and My Coaches layout */}
      <div className="p-6 border-b border-white/5 flex items-center justify-between flex-shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 flex items-center justify-center rounded-xl bg-gradient-to-br from-pierre-activity to-pierre-activity-dark text-white shadow-glow-sm">
            <Compass className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-xl font-semibold text-white">Discover</h2>
            <p className="text-sm text-zinc-400">Find AI coaching assistants</p>
          </div>
        </div>
      </div>

      {/* Search Bar */}
      <div className="px-6 py-4 border-b border-white/10">
        <div className="relative">
          <svg
            className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-500"
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
            className="w-full pl-10 pr-10 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-pierre-violet/30 focus:border-pierre-violet transition-colors"
          />
          {searchQuery && (
            <button
              onClick={handleClearSearch}
              className="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-gray-300"
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
      <div className="px-6 py-3 border-b border-white/10 overflow-x-auto">
        <div className="flex items-center gap-2">
          {CATEGORY_FILTERS.map((filter) => (
            <button
              key={filter.key}
              onClick={() => setSelectedCategory(filter.key)}
              className={clsx(
                'px-4 py-1.5 text-sm font-medium rounded-full whitespace-nowrap transition-colors',
                selectedCategory === filter.key
                  ? 'bg-pierre-violet text-white shadow-glow-sm'
                  : 'bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-300'
              )}
            >
              {filter.label}
            </button>
          ))}
        </div>
      </div>

      {/* Sort Options */}
      <div className="px-6 py-2 bg-white/5 border-b border-white/10 flex items-center gap-3">
        <span className="text-sm text-gray-500">Sort by:</span>
        {SORT_OPTIONS.map((option) => (
          <button
            key={option.key}
            onClick={() => setSelectedSort(option.key)}
            className={clsx(
              'px-3 py-1 text-sm rounded transition-colors',
              selectedSort === option.key
                ? 'bg-pierre-violet/20 text-pierre-violet font-medium'
                : 'text-gray-400 hover:text-pierre-violet'
            )}
          >
            {option.label}
          </button>
        ))}
      </div>

      {/* Coach Grid */}
      <div className="flex-1 overflow-y-auto p-6 sidebar-scroll">
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <div className="text-center">
              <div className="w-8 h-8 border-2 border-pierre-violet border-t-transparent rounded-full animate-spin mx-auto" />
              <p className="mt-3 text-sm text-gray-500">Loading coaches...</p>
            </div>
          </div>
        ) : coaches.length === 0 ? (
          <div className="text-center py-12">
            <svg
              className="w-12 h-12 text-gray-600 mx-auto mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <h3 className="text-lg font-medium text-white">
              {searchQuery ? 'No coaches found' : 'Store is empty'}
            </h3>
            <p className="text-sm text-gray-500 mt-1">
              {searchQuery
                ? `No coaches match "${searchQuery}"`
                : 'No published coaches available yet'}
            </p>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {coaches.map((coach) => (
                <CoachCard key={coach.id} coach={coach} onClick={() => onSelectCoach(coach.id)} />
              ))}
            </div>

            {/* Infinite scroll trigger */}
            {!debouncedSearch && (
              <div ref={loadMoreRef} className="py-8 flex justify-center">
                {isFetchingNextPage ? (
                  <div className="flex items-center gap-2">
                    <div className="w-5 h-5 border-2 border-pierre-violet border-t-transparent rounded-full animate-spin" />
                    <span className="text-sm text-gray-500">Loading more...</span>
                  </div>
                ) : hasNextPage ? (
                  <span className="text-sm text-gray-500">Scroll for more</span>
                ) : coaches.length > 0 ? (
                  <span className="text-sm text-gray-500">You've seen all coaches</span>
                ) : null}
              </div>
            )}
          </>
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
  const categoryColors = COACH_CATEGORY_COLORS[coach.category] ?? 'bg-gray-500/20 text-gray-400';

  return (
    <button
      onClick={onClick}
      className="text-left p-4 bg-white/5 border border-white/10 rounded-xl hover:border-pierre-violet/40 hover:bg-white/10 hover:shadow-glow-sm transition-all duration-200 group"
    >
      {/* Header with category and install count */}
      <div className="flex items-center justify-between mb-2">
        <span className={clsx('px-2.5 py-0.5 text-xs font-medium rounded-full capitalize', categoryColors)}>
          {coach.category}
        </span>
        <span className="text-xs text-gray-500">
          {coach.install_count} {coach.install_count === 1 ? 'user' : 'users'}
        </span>
      </div>

      {/* Title */}
      <h3 className="font-semibold text-white mb-1 line-clamp-1 group-hover:text-pierre-violet transition-colors">
        {coach.title}
      </h3>

      {/* Description */}
      {coach.description && (
        <p className="text-sm text-gray-400 line-clamp-2 mb-3">{coach.description}</p>
      )}

      {/* Tags */}
      {coach.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {coach.tags.slice(0, 3).map((tag, index) => (
            <span
              key={index}
              className="px-2 py-0.5 text-xs bg-white/10 text-gray-400 rounded"
            >
              {tag}
            </span>
          ))}
          {coach.tags.length > 3 && (
            <span className="text-xs text-gray-500">+{coach.tags.length - 3}</span>
          )}
        </div>
      )}
    </button>
  );
}
