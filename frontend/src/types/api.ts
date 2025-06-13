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
  current_usage: {
    minute: number;
    hour: number;
    day: number;
  };
  reset_times: {
    minute: string;
    hour: string;
    day: string;
  };
}