// ABOUTME: Role-based permission system with super_admin, admin, user hierarchy
// ABOUTME: Provides extensible permission checking via trait and bitflags
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Impersonation system for super admins to act as other users
pub mod impersonation;

/// User roles in order of privilege (higher ordinal = more access)
///
/// The role hierarchy is: `SuperAdmin` > `Admin` > `User`
/// Each role inherits all permissions from lower roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// Regular user with access to own data only
    #[default]
    User,
    /// Administrator with user management capabilities
    Admin,
    /// Super administrator with full system access including impersonation
    SuperAdmin,
}

impl UserRole {
    /// Get the privilege level (0 = lowest, 2 = highest)
    #[must_use]
    pub const fn privilege_level(&self) -> u8 {
        match self {
            Self::User => 0,
            Self::Admin => 1,
            Self::SuperAdmin => 2,
        }
    }

    /// Check if this role has at least the given privilege level
    #[must_use]
    pub const fn has_privilege(&self, required: Self) -> bool {
        self.privilege_level() >= required.privilege_level()
    }

    /// Check if this role is admin or higher
    #[must_use]
    pub const fn is_admin_or_higher(&self) -> bool {
        matches!(self, Self::Admin | Self::SuperAdmin)
    }

    /// Check if this role is super admin
    #[must_use]
    pub const fn is_super_admin(&self) -> bool {
        matches!(self, Self::SuperAdmin)
    }

    /// Get default permissions for this role
    #[must_use]
    pub const fn default_permissions(&self) -> Permissions {
        match self {
            Self::User => Permissions::USER_DEFAULT,
            Self::Admin => Permissions::ADMIN_DEFAULT,
            Self::SuperAdmin => Permissions::all(),
        }
    }

    /// Convert to database string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
            Self::SuperAdmin => "super_admin",
        }
    }

    /// Parse from database string
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "super_admin" => Self::SuperAdmin,
            "admin" => Self::Admin,
            _ => Self::User,
        }
    }

    /// Get display name for UI
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Admin => "Admin",
            Self::SuperAdmin => "Super Admin",
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for UserRole {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "super_admin" => Ok(Self::SuperAdmin),
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            _ => Err(AppError::invalid_input(format!("Invalid user role: {s}"))),
        }
    }
}

bitflags::bitflags! {
    /// Permission flags for fine-grained access control
    ///
    /// Permissions are organized in bit ranges:
    /// - Bits 0-15: User permissions (basic access)
    /// - Bits 16-31: Admin permissions (user management)
    /// - Bits 32-47: Super admin permissions (system administration)
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Permissions: u64 {
        // User permissions (bits 0-15)
        /// View own fitness data and profile
        const VIEW_OWN_DATA = 1 << 0;
        /// Edit own profile settings
        const EDIT_OWN_PROFILE = 1 << 1;
        /// Create MCP tokens for AI clients
        const CREATE_MCP_TOKENS = 1 << 2;
        /// Use AI chat functionality
        const USE_CHAT = 1 << 3;
        /// Connect fitness providers (Strava, etc.)
        const CONNECT_PROVIDERS = 1 << 4;
        /// View own analytics
        const VIEW_OWN_ANALYTICS = 1 << 5;

        // Admin permissions (bits 16-31)
        /// View all users in the system
        const VIEW_ALL_USERS = 1 << 16;
        /// Approve pending user registrations
        const APPROVE_USERS = 1 << 17;
        /// Suspend user accounts
        const SUSPEND_USERS = 1 << 18;
        /// View system-wide analytics
        const VIEW_ANALYTICS = 1 << 19;
        /// Manage API keys
        const MANAGE_API_KEYS = 1 << 20;
        /// View A2A clients
        const VIEW_A2A_CLIENTS = 1 << 21;
        /// Manage admin tokens
        const MANAGE_ADMIN_TOKENS = 1 << 22;

        // Super admin permissions (bits 32-47)
        /// Impersonate other users
        const IMPERSONATE_USERS = 1 << 32;
        /// Manage admin and super admin roles
        const MANAGE_ADMINS = 1 << 33;
        /// Access system configuration
        const SYSTEM_CONFIG = 1 << 34;
        /// View audit logs
        const VIEW_AUDIT_LOGS = 1 << 35;
        /// Delete users permanently
        const DELETE_USERS = 1 << 36;

        // Role presets
        /// Default permissions for regular users
        const USER_DEFAULT = Self::VIEW_OWN_DATA.bits()
            | Self::EDIT_OWN_PROFILE.bits()
            | Self::CREATE_MCP_TOKENS.bits()
            | Self::USE_CHAT.bits()
            | Self::CONNECT_PROVIDERS.bits()
            | Self::VIEW_OWN_ANALYTICS.bits();

        /// Default permissions for admins (includes user permissions)
        const ADMIN_DEFAULT = Self::USER_DEFAULT.bits()
            | Self::VIEW_ALL_USERS.bits()
            | Self::APPROVE_USERS.bits()
            | Self::SUSPEND_USERS.bits()
            | Self::VIEW_ANALYTICS.bits()
            | Self::MANAGE_API_KEYS.bits()
            | Self::VIEW_A2A_CLIENTS.bits()
            | Self::MANAGE_ADMIN_TOKENS.bits();
    }
}

impl Permissions {
    /// Check if all specified permissions are present
    #[must_use]
    pub const fn has_all(&self, required: Self) -> bool {
        self.bits() & required.bits() == required.bits()
    }

    /// Check if any of the specified permissions are present
    #[must_use]
    pub const fn has_any(&self, required: Self) -> bool {
        self.bits() & required.bits() != 0
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::USER_DEFAULT
    }
}

/// Type alias for role lookup function to reduce complexity
type RoleLookupFn = Box<dyn Fn(&Uuid) -> Option<UserRole> + Send + Sync>;

/// Trait for checking permissions - extensible hook point
///
/// Implement this trait to customize permission checking logic,
/// for example to check delegated permissions or temporary grants.
pub trait PermissionChecker: Send + Sync {
    /// Check if user has specific permission
    fn has_permission(&self, user_id: &Uuid, permission: Permissions) -> bool;

    /// Get effective permissions for user (including delegations)
    fn effective_permissions(&self, user_id: &Uuid) -> Permissions;

    /// Check if user can perform action on another user
    fn can_act_on_user(&self, actor_id: &Uuid, target_id: &Uuid, action: Permissions) -> bool;
}

/// Default permission checker based on user role
pub struct RoleBasedPermissionChecker {
    /// Function to get user role by ID
    get_user_role: RoleLookupFn,
}

impl RoleBasedPermissionChecker {
    /// Create new checker with role lookup function
    pub fn new<F>(get_role: F) -> Self
    where
        F: Fn(&Uuid) -> Option<UserRole> + Send + Sync + 'static,
    {
        Self {
            get_user_role: Box::new(get_role),
        }
    }
}

impl PermissionChecker for RoleBasedPermissionChecker {
    fn has_permission(&self, user_id: &Uuid, permission: Permissions) -> bool {
        (self.get_user_role)(user_id)
            .is_some_and(|role| role.default_permissions().has_all(permission))
    }

    fn effective_permissions(&self, user_id: &Uuid) -> Permissions {
        (self.get_user_role)(user_id)
            .map_or(Permissions::empty(), |role| role.default_permissions())
    }

    fn can_act_on_user(&self, actor_id: &Uuid, target_id: &Uuid, action: Permissions) -> bool {
        // Users can only act on themselves for user-level permissions
        if actor_id == target_id {
            return self.has_permission(actor_id, action);
        }

        // For actions on other users, must have admin+ permissions
        let actor_role = (self.get_user_role)(actor_id);
        let target_role = (self.get_user_role)(target_id);

        match (actor_role, target_role) {
            (Some(actor), Some(target)) => {
                // Must have higher privilege than target
                actor.privilege_level() > target.privilege_level()
                    && self.has_permission(actor_id, action)
            }
            _ => false,
        }
    }
}
