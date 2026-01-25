// ABOUTME: User models for multi-tenant authentication system
// ABOUTME: User, UserTier, UserStatus, and UserPhysiologicalProfile definitions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::profiles::FitnessLevel;
use crate::constants::tiers;
use crate::errors::AppError;
use crate::intelligence::algorithms::MaxHrAlgorithm;
use crate::permissions::UserRole;

use super::{EncryptedToken, SportType};

/// User tier for rate limiting - same as `API` key tiers for consistency
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserTier {
    /// Free tier with basic limits
    Starter,
    /// Professional tier with higher limits
    Professional,
    /// Enterprise tier with unlimited access
    Enterprise,
}

impl Display for UserTier {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Starter => write!(f, "Starter"),
            Self::Professional => write!(f, "Professional"),
            Self::Enterprise => write!(f, "Enterprise"),
        }
    }
}

impl UserTier {
    /// Get monthly request limit for this tier
    #[must_use]
    pub const fn monthly_limit(&self) -> Option<u32> {
        match self {
            Self::Starter => Some(10_000),
            Self::Professional => Some(100_000),
            Self::Enterprise => None, // Unlimited
        }
    }

    /// Get display name for this tier
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Starter => "Starter",
            Self::Professional => "Professional",
            Self::Enterprise => "Enterprise",
        }
    }

    /// Convert to string for database storage
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Starter => tiers::STARTER,
            Self::Professional => tiers::PROFESSIONAL,
            Self::Enterprise => tiers::ENTERPRISE,
        }
    }
}

impl FromStr for UserTier {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            tiers::STARTER => Ok(Self::Starter),
            tiers::PROFESSIONAL => Ok(Self::Professional),
            tiers::ENTERPRISE => Ok(Self::Enterprise),
            _ => Err(AppError::invalid_input(format!("Invalid user tier: {s}"))),
        }
    }
}

/// User account status for admin approval workflow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum UserStatus {
    /// Account pending admin approval (new registrations)
    #[default]
    Pending,
    /// Account approved and active
    Active,
    /// Account suspended by admin
    Suspended,
}

impl UserStatus {
    /// Check if user can login
    #[must_use]
    pub const fn can_login(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Get user-friendly status message
    #[must_use]
    pub const fn to_message(&self) -> &'static str {
        match self {
            Self::Pending => "Your account is pending admin approval",
            Self::Active => "Account is active",
            Self::Suspended => "Your account has been suspended",
        }
    }
}

impl Display for UserStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

/// Represents a user in the multi-tenant system
///
/// Users are authenticated through `OAuth` providers and have encrypted tokens
/// stored securely for accessing their fitness data.
///
/// Multi-tenant membership is managed via the `tenant_users` junction table,
/// allowing users to belong to multiple tenants (like Slack workspaces).
/// The active tenant context is determined per-session via JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: Uuid,
    /// User email address (used for identification)
    pub email: String,
    /// Display name
    pub display_name: Option<String>,
    /// Hashed password for authentication
    pub password_hash: String,
    /// User tier for rate limiting
    pub tier: UserTier,
    /// Encrypted Strava tokens
    pub strava_token: Option<EncryptedToken>,
    /// Encrypted Fitbit tokens
    pub fitbit_token: Option<EncryptedToken>,
    /// When the user account was created
    pub created_at: DateTime<Utc>,
    /// Last time user accessed the system
    pub last_active: DateTime<Utc>,
    /// Whether the user account is active
    pub is_active: bool,
    /// User account status for admin approval workflow
    pub user_status: UserStatus,
    /// Whether this user has admin privileges (legacy - use role instead)
    pub is_admin: bool,
    /// User role for permission system (`super_admin`, `admin`, `user`)
    pub role: UserRole,
    /// Admin who approved this user (if approved)
    pub approved_by: Option<Uuid>,
    /// When the user was approved by admin
    pub approved_at: Option<DateTime<Utc>>,
    /// Firebase UID if user authenticated via Firebase (Google, Apple, etc.)
    pub firebase_uid: Option<String>,
    /// Authentication provider: "email", "google.com", "apple.com", "github.com"
    pub auth_provider: String,
}

impl User {
    /// Create a new user with the given email and password hash
    ///
    /// Tenant membership is managed separately via the `tenant_users` table.
    #[must_use]
    pub fn new(email: String, password_hash: String, display_name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            display_name,
            password_hash,
            tier: UserTier::Starter, // Default to starter tier
            strava_token: None,
            fitbit_token: None,
            created_at: now,
            last_active: now,
            is_active: true,
            user_status: UserStatus::Pending, // New users need admin approval
            is_admin: false,                  // Regular users are not admins by default
            role: UserRole::User,             // Default to regular user
            approved_by: None,
            approved_at: None,
            firebase_uid: None, // No Firebase UID for email/password users
            auth_provider: "email".to_owned(), // Default to email provider
        }
    }

    /// Check if user has valid Strava token
    #[must_use]
    pub fn has_strava_access(&self) -> bool {
        self.strava_token
            .as_ref()
            .is_some_and(|token| token.expires_at > Utc::now())
    }

    /// Check if user has valid Fitbit token
    #[must_use]
    pub fn has_fitbit_access(&self) -> bool {
        self.fitbit_token
            .as_ref()
            .is_some_and(|token| token.expires_at > Utc::now())
    }

    /// Get list of available providers for this user
    #[must_use]
    pub fn available_providers(&self) -> Vec<String> {
        let mut providers = Vec::with_capacity(2); // Typically Strava and Fitbit
        if self.has_strava_access() {
            providers.push("strava".into());
        }
        if self.has_fitbit_access() {
            providers.push("fitbit".into());
        }
        providers
    }

    /// Update last active timestamp
    pub fn update_last_active(&mut self) {
        self.last_active = Utc::now();
    }
}

/// User physiological profile for personalized analysis
///
/// Contains physiological data used for calculating personalized heart rate zones,
/// pace zones, and other performance thresholds based on individual fitness metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPhysiologicalProfile {
    /// User `ID` this profile belongs to
    pub user_id: Uuid,
    /// VO2 max in ml/kg/min (if measured or estimated)
    pub vo2_max: Option<f64>,
    /// Resting heart rate in bpm
    pub resting_hr: Option<u16>,
    /// Maximum heart rate in bpm
    pub max_hr: Option<u16>,
    /// Lactate threshold as percentage of VO2 max (0.65-0.95)
    pub lactate_threshold_percentage: Option<f64>,
    /// Age in years
    pub age: Option<u16>,
    /// Weight in kg
    pub weight: Option<f64>,
    /// Overall fitness level
    pub fitness_level: FitnessLevel,
    /// Primary sport for specialized calculations
    pub primary_sport: SportType,
    /// Years of training experience
    pub training_experience_years: Option<u8>,
}

impl UserPhysiologicalProfile {
    /// Create a new physiological profile
    #[must_use]
    pub const fn new(user_id: Uuid, primary_sport: SportType) -> Self {
        Self {
            user_id,
            vo2_max: None,
            resting_hr: None,
            max_hr: None,
            lactate_threshold_percentage: None,
            age: None,
            weight: None,
            fitness_level: FitnessLevel::Recreational,
            primary_sport,
            training_experience_years: None,
        }
    }

    /// Estimate max heart rate from age if not provided using Tanaka formula
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Safe: HR is constrained to 0-220 range
    #[allow(clippy::cast_sign_loss)] // Safe: HR is always positive from algorithm
    pub fn estimated_max_hr(&self) -> Option<u16> {
        self.max_hr.or_else(|| {
            self.age.map(|age| {
                // Use Tanaka formula via enum (gold standard: 208 - 0.7xage)
                MaxHrAlgorithm::Tanaka
                    .estimate(u32::from(age), None)
                    .ok()
                    .map_or_else(|| 220_u16.saturating_sub(age), |hr| hr.round() as u16)
            })
        })
    }

    /// Check if profile has sufficient data for VO2 max calculations
    #[must_use]
    pub const fn has_vo2_max_data(&self) -> bool {
        self.vo2_max.is_some()
            && self.resting_hr.is_some()
            && (self.max_hr.is_some() || self.age.is_some())
    }

    /// Get fitness level from VO2 max if available
    #[must_use]
    pub fn fitness_level_from_vo2_max(&self) -> FitnessLevel {
        self.vo2_max.map_or(self.fitness_level, |vo2_max| {
            FitnessLevel::from_vo2_max(
                vo2_max, self.age, None, // Gender not stored in this profile
            )
        })
    }
}
