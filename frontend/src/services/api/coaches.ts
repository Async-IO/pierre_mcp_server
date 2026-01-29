// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Coach management API methods - CRUD operations, favorites, versioning
// ABOUTME: Handles user-created coaches and coach library functionality

import { axios } from './client';
import type { Coach } from '@pierre/shared-types';

// Re-export Coach type for backward compatibility
export type { Coach } from '@pierre/shared-types';

export const coachesApi = {
  async getCoaches(options?: {
    category?: string;
    favorites_only?: boolean;
    include_hidden?: boolean;
    limit?: number;
    offset?: number;
  }): Promise<{
    coaches: Coach[];
    total: number;
    metadata: {
      timestamp: string;
      api_version: string;
    };
  }> {
    const params = new URLSearchParams();
    if (options?.category) params.append('category', options.category);
    if (options?.favorites_only) params.append('favorites_only', 'true');
    if (options?.include_hidden) params.append('include_hidden', 'true');
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.offset) params.append('offset', options.offset.toString());
    const queryString = params.toString();
    const url = queryString ? `/api/coaches?${queryString}` : '/api/coaches';
    const response = await axios.get(url);
    return response.data;
  },

  async toggleCoachFavorite(coachId: string): Promise<{ is_favorite: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/favorite`);
    return response.data;
  },

  async recordCoachUsage(coachId: string): Promise<{ success: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/usage`);
    return response.data;
  },

  async createCoach(data: {
    title: string;
    description?: string;
    system_prompt: string;
    category?: string;
    tags?: string[];
  }): Promise<Coach> {
    const response = await axios.post('/api/coaches', data);
    return response.data;
  },

  async updateCoach(coachId: string, data: {
    title?: string;
    description?: string;
    system_prompt?: string;
    category?: string;
    tags?: string[];
  }): Promise<Coach> {
    const response = await axios.put(`/api/coaches/${coachId}`, data);
    return response.data;
  },

  async deleteCoach(coachId: string): Promise<void> {
    await axios.delete(`/api/coaches/${coachId}`);
  },

  async hideCoach(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/hide`);
    return response.data;
  },

  async showCoach(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
    const response = await axios.delete(`/api/coaches/${coachId}/hide`);
    return response.data;
  },

  async getHiddenCoaches(): Promise<{ coaches: Coach[] }> {
    const response = await axios.get('/api/coaches/hidden');
    return response.data;
  },

  async forkCoach(coachId: string): Promise<{ coach: Coach }> {
    const response = await axios.post(`/api/coaches/${coachId}/fork`);
    return response.data;
  },

  // Version History
  async getCoachVersions(coachId: string, limit?: number): Promise<{
    versions: Array<{
      version: number;
      content_snapshot: Record<string, unknown>;
      change_summary: string | null;
      created_at: string;
      created_by_name: string | null;
    }>;
    current_version: number;
    total: number;
  }> {
    const params = new URLSearchParams();
    if (limit) params.append('limit', limit.toString());
    const url = params.toString()
      ? `/api/coaches/${coachId}/versions?${params}`
      : `/api/coaches/${coachId}/versions`;
    const response = await axios.get(url);
    return response.data;
  },

  async getCoachVersion(coachId: string, version: number): Promise<{
    version: number;
    content_snapshot: Record<string, unknown>;
    change_summary: string | null;
    created_at: string;
    created_by_name: string | null;
  }> {
    const response = await axios.get(`/api/coaches/${coachId}/versions/${version}`);
    return response.data;
  },

  async revertCoachToVersion(coachId: string, version: number): Promise<{
    coach: Coach;
    reverted_to_version: number;
    new_version: number;
  }> {
    const response = await axios.post(`/api/coaches/${coachId}/versions/${version}/revert`);
    return response.data;
  },

  async getCoachVersionDiff(coachId: string, fromVersion: number, toVersion: number): Promise<{
    from_version: number;
    to_version: number;
    changes: Array<{
      field: string;
      old_value: unknown | null;
      new_value: unknown | null;
    }>;
  }> {
    const response = await axios.get(`/api/coaches/${coachId}/versions/${fromVersion}/diff/${toVersion}`);
    return response.data;
  },

  // Coach Generation from Conversation
  async generateCoachFromConversation(data: {
    conversation_id: string;
    max_messages?: number;
  }): Promise<{
    title: string;
    description: string;
    system_prompt: string;
    category: string;
    tags: string[];
    messages_analyzed: number;
    total_messages: number;
  }> {
    const response = await axios.post('/api/coaches/generate', data);
    return response.data;
  },

  // Prompt Suggestions
  async getPromptSuggestions(): Promise<{
    categories: Array<{
      category_key: string;
      category_title: string;
      category_icon: string;
      pillar: 'activity' | 'nutrition' | 'recovery';
      prompts: string[];
    }>;
    welcome_prompt: string;
    metadata: {
      timestamp: string;
      api_version: string;
    };
  }> {
    const response = await axios.get('/api/prompts/suggestions');
    return response.data;
  },
};
