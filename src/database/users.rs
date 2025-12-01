// ABOUTME: User management database operations
// ABOUTME: Handles user registration, authentication, and profile management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::Database;
use crate::database_plugins::shared;
use crate::errors::{AppError, AppResult};
use crate::models::{EncryptedToken, User, UserStatus};
use crate::pagination::{Cursor, CursorPage, PaginationParams};
use sqlx::Row;
use uuid::Uuid;

impl Database {
    /// Create or update a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The email is already in use by another user
    /// - Database operation fails
    // Long function: Comprehensive user creation with validation, duplicate checking, and database-specific implementations
    #[allow(clippy::too_many_lines)]
    pub async fn create_user_impl(&self, user: &User) -> AppResult<Uuid> {
        // Check if user exists by email
        let existing = self.get_user_by_email_impl(&user.email).await?;
        if let Some(existing_user) = existing {
            if existing_user.id != user.id {
                return Err(AppError::invalid_input(
                    "Email already in use by another user",
                ));
            }
            // Update existing user (including tokens)
            let (strava_access, strava_refresh, strava_expires, strava_scope) = user
                .strava_token
                .as_ref()
                .map_or((None, None, None, None), |token| {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                    )
                });

            let (fitbit_access, fitbit_refresh, fitbit_expires, fitbit_scope) = user
                .fitbit_token
                .as_ref()
                .map_or((None, None, None, None), |token| {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                    )
                });

            sqlx::query(
                r"
                UPDATE users SET
                    display_name = $2,
                    password_hash = $3,
                    tier = $4,
                    tenant_id = $5,
                    strava_access_token = $6,
                    strava_refresh_token = $7,
                    strava_expires_at = $8,
                    strava_scope = $9,
                    fitbit_access_token = $10,
                    fitbit_refresh_token = $11,
                    fitbit_expires_at = $12,
                    fitbit_scope = $13,
                    is_active = $14,
                    user_status = $15,
                    approved_by = $16,
                    approved_at = $17,
                    last_active = CURRENT_TIMESTAMP
                WHERE id = $1
                ",
            )
            .bind(user.id.to_string())
            .bind(&user.display_name)
            .bind(&user.password_hash)
            .bind(user.tier.as_str())
            .bind(&user.tenant_id)
            .bind(strava_access)
            .bind(strava_refresh)
            .bind(strava_expires)
            .bind(strava_scope)
            .bind(fitbit_access)
            .bind(fitbit_refresh)
            .bind(fitbit_expires)
            .bind(fitbit_scope)
            .bind(user.is_active)
            .bind(shared::enums::user_status_to_str(&user.user_status))
            .bind(user.is_admin)
            .bind(user.approved_by.map(|id| id.to_string()))
            .bind(user.approved_at)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to update user: {e}")))?;
        } else {
            // Insert new user (including tokens)
            let (strava_access, strava_refresh, strava_expires, strava_scope) = user
                .strava_token
                .as_ref()
                .map_or((None, None, None, None), |token| {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                    )
                });

            let (fitbit_access, fitbit_refresh, fitbit_expires, fitbit_scope) = user
                .fitbit_token
                .as_ref()
                .map_or((None, None, None, None), |token| {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                    )
                });

            sqlx::query(
                r"
                INSERT INTO users (
                    id, email, display_name, password_hash, tier, tenant_id,
                    strava_access_token, strava_refresh_token, strava_expires_at, strava_scope,
                    fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope,
                    is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
                ",
            )
            .bind(user.id.to_string())
            .bind(&user.email)
            .bind(&user.display_name)
            .bind(&user.password_hash)
            .bind(user.tier.as_str())
            .bind(&user.tenant_id)
            .bind(strava_access)
            .bind(strava_refresh)
            .bind(strava_expires)
            .bind(strava_scope)
            .bind(fitbit_access)
            .bind(fitbit_refresh)
            .bind(fitbit_expires)
            .bind(fitbit_scope)
            .bind(user.is_active)
            .bind(shared::enums::user_status_to_str(&user.user_status))
            .bind(user.is_admin)
            .bind(user.approved_by.map(|id| id.to_string()))
            .bind(user.approved_at)
            .bind(user.created_at)
            .bind(user.last_active)
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
        let query = format!(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id,
                   strava_access_token, strava_refresh_token, strava_expires_at, strava_scope,
                   fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope,
                   is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
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
    fn row_to_user(row: &sqlx::sqlite::SqliteRow) -> AppResult<User> {
        let id: String = row.get("id");
        let email: String = row.get("email");
        let display_name: Option<String> = row.get("display_name");
        let password_hash: String = row.get("password_hash");
        let tier: String = row.get("tier");
        let tenant_id: Option<String> = row.get("tenant_id");
        let is_active: bool = row.get("is_active");
        let user_status_str: String = row.get("user_status");
        let user_status = shared::enums::str_to_user_status(&user_status_str);
        let is_admin: bool = row.get("is_admin");
        let approved_by: Option<String> = row.get("approved_by");
        let approved_at: Option<chrono::DateTime<chrono::Utc>> = row.get("approved_at");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let last_active: chrono::DateTime<chrono::Utc> = row.get("last_active");

        // Handle Strava token
        let strava_token = if let (Some(access), Some(refresh), Some(expires_at)) = (
            row.get::<Option<String>, _>("strava_access_token"),
            row.get::<Option<String>, _>("strava_refresh_token"),
            row.get::<Option<i64>, _>("strava_expires_at"),
        ) {
            let scope: Option<String> = row.get("strava_scope");

            Some(EncryptedToken {
                access_token: access,
                refresh_token: refresh,
                expires_at: chrono::DateTime::from_timestamp(expires_at, 0).unwrap_or_else(|| {
                    tracing::warn!(
                        user_id = %id,
                        provider = "strava",
                        expires_at = %expires_at,
                        "Invalid OAuth token expiry timestamp - using epoch default"
                    );
                    chrono::DateTime::default()
                }),
                scope: scope.unwrap_or_default(),
            })
        } else {
            None
        };

        // Handle Fitbit token
        let fitbit_token = if let (Some(access), Some(refresh), Some(expires_at)) = (
            row.get::<Option<String>, _>("fitbit_access_token"),
            row.get::<Option<String>, _>("fitbit_refresh_token"),
            row.get::<Option<i64>, _>("fitbit_expires_at"),
        ) {
            let scope: Option<String> = row.get("fitbit_scope");

            Some(EncryptedToken {
                access_token: access,
                refresh_token: refresh,
                expires_at: chrono::DateTime::from_timestamp(expires_at, 0).unwrap_or_else(|| {
                    tracing::warn!(
                        user_id = %id,
                        provider = "fitbit",
                        expires_at = %expires_at,
                        "Invalid OAuth token expiry timestamp - using epoch default"
                    );
                    chrono::DateTime::default()
                }),
                scope: scope.unwrap_or_default(),
            })
        } else {
            None
        };

        Ok(User {
            id: Uuid::parse_str(&id)
                .map_err(|e| AppError::internal(format!("Failed to parse user id UUID: {e}")))?,
            email,
            display_name,
            password_hash,
            tier: tier
                .parse()
                .map_err(|e| AppError::internal(format!("Failed to parse tier: {e}")))?,
            tenant_id,
            strava_token,
            fitbit_token,
            is_active,
            user_status,
            is_admin,
            approved_by: approved_by.and_then(|id_str| {
                Uuid::parse_str(&id_str)
                    .inspect_err(|e| {
                        tracing::warn!(
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
    ) -> AppResult<Option<crate::intelligence::UserFitnessProfile>> {
        self.get_user_profile_impl(user_id).await?.map_or_else(
            || Ok(None),
            |profile_data| {
                // Try to deserialize as UserFitnessProfile
                serde_json::from_value(profile_data).map_or_else(
                    |_| {
                        // If profile data doesn't match UserFitnessProfile structure,
                        // create a default profile with user_id
                        Ok(Some(crate::intelligence::UserFitnessProfile {
                            user_id: user_id.to_string(),
                            age: None,
                            gender: None,
                            weight: None,
                            height: None,
                            fitness_level: crate::intelligence::FitnessLevel::Beginner,
                            primary_sports: vec![],
                            training_history_months: 0,
                            preferences: crate::intelligence::UserPreferences {
                                preferred_units: "metric".into(),
                                training_focus: vec![],
                                injury_history: vec![],
                                time_availability: crate::intelligence::TimeAvailability {
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
        profile: &crate::intelligence::UserFitnessProfile,
    ) -> AppResult<()> {
        let profile_data = serde_json::to_value(profile)?;
        self.upsert_user_profile_impl(user_id, profile_data).await
    }

    /// Get last sync timestamp for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The provider is not supported
    /// - The database query fails
    pub async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<chrono::DateTime<chrono::Utc>>> {
        let column = match provider {
            "strava" => "strava_last_sync",
            "fitbit" => "fitbit_last_sync",
            _ => {
                return Err(AppError::invalid_input(format!(
                    "Unsupported provider: {provider}"
                )))
            }
        };

        let query = format!("SELECT {column} FROM users WHERE id = $1");
        let last_sync: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(&query)
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get provider last sync: {e}")))?;

        Ok(last_sync)
    }

    /// Update last sync timestamp for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The provider is not supported
    /// - The database query fails
    pub async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()> {
        let column = match provider {
            "strava" => "strava_last_sync",
            "fitbit" => "fitbit_last_sync",
            _ => {
                return Err(AppError::invalid_input(format!(
                    "Unsupported provider: {provider}"
                )))
            }
        };

        let query = format!("UPDATE users SET {column} = $1 WHERE id = $2");
        sqlx::query(&query)
            .bind(sync_time)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to update provider last sync: {e}")))?;

        Ok(())
    }

    /// Get users by status (offset-based - legacy)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_users_by_status_impl(&self, status: &str) -> AppResult<Vec<User>> {
        let rows =
            sqlx::query("SELECT * FROM users WHERE user_status = ?1 ORDER BY created_at DESC")
                .bind(status)
                .fetch_all(&self.pool)
                .await
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
                       strava_access_token, strava_refresh_token, strava_expires_at, strava_scope,
                       fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope,
                       is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
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
                       strava_access_token, strava_refresh_token, strava_expires_at, strava_scope,
                       fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope,
                       is_active, user_status, is_admin, approved_by, approved_at, created_at, last_active
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
    /// # Errors
    ///
    /// Returns an error if the user is not found or database update fails
    pub async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: UserStatus,
        admin_token_id: &str,
    ) -> AppResult<User> {
        let status_str = shared::enums::user_status_to_str(&new_status);

        let admin_uuid = if new_status == UserStatus::Active && !admin_token_id.is_empty() {
            Some(admin_token_id)
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
        .bind(admin_uuid)
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
        tenant_id: &str,
    ) -> AppResult<()> {
        let query = sqlx::query(
            r"
            UPDATE users 
            SET tenant_id = $1
            WHERE id = $2
            ",
        )
        .bind(tenant_id)
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
    pub async fn get_users_by_status(&self, status: &str) -> AppResult<Vec<User>> {
        self.get_users_by_status_impl(status).await
    }

    /// Update user's tenant ID (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> AppResult<()> {
        self.update_user_tenant_id_impl(user_id, tenant_id).await
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
