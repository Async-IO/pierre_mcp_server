// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Re-exports shared types for the web frontend
// ABOUTME: All types are now centralized in @pierre/shared-types

// ========== ADMIN TYPES ==========
// API keys, admin tokens, A2A protocol, dashboard analytics

export type {
  ApiKeyStatus,
  ApiKey,
  ApiKeysResponse,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  AdminPermission,
  AdminToken,
  AdminTokensResponse,
  CreateAdminTokenRequest,
  CreateAdminTokenResponse,
  AdminTokenAudit,
  AdminTokenUsageStats,
  TierUsage,
  DashboardOverview,
  RateLimitOverview,
  RequestLog,
  RequestStats,
  RequestFilter,
  ToolUsageBreakdown,
  A2AClient,
  A2AClientRegistrationRequest,
  A2AClientCredentials,
  A2ASession,
  A2ARateLimitStatus,
  A2AUsageStats,
  A2AUsageRecord,
  A2ADashboardOverview,
  SetupStatusResponse,
  ProvisionedKey,
  MilestoneConfig,
  DistanceMilestoneConfig,
  StreakConfig,
  MilestoneRelevanceScores,
  DistanceRelevanceScores,
  StreakRelevanceScores,
  RelevanceConfig,
  ActivityFetchLimitsConfig,
  SocialInsightsConfig,
} from '@pierre/shared-types';

// ========== AUTH TYPES ==========
// Users, login, OAuth

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
  UserManagementResponse,
  ApproveUserRequest,
  SuspendUserRequest,
} from '@pierre/shared-types';

// ========== COACH TYPES ==========
// AI coaching personas, store, versions

export type {
  CoachCategory,
  CoachVisibility,
  PublishStatus,
  Coach,
  ForkCoachResponse,
  CreateCoachRequest,
  UpdateCoachRequest,
  CoachMetadata,
  ListCoachesResponse,
  CoachVersion,
  ListVersionsResponse,
  RevertVersionResponse,
  FieldChange,
  CoachDiffResponse,
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
  CoachAssignment,
  AssignCoachResponse,
  UnassignCoachResponse,
  ListAssignmentsResponse,
} from '@pierre/shared-types';

// ========== API TYPES ==========
// Chat, prompts, common patterns

export type {
  Conversation,
  Message,
  ActivityPillar,
  PromptCategory,
  PromptSuggestionsResponse,
  ApiMetadata,
  PaginatedResponse,
  ListResponse,
} from '@pierre/shared-types';
