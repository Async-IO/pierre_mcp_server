// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Store domain API - browse, search, install/uninstall coaches
// ABOUTME: Handles the public coach marketplace functionality

import type { AxiosInstance } from 'axios';
import type {
  BrowseCoachesResponse,
  SearchCoachesResponse,
  CategoriesResponse,
  InstallCoachResponse,
  UninstallCoachResponse,
  InstallationsResponse,
  StoreCoachDetail,
} from '@pierre/shared-types';
import { ENDPOINTS } from '../core/endpoints';

export interface BrowseOptions {
  category?: string;
  sort_by?: 'newest' | 'popular' | 'title';
  limit?: number;
  cursor?: string;
}

/**
 * Creates the store API methods bound to an axios instance.
 */
export function createStoreApi(axios: AxiosInstance) {
  const api = {
    /**
     * Browse store coaches with optional filters.
     */
    async browse(options?: BrowseOptions): Promise<BrowseCoachesResponse> {
      const params = new URLSearchParams();
      if (options?.category) params.append('category', options.category);
      if (options?.sort_by) params.append('sort_by', options.sort_by);
      if (options?.limit) params.append('limit', options.limit.toString());
      if (options?.cursor) params.append('cursor', options.cursor);

      const queryString = params.toString();
      const url = queryString ? `${ENDPOINTS.STORE.COACHES}?${queryString}` : ENDPOINTS.STORE.COACHES;

      const response = await axios.get<BrowseCoachesResponse>(url);
      return response.data;
    },

    /**
     * Search store coaches.
     */
    async search(query: string, limit?: number): Promise<SearchCoachesResponse> {
      const params = new URLSearchParams();
      params.append('q', query);
      if (limit) params.append('limit', limit.toString());

      const response = await axios.get<SearchCoachesResponse>(
        `${ENDPOINTS.STORE.SEARCH}?${params}`
      );
      return response.data;
    },

    /**
     * Get a specific store coach by ID.
     */
    async get(coachId: string): Promise<StoreCoachDetail> {
      const response = await axios.get<StoreCoachDetail>(ENDPOINTS.STORE.COACH(coachId));
      return response.data;
    },

    /**
     * Get store categories with counts.
     */
    async getCategories(): Promise<CategoriesResponse> {
      const response = await axios.get<CategoriesResponse>(ENDPOINTS.STORE.CATEGORIES);
      return response.data;
    },

    /**
     * Install a coach from the store.
     */
    async install(coachId: string): Promise<InstallCoachResponse> {
      const response = await axios.post<InstallCoachResponse>(ENDPOINTS.STORE.INSTALL(coachId));
      return response.data;
    },

    /**
     * Uninstall a previously installed coach.
     */
    async uninstall(coachId: string): Promise<UninstallCoachResponse> {
      const response = await axios.delete<UninstallCoachResponse>(ENDPOINTS.STORE.INSTALL(coachId));
      return response.data;
    },

    /**
     * Get list of installed coaches.
     */
    async getInstallations(): Promise<InstallationsResponse> {
      const response = await axios.get<InstallationsResponse>(ENDPOINTS.STORE.INSTALLATIONS);
      return response.data;
    },
  };

  // Add aliases for backward compatibility
  return {
    ...api,
    // Aliases
    browseStoreCoaches: api.browse,
    searchStoreCoaches: api.search,
    getStoreCoach: api.get,
    getStoreCategories: api.getCategories,
    installStoreCoach: api.install,
    uninstallStoreCoach: api.uninstall,
    getStoreInstallations: api.getInstallations,
    getInstalledCoaches: api.getInstallations,
  };
}

export type StoreApi = ReturnType<typeof createStoreApi>;
