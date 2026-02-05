// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Coach Store browse screen for discovering and installing coaches
// ABOUTME: Lists published coaches with category filters, search, and detail view with install/uninstall

import { useState, useEffect, useCallback, useMemo, useRef, memo } from 'react';
import { useQuery, useInfiniteQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { Compass, ArrowLeft, Plus, Trash2 } from 'lucide-react';
import { storeApi } from '../services/api';

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

interface StoreCoachDetail extends StoreCoach {
  system_prompt: string;
  created_at: string;
  publish_status: string;
}

interface StoreScreenProps {
  onNavigateToCoaches?: () => void;
}

export default function StoreScreen({ onNavigateToCoaches }: StoreScreenProps) {
  const queryClient = useQueryClient();
  const [selectedCoachId, setSelectedCoachId] = useState<string | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<CategoryFilter>('all');
  const [selectedSort, setSelectedSort] = useState<SortOption>('popular');
  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);

  // Debounce search query
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Clear success message after 5 seconds
  useEffect(() => {
    if (successMessage) {
      const timer = setTimeout(() => setSuccessMessage(null), 5000);
      return () => clearTimeout(timer);
    }
  }, [successMessage]);

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
      storeApi.browse({
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
    queryFn: () => storeApi.search(debouncedSearch, 50),
    enabled: !!debouncedSearch,
    staleTime: 30_000,
  });

  // Fetch coach detail when selected
  const { data: coachDetail, isLoading: isLoadingDetail } = useQuery({
    queryKey: ['store-coach-detail', selectedCoachId],
    queryFn: () => storeApi.get(selectedCoachId!),
    enabled: !!selectedCoachId,
    staleTime: 30_000,
  });

  // Fetch installed coaches to check if selected coach is installed
  const { data: installedCoaches } = useQuery({
    queryKey: ['store-installations'],
    queryFn: () => storeApi.getInstallations(),
    staleTime: 30_000,
  });

  const isInstalled = useMemo(() => {
    if (!selectedCoachId || !installedCoaches) return false;
    return installedCoaches.coaches.some(c => c.id === selectedCoachId);
  }, [selectedCoachId, installedCoaches]);

  // Install mutation
  const installMutation = useMutation({
    mutationFn: (coachId: string) => storeApi.install(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['store-installations'] });
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setSuccessMessage(`"${coachDetail?.title}" has been added to your coaches.`);
    },
  });

  // Uninstall mutation
  const uninstallMutation = useMutation({
    mutationFn: (coachId: string) => storeApi.uninstall(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['store-installations'] });
      queryClient.invalidateQueries({ queryKey: ['user-coaches'] });
      setSuccessMessage(`Coach has been removed from your library.`);
    },
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

  const handleSelectCoach = useCallback((coachId: string) => {
    setSelectedCoachId(coachId);
  }, []);

  const handleBackToStore = useCallback(() => {
    setSelectedCoachId(null);
    setSuccessMessage(null);
  }, []);

  const handleInstall = useCallback(() => {
    if (selectedCoachId) {
      installMutation.mutate(selectedCoachId);
    }
  }, [selectedCoachId, installMutation]);

  const handleRemove = useCallback(() => {
    if (selectedCoachId && window.confirm(`Remove Coach?\n\nRemove "${coachDetail?.title}" from your coaches? You can always reinstall it later.`)) {
      uninstallMutation.mutate(selectedCoachId);
    }
  }, [selectedCoachId, coachDetail, uninstallMutation]);

  // Render detail view if a coach is selected
  if (selectedCoachId) {
    return (
      <CoachDetailView
        coach={coachDetail as StoreCoachDetail | undefined}
        isLoading={isLoadingDetail}
        isInstalled={isInstalled}
        isInstalling={installMutation.isPending || uninstallMutation.isPending}
        successMessage={successMessage}
        onBack={handleBackToStore}
        onInstall={handleInstall}
        onRemove={handleRemove}
        onNavigateToCoaches={onNavigateToCoaches}
      />
    );
  }

  return (
    <div className="h-full flex flex-col bg-pierre-dark">
      {/* Header - matches Chat and My Coaches layout */}
      <div className="p-6 border-b border-white/5 flex items-center justify-between flex-shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 flex items-center justify-center rounded-xl bg-gradient-to-br from-pierre-activity to-pierre-activity-dark text-white shadow-glow-sm">
            <Compass className="w-5 h-5" />
          </div>
          <p className="text-sm text-zinc-400">Find AI coaching assistants</p>
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
            aria-hidden="true"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            type="search"
            placeholder="Search coaches..."
            aria-label="Search coaches"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 pr-10 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-pierre-violet/30 focus:border-pierre-violet transition-colors"
          />
          {searchQuery && (
            <button
              onClick={handleClearSearch}
              aria-label="Clear search"
              className="absolute right-1 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-gray-300 min-w-[44px] min-h-[44px] flex items-center justify-center"
            >
              <svg className="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
          {isSearching && (
            <div className="absolute right-3 top-1/2 transform -translate-y-1/2" aria-hidden="true">
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
                'px-4 py-1.5 text-sm font-medium rounded-full whitespace-nowrap transition-colors min-h-[44px] flex items-center',
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
              'px-3 py-1 text-sm rounded transition-colors min-h-[44px] flex items-center',
              selectedSort === option.key
                ? 'bg-pierre-violet/20 text-pierre-violet-light font-medium'
                : 'text-gray-400 hover:text-pierre-violet-light'
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
              aria-hidden="true"
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
                <CoachCard key={coach.id} coach={coach} onClick={() => handleSelectCoach(coach.id)} />
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

// Store coach card - memoized to prevent unnecessary re-renders during scrolling
interface CoachCardProps {
  coach: StoreCoach;
  onClick: () => void;
}

const CoachCard = memo(function CoachCard({ coach, onClick }: CoachCardProps) {
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
});

// Coach detail view component
interface CoachDetailViewProps {
  coach: StoreCoachDetail | undefined;
  isLoading: boolean;
  isInstalled: boolean;
  isInstalling: boolean;
  successMessage: string | null;
  onBack: () => void;
  onInstall: () => void;
  onRemove: () => void;
  onNavigateToCoaches?: () => void;
}

function CoachDetailView({
  coach,
  isLoading,
  isInstalled,
  isInstalling,
  successMessage,
  onBack,
  onInstall,
  onRemove,
  onNavigateToCoaches,
}: CoachDetailViewProps) {
  if (isLoading) {
    return (
      <div className="h-full flex flex-col bg-pierre-dark">
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <div className="w-8 h-8 border-2 border-pierre-violet border-t-transparent rounded-full animate-spin mx-auto" />
            <p className="mt-3 text-sm text-gray-500">Loading coach details...</p>
          </div>
        </div>
      </div>
    );
  }

  if (!coach) {
    return (
      <div className="h-full flex flex-col bg-pierre-dark">
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <p className="text-lg text-gray-400 mb-4">Coach not found</p>
            <button
              onClick={onBack}
              className="px-4 py-2 bg-pierre-violet text-white rounded-lg hover:bg-pierre-violet/80 transition-colors"
            >
              Go Back
            </button>
          </div>
        </div>
      </div>
    );
  }

  const categoryColors = COACH_CATEGORY_COLORS[coach.category] ?? 'bg-gray-500/20 text-gray-400';

  return (
    <div className="h-full flex flex-col bg-pierre-dark">
      {/* Header with back button */}
      <div className="p-4 border-b border-white/10 flex items-center gap-3">
        <button
          onClick={onBack}
          title="Back to Store"
          aria-label="Back to Store"
          className="p-2 text-gray-400 hover:text-white hover:bg-white/10 rounded-lg transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center"
        >
          <ArrowLeft className="w-5 h-5" aria-hidden="true" />
        </button>
        <h2 className="text-lg font-semibold text-white truncate flex-1">{coach.title}</h2>
      </div>

      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto sidebar-scroll">
        <div className="p-6 space-y-6">
          {/* Category & Stats */}
          <div className="flex items-center justify-between">
            <span className={clsx('px-3 py-1 text-sm font-medium rounded-full capitalize', categoryColors)}>
              {coach.category}
            </span>
            <span className="text-sm text-gray-500">
              {coach.install_count} {coach.install_count === 1 ? 'user' : 'users'}
            </span>
          </div>

          {/* Description */}
          {coach.description && (
            <p className="text-base text-gray-300 leading-relaxed">{coach.description}</p>
          )}

          {/* Tags */}
          {coach.tags.length > 0 && (
            <div>
              <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-2">Tags</h3>
              <div className="flex flex-wrap gap-2">
                {coach.tags.map((tag, index) => (
                  <span
                    key={index}
                    className="px-3 py-1 text-sm bg-white/10 text-gray-300 rounded-full border border-white/10"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Sample Prompts */}
          {coach.sample_prompts.length > 0 && (
            <div>
              <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-2">Sample Prompts</h3>
              <div className="space-y-2">
                {coach.sample_prompts.map((prompt, index) => (
                  <div
                    key={index}
                    className="p-3 bg-white/5 border border-white/10 rounded-lg text-sm text-gray-300"
                  >
                    {prompt}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* System Prompt Preview */}
          <div>
            <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-2">System Prompt</h3>
            <div className="p-3 bg-white/5 border border-white/10 rounded-lg">
              <p className="text-sm text-gray-400 font-mono whitespace-pre-wrap line-clamp-10">
                {coach.system_prompt}
              </p>
              {coach.system_prompt.length > 500 && (
                <p className="text-xs text-gray-500 italic mt-2">
                  ...and more ({coach.token_count.toLocaleString()} tokens)
                </p>
              )}
            </div>
          </div>

          {/* Details */}
          <div>
            <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-2">Details</h3>
            <div className="bg-white/5 border border-white/10 rounded-lg overflow-hidden">
              <div className="flex justify-between items-center px-4 py-3 border-b border-white/10">
                <span className="text-sm text-gray-500">Token Count</span>
                <span className="text-sm text-white font-medium">{coach.token_count.toLocaleString()}</span>
              </div>
              {coach.published_at && (
                <div className="flex justify-between items-center px-4 py-3">
                  <span className="text-sm text-gray-500">Published</span>
                  <span className="text-sm text-white font-medium">
                    {new Date(coach.published_at).toLocaleDateString()}
                  </span>
                </div>
              )}
            </div>
          </div>

          {/* Success message */}
          {successMessage && (
            <div className="p-4 bg-emerald-500/20 border border-emerald-500/30 rounded-lg">
              <p className="text-sm text-emerald-400">{successMessage}</p>
              {isInstalled && onNavigateToCoaches && (
                <button
                  onClick={onNavigateToCoaches}
                  className="mt-2 text-sm text-emerald-400 hover:text-emerald-300 underline"
                >
                  View My Coaches →
                </button>
              )}
            </div>
          )}

          {/* Bottom spacer for fixed button */}
          <div className="h-20" />
        </div>
      </div>

      {/* Fixed action button at bottom */}
      <div className="p-4 border-t border-white/10 bg-pierre-dark">
        {isInstalled ? (
          <button
            onClick={onRemove}
            disabled={isInstalling}
            className="w-full py-3 px-4 bg-white/10 border border-white/20 rounded-lg text-white font-medium hover:bg-white/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {isInstalling ? (
              <div className="w-5 h-5 border-2 border-white border-t-transparent rounded-full animate-spin" />
            ) : (
              <>
                <Trash2 className="w-4 h-4" />
                Remove
              </>
            )}
          </button>
        ) : (
          <button
            onClick={onInstall}
            disabled={isInstalling}
            className="w-full py-3 px-4 bg-pierre-violet text-white font-medium rounded-lg hover:bg-pierre-violet/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {isInstalling ? (
              <div className="w-5 h-5 border-2 border-white border-t-transparent rounded-full animate-spin" />
            ) : (
              <>
                <Plus className="w-4 h-4" />
                Add Coach
              </>
            )}
          </button>
        )}
      </div>
    </div>
  );
}
