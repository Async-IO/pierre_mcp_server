// ABOUTME: Database operations for coach authors (creator profiles for Store)
// ABOUTME: Handles CRUD operations for author profiles with tenant isolation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Utc};
use pierre_core::models::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

/// A coach author is a public profile for coach creators in the Store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachAuthor {
    /// Unique identifier
    pub id: Uuid,
    /// User who is the author
    pub user_id: Uuid,
    /// Tenant for multi-tenancy isolation
    pub tenant_id: String,
    /// Display name shown in Store
    pub display_name: String,
    /// Author bio/description
    pub bio: Option<String>,
    /// Avatar image URL
    pub avatar_url: Option<String>,
    /// Personal website URL
    pub website_url: Option<String>,
    /// Whether the author is verified (trusted)
    pub is_verified: bool,
    /// When the author was verified
    pub verified_at: Option<DateTime<Utc>>,
    /// Admin who verified the author
    pub verified_by: Option<Uuid>,
    /// Number of published coaches
    pub published_coach_count: u32,
    /// Total installs across all coaches
    pub total_install_count: u32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Request to create an author profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuthorRequest {
    /// Display name shown in Store
    pub display_name: String,
    /// Author bio/description
    pub bio: Option<String>,
    /// Avatar image URL
    pub avatar_url: Option<String>,
    /// Personal website URL
    pub website_url: Option<String>,
}

/// Request to update an author profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAuthorRequest {
    /// New display name (if provided)
    pub display_name: Option<String>,
    /// New bio (if provided)
    pub bio: Option<String>,
    /// New avatar URL (if provided)
    pub avatar_url: Option<String>,
    /// New website URL (if provided)
    pub website_url: Option<String>,
}

/// Manager for coach author operations
pub struct CoachAuthorsManager {
    pool: SqlitePool,
}

impl CoachAuthorsManager {
    /// Create a new manager with the given connection pool
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create an author profile for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User already has an author profile
    /// - Database operation fails
    pub async fn create(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &CreateAuthorRequest,
    ) -> AppResult<CoachAuthor> {
        let now = Utc::now();
        let id = Uuid::new_v4();

        sqlx::query(
            r"
            INSERT INTO coach_authors (
                id, user_id, tenant_id, display_name, bio, avatar_url, website_url,
                is_verified, published_coach_count, total_install_count, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(&request.display_name)
        .bind(&request.bio)
        .bind(&request.avatar_url)
        .bind(&request.website_url)
        .bind(0i64) // is_verified
        .bind(0i64) // published_coach_count
        .bind(0i64) // total_install_count
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                AppError::invalid_input("Author profile already exists for this user")
            } else {
                AppError::database(format!("Failed to create author profile: {e}"))
            }
        })?;

        Ok(CoachAuthor {
            id,
            user_id,
            tenant_id: tenant_id.to_string(),
            display_name: request.display_name.clone(),
            bio: request.bio.clone(),
            avatar_url: request.avatar_url.clone(),
            website_url: request.website_url.clone(),
            is_verified: false,
            verified_at: None,
            verified_by: None,
            published_coach_count: 0,
            total_install_count: 0,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get author profile by user ID
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_by_user(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<Option<CoachAuthor>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, display_name, bio, avatar_url, website_url,
                   is_verified, verified_at, verified_by, published_coach_count,
                   total_install_count, created_at, updated_at
            FROM coach_authors
            WHERE user_id = $1 AND tenant_id = $2
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get author profile: {e}")))?;

        row.map(|r| row_to_author(&r)).transpose()
    }

    /// Get author profile by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_by_id(&self, author_id: &str) -> AppResult<Option<CoachAuthor>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, display_name, bio, avatar_url, website_url,
                   is_verified, verified_at, verified_by, published_coach_count,
                   total_install_count, created_at, updated_at
            FROM coach_authors
            WHERE id = $1
            ",
        )
        .bind(author_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get author profile: {e}")))?;

        row.map(|r| row_to_author(&r)).transpose()
    }

    /// Update an author profile
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Author profile not found
    /// - Database operation fails
    pub async fn update(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &UpdateAuthorRequest,
    ) -> AppResult<Option<CoachAuthor>> {
        let existing = self
            .get_by_user(user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Author profile"))?;

        let now = Utc::now();
        let display_name = request
            .display_name
            .as_ref()
            .unwrap_or(&existing.display_name);
        let bio = request.bio.as_ref().or(existing.bio.as_ref());
        let avatar_url = request.avatar_url.as_ref().or(existing.avatar_url.as_ref());
        let website_url = request
            .website_url
            .as_ref()
            .or(existing.website_url.as_ref());

        let result = sqlx::query(
            r"
            UPDATE coach_authors SET
                display_name = $1, bio = $2, avatar_url = $3, website_url = $4, updated_at = $5
            WHERE user_id = $6 AND tenant_id = $7
            ",
        )
        .bind(display_name)
        .bind(bio)
        .bind(avatar_url)
        .bind(website_url)
        .bind(now.to_rfc3339())
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update author profile: {e}")))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_by_user(user_id, tenant_id).await
    }

    /// Verify an author (admin operation)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn verify_author(
        &self,
        author_id: &str,
        admin_user_id: Uuid,
    ) -> AppResult<CoachAuthor> {
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE coach_authors SET
                is_verified = 1, verified_at = $1, verified_by = $2, updated_at = $1
            WHERE id = $3
            ",
        )
        .bind(now.to_rfc3339())
        .bind(admin_user_id.to_string())
        .bind(author_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to verify author: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Author profile"));
        }

        self.get_by_id(author_id)
            .await?
            .ok_or_else(|| AppError::not_found("Author profile"))
    }

    /// Increment published coach count for an author
    ///
    /// Called when a coach is approved for the Store.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn increment_published_count(&self, author_id: &str) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE coach_authors SET
                published_coach_count = published_coach_count + 1,
                updated_at = $1
            WHERE id = $2
            ",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(author_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to increment published count: {e}")))?;

        Ok(())
    }

    /// Update total install count for an author
    ///
    /// Called when aggregating install counts from all coaches.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_install_count(&self, author_id: &str, delta: i32) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE coach_authors SET
                total_install_count = MAX(0, total_install_count + $1),
                updated_at = $2
            WHERE id = $3
            ",
        )
        .bind(i64::from(delta))
        .bind(Utc::now().to_rfc3339())
        .bind(author_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update install count: {e}")))?;

        Ok(())
    }

    /// List popular authors (most total installs)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_popular(
        &self,
        tenant_id: TenantId,
        limit: Option<u32>,
    ) -> AppResult<Vec<CoachAuthor>> {
        let limit_val = i64::from(limit.unwrap_or(20).min(100));

        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, display_name, bio, avatar_url, website_url,
                   is_verified, verified_at, verified_by, published_coach_count,
                   total_install_count, created_at, updated_at
            FROM coach_authors
            WHERE tenant_id = $1 AND published_coach_count > 0
            ORDER BY total_install_count DESC
            LIMIT $2
            ",
        )
        .bind(tenant_id.to_string())
        .bind(limit_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list popular authors: {e}")))?;

        rows.iter().map(row_to_author).collect()
    }

    /// List verified authors
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_verified(
        &self,
        tenant_id: TenantId,
        limit: Option<u32>,
    ) -> AppResult<Vec<CoachAuthor>> {
        let limit_val = i64::from(limit.unwrap_or(20).min(100));

        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, display_name, bio, avatar_url, website_url,
                   is_verified, verified_at, verified_by, published_coach_count,
                   total_install_count, created_at, updated_at
            FROM coach_authors
            WHERE tenant_id = $1 AND is_verified = 1
            ORDER BY total_install_count DESC
            LIMIT $2
            ",
        )
        .bind(tenant_id.to_string())
        .bind(limit_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list verified authors: {e}")))?;

        rows.iter().map(row_to_author).collect()
    }

    /// Get or create author profile for a user
    ///
    /// Returns existing profile if it exists, creates a new one otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_or_create(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        display_name: &str,
    ) -> AppResult<CoachAuthor> {
        if let Some(author) = self.get_by_user(user_id, tenant_id).await? {
            return Ok(author);
        }

        let request = CreateAuthorRequest {
            display_name: display_name.to_owned(),
            bio: None,
            avatar_url: None,
            website_url: None,
        };

        self.create(user_id, tenant_id, &request).await
    }
}

/// Convert a database row to a `CoachAuthor` struct
fn row_to_author(row: &SqliteRow) -> AppResult<CoachAuthor> {
    let id_str: String = row.get("id");
    let user_id_str: String = row.get("user_id");
    let is_verified: i64 = row.get("is_verified");
    let verified_at_str: Option<String> = row.get("verified_at");
    let verified_by_str: Option<String> = row.get("verified_by");
    let published_coach_count: i64 = row.get("published_coach_count");
    let total_install_count: i64 = row.get("total_install_count");
    let created_at_str: String = row.get("created_at");
    let updated_at_str: String = row.get("updated_at");

    Ok(CoachAuthor {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| AppError::internal(format!("Invalid UUID: {e}")))?,
        user_id: Uuid::parse_str(&user_id_str)
            .map_err(|e| AppError::internal(format!("Invalid UUID: {e}")))?,
        tenant_id: row.get("tenant_id"),
        display_name: row.get("display_name"),
        bio: row.get("bio"),
        avatar_url: row.get("avatar_url"),
        website_url: row.get("website_url"),
        is_verified: is_verified == 1,
        verified_at: verified_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
        verified_by: verified_by_str.and_then(|s| Uuid::parse_str(&s).ok()),
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        published_coach_count: published_coach_count as u32,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        total_install_count: total_install_count as u32,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| AppError::internal(format!("Invalid datetime: {e}")))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| AppError::internal(format!("Invalid datetime: {e}")))?
            .with_timezone(&Utc),
    })
}
