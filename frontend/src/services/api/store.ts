// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Coach Store API methods - browse, search, install, uninstall coaches
// ABOUTME: Handles the public coach marketplace functionality

import { axios } from './client';

// Store coach type
export interface StoreCoach {
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

export const storeApi = {
  async browseStoreCoaches(options?: {
    category?: string;
    sort_by?: 'newest' | 'popular' | 'title';
    limit?: number;
    cursor?: string;
  }): Promise<{
    coaches: StoreCoach[];
    next_cursor: string | null;
    has_more: boolean;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    if (options?.category) params.append('category', options.category);
    if (options?.sort_by) params.append('sort_by', options.sort_by);
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.cursor) params.append('cursor', options.cursor);
    const queryString = params.toString();
    const url = queryString ? `/api/store/coaches?${queryString}` : '/api/store/coaches';
    const response = await axios.get(url);
    return response.data;
  },

  async searchStoreCoaches(query: string, limit?: number): Promise<{
    coaches: StoreCoach[];
    query: string;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    params.append('q', query);
    if (limit) params.append('limit', limit.toString());
    const response = await axios.get(`/api/store/search?${params}`);
    return response.data;
  },

  async getStoreCoach(coachId: string): Promise<StoreCoach & {
    system_prompt: string;
    created_at: string;
    publish_status: string;
  }> {
    const response = await axios.get(`/api/store/coaches/${coachId}`);
    return response.data;
  },

  async getStoreCategories(): Promise<{
    categories: Array<{
      category: string;
      name: string;
      count: number;
    }>;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.get('/api/store/categories');
    return response.data;
  },

  async installStoreCoach(coachId: string): Promise<{
    message: string;
    coach: StoreCoach;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.post(`/api/store/coaches/${coachId}/install`);
    return response.data;
  },

  async uninstallStoreCoach(coachId: string): Promise<{
    message: string;
    source_coach_id: string;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.delete(`/api/store/coaches/${coachId}/install`);
    return response.data;
  },

  async getStoreInstallations(): Promise<{
    coaches: StoreCoach[];
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.get('/api/store/installations');
    return response.data;
  },
};
