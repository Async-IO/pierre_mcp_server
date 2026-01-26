// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Coaches domain API - CRUD, favorites, versions, fork operations
// ABOUTME: Manages AI coaching personas with full version history support

import type { AxiosInstance } from 'axios';
import type {
  Coach,
  CreateCoachRequest,
  UpdateCoachRequest,
  ListCoachesResponse,
  CoachVersion,
  FieldChange,
  ForkCoachResponse,
} from '@pierre/shared-types';
import { ENDPOINTS } from '../core/endpoints';

// Re-export types for consumers
export type { Coach, CreateCoachRequest, UpdateCoachRequest, ListCoachesResponse, CoachVersion, ForkCoachResponse };

export interface ListCoachesOptions {
  category?: string;
  favorites_only?: boolean;
  include_hidden?: boolean;
  limit?: number;
  offset?: number;
}

export interface PromptSuggestion {
  id: string;
  text: string;
  category: string;
}

export interface PromptSuggestionsResponse {
  suggestions: PromptSuggestion[];
}

/**
 * Creates the coaches API methods bound to an axios instance.
 */
export function createCoachesApi(axios: AxiosInstance) {
  const api = {
    /**
     * List coaches with optional filters.
     */
    async list(options?: ListCoachesOptions): Promise<ListCoachesResponse> {
      const params = new URLSearchParams();
      if (options?.category) params.append('category', options.category);
      if (options?.favorites_only) params.append('favorites_only', 'true');
      if (options?.include_hidden) params.append('include_hidden', 'true');
      if (options?.limit) params.append('limit', options.limit.toString());
      if (options?.offset) params.append('offset', options.offset.toString());

      const queryString = params.toString();
      const url = queryString ? `${ENDPOINTS.COACHES.LIST}?${queryString}` : ENDPOINTS.COACHES.LIST;

      const response = await axios.get<ListCoachesResponse>(url);
      return response.data;
    },

    /**
     * Get a specific coach by ID.
     */
    async get(coachId: string): Promise<Coach> {
      const response = await axios.get<Coach>(ENDPOINTS.COACHES.COACH(coachId));
      return response.data;
    },

    /**
     * Create a new coach.
     */
    async create(request: CreateCoachRequest): Promise<Coach> {
      const response = await axios.post<Coach>(ENDPOINTS.COACHES.LIST, request);
      return response.data;
    },

    /**
     * Update an existing coach.
     */
    async update(coachId: string, request: UpdateCoachRequest): Promise<Coach> {
      const response = await axios.put<Coach>(ENDPOINTS.COACHES.COACH(coachId), request);
      return response.data;
    },

    /**
     * Delete a coach.
     */
    async delete(coachId: string): Promise<void> {
      await axios.delete(ENDPOINTS.COACHES.COACH(coachId));
    },

    /**
     * Toggle favorite status for a coach.
     */
    async toggleFavorite(coachId: string): Promise<{ is_favorite: boolean }> {
      const response = await axios.post<{ is_favorite: boolean }>(
        ENDPOINTS.COACHES.FAVORITE(coachId)
      );
      return response.data;
    },

    /**
     * Record coach usage (for analytics).
     */
    async recordUsage(coachId: string): Promise<void> {
      try {
        await axios.post(ENDPOINTS.COACHES.USE(coachId));
      } catch {
        // Silent failure - usage tracking is non-critical
      }
    },

    /**
     * Hide a coach from the user's view.
     */
    async hide(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
      const response = await axios.post<{ success: boolean; is_hidden: boolean }>(
        ENDPOINTS.COACHES.HIDE(coachId)
      );
      return response.data;
    },

    /**
     * Show a previously hidden coach.
     */
    async show(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
      const response = await axios.delete<{ success: boolean; is_hidden: boolean }>(
        ENDPOINTS.COACHES.HIDE(coachId)
      );
      return response.data;
    },

    /**
     * List hidden coaches.
     */
    async getHidden(): Promise<ListCoachesResponse> {
      const response = await axios.get<ListCoachesResponse>(ENDPOINTS.COACHES.HIDDEN);
      return response.data;
    },

    /**
     * Fork (copy) a coach to create a personal version.
     */
    async fork(coachId: string): Promise<ForkCoachResponse> {
      const response = await axios.post<ForkCoachResponse>(ENDPOINTS.COACHES.FORK(coachId));
      return response.data;
    },

    /**
     * Get version history for a coach.
     */
    async getVersions(
      coachId: string,
      limit?: number
    ): Promise<{
      versions: CoachVersion[];
      current_version: number;
      total: number;
    }> {
      const params = new URLSearchParams();
      if (limit) params.append('limit', limit.toString());

      const queryString = params.toString();
      const url = queryString
        ? `${ENDPOINTS.COACHES.VERSIONS(coachId)}?${queryString}`
        : ENDPOINTS.COACHES.VERSIONS(coachId);

      const response = await axios.get(url);
      return response.data;
    },

    /**
     * Get a specific version of a coach.
     */
    async getVersion(coachId: string, version: number): Promise<CoachVersion> {
      const response = await axios.get<CoachVersion>(ENDPOINTS.COACHES.VERSION(coachId, version));
      return response.data;
    },

    /**
     * Revert a coach to a previous version.
     */
    async revertToVersion(
      coachId: string,
      version: number
    ): Promise<{
      coach: Coach;
      reverted_to_version: number;
      new_version: number;
    }> {
      const response = await axios.post(ENDPOINTS.COACHES.VERSION_REVERT(coachId, version));
      return response.data;
    },

    /**
     * Get the diff between two versions of a coach.
     */
    async getVersionDiff(
      coachId: string,
      fromVersion: number,
      toVersion: number
    ): Promise<{
      from_version: number;
      to_version: number;
      changes: FieldChange[];
    }> {
      const response = await axios.get(
        ENDPOINTS.COACHES.VERSION_DIFF(coachId, fromVersion, toVersion)
      );
      return response.data;
    },

    /**
     * Get prompt suggestions for coaches.
     */
    async getPromptSuggestions(): Promise<PromptSuggestionsResponse> {
      const response = await axios.get<PromptSuggestionsResponse>(ENDPOINTS.PROMPTS.SUGGESTIONS);
      return response.data;
    },
  };

  // Add aliases for backward compatibility
  return {
    ...api,
    // Aliases
    listCoaches: api.list,
    getCoaches: api.list,
    getCoach: api.get,
    createCoach: api.create,
    updateCoach: api.update,
    deleteCoach: api.delete,
    listHiddenCoaches: api.getHidden,
    getHiddenCoaches: api.getHidden,
    forkCoach: api.fork,
    getCoachVersions: api.getVersions,
    getCoachVersion: api.getVersion,
    revertCoachToVersion: api.revertToVersion,
    getCoachVersionDiff: api.getVersionDiff,
  };
}

export type CoachesApi = ReturnType<typeof createCoachesApi>;
