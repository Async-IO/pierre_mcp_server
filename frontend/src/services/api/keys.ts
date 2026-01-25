// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: API Key management methods - create, list, deactivate, usage tracking
// ABOUTME: Handles both regular API keys and trial keys

import { axios } from './client';

export const keysApi = {
  async createApiKey(data: {
    name: string;
    description?: string;
    rate_limit_requests: number; // 0 = unlimited
    expires_in_days?: number;
  }) {
    const response = await axios.post('/api/keys', data);
    return response.data;
  },

  async createTrialKey(data: {
    name: string;
    description?: string;
  }) {
    // Create trial key with 1000 requests/month and 14-day expiry
    const trialData = {
      name: data.name,
      description: data.description,
      rate_limit_requests: 1000,
      expires_in_days: 14,
    };
    const response = await axios.post('/api/keys', trialData);
    return response.data;
  },

  async getApiKeys() {
    const response = await axios.get('/api/keys');
    return response.data;
  },

  async deactivateApiKey(keyId: string) {
    const response = await axios.delete(`/api/keys/${keyId}`);
    return response.data;
  },

  async getApiKeyUsage(keyId: string, startDate?: string, endDate?: string) {
    const params = new URLSearchParams();
    if (startDate) params.append('start_date', startDate);
    if (endDate) params.append('end_date', endDate);

    const response = await axios.get(`/api/keys/${keyId}/usage?${params}`);
    return response.data;
  },
};
