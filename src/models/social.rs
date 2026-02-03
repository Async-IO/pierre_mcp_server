// ABOUTME: Social features models for coach-mediated sharing
// ABOUTME: Friend connections, shared insights, reactions, and privacy settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;
use crate::intelligence::InsightSharingPolicy;

// ============================================================================
// Enums
// ============================================================================

/// Status of a friend connection request
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FriendStatus {
    /// Request sent, awaiting acceptance
    #[default]
    Pending,
    /// Both users have connected
    Accepted,
    /// Request was declined by receiver
    Declined,
    /// One user blocked the other
    Blocked,
}

impl Display for FriendStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Accepted => write!(f, "accepted"),
            Self::Declined => write!(f, "declined"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

impl FromStr for FriendStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "accepted" => Ok(Self::Accepted),
            "declined" => Ok(Self::Declined),
            "blocked" => Ok(Self::Blocked),
            _ => Err(AppError::invalid_input(format!(
                "Invalid friend status: {s}"
            ))),
        }
    }
}

impl FriendStatus {
    /// Database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Blocked => "blocked",
        }
    }

    /// Whether this status represents an active friendship
    #[must_use]
    pub const fn is_connected(&self) -> bool {
        matches!(self, Self::Accepted)
    }
}

/// Visibility setting for shared insights
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShareVisibility {
    /// Only visible to friends
    #[default]
    FriendsOnly,
    /// Visible to all users on the platform
    Public,
}

impl Display for ShareVisibility {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::FriendsOnly => write!(f, "friends_only"),
            Self::Public => write!(f, "public"),
        }
    }
}

impl FromStr for ShareVisibility {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "friends_only" => Ok(Self::FriendsOnly),
            "public" => Ok(Self::Public),
            _ => Err(AppError::invalid_input(format!(
                "Invalid share visibility: {s}"
            ))),
        }
    }
}

impl ShareVisibility {
    /// Database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FriendsOnly => "friends_only",
            Self::Public => "public",
        }
    }
}

/// Type of coach insight that can be shared
///
/// These represent the categories of shareable content that the AI coach generates.
/// Each type has different privacy implications and sharing contexts.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    /// A fitness achievement (PR, streak, goal reached)
    Achievement,
    /// A training milestone (100th run, 1000km total, etc.)
    Milestone,
    /// A training tip or recommendation from the coach
    TrainingTip,
    /// Recovery-related insight (good recovery, rest suggestion)
    Recovery,
    /// Motivational content or encouragement
    Motivation,
    /// A direct message from a coach chat conversation
    CoachingInsight,
}

impl Display for InsightType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Achievement => write!(f, "achievement"),
            Self::Milestone => write!(f, "milestone"),
            Self::TrainingTip => write!(f, "training_tip"),
            Self::Recovery => write!(f, "recovery"),
            Self::Motivation => write!(f, "motivation"),
            Self::CoachingInsight => write!(f, "coaching_insight"),
        }
    }
}

impl FromStr for InsightType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "achievement" => Ok(Self::Achievement),
            "milestone" => Ok(Self::Milestone),
            "training_tip" => Ok(Self::TrainingTip),
            "recovery" => Ok(Self::Recovery),
            "motivation" => Ok(Self::Motivation),
            "coaching_insight" => Ok(Self::CoachingInsight),
            _ => Err(AppError::invalid_input(format!(
                "Invalid insight type: {s}"
            ))),
        }
    }
}

impl InsightType {
    /// Database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Achievement => "achievement",
            Self::Milestone => "milestone",
            Self::TrainingTip => "training_tip",
            Self::Recovery => "recovery",
            Self::Motivation => "motivation",
            Self::CoachingInsight => "coaching_insight",
        }
    }

    /// Human-readable description of this insight type
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Achievement => "Fitness achievement",
            Self::Milestone => "Training milestone",
            Self::TrainingTip => "Training recommendation",
            Self::Recovery => "Recovery insight",
            Self::Motivation => "Motivational message",
            Self::CoachingInsight => "Coach chat message",
        }
    }
}

/// Types of reactions users can give to shared insights
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReactionType {
    /// Simple like/thumbs up
    Like,
    /// Celebration for achievements
    Celebrate,
    /// Inspired by the insight
    Inspire,
    /// Supportive reaction
    Support,
}

impl Display for ReactionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Like => write!(f, "like"),
            Self::Celebrate => write!(f, "celebrate"),
            Self::Inspire => write!(f, "inspire"),
            Self::Support => write!(f, "support"),
        }
    }
}

impl FromStr for ReactionType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "like" => Ok(Self::Like),
            "celebrate" => Ok(Self::Celebrate),
            "inspire" => Ok(Self::Inspire),
            "support" => Ok(Self::Support),
            _ => Err(AppError::invalid_input(format!(
                "Invalid reaction type: {s}"
            ))),
        }
    }
}

impl ReactionType {
    /// Database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Like => "like",
            Self::Celebrate => "celebrate",
            Self::Inspire => "inspire",
            Self::Support => "support",
        }
    }

    /// Emoji representation for UI display
    #[must_use]
    pub const fn emoji(&self) -> &'static str {
        match self {
            Self::Like => "👍",
            Self::Celebrate => "🎉",
            Self::Inspire => "💪",
            Self::Support => "🤗",
        }
    }
}

/// Training phase context for insights
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingPhase {
    /// Building aerobic base
    Base,
    /// Increasing intensity and volume
    Build,
    /// Peak performance preparation
    Peak,
    /// Recovery and rest period
    Recovery,
}

impl Display for TrainingPhase {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Base => write!(f, "base"),
            Self::Build => write!(f, "build"),
            Self::Peak => write!(f, "peak"),
            Self::Recovery => write!(f, "recovery"),
        }
    }
}

impl FromStr for TrainingPhase {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "base" => Ok(Self::Base),
            "build" => Ok(Self::Build),
            "peak" => Ok(Self::Peak),
            "recovery" => Ok(Self::Recovery),
            _ => Err(AppError::invalid_input(format!(
                "Invalid training phase: {s}"
            ))),
        }
    }
}

impl TrainingPhase {
    /// Database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Build => "build",
            Self::Peak => "peak",
            Self::Recovery => "recovery",
        }
    }
}

// ============================================================================
// Structs
// ============================================================================

/// Represents a friend connection between two users
///
/// Connections are bidirectional and require mutual acceptance.
/// The initiator sends the request, the receiver accepts/declines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendConnection {
    /// Unique identifier for this connection
    pub id: Uuid,
    /// User who initiated the friend request
    pub initiator_id: Uuid,
    /// User who received the friend request
    pub receiver_id: Uuid,
    /// Current status of the connection
    pub status: FriendStatus,
    /// When the request was created
    pub created_at: DateTime<Utc>,
    /// When the connection was last updated
    pub updated_at: DateTime<Utc>,
    /// When the request was accepted (if accepted)
    pub accepted_at: Option<DateTime<Utc>>,
}

impl FriendConnection {
    /// Create a new pending friend connection
    #[must_use]
    pub fn new(initiator_id: Uuid, receiver_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            initiator_id,
            receiver_id,
            status: FriendStatus::Pending,
            created_at: now,
            updated_at: now,
            accepted_at: None,
        }
    }

    /// Accept the friend request
    pub fn accept(&mut self) {
        let now = Utc::now();
        self.status = FriendStatus::Accepted;
        self.updated_at = now;
        self.accepted_at = Some(now);
    }

    /// Decline the friend request
    pub fn decline(&mut self) {
        self.status = FriendStatus::Declined;
        self.updated_at = Utc::now();
    }

    /// Block the user
    pub fn block(&mut self) {
        self.status = FriendStatus::Blocked;
        self.updated_at = Utc::now();
    }

    /// Check if the given user is part of this connection
    #[must_use]
    pub fn involves_user(&self, user_id: Uuid) -> bool {
        self.initiator_id == user_id || self.receiver_id == user_id
    }

    /// Get the other user in this connection
    #[must_use]
    pub fn other_user(&self, user_id: Uuid) -> Option<Uuid> {
        if self.initiator_id == user_id {
            Some(self.receiver_id)
        } else if self.receiver_id == user_id {
            Some(self.initiator_id)
        } else {
            None
        }
    }
}

/// Notification preferences for social features
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationPreferences {
    /// Receive notifications for friend requests
    pub friend_requests: bool,
    /// Receive notifications for reactions on insights
    pub insight_reactions: bool,
    /// Receive notifications when insights are adapted
    pub adapted_insights: bool,
}

impl NotificationPreferences {
    /// Create default notification preferences (all enabled)
    #[must_use]
    pub const fn all_enabled() -> Self {
        Self {
            friend_requests: true,
            insight_reactions: true,
            adapted_insights: true,
        }
    }

    /// Create notification preferences with all disabled
    #[must_use]
    pub const fn all_disabled() -> Self {
        Self {
            friend_requests: false,
            insight_reactions: false,
            adapted_insights: false,
        }
    }
}

/// User's social and privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSocialSettings {
    /// User ID these settings belong to
    pub user_id: Uuid,
    /// Whether user can be found in friend search
    pub discoverable: bool,
    /// Default visibility for new shared insights
    pub default_visibility: ShareVisibility,
    /// Activity types to auto-suggest for sharing
    pub share_activity_types: Vec<String>,
    /// Notification preferences
    pub notifications: NotificationPreferences,
    /// Policy for data detail level in shared insights
    pub insight_sharing_policy: InsightSharingPolicy,
    /// When settings were created
    pub created_at: DateTime<Utc>,
    /// When settings were last updated
    pub updated_at: DateTime<Utc>,
}

impl UserSocialSettings {
    /// Create default settings for a user
    #[must_use]
    pub fn default_for_user(user_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            discoverable: true,
            default_visibility: ShareVisibility::FriendsOnly,
            share_activity_types: vec!["run".to_owned(), "ride".to_owned(), "swim".to_owned()],
            notifications: NotificationPreferences::all_enabled(),
            insight_sharing_policy: InsightSharingPolicy::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// A coach insight shared by a user
///
/// Represents a sanitized, privacy-safe insight that users can share with friends.
/// The content is generated by the AI coach and contains no private data like
/// GPS coordinates, exact paces, or recovery scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedInsight {
    /// Unique identifier
    pub id: Uuid,
    /// User who shared this insight
    pub user_id: Uuid,
    /// Visibility setting
    pub visibility: ShareVisibility,
    /// Type of insight
    pub insight_type: InsightType,
    /// Sport type context (optional)
    pub sport_type: Option<String>,
    /// The shareable content (sanitized, no private data)
    pub content: String,
    /// Optional title
    pub title: Option<String>,
    /// Training phase context
    pub training_phase: Option<TrainingPhase>,
    /// Number of reactions received
    pub reaction_count: i32,
    /// Number of times adapted by others
    pub adapt_count: i32,
    /// When the insight was shared
    pub created_at: DateTime<Utc>,
    /// When the insight was last updated
    pub updated_at: DateTime<Utc>,
    /// Optional expiry time for time-sensitive insights
    pub expires_at: Option<DateTime<Utc>>,
    /// Source activity ID that generated this insight (for coach-mediated sharing)
    pub source_activity_id: Option<String>,
    /// Whether this insight was coach-generated (vs manual entry)
    pub coach_generated: bool,
}

impl SharedInsight {
    /// Create a new shared insight
    #[must_use]
    pub fn new(
        user_id: Uuid,
        insight_type: InsightType,
        content: String,
        visibility: ShareVisibility,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            visibility,
            insight_type,
            sport_type: None,
            content,
            title: None,
            training_phase: None,
            reaction_count: 0,
            adapt_count: 0,
            created_at: now,
            updated_at: now,
            expires_at: None,
            source_activity_id: None,
            coach_generated: false,
        }
    }

    /// Create a coach-generated insight linked to an activity
    #[must_use]
    pub fn coach_generated(
        user_id: Uuid,
        insight_type: InsightType,
        content: String,
        visibility: ShareVisibility,
        source_activity_id: String,
    ) -> Self {
        let mut insight = Self::new(user_id, insight_type, content, visibility);
        insight.source_activity_id = Some(source_activity_id);
        insight.coach_generated = true;
        insight
    }

    /// Check if this insight is expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| exp < Utc::now())
    }

    /// Check if this insight is visible to the given user
    #[must_use]
    pub fn is_visible_to(&self, viewer_id: Uuid, is_friend: bool) -> bool {
        // Owner can always see their own insights
        if self.user_id == viewer_id {
            return true;
        }

        // Check expiry
        if self.is_expired() {
            return false;
        }

        // Check visibility
        match self.visibility {
            ShareVisibility::Public => true,
            ShareVisibility::FriendsOnly => is_friend,
        }
    }
}

/// A reaction to a shared insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightReaction {
    /// Unique identifier
    pub id: Uuid,
    /// The insight being reacted to
    pub insight_id: Uuid,
    /// User who reacted
    pub user_id: Uuid,
    /// Type of reaction
    pub reaction_type: ReactionType,
    /// When the reaction was created
    pub created_at: DateTime<Utc>,
}

impl InsightReaction {
    /// Create a new reaction
    #[must_use]
    pub fn new(insight_id: Uuid, user_id: Uuid, reaction_type: ReactionType) -> Self {
        Self {
            id: Uuid::new_v4(),
            insight_id,
            user_id,
            reaction_type,
            created_at: Utc::now(),
        }
    }
}

/// An insight adapted for another user's training context
///
/// When a user taps "Adapt to My Training" on a friend's insight,
/// the AI coach generates a personalized version for their context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptedInsight {
    /// Unique identifier
    pub id: Uuid,
    /// The original shared insight
    pub source_insight_id: Uuid,
    /// User who requested the adaptation
    pub user_id: Uuid,
    /// The personalized content
    pub adapted_content: String,
    /// Context used for adaptation (JSON)
    pub adaptation_context: Option<String>,
    /// Whether the user found this helpful
    pub was_helpful: Option<bool>,
    /// When the adaptation was created
    pub created_at: DateTime<Utc>,
}

impl AdaptedInsight {
    /// Create a new adapted insight
    #[must_use]
    pub fn new(source_insight_id: Uuid, user_id: Uuid, adapted_content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_insight_id,
            user_id,
            adapted_content,
            adaptation_context: None,
            was_helpful: None,
            created_at: Utc::now(),
        }
    }

    /// Mark this adaptation as helpful or not
    pub const fn set_helpful(&mut self, helpful: bool) {
        self.was_helpful = Some(helpful);
    }
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to send a friend request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendFriendRequestRequest {
    /// ID of the user to send request to
    pub receiver_id: Uuid,
}

/// Request to respond to a friend request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespondFriendRequestRequest {
    /// Whether to accept the request
    pub accept: bool,
}

/// Request to share an insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInsightRequest {
    /// Type of insight
    pub insight_type: InsightType,
    /// Content to share
    pub content: String,
    /// Optional title
    pub title: Option<String>,
    /// Visibility setting
    pub visibility: Option<ShareVisibility>,
    /// Sport type context
    pub sport_type: Option<String>,
    /// Training phase context
    pub training_phase: Option<TrainingPhase>,
}

/// Request to react to an insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactToInsightRequest {
    /// Type of reaction
    pub reaction_type: ReactionType,
}

/// Request to adapt an insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptInsightRequest {
    /// Optional context to include in adaptation
    pub context: Option<String>,
}

/// Request to update social settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSocialSettingsRequest {
    /// Whether user can be found in search
    pub discoverable: Option<bool>,
    /// Default visibility for new insights
    pub default_visibility: Option<ShareVisibility>,
    /// Activity types to suggest for sharing
    pub share_activity_types: Option<Vec<String>>,
    /// Notification preferences update
    pub notifications: Option<NotificationPreferences>,
}

/// Friend information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendInfo {
    /// User ID
    pub user_id: Uuid,
    /// Display name
    pub display_name: Option<String>,
    /// Email (for identification)
    pub email: String,
    /// When they became friends
    pub friends_since: DateTime<Utc>,
}

/// Feed item for social feed responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    /// The shared insight
    pub insight: SharedInsight,
    /// Author information
    pub author: FriendInfo,
    /// Reaction counts by type
    pub reactions: ReactionSummary,
    /// Current user's reaction (if any)
    pub user_reaction: Option<ReactionType>,
    /// Whether current user has adapted this
    pub user_has_adapted: bool,
}

/// Summary of reaction counts by type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReactionSummary {
    /// Number of likes
    pub like_count: i32,
    /// Number of celebrations
    pub celebrate_count: i32,
    /// Number of inspires
    pub inspire_count: i32,
    /// Number of supports
    pub support_count: i32,
    /// Total reactions
    pub total: i32,
}
