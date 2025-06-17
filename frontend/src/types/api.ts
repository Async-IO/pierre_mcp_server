export interface ApiKey {
  id: string;
  name: string;
  description?: string;
  prefix: string;
  key_prefix: string; // Backend uses this field
  tier: 'starter' | 'professional' | 'enterprise';
  status: 'active' | 'inactive' | 'revoked';
  is_active: boolean; // Backend uses this field
  created_at: string;
  expires_at?: string;
  last_used_at?: string;
  usage_count: number;
  rate_limit_remaining?: number;
  rate_limit_reset?: string;
}

export interface ApiKeysResponse {
  api_keys: ApiKey[];
  total_count: number;
}

export interface CreateApiKeyRequest {
  name: string;
  description?: string;
  tier: 'starter' | 'professional' | 'enterprise';
  expires_in_days?: number;
}

export interface CreateApiKeyResponse {
  api_key: ApiKey;
  secret_key: string; // Only returned once
}

export interface TierUsage {
  tier: string;
  usage_count: number;
  percentage: number;
  key_count: number;
  total_requests: number;
  average_requests_per_key: number;
}

export interface DashboardOverview {
  total_requests_today: number;
  total_requests_this_month: number;
  active_api_keys: number;
  total_api_keys: number;
  error_rate_today: number;
  rate_limit_status: {
    current: number;
    limit: number;
    reset_time: string;
  };
  current_month_usage_by_tier?: TierUsage[];
}

export interface RateLimitOverview {
  api_key_id: string;
  api_key_name: string;
  tier: string;
  requests_per_minute: number;
  requests_per_hour: number;
  requests_per_day: number;
  limit: number;
  usage_percentage: number;
  current_usage: number;
  reset_times: {
    minute: string;
    hour: string;
    day: string;
  };
}

export interface RequestLog {
  id: string;
  api_key_id: string;
  api_key_prefix: string;
  timestamp: string;
  tool_name: string;
  status_code: number;
  response_time_ms?: number;
  error_message?: string;
  request_size_bytes?: number;
  response_size_bytes?: number;
  ip_address?: string;
  user_agent?: string;
}

export interface RequestStats {
  total_requests: number;
  success_rate: number;
  avg_response_time: number;
  requests_per_minute: number;
  error_count: number;
  most_used_tool: string;
  peak_hour: string;
}

export interface RequestFilter {
  timeRange: string;
  status: string;
  tool: string;
}

export interface ToolUsageBreakdown {
  tool_name: string;
  request_count: number;
  success_rate: number;
  avg_response_time: number;
  error_count: number;
  percentage_of_total: number;
}

// A2A (Agent-to-Agent) Protocol Types

export interface A2AClient {
  id: string;
  name: string;
  description: string;
  public_key?: string;
  capabilities: string[];
  redirect_uris: string[];
  agent_version?: string;
  contact_email?: string;
  documentation_url?: string;
  is_verified: boolean;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface A2AClientRegistrationRequest {
  name: string;
  description: string;
  capabilities: string[];
  redirect_uris?: string[];
  contact_email: string;
  agent_version?: string;
  documentation_url?: string;
}

export interface A2AClientCredentials {
  client_id: string;
  client_secret: string;
  api_key: string;
}

export interface A2ASession {
  id: string;
  client_id: string;
  user_id?: string;
  granted_scopes: string[];
  created_at: string;
  expires_at: string;
  last_activity: string;
  requests_count: number;
}

export interface A2ARateLimitStatus {
  is_rate_limited: boolean;
  limit?: number;
  remaining?: number;
  reset_at?: string;
  tier: string;
}

export interface A2AUsageStats {
  client_id: string;
  requests_today: number;
  requests_this_month: number;
  total_requests: number;
  last_request_at?: string;
  rate_limit_tier: string;
  tool_usage_breakdown: Array<{
    tool_name: string;
    usage_count: number;
    percentage: number;
  }>;
  capability_usage: Array<{
    capability: string;
    usage_count: number;
    percentage: number;
  }>;
}

export interface A2AUsageRecord {
  id: number;
  client_id: string;
  session_token?: string;
  timestamp: string;
  tool_name: string;
  response_time_ms?: number;
  status_code: number;
  error_message?: string;
  request_size_bytes?: number;
  response_size_bytes?: number;
  ip_address?: string;
  user_agent?: string;
  protocol_version: string;
  client_capabilities: string[];
  granted_scopes: string[];
}

export interface A2ADashboardOverview {
  total_clients: number;
  active_clients: number;
  total_sessions: number;
  active_sessions: number;
  requests_today: number;
  requests_this_month: number;
  most_used_capability: string;
  error_rate: number;
  usage_by_tier: Array<{
    tier: string;
    client_count: number;
    request_count: number;
    percentage: number;
  }>;
}