// ABOUTME: TypeScript type definitions for Pierre Mobile app
// ABOUTME: Re-exports shared types for convenient importing

// Re-export all types from shared packages
// Auth types
export type {
  UserRole,
  UserStatus,
  UserTier,
  User,
  AdminUser,
  LoginResponse,
  RegisterResponse,
  FirebaseLoginResponse,
  ProviderStatus,
  OAuthApp,
  OAuthAppCredentials,
  OAuthProvider,
  McpToken,
} from '@pierre/shared-types';

// API types (chat, prompts)
export type {
  Conversation,
  Message,
  ActivityPillar,
  PromptCategory,
  PromptSuggestionsResponse,
} from '@pierre/shared-types';

// Coach types
export type {
  CoachCategory,
  CoachVisibility,
  PublishStatus,
  Coach,
  ForkCoachResponse,
  CreateCoachRequest,
  UpdateCoachRequest,
  ListCoachesResponse,
  StoreCoach,
  StoreCoachDetail,
  StoreMetadata,
  BrowseCoachesResponse,
  SearchCoachesResponse,
  CategoryCount,
  CategoriesResponse,
  InstallCoachResponse,
  UninstallCoachResponse,
  InstallationsResponse,
} from '@pierre/shared-types';

// Re-export social types (local to mobile for now)
export * from './social';
