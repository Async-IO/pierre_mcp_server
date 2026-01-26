// ABOUTME: Shared TypeScript types for authentication and user management
// ABOUTME: User types, login responses, OAuth types

// ========== USER TYPES ==========

/** User role */
export type UserRole = 'super_admin' | 'admin' | 'user';

/** User account status */
export type UserStatus = 'pending' | 'active' | 'suspended';

/** User subscription tier */
export type UserTier = 'starter' | 'professional' | 'enterprise';

/** A user in the system */
export interface User {
  /** Primary user identifier */
  id: string;
  /** Alternative identifier (alias for id) */
  user_id?: string;
  email: string;
  display_name?: string;
  is_admin: boolean;
  role: UserRole;
  /** Account status (use user_status or status) */
  user_status?: UserStatus;
  status?: UserStatus;
  /** Subscription tier (always present in user listings) */
  tier: UserTier;
  /** Account creation timestamp (always present) */
  created_at: string;
  /** Last activity timestamp */
  last_active?: string;
  /** Admin who approved this user */
  approved_by?: string;
  /** Approval timestamp */
  approved_at?: string;
}

/** Extended user for admin views (deprecated: use User directly) */
export type AdminUser = User;

// ========== AUTH RESPONSE TYPES ==========

/** Response from login endpoint */
export interface LoginResponse {
  access_token: string;
  token_type: string;
  expires_in?: number;
  refresh_token?: string;
  user: User;
  csrf_token: string;
}

/** Response from registration endpoint */
export interface RegisterResponse {
  user_id: string;
  email: string;
  message: string;
}

/** Response from Firebase login */
export interface FirebaseLoginResponse {
  csrf_token: string;
  jwt_token: string;
  user: User;
  is_new_user: boolean;
}

// ========== OAUTH TYPES ==========

/** Status of a provider connection */
export interface ProviderStatus {
  provider: string;
  connected: boolean;
  last_sync: string | null;
}

/** OAuth app credentials (user-provided) */
export interface OAuthApp {
  provider: string;
  client_id: string;
  redirect_uri: string;
  created_at: string;
}

/** OAuth app credentials with secret (only for registration) */
export interface OAuthAppCredentials {
  provider: string;
  client_id: string;
  client_secret: string;
  redirect_uri: string;
}

/** Known OAuth providers */
export interface OAuthProvider {
  id: string;
  name: string;
  color: string;
}

// ========== MCP TOKEN TYPES ==========

/** An MCP token for API access */
export interface McpToken {
  id: string;
  name: string;
  token_prefix: string;
  /** Only returned once on creation */
  token_value?: string;
  expires_at: string | null;
  last_used_at: string | null;
  usage_count: number;
  is_revoked: boolean;
  created_at: string;
}

// ========== USER MANAGEMENT TYPES ==========

/** Response for user management operations */
export interface UserManagementResponse {
  success: boolean;
  message: string;
  user?: AdminUser;
}

/** Request to approve a user */
export interface ApproveUserRequest {
  reason?: string;
}

/** Request to suspend a user */
export interface SuspendUserRequest {
  reason?: string;
}
