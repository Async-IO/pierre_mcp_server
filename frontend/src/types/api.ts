// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

export interface ApiKey {
  id: string;
  name: string;
  description?: string;
  prefix: string;
  key_prefix: string; // Backend uses this field
  rate_limit_requests: number; // Requests per month (0 = unlimited)
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
  rate_limit_requests: number; // 0 = unlimited
  expires_in_days?: number;
}

export interface CreateApiKeyResponse {
  api_key: ApiKey;
  secret_key: string; // Only returned once
}

// Admin Token Management Types
export interface AdminToken {
  id: string;
  service_name: string;
  service_description?: string;
  permissions: AdminPermission[];
  is_super_admin: boolean;
  is_active: boolean;
  created_at: string;
  expires_at?: string;
  last_used_at?: string;
  usage_count: number;
  token_prefix: string;
}

export interface AdminTokensResponse {
  admin_tokens: AdminToken[];
  total_count: number;
}

export interface CreateAdminTokenRequest {
  service_name: string;
  service_description?: string;
  permissions: AdminPermission[];
  is_super_admin?: boolean;
  expires_in_days?: number;
}

export interface CreateAdminTokenResponse {
  admin_token: AdminToken;
  jwt_token: string; // Only returned once
}

export interface AdminTokenAudit {
  id: string;
  admin_token_id: string;
  timestamp: string;
  action: string;
  target_resource?: string;
  ip_address?: string;
  success: boolean;
  error_message?: string;
}

export interface AdminTokenUsageStats {
  total_actions: number;
  actions_last_24h: number;
  actions_last_7d: number;
  most_common_actions: Array<{
    action: string;
    count: number;
  }>;
}

export type AdminPermission = 
  | 'provision_keys'
  | 'revoke_keys' 
  | 'list_keys'
  | 'manage_admin_tokens'
  | 'view_audit_logs'
  | 'super_admin';

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
  api_key_name: string;
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
  successful_requests: number;
  failed_requests: number;
  average_response_time: number;
  min_response_time?: number;
  max_response_time?: number;
  requests_per_minute: number;
  error_rate: number;
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
  average_response_time: number;
  error_count?: number;
  percentage_of_total?: number;
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

// User Management Types
export type UserRole = 'super_admin' | 'admin' | 'user';

export interface User {
  id: string;
  email: string;
  display_name?: string;
  role: UserRole;
  user_status: 'pending' | 'active' | 'suspended';
  tier: 'starter' | 'professional' | 'enterprise';
  created_at: string;
  last_active: string;
  approved_by?: string;
  approved_at?: string;
}

export interface UserManagementResponse {
  success: boolean;
  message: string;
  user?: User;
}

export interface ApproveUserRequest {
  reason?: string;
}

export interface SuspendUserRequest {
  reason?: string;
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

export interface SetupStatusResponse {
  needs_setup: boolean;
  admin_user_exists: boolean;
  message?: string;
}

export interface ProvisionedKey {
  api_key_id: string;
  user_email: string;
  requested_tier: string;
  key_status: 'active' | 'inactive' | 'revoked';
  created_at: string;
  expires_at?: string;
}

// Coach Management Types
export interface Coach {
  id: string;
  title: string;
  description: string | null;
  system_prompt: string;
  category: string;
  tags: string[];
  token_count: number;
  is_favorite: boolean;
  use_count: number;
  last_used_at: string | null;
  created_at: string;
  updated_at: string;
  is_system: boolean;
  visibility: string;
  is_assigned: boolean;
}

export interface ListCoachesResponse {
  coaches: Coach[];
  total: number;
  metadata: {
    timestamp: string;
    api_version: string;
  };
}

export interface CreateCoachRequest {
  title: string;
  description?: string;
  system_prompt: string;
  category?: string;
  tags?: string[];
  visibility?: string;
}

export interface UpdateCoachRequest {
  title?: string;
  description?: string;
  system_prompt?: string;
  category?: string;
  tags?: string[];
}

export interface CoachAssignment {
  user_id: string;
  user_email?: string;
  assigned_at: string;
  assigned_by?: string;
}

export interface AssignCoachResponse {
  coach_id: string;
  assigned_count: number;
  total_requested: number;
}

export interface UnassignCoachResponse {
  coach_id: string;
  removed_count: number;
  total_requested: number;
}

export interface ListAssignmentsResponse {
  coach_id: string;
  assignments: CoachAssignment[];
}

// Coach Version History Types (ASY-153)
export interface CoachVersion {
  version: number;
  content_snapshot: Record<string, unknown>;
  change_summary: string | null;
  created_at: string;
  created_by_name: string | null;
}

export interface ListVersionsResponse {
  versions: CoachVersion[];
  current_version: number;
  total: number;
}

export interface RevertVersionResponse {
  coach: Coach;
  reverted_to_version: number;
  new_version: number;
}

export interface FieldChange {
  field: string;
  old_value: unknown | null;
  new_value: unknown | null;
}

export interface CoachDiffResponse {
  from_version: number;
  to_version: number;
  changes: FieldChange[];
}

// Coach Store Types
export type CoachCategory = 'training' | 'nutrition' | 'recovery' | 'recipes' | 'mobility' | 'custom';

export interface StoreCoach {
  id: string;
  title: string;
  description: string | null;
  category: CoachCategory;
  tags: string[];
  sample_prompts: string[];
  token_count: number;
  install_count: number;
  icon_url: string | null;
  published_at: string | null;
  author_id: string | null;
}

export interface StoreCoachDetail extends StoreCoach {
  system_prompt: string;
  created_at: string;
  publish_status: 'draft' | 'pending_review' | 'published' | 'rejected';
}

export interface StoreMetadata {
  timestamp: string;
  api_version: string;
}

export interface BrowseCoachesResponse {
  coaches: StoreCoach[];
  total: number;
  metadata: StoreMetadata;
}

export interface SearchCoachesResponse {
  coaches: StoreCoach[];
  query: string;
  metadata: StoreMetadata;
}

export interface CategoryCount {
  category: CoachCategory;
  name: string;
  count: number;
}

export interface CategoriesResponse {
  categories: CategoryCount[];
  metadata: StoreMetadata;
}

export interface InstallCoachResponse {
  message: string;
  coach: StoreCoach;
  metadata: StoreMetadata;
}

export interface UninstallCoachResponse {
  message: string;
  source_coach_id: string;
  metadata: StoreMetadata;
}

export interface InstallationsResponse {
  coaches: StoreCoach[];
  metadata: StoreMetadata;
}