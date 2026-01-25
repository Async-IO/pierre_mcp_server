// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Agent-to-Agent (A2A) protocol API methods - client management, sessions, analytics
// ABOUTME: Handles A2A protocol for agent-to-agent communication

import { axios } from './client';

export const a2aApi = {
  async registerA2AClient(data: {
    name: string;
    description: string;
    capabilities: string[];
    redirect_uris?: string[];
    contact_email: string;
    agent_version?: string;
    documentation_url?: string;
  }) {
    const response = await axios.post('/a2a/clients', data);
    return response.data;
  },

  async getA2AClients() {
    const response = await axios.get('/a2a/clients');
    return response.data;
  },

  async getA2AClient(clientId: string) {
    const response = await axios.get(`/a2a/clients/${clientId}`);
    return response.data;
  },

  async updateA2AClient(clientId: string, data: {
    name?: string;
    description?: string;
    capabilities?: string[];
    redirect_uris?: string[];
    contact_email?: string;
    agent_version?: string;
    documentation_url?: string;
  }) {
    const response = await axios.put(`/a2a/clients/${clientId}`, data);
    return response.data;
  },

  async deactivateA2AClient(clientId: string) {
    const response = await axios.delete(`/a2a/clients/${clientId}`);
    return response.data;
  },

  async getA2AClientUsage(clientId: string, startDate?: string, endDate?: string) {
    const params = new URLSearchParams();
    if (startDate) params.append('start_date', startDate);
    if (endDate) params.append('end_date', endDate);

    const response = await axios.get(`/a2a/clients/${clientId}/usage?${params}`);
    return response.data;
  },

  async getA2AClientRateLimit(clientId: string) {
    const response = await axios.get(`/a2a/clients/${clientId}/rate-limit`);
    return response.data;
  },

  async getA2ASessions(clientId?: string) {
    const params = new URLSearchParams();
    if (clientId) params.append('client_id', clientId);

    const response = await axios.get(`/a2a/sessions?${params}`);
    return response.data;
  },

  async getA2ADashboardOverview() {
    const response = await axios.get('/a2a/dashboard/overview');
    return response.data;
  },

  async getA2AUsageAnalytics(days: number = 30) {
    const response = await axios.get(`/a2a/dashboard/analytics?days=${days}`);
    return response.data;
  },

  async getA2AAgentCard() {
    const response = await axios.get('/a2a/agent-card');
    return response.data;
  },

  async getA2ARequestLogs(clientId?: string, filter?: {
    timeRange: string;
    status: string;
    tool: string;
  }) {
    const params = new URLSearchParams();
    if (clientId) params.append('client_id', clientId);
    if (filter?.timeRange) params.append('time_range', filter.timeRange);
    if (filter?.status && filter.status !== 'all') params.append('status', filter.status);
    if (filter?.tool && filter.tool !== 'all') params.append('tool', filter.tool);

    const response = await axios.get(`/a2a/dashboard/request-logs?${params}`);
    return response.data;
  },
};
