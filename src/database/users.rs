// ABOUTME: User management database operations
// ABOUTME: Handles user registration, authentication, and profile management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::database_plugins::shared;
use crate::errors::{AppError, AppResult};
use crate::intelligence::{FitnessLevel, TimeAvailability, UserFitnessProfile, UserPreferences};
use crate::models::{TenantId, User, UserStatus};
use crate::pagination::{Cursor, CursorPage, PaginationParams};
use crate::permissions::UserRole;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use tracing::warn;
use uuid::Uuid;

impl Database {
    /// Create or update a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The email is already in use by another user
    /// - Database operation fails
    pub async fn create_user_impl(&self, user: &User) -> AppResult<Uuid> {
        // Check if user exists by email
        let existing = self.get_user_by_email_impl(&user.email).await?;
        if let Some(existing_user) = existing {
            if existing_user.id != user.id {
                return Err(AppError::invalid_input(
                    "Email already in use by another user",
                ));
            }
            // Update existing user (tokens are stored in user_oauth_tokens table)
            // NOTE: tenant_id is no longer stored on User - use tenant_users junction table
            sqlx::query(
                r"
                UPDATE users SET
                    display_name = $2,
                    password_hash = $3,
                    tier = $4,
                    is_active = $5,
                    user_status = $6,
                    approved_by = $7,
                    approved_at = $8,
                    last_active = CURRENT_TIMESTAMP
                WHERE id = $1
                ",
            )
            .bind(user.id.to_string())
            .bind(&user.display_name)
            .bind(&user.password_hash)
            .bind(user.tier.as_str())
            .bind(user.is_active)
            .bind(shared::enums::user_status_to_str(&user.user_status))
            .bind(user.approved_by.map(|id| id.to_string()))
            .bind(user.approved_at)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to update user: {e}")))?;
        } else {
            // Insert new user (tokens are stored in user_oauth_tokens table)
            // NOTE: tenant_id is no longer stored on User - use tenant_users junction table
            sqlx::query(
                r"
                INSERT INTO users (
                    id, email, display_name, password_hash, tier,
                    is_active, user_status, is_admin, approved_by, approved_at,
                    created_at, last_active, firebase_uid, auth_provider
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                ",
            )
            .bind(user.id.to_string())
            .bind(&user.email)
            .bind(&user.display_name)
            .bind(&user.password_hash)
            .bind(user.tier.as_str())
            .bind(user.is_active)
            .bind(shared::enums::user_status_to_str(&user.user_status))
            .bind(user.is_admin)
            .bind(user.approved_by.map(|id| id.to_string()))
            .bind(user.approved_at)
            .bind(user.created_at)
            .bind(user.last_active)
            .bind(&user.firebase_uid)
            .bind(&user.auth_provider)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create user: {e}")))?;
        }

        Ok(user.id)
    }

    /// Get a user by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_impl(&self, user_id: Uuid) -> AppResult<Option<User>> {
        let user_id_str = user_id.to_string();
        self.get_user_by_field("id", &user_id_str).await
    }

    /// Get a user by ID (alias for compatibility)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_by_id(&self, user_id: Uuid) -> AppResult<Option<User>> {
        self.get_user_impl(user_id).await
    }

    /// Get a user by email
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_by_email_impl(&self, email: &str) -> AppResult<Option<User>> {
        self.get_user_by_field("email", email).await
    }

    /// Get a user by email, returning an error if not found
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database query fails
    /// - The user is not found
    pub async fn get_user_by_email_required_impl(&self, email: &str) -> AppResult<User> {
        self.get_user_by_email_impl(email)
            .await?
            .ok_or_else(|| AppError::not_found(format!("User with email: {email}")))
    }

    /// Internal implementation for getting a user
    async fn get_user_by_field(&self, field: &str, value: &str) -> AppResult<Option<User>> {
        // NOTE: tenant_id is no longer stored on User - use tenant_users junction table
        let query = format!(
            r"
            SELECT id, email, display_name, password_hash, tier,
                   is_active, user_status, is_admin, approved_by, approved_at,
                   created_at, last_active, firebase_uid, auth_provider
            FROM users WHERE {field} = $1
            "
        );

        let row = sqlx::query(&query)
            .bind(value)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user by {field}: {e}")))?;

        if let Some(row) = row {
            let user = Self::row_to_user(&row)?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Convert a database row to a User struct
    /// OAuth tokens are loaded separately via `user_oauth_tokens` table
    /// Tenant membership is loaded separately via `tenant_users` table
    fn row_to_user(row: &SqliteRow) -> AppResult<User> {
        let id: String = row.get("id");
        let email: String = row.get("email");
        let display_name: Option<String> = row.get("display_name");
        let password_hash: String = row.get("password_hash");
        let tier: String = row.get("tier");
        let is_active: bool = row.get("is_active");
        let user_status_str: String = row.get("user_status");
        let user_status = shared::enums::str_to_user_status(&user_status_str);
        let is_admin: bool = row.get("is_admin");
        let role_str: Option<String> = row.try_get("role").ok();
        let approved_by: Option<String> = row.get("approved_by");
        let approved_at: Option<chrono::DateTime<chrono::Utc>> = row.get("approved_at");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let last_active: chrono::DateTime<chrono::Utc> = row.get("last_active");
        let firebase_uid: Option<String> = row.try_get("firebase_uid").ok().flatten();
        // Default to "email" for existing users without auth_provider column
        let auth_provider: String = row
            .try_get("auth_provider")
            .ok()
            .unwrap_or_else(|| "email".to_owned());

        // Derive role from explicit role column if present, otherwise from is_admin
        let role = role_str.map_or_else(
            || {
                if is_admin {
                    UserRole::Admin
                } else {
                    UserRole::User
                }
            },
            |r| UserRole::from_str_lossy(&r),
        );

        Ok(User {
            id: Uuid::parse_str(&id)
                .map_err(|e| AppError::internal(format!("Failed to parse user id UUID: {e}")))?,
            email,
            display_name,
            password_hash,
            tier: tier
                .parse()
                .map_err(|e| AppError::internal(format!("Failed to parse tier: {e}")))?,
            strava_token: None, // Loaded separately via user_oauth_tokens
            fitbit_token: None, // Loaded separately via user_oauth_tokens
            is_active,
            user_status,
            is_admin,
            role,
            approved_by: approved_by.and_then(|id_str| {
                Uuid::parse_str(&id_str)
                    .inspect_err(|e| {
                        warn!(
                            user_id = %id,
                            approved_by_str = %id_str,
                            error = %e,
                            "Invalid approved_by UUID in database - setting to None"
                        );
                    })
                    .ok()
            }),
            approved_at,
            created_at,
            last_active,
            firebase_uid,
            auth_provider,
        })
    }

    /// Update user's last active timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn update_last_active_impl(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE users SET last_active = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to update last active: {e}")))?;
        Ok(())
    }

    /// Get total user count
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_count_impl(&self) -> AppResult<i64> {
        let count = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user count: {e}")))?;
        Ok(count)
    }

    /// Update or insert user profile data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database query fails
    /// - JSON serialization fails
    pub async fn upsert_user_profile_impl(
        &self,
        user_id: Uuid,
        profile_data: serde_json::Value,
    ) -> AppResult<()> {
        let profile_json = serde_json::to_string(&profile_data)?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO user_profiles (user_id, profile_data, created_at, updated_at)
            VALUES ($1, $2, $3, $3)
            ON CONFLICT(user_id) DO UPDATE SET
                profile_data = $2,
                updated_at = $3
            ",
        )
        .bind(user_id.to_string())
        .bind(profile_json)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to upsert user profile: {e}")))?;

        Ok(())
    }

    /// Get user profile data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database query fails
    /// - JSON deserialization fails
    pub async fn get_user_profile_impl(
        &self,
        user_id: Uuid,
    ) -> AppResult<Option<serde_json::Value>> {
        let row = sqlx::query(
            r"
            SELECT profile_data FROM user_profiles WHERE user_id = $1
            ",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user profile: {e}")))?;

        if let Some(row) = row {
            let profile_json: String = row.get("profile_data");
            let profile_data: serde_json::Value = serde_json::from_str(&profile_json)?;
            Ok(Some(profile_data))
        } else {
            Ok(None)
        }
    }

    /// Get user fitness profile with proper typing
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_fitness_profile(
        &self,
        user_id: Uuid,
    ) -> AppResult<Option<UserFitnessProfile>> {
        self.get_user_profile_impl(user_id).await?.map_or_else(
            || Ok(None),
            |profile_data| {
                // Try to deserialize as UserFitnessProfile
                serde_json::from_value(profile_data).map_or_else(
                    |_| {
                        // If profile data doesn't match UserFitnessProfile structure,
                        // create a default profile with user_id
                        Ok(Some(UserFitnessProfile {
                            user_id: user_id.to_string(),
                            age: None,
                            gender: None,
                            weight: None,
                            height: None,
                            fitness_level: FitnessLevel::Beginner,
                            primary_sports: vec![],
                            training_history_months: 0,
                            preferences: UserPreferences {
                                preferred_units: "metric".into(),
                                training_focus: vec![],
                                injury_history: vec![],
                                time_availability: TimeAvailability {
                                    hours_per_week: 3.0,
                                    preferred_days: vec![],
                                    preferred_duration_minutes: Some(30),
                                },
                            },
                        }))
                    },
                    |fitness_profile| Ok(Some(fitness_profile)),
                )
            },
        )
    }

    /// Update user fitness profile
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JSON serialization fails
    /// - The database operation fails
    pub async fn update_user_fitness_profile(
        &self,
        user_id: Uuid,
        profile: &UserFitnessProfile,
    ) -> AppResult<()> {
        let profile_data = serde_json::to_value(profile)?;
        self.upsert_user_profile_impl(user_id, profile_data).await
    }

    /// Get last sync timestamp for a provider from `user_oauth_tokens`
    ///
    /// Includes `tenant_id` in the query to prevent cross-tenant sync timestamp
    /// collisions in multi-tenant deployments.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> AppResult<Option<chrono::DateTime<chrono::Utc>>> {
        let last_sync: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT last_sync FROM user_oauth_tokens WHERE user_id = $1 AND tenant_id = $2 AND provider = $3",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get provider last sync: {e}")))?;

        Ok(last_sync)
    }

    /// Update last sync timestamp for a provider in `user_oauth_tokens`
    ///
    /// Includes `tenant_id` in the query to prevent cross-tenant sync timestamp
    /// collisions in multi-tenant deployments.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        sync_time: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE user_oauth_tokens SET last_sync = $1 WHERE user_id = $2 AND tenant_id = $3 AND provider = $4",
        )
        .bind(sync_time)
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(provider)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update provider last sync: {e}")))?;

        Ok(())
    }

    /// Get users by status with offset-based pagination
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_users_by_status_impl(
        &self,
        status: &str,
        tenant_id: Option<TenantId>,
    ) -> AppResult<Vec<User>> {
        let rows = if let Some(tid) = tenant_id {
            sqlx::query(
                "SELECT * FROM users WHERE user_status = ?1 AND tenant_id = ?2 ORDER BY created_at DESC",
            )
            .bind(status)
            .bind(tid.to_string())
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query("SELECT * FROM users WHERE user_status = ?1 ORDER BY created_at DESC")
                .bind(status)
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| AppError::database(format!("Failed to get users by status: {e}")))?;

        let mut users = Vec::new();
        for row in rows {
            users.push(Self::row_to_user(&row)?);
        }

        Ok(users)
    }

    /// Get users by status with cursor-based pagination
    ///
    /// Implements efficient keyset pagination using (`created_at`, `id`) composite cursor
    /// to prevent duplicates and missing items when data changes during pagination.
    ///
    /// # Arguments
    ///
    /// * `status` - User status filter ("pending", "active", "suspended")
    /// * `params` - Pagination parameters (cursor, limit, direction)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or cursor is invalid
    pub async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<User>> {
        // Fetch one extra item to determine if there are more pages
        let fetch_limit = params.limit + 1;

        // Convert to i64 for SQL LIMIT clause (pagination limits are always reasonable)
        let fetch_limit_i64 = i64::try_from(fetch_limit)
            .map_err(|_| AppError::invalid_input("Pagination limit too large"))?;

        let (query, cursor_timestamp, cursor_id) = if let Some(ref cursor) = params.cursor {
            // Decode cursor to get position
            let (timestamp, id) = cursor
                .decode()
                .ok_or_else(|| AppError::invalid_input("Invalid cursor format"))?;

            // Cursor-based query: WHERE (created_at, id) < (cursor_created_at, cursor_id)
            // This ensures consistent pagination even when new items are added
            let query = r"
                SELECT id, email, display_name, password_hash, tier, tenant_id,
                       is_active, user_status, is_admin, approved_by, approved_at,
                       created_at, last_active, firebase_uid, auth_provider
                FROM users
                WHERE user_status = ?1
                  AND (created_at < ?2 OR (created_at = ?2 AND id < ?3))
                ORDER BY created_at DESC, id DESC
                LIMIT ?4
            ";
            (query, Some(timestamp), Some(id))
        } else {
            // First page - no cursor
            let query = r"
                SELECT id, email, display_name, password_hash, tier, tenant_id,
                       is_active, user_status, is_admin, approved_by, approved_at,
                       created_at, last_active, firebase_uid, auth_provider
                FROM users
                WHERE user_status = ?1
                ORDER BY created_at DESC, id DESC
                LIMIT ?2
            ";
            (query, None, None)
        };

        let rows = if let (Some(ts), Some(id)) = (cursor_timestamp, cursor_id) {
            sqlx::query(query)
                .bind(status)
                .bind(ts)
                .bind(id)
                .bind(fetch_limit_i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to get users by status (cursor): {e}"))
                })?
        } else {
            sqlx::query(query)
                .bind(status)
                .bind(fetch_limit_i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to get users by status (first page): {e}"))
                })?
        };

        // Convert rows to users
        let mut all_users: Vec<User> = Vec::new();
        for row in rows {
            all_users.push(Self::row_to_user(&row)?);
        }

        // Check if we fetched more than requested (indicates more pages)
        let has_more = all_users.len() > params.limit;

        // Trim to requested limit
        let users: Vec<User> = all_users.into_iter().take(params.limit).collect();

        // Generate next cursor from last item
        let next_cursor = if has_more {
            users.last().map(|user| {
                let user_id_str = user.id.to_string();
                Cursor::new(user.created_at, &user_id_str)
            })
        } else {
            None
        };

        // For backward pagination, we'd need to implement prev_cursor
        // For now, we only support forward pagination
        let prev_cursor = None;

        Ok(CursorPage::new(users, next_cursor, prev_cursor, has_more))
    }

    /// Update user status (approve/suspend)
    ///
    /// # Arguments
    /// * `user_id` - The user to update
    /// * `new_status` - The new status to set
    /// * `approved_by` - UUID of the admin user who approved (None for service token approvals)
    ///
    /// # Errors
    ///
    /// Returns an error if the user is not found or database update fails
    pub async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: UserStatus,
        approved_by: Option<Uuid>,
    ) -> AppResult<User> {
        let status_str = shared::enums::user_status_to_str(&new_status);

        // Only set approved_by when activating a user and an approver UUID is provided
        let approved_by_str = if new_status == UserStatus::Active {
            approved_by.map(|uuid| uuid.to_string())
        } else {
            None
        };

        let approved_at = if new_status == UserStatus::Active {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let result = sqlx::query(
            r"
            UPDATE users SET
                user_status = ?1,
                approved_by = ?2,
                approved_at = ?3,
                last_active = CURRENT_TIMESTAMP
            WHERE id = ?4
            ",
        )
        .bind(status_str)
        .bind(approved_by_str)
        .bind(approved_at)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user status: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User with ID: {user_id}")));
        }

        // Return updated user
        self.get_user_impl(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User after status update"))
    }

    /// Update user's `tenant_id` to link them to a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if the user is not found or database update fails
    pub async fn update_user_tenant_id_impl(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        let query = sqlx::query(
            r"
            UPDATE users
            SET tenant_id = $1
            WHERE id = $2
            ",
        )
        .bind(tenant_id.to_string())
        .bind(user_id.to_string());

        let result = query
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to update user tenant ID: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User with ID: {user_id}")));
        }

        Ok(())
    }
    // Public wrapper methods (delegate to _impl versions)

    /// Create a new user (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn create_user(&self, user: &User) -> AppResult<Uuid> {
        self.create_user_impl(user).await
    }

    /// Get user by ID (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user(&self, user_id: Uuid) -> AppResult<Option<User>> {
        self.get_user_impl(user_id).await
    }

    /// Get user by email (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
        self.get_user_by_email_impl(email).await
    }

    /// Get user by Firebase UID
    ///
    /// Looks up a user by their Firebase authentication UID. Used when
    /// authenticating users via Firebase (Google Sign-In, Apple Sign-In, etc.)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user_by_firebase_uid(&self, firebase_uid: &str) -> AppResult<Option<User>> {
        self.get_user_by_field("firebase_uid", firebase_uid).await
    }

    /// Get user by email, returning error if not found (public API)
    ///
    /// # Errors
    /// Returns error if user not found or database operation fails
    pub async fn get_user_by_email_required(&self, email: &str) -> AppResult<User> {
        self.get_user_by_email_required_impl(email).await
    }

    /// Update user's last active timestamp (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn update_last_active(&self, user_id: Uuid) -> AppResult<()> {
        self.update_last_active_impl(user_id).await
    }

    /// Get total user count (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user_count(&self) -> AppResult<i64> {
        self.get_user_count_impl().await
    }

    /// Upsert user profile data (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn upsert_user_profile(
        &self,
        user_id: Uuid,
        profile_data: serde_json::Value,
    ) -> AppResult<()> {
        self.upsert_user_profile_impl(user_id, profile_data).await
    }

    /// Get user profile data (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user_profile(&self, user_id: Uuid) -> AppResult<Option<serde_json::Value>> {
        self.get_user_profile_impl(user_id).await
    }

    /// Get users by status (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_users_by_status(
        &self,
        status: &str,
        tenant_id: Option<TenantId>,
    ) -> AppResult<Vec<User>> {
        self.get_users_by_status_impl(status, tenant_id).await
    }

    /// Get the first admin user in the database
    ///
    /// Used for system coach seeding - needs an admin user to associate coaches with.
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_first_admin_user(&self) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, password_hash, display_name, tier, plan_tier, is_admin,
                   is_active, user_status, created_at, last_active, tenant_id
            FROM users
            WHERE is_admin = 1
            ORDER BY created_at ASC
            LIMIT 1
            ",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get first admin user: {e}")))?;

        row.map(|r| Self::row_to_user(&r)).transpose()
    }

    /// Update user's tenant ID (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: TenantId) -> AppResult<()> {
        self.update_user_tenant_id_impl(user_id, tenant_id).await
    }

    /// Delete a user and all associated data
    ///
    /// Permanently removes the user from the database. Related records in other tables
    /// are automatically deleted via foreign key CASCADE constraints.
    ///
    /// # Errors
    /// Returns error if user not found or database operation fails
    pub async fn delete_user(&self, user_id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            r"
            DELETE FROM users WHERE id = ?1
            ",
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete user: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User {user_id} not found")));
        }

        Ok(())
    }

    /// Update user's display name
    ///
    /// # Errors
    ///
    /// Returns an error if the user is not found or database update fails
    pub async fn update_user_display_name(
        &self,
        user_id: Uuid,
        display_name: &str,
    ) -> AppResult<User> {
        let result = sqlx::query(
            r"
            UPDATE users SET
                display_name = ?1,
                last_active = CURRENT_TIMESTAMP
            WHERE id = ?2
            ",
        )
        .bind(display_name)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user display name: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User with ID: {user_id}")));
        }

        // Return updated user
        self.get_user_impl(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User after display name update"))
    }

    /// Update user's password hash
    ///
    /// # Errors
    ///
    /// Returns an error if the user is not found or database update fails
    pub async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> AppResult<()> {
        let result = sqlx::query(
            r"
            UPDATE users SET
                password_hash = ?1,
                last_active = CURRENT_TIMESTAMP
            WHERE id = ?2
            ",
        )
        .bind(password_hash)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user password: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User with ID: {user_id}")));
        }

        Ok(())
    }
}
