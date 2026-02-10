// ABOUTME: Core data models and types for the Pierre fitness API
// ABOUTME: Re-exports Activity, User, SportType and other fundamental data structures
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Data Models
//!
// NOTE: All `.clone()` calls in this module are Safe - they are necessary for:
// - HashMap key ownership for statistics aggregation (stage_type.clone())
// - Data structure ownership transfers across model boundaries
//!
//! This module contains the core data structures used throughout the Pierre MCP Server.
//! These models provide a unified representation of fitness data from various providers
//! like Strava and Fitbit.
//!
//! ## Design Principles
//!
//! - **Provider Agnostic**: Models abstract away provider-specific differences
//! - **Extensible**: Optional fields accommodate different provider capabilities
//! - **Serializable**: All models support JSON serialization for MCP protocol
//! - **Type Safe**: Strong typing prevents common data handling errors
//!
//! ## Core Models
//!
//! - `Activity`: Represents a single fitness activity (run, ride, etc.)
//! - `Athlete`: User profile information
//! - `Stats`: Aggregated fitness statistics
//! - `PersonalRecord`: Individual performance records
//! - `SportType`: Enumeration of supported activity types

// Domain modules
mod activity;
mod athlete;
mod health;
mod nutrition;
mod oauth;
mod sleep;
mod social;
mod sport;
mod tenant;
mod tool_selection;
mod user;

// Re-export all public types for convenience
// Activity domain
pub use activity::{
    Activity, ActivityBuilder, HeartRateZone, PowerZone, SegmentEffort, TimeSeriesData,
};

// Sport types
pub use sport::SportType;

// Sleep domain
pub use sleep::{SleepSession, SleepStage, SleepStageType};

// Health domain
pub use health::{HealthMetrics, RecoveryMetrics};

// Nutrition domain
pub use nutrition::{FoodItem, MealEntry, MealType, NutritionLog};

// Athlete domain
pub use athlete::{Athlete, PersonalRecord, PrMetric, Stats};

// User domain
pub use user::{User, UserPhysiologicalProfile, UserStatus, UserTier};

// OAuth domain
pub use oauth::{
    AuthRequest, AuthResponse, DecryptedToken, EncryptedToken, OAuthNotification, UserOAuthApp,
    UserOAuthToken, UserSession,
};

// Tenant domain
pub use tenant::{AuthorizationCode, OAuthApp, OAuthAppParams, Tenant};

// Tool selection domain
pub use tool_selection::{
    CategorySummary, EffectiveTool, SetToolOverrideRequest, TenantPlan, TenantToolOverride,
    ToolAvailabilitySummary, ToolCatalogEntry, ToolCategory, ToolEnablementSource,
};

// Social domain
pub use social::{
    AdaptInsightRequest, AdaptedInsight, FeedItem, FriendConnection, FriendInfo, FriendStatus,
    InsightReaction, InsightType, NotificationPreferences, ReactToInsightRequest, ReactionSummary,
    ReactionType, RespondFriendRequestRequest, SendFriendRequestRequest, ShareInsightRequest,
    ShareVisibility, SharedInsight, TrainingPhase, UpdateSocialSettingsRequest, UserSocialSettings,
};

// OAuth 2.0 server persistence models
mod oauth2_server;
pub use oauth2_server::{OAuth2AuthCode, OAuth2Client, OAuth2RefreshToken, OAuth2State};

// User MCP token types for AI client authentication
mod user_mcp_token;
pub use user_mcp_token::{
    CreateUserMcpTokenRequest, UserMcpToken, UserMcpTokenCreated, UserMcpTokenInfo,
};

// Chat conversation and message record types
mod conversation;
pub use conversation::{ConversationRecord, ConversationSummary, MessageRecord};

// Security audit event types
mod audit;
pub use audit::{AuditEvent, AuditEventType, AuditSeverity};

// Key rotation configuration and version types
mod key_rotation;
pub use key_rotation::{KeyRotationConfig, KeyVersion, RotationStatus};

// API key types for authentication and rate limiting
mod api_key;
pub use api_key::{
    ApiKey, ApiKeyData, ApiKeyResponse, ApiKeyTier, ApiKeyUsage, ApiKeyUsageStats,
    CreateApiKeyRequest, CreateApiKeyRequestSimple, RateLimitStatus,
};
