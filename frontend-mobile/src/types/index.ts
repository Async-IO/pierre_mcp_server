// ABOUTME: TypeScript type definitions for Pierre Mobile app
// ABOUTME: User types, chat types, provider connection types

export type UserRole = 'super_admin' | 'admin' | 'user';
export type UserStatus = 'pending' | 'active' | 'suspended';

export interface User {
  user_id: string;
  email: string;
  display_name?: string;
  is_admin: boolean;
  role: UserRole;
  user_status: UserStatus;
}

export interface Conversation {
  id: string;
  title: string;
  model: string;
  system_prompt?: string;
  total_tokens: number;
  message_count: number;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  token_count?: number;
  created_at: string;
  // Response metadata for assistant messages
  model?: string;
  execution_time_ms?: number;
  // Error flag for failed message responses
  isError?: boolean;
}

export interface ProviderStatus {
  provider: string;
  connected: boolean;
  last_sync: string | null;
}

export interface McpToken {
  id: string;
  name: string;
  token_prefix: string;
  token_value?: string; // Only returned once on creation
  expires_at: string | null;
  last_used_at: string | null;
  usage_count: number;
  is_revoked: boolean;
  created_at: string;
}

export interface PromptCategory {
  category_key: string;
  category_title: string;
  category_icon: string;
  pillar: 'activity' | 'nutrition' | 'recovery';
  prompts: string[];
}

export interface PromptSuggestionsResponse {
  categories: PromptCategory[];
  welcome_prompt: string;
  metadata: {
    timestamp: string;
    api_version: string;
  };
}

export interface LoginResponse {
  access_token: string;
  token_type: string;
  expires_in?: number;
  refresh_token?: string;
  user: User;
  csrf_token: string;
}

export interface RegisterResponse {
  user_id: string;
  email: string;
  message: string;
}

export interface OAuthApp {
  provider: string;
  client_id: string;
  redirect_uri: string;
  created_at: string;
}

export interface OAuthAppCredentials {
  provider: string;
  client_id: string;
  client_secret: string;
  redirect_uri: string;
}

export interface OAuthProvider {
  id: string;
  name: string;
  color: string;
}

export interface FirebaseLoginResponse {
  csrf_token: string;
  jwt_token: string;
  user: User;
  is_new_user: boolean;
}

// Coach types for AI coaching personas
export type CoachCategory = 'training' | 'nutrition' | 'recovery' | 'recipes' | 'mobility' | 'custom';
export type CoachVisibility = 'private' | 'tenant' | 'global';

export interface Coach {
  id: string;
  title: string;
  description: string | null;
  system_prompt: string;
  category: CoachCategory;
  tags: string[];
  token_count: number;
  is_favorite: boolean;
  use_count: number;
  last_used_at: string | null;
  created_at: string;
  updated_at: string;
  is_system: boolean;
  visibility?: CoachVisibility;
  is_assigned?: boolean;
  is_hidden?: boolean;
  forked_from?: string; // ID of source coach if forked
}

// Response when forking a coach
export interface ForkCoachResponse {
  coach: Coach;
  source_coach_id: string;
}

export interface CreateCoachRequest {
  title: string;
  description?: string;
  system_prompt: string;
  category: CoachCategory;
  tags?: string[];
}

export interface UpdateCoachRequest {
  title?: string;
  description?: string;
  system_prompt?: string;
  category?: CoachCategory;
  tags?: string[];
}

export interface ListCoachesResponse {
  coaches: Coach[];
  total: number;
  metadata: {
    timestamp: string;
    api_version: string;
  };
}

// Store types for Coach Store (browse, search, install coaches)
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
  next_cursor: string | null;
  has_more: boolean;
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

// Re-export social types
export * from './social';
