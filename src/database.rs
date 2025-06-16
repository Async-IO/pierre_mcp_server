// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Database Management
//!
//! This module provides database functionality for the multi-tenant Pierre MCP Server.
//! It handles user storage, token encryption, and secure data access patterns.

use crate::api_keys::{ApiKey, ApiKeyTier, ApiKeyUsage, ApiKeyUsageStats};
use crate::models::{DecryptedToken, EncryptedToken, User};
use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike, Utc};
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use uuid::Uuid;

/// Database manager for user and token storage
#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
    encryption_key: Vec<u8>,
}

impl Database {
    /// Create a new database connection
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Ensure SQLite creates the database file if it doesn't exist
        let connection_options = if database_url.starts_with("sqlite:") {
            format!("{database_url}?mode=rwc")
        } else {
            database_url.to_string()
        };

        let pool = SqlitePool::connect(&connection_options).await?;

        let db = Self {
            pool,
            encryption_key,
        };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT NOT NULL,
                strava_access_token TEXT,
                strava_refresh_token TEXT,
                strava_expires_at TEXT,
                strava_scope TEXT,
                strava_nonce TEXT,
                fitbit_access_token TEXT,
                fitbit_refresh_token TEXT,
                fitbit_expires_at TEXT,
                fitbit_scope TEXT,
                fitbit_nonce TEXT,
                created_at TEXT NOT NULL,
                last_active TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on email for fast lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await?;

        // Create user_profiles table for fitness analytics
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_profiles (
                user_id TEXT PRIMARY KEY,
                age INTEGER,
                gender TEXT,
                weight_kg REAL,
                height_cm REAL,
                fitness_level TEXT NOT NULL DEFAULT 'beginner',
                primary_sports TEXT, -- JSON array
                training_history_months INTEGER DEFAULT 0,
                preferred_units TEXT DEFAULT 'metric',
                training_focus TEXT, -- JSON array
                injury_history TEXT, -- JSON array
                hours_per_week REAL DEFAULT 0,
                preferred_days TEXT, -- JSON array
                preferred_duration_minutes INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create goals table for fitness goal tracking
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS goals (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                goal_type TEXT NOT NULL, -- 'distance', 'time', 'frequency', 'performance', 'custom'
                sport_type TEXT,
                target_value REAL NOT NULL,
                target_date TEXT NOT NULL,
                current_value REAL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'active', -- 'active', 'completed', 'paused', 'cancelled'
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create goal_milestones table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS goal_milestones (
                id TEXT PRIMARY KEY,
                goal_id TEXT NOT NULL,
                name TEXT NOT NULL,
                target_value REAL NOT NULL,
                achieved BOOLEAN DEFAULT 0,
                achieved_date TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (goal_id) REFERENCES goals (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create analytics_insights table for storing analysis results
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS analytics_insights (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                activity_id TEXT,
                insight_type TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                confidence REAL NOT NULL,
                severity TEXT NOT NULL, -- 'info', 'warning', 'critical'
                metadata TEXT, -- JSON
                created_at TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_goals_user_id ON goals(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_goal_milestones_goal_id ON goal_milestones(goal_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_analytics_insights_user_id ON analytics_insights(user_id)")
            .execute(&self.pool)
            .await?;

        // Create API key management tables
        self.create_api_key_tables().await?;

        Ok(())
    }

    /// Create API key management tables (migration 002)
    async fn create_api_key_tables(&self) -> Result<()> {
        // API Keys table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                
                -- Key information
                name TEXT NOT NULL,
                key_prefix TEXT NOT NULL, -- First 12 chars for identification (pk_live_xxxx)
                key_hash TEXT NOT NULL, -- SHA-256 hash of full key
                
                -- Metadata
                description TEXT,
                tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
                
                -- Rate limiting
                rate_limit_requests INTEGER NOT NULL DEFAULT 10000, -- Requests per month
                rate_limit_window INTEGER NOT NULL DEFAULT 2592000, -- 30 days in seconds
                
                -- Status
                is_active BOOLEAN NOT NULL DEFAULT true,
                last_used_at TIMESTAMP,
                expires_at TIMESTAMP,
                
                -- Timestamps
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                
                -- Ensure unique key names per user
                UNIQUE(user_id, name)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Index for fast key lookup
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_active ON api_keys(is_active)")
            .execute(&self.pool)
            .await?;

        // API Key Usage table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_key_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
                
                -- Usage metrics
                timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                tool_name TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code INTEGER NOT NULL,
                error_message TEXT,
                
                -- Request metadata
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address TEXT,
                user_agent TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Indexes for analytics
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_api_key_id ON api_key_usage(api_key_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON api_key_usage(timestamp)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_tool_name ON api_key_usage(tool_name)")
            .execute(&self.pool)
            .await?;

        // Aggregated usage stats (for performance)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_key_usage_stats (
                api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
                period_start TIMESTAMP NOT NULL,
                period_end TIMESTAMP NOT NULL,
                
                -- Aggregated metrics
                total_requests INTEGER NOT NULL DEFAULT 0,
                successful_requests INTEGER NOT NULL DEFAULT 0,
                failed_requests INTEGER NOT NULL DEFAULT 0,
                total_response_time_ms INTEGER NOT NULL DEFAULT 0,
                
                -- Per-tool breakdown (JSON)
                tool_usage TEXT NOT NULL DEFAULT '{}', -- JSON object with tool counts
                
                PRIMARY KEY (api_key_id, period_start)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_stats_period ON api_key_usage_stats(period_start, period_end)")
            .execute(&self.pool)
            .await?;

        // Rate limit tracking (sliding window)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_key_rate_limits (
                api_key_id TEXT PRIMARY KEY REFERENCES api_keys(id) ON DELETE CASCADE,
                
                -- Current window
                window_start TIMESTAMP NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                
                -- Quick lookup
                is_rate_limited BOOLEAN NOT NULL DEFAULT false,
                rate_limit_reset_at TIMESTAMP,
                
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a new user
    pub async fn create_user(&self, user: &User) -> Result<Uuid> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, display_name, password_hash, created_at, last_active, is_active)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(user.id.to_string())
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(user.created_at.to_rfc3339())
        .bind(user.last_active.to_rfc3339())
        .bind(user.is_active)
        .execute(&self.pool)
        .await?;

        Ok(user.id)
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE id = ?1")
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row)?)),
            None => Ok(None),
        }
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE email = ?1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row)?)),
            None => Ok(None),
        }
    }

    /// Get user by email, returning error if not found (for authentication)
    pub async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        match self.get_user_by_email(email).await? {
            Some(user) => Ok(user),
            None => Err(anyhow::anyhow!("User not found")),
        }
    }

    /// Update user's Strava token
    pub async fn update_strava_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()> {
        let encrypted_token = EncryptedToken::new(
            access_token,
            refresh_token,
            expires_at,
            scope,
            &self.encryption_key,
        )?;

        sqlx::query(
            r#"
            UPDATE users 
            SET strava_access_token = ?1, strava_refresh_token = ?2, strava_expires_at = ?3, 
                strava_scope = ?4, strava_nonce = ?5, last_active = ?6
            WHERE id = ?7
            "#,
        )
        .bind(&encrypted_token.access_token)
        .bind(&encrypted_token.refresh_token)
        .bind(encrypted_token.expires_at.to_rfc3339())
        .bind(&encrypted_token.scope)
        .bind(&encrypted_token.nonce)
        .bind(Utc::now().to_rfc3339())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get decrypted Strava token for user
    pub async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r#"
            SELECT strava_access_token, strava_refresh_token, strava_expires_at, 
                   strava_scope, strava_nonce 
            FROM users WHERE id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let access_token: Option<String> = row.try_get("strava_access_token")?;
                let refresh_token: Option<String> = row.try_get("strava_refresh_token")?;
                let expires_at: Option<String> = row.try_get("strava_expires_at")?;
                let scope: Option<String> = row.try_get("strava_scope")?;
                let nonce: Option<String> = row.try_get("strava_nonce")?;

                if let (Some(access), Some(refresh), Some(expires), Some(scope), Some(nonce)) =
                    (access_token, refresh_token, expires_at, scope, nonce)
                {
                    let encrypted_token = EncryptedToken {
                        access_token: access,
                        refresh_token: refresh,
                        expires_at: DateTime::parse_from_rfc3339(&expires)?.with_timezone(&Utc),
                        scope,
                        nonce,
                    };

                    let decrypted = encrypted_token.decrypt(&self.encryption_key)?;
                    Ok(Some(decrypted))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Update user's Fitbit token
    pub async fn update_fitbit_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()> {
        let encrypted_token = EncryptedToken::new(
            access_token,
            refresh_token,
            expires_at,
            scope,
            &self.encryption_key,
        )?;

        sqlx::query(
            r#"
            UPDATE users 
            SET fitbit_access_token = ?1, fitbit_refresh_token = ?2, fitbit_expires_at = ?3, 
                fitbit_scope = ?4, fitbit_nonce = ?5, last_active = ?6
            WHERE id = ?7
            "#,
        )
        .bind(&encrypted_token.access_token)
        .bind(&encrypted_token.refresh_token)
        .bind(encrypted_token.expires_at.to_rfc3339())
        .bind(&encrypted_token.scope)
        .bind(&encrypted_token.nonce)
        .bind(Utc::now().to_rfc3339())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get decrypted Fitbit token for user
    pub async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r#"
            SELECT fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, 
                   fitbit_scope, fitbit_nonce 
            FROM users WHERE id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let access_token: Option<String> = row.try_get("fitbit_access_token")?;
                let refresh_token: Option<String> = row.try_get("fitbit_refresh_token")?;
                let expires_at: Option<String> = row.try_get("fitbit_expires_at")?;
                let scope: Option<String> = row.try_get("fitbit_scope")?;
                let nonce: Option<String> = row.try_get("fitbit_nonce")?;

                if let (Some(access), Some(refresh), Some(expires), Some(scope), Some(nonce)) =
                    (access_token, refresh_token, expires_at, scope, nonce)
                {
                    let encrypted_token = EncryptedToken {
                        access_token: access,
                        refresh_token: refresh,
                        expires_at: DateTime::parse_from_rfc3339(&expires)?.with_timezone(&Utc),
                        scope,
                        nonce,
                    };

                    let decrypted = encrypted_token.decrypt(&self.encryption_key)?;
                    Ok(Some(decrypted))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Update user's last active timestamp
    pub async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE users SET last_active = ?1 WHERE id = ?2")
            .bind(Utc::now().to_rfc3339())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get total number of users (for health checks)
    pub async fn get_user_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Convert database row to User model
    fn row_to_user(&self, row: sqlx::sqlite::SqliteRow) -> Result<User> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str)?;

        let email: String = row.try_get("email")?;
        let display_name: Option<String> = row.try_get("display_name")?;
        let password_hash: String = row.try_get("password_hash")?;

        let created_at_str: String = row.try_get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);

        let last_active_str: String = row.try_get("last_active")?;
        let last_active = DateTime::parse_from_rfc3339(&last_active_str)?.with_timezone(&Utc);

        let is_active: bool = row.try_get("is_active")?;

        // Build encrypted tokens if they exist
        let strava_token = self.build_encrypted_token(&row, "strava")?;
        let fitbit_token = self.build_encrypted_token(&row, "fitbit")?;

        Ok(User {
            id,
            email,
            display_name,
            password_hash,
            strava_token,
            fitbit_token,
            created_at,
            last_active,
            is_active,
        })
    }

    /// Build encrypted token from database row
    fn build_encrypted_token(
        &self,
        row: &sqlx::sqlite::SqliteRow,
        provider: &str,
    ) -> Result<Option<EncryptedToken>> {
        let access_token: Option<String> = match provider {
            "strava" => row.try_get("strava_access_token")?,
            "fitbit" => row.try_get("fitbit_access_token")?,
            _ => None,
        };
        let refresh_token: Option<String> = match provider {
            "strava" => row.try_get("strava_refresh_token")?,
            "fitbit" => row.try_get("fitbit_refresh_token")?,
            _ => None,
        };
        let expires_at: Option<String> = match provider {
            "strava" => row.try_get("strava_expires_at")?,
            "fitbit" => row.try_get("fitbit_expires_at")?,
            _ => None,
        };
        let scope: Option<String> = match provider {
            "strava" => row.try_get("strava_scope")?,
            "fitbit" => row.try_get("fitbit_scope")?,
            _ => None,
        };
        let nonce: Option<String> = match provider {
            "strava" => row.try_get("strava_nonce")?,
            "fitbit" => row.try_get("fitbit_nonce")?,
            _ => None,
        };

        if let (Some(access), Some(refresh), Some(expires), Some(scope), Some(nonce)) =
            (access_token, refresh_token, expires_at, scope, nonce)
        {
            Ok(Some(EncryptedToken {
                access_token: access,
                refresh_token: refresh,
                expires_at: DateTime::parse_from_rfc3339(&expires)?.with_timezone(&Utc),
                scope,
                nonce,
            }))
        } else {
            Ok(None)
        }
    }

    // === ANALYTICS METHODS ===

    /// Create or update user fitness profile
    pub async fn upsert_user_profile(
        &self,
        user_id: Uuid,
        profile_data: serde_json::Value,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO user_profiles (
                user_id, age, gender, weight_kg, height_cm, fitness_level,
                primary_sports, training_history_months, preferred_units,
                training_focus, injury_history, hours_per_week, preferred_days,
                preferred_duration_minutes, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 
                     COALESCE((SELECT created_at FROM user_profiles WHERE user_id = ?1), ?15), ?16)
            "#,
        )
        .bind(user_id.to_string())
        .bind(profile_data.get("age").and_then(|v| v.as_i64()))
        .bind(profile_data.get("gender").and_then(|v| v.as_str()))
        .bind(profile_data.get("weight_kg").and_then(|v| v.as_f64()))
        .bind(profile_data.get("height_cm").and_then(|v| v.as_f64()))
        .bind(
            profile_data
                .get("fitness_level")
                .and_then(|v| v.as_str())
                .unwrap_or("beginner"),
        )
        .bind(
            profile_data
                .get("primary_sports")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string()),
        )
        .bind(
            profile_data
                .get("training_history_months")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        )
        .bind(
            profile_data
                .get("preferred_units")
                .and_then(|v| v.as_str())
                .unwrap_or("metric"),
        )
        .bind(
            profile_data
                .get("training_focus")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string()),
        )
        .bind(
            profile_data
                .get("injury_history")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string()),
        )
        .bind(
            profile_data
                .get("hours_per_week")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        )
        .bind(
            profile_data
                .get("preferred_days")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string()),
        )
        .bind(
            profile_data
                .get("preferred_duration_minutes")
                .and_then(|v| v.as_i64()),
        )
        .bind(&now) // for created_at when inserting new record
        .bind(&now) // for updated_at
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user fitness profile
    pub async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query("SELECT * FROM user_profiles WHERE user_id = ?1")
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let mut profile = serde_json::Map::new();

            if let Ok(Some(age)) = row.try_get::<Option<i64>, _>("age") {
                profile.insert("age".to_string(), serde_json::Value::Number(age.into()));
            }

            if let Ok(Some(gender)) = row.try_get::<Option<String>, _>("gender") {
                profile.insert("gender".to_string(), serde_json::Value::String(gender));
            }

            if let Ok(Some(weight)) = row.try_get::<Option<f64>, _>("weight_kg") {
                profile.insert(
                    "weight_kg".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(weight).unwrap_or_else(|| 0.into()),
                    ),
                );
            }

            if let Ok(fitness_level) = row.try_get::<String, _>("fitness_level") {
                profile.insert(
                    "fitness_level".to_string(),
                    serde_json::Value::String(fitness_level),
                );
            }

            Ok(Some(serde_json::Value::Object(profile)))
        } else {
            Ok(None)
        }
    }

    /// Create a new goal
    pub async fn create_goal(&self, user_id: Uuid, goal_data: serde_json::Value) -> Result<String> {
        let goal_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO goals (
                id, user_id, title, description, goal_type, sport_type,
                target_value, target_date, current_value, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(&goal_id)
        .bind(user_id.to_string())
        .bind(
            goal_data
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Goal"),
        )
        .bind(
            goal_data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(
            goal_data
                .get("goal_type")
                .and_then(|v| v.as_str())
                .unwrap_or("custom"),
        )
        .bind(goal_data.get("sport_type").and_then(|v| v.as_str()))
        .bind(
            goal_data
                .get("target_value")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        )
        .bind(
            goal_data
                .get("target_date")
                .and_then(|v| v.as_str())
                .unwrap_or(&now),
        )
        .bind(
            goal_data
                .get("current_value")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        )
        .bind("active")
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(goal_id)
    }

    /// Get user goals
    pub async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query("SELECT * FROM goals WHERE user_id = ?1 ORDER BY created_at DESC")
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut goals = Vec::new();
        for row in rows {
            let mut goal = serde_json::Map::new();

            if let Ok(id) = row.try_get::<String, _>("id") {
                goal.insert("id".to_string(), serde_json::Value::String(id));
            }
            if let Ok(title) = row.try_get::<String, _>("title") {
                goal.insert("title".to_string(), serde_json::Value::String(title));
            }
            if let Ok(goal_type) = row.try_get::<String, _>("goal_type") {
                goal.insert(
                    "goal_type".to_string(),
                    serde_json::Value::String(goal_type),
                );
            }
            if let Ok(target_value) = row.try_get::<f64, _>("target_value") {
                goal.insert(
                    "target_value".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(target_value).unwrap_or_else(|| 0.into()),
                    ),
                );
            }
            if let Ok(current_value) = row.try_get::<f64, _>("current_value") {
                goal.insert(
                    "current_value".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(current_value).unwrap_or_else(|| 0.into()),
                    ),
                );
            }
            if let Ok(status) = row.try_get::<String, _>("status") {
                goal.insert("status".to_string(), serde_json::Value::String(status));
            }

            goals.push(serde_json::Value::Object(goal));
        }

        Ok(goals)
    }

    /// Update goal progress
    pub async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query("UPDATE goals SET current_value = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(current_value)
            .bind(&now)
            .bind(goal_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store analytics insight
    pub async fn store_insight(
        &self,
        user_id: Uuid,
        insight_data: serde_json::Value,
    ) -> Result<String> {
        let insight_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO analytics_insights (
                id, user_id, activity_id, insight_type, title, description,
                confidence, severity, metadata, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&insight_id)
        .bind(user_id.to_string())
        .bind(insight_data.get("activity_id").and_then(|v| v.as_str()))
        .bind(
            insight_data
                .get("insight_type")
                .and_then(|v| v.as_str())
                .unwrap_or("general"),
        )
        .bind(
            insight_data
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Insight"),
        )
        .bind(
            insight_data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(
            insight_data
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5),
        )
        .bind(
            insight_data
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("info"),
        )
        .bind(
            insight_data
                .get("metadata")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "{}".to_string()),
        )
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(insight_id)
    }

    /// Get user insights
    pub async fn get_user_insights(
        &self,
        user_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<serde_json::Value>> {
        let limit = limit.unwrap_or(50);

        let rows = sqlx::query(
            "SELECT * FROM analytics_insights WHERE user_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .bind(user_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut insights = Vec::new();
        for row in rows {
            let mut insight = serde_json::Map::new();

            if let Ok(id) = row.try_get::<String, _>("id") {
                insight.insert("id".to_string(), serde_json::Value::String(id));
            }
            if let Ok(insight_type) = row.try_get::<String, _>("insight_type") {
                insight.insert(
                    "insight_type".to_string(),
                    serde_json::Value::String(insight_type),
                );
            }
            if let Ok(title) = row.try_get::<String, _>("title") {
                insight.insert("title".to_string(), serde_json::Value::String(title));
            }
            if let Ok(description) = row.try_get::<String, _>("description") {
                insight.insert(
                    "description".to_string(),
                    serde_json::Value::String(description),
                );
            }

            insights.push(serde_json::Value::Object(insight));
        }

        Ok(insights)
    }
}

/// Generate a random encryption key for token storage
pub fn generate_encryption_key() -> [u8; 32] {
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut key = [0u8; 32];
    rng.fill(&mut key)
        .expect("Failed to generate encryption key");
    key
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    async fn create_test_db() -> Database {
        let database_url = "sqlite::memory:";
        let encryption_key = generate_encryption_key().to_vec();

        Database::new(database_url, encryption_key).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let db = create_test_db().await;

        let user = User::new(
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            Some("Test User".to_string()),
        );
        let user_id = db.create_user(&user).await.unwrap();

        let retrieved = db.get_user(user_id).await.unwrap().unwrap();
        assert_eq!(retrieved.email, "test@example.com");
        assert_eq!(retrieved.display_name, Some("Test User".to_string()));
        assert_eq!(retrieved.password_hash, "hashed_password");
        assert!(retrieved.is_active);
    }

    #[tokio::test]
    async fn test_get_user_by_email() {
        let db = create_test_db().await;

        let user = User::new(
            "email@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let retrieved = db
            .get_user_by_email("email@example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, user_id);
        assert_eq!(retrieved.email, "email@example.com");
    }

    #[tokio::test]
    async fn test_strava_token_storage() {
        let db = create_test_db().await;

        let user = User::new(
            "strava@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let expires_at = Utc::now() + chrono::Duration::hours(6);

        // Store token
        db.update_strava_token(
            user_id,
            "access_token_123",
            "refresh_token_456",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

        // Retrieve token
        let token = db.get_strava_token(user_id).await.unwrap().unwrap();
        assert_eq!(token.access_token, "access_token_123");
        assert_eq!(token.refresh_token, "refresh_token_456");
        assert_eq!(token.scope, "read,activity:read_all");

        // Check token expiry is close to what we set
        let diff = (token.expires_at - expires_at).num_seconds().abs();
        assert!(diff < 2); // Within 2 seconds
    }

    #[tokio::test]
    async fn test_fitbit_token_storage() {
        let db = create_test_db().await;

        let user = User::new(
            "fitbit@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let expires_at = Utc::now() + chrono::Duration::hours(8);

        // Store token
        db.update_fitbit_token(
            user_id,
            "fitbit_access_789",
            "fitbit_refresh_101112",
            expires_at,
            "activity heartrate profile".to_string(),
        )
        .await
        .unwrap();

        // Retrieve token
        let token = db.get_fitbit_token(user_id).await.unwrap().unwrap();
        assert_eq!(token.access_token, "fitbit_access_789");
        assert_eq!(token.refresh_token, "fitbit_refresh_101112");
        assert_eq!(token.scope, "activity heartrate profile");
    }

    #[tokio::test]
    async fn test_last_active_update() {
        let db = create_test_db().await;

        let user = User::new(
            "active@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let initial_active = user.last_active;
        let user_id = db.create_user(&user).await.unwrap();

        // Wait a bit and update
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        db.update_last_active(user_id).await.unwrap();

        let updated_user = db.get_user(user_id).await.unwrap().unwrap();
        assert!(updated_user.last_active > initial_active);
    }

    // === API KEY TESTS ===

    #[tokio::test]
    async fn test_create_and_retrieve_api_key() {
        let db = create_test_db().await;

        // Create test user
        let user = User::new(
            "apikey@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        // Create API key
        let api_key = ApiKey {
            id: "test_key_id".to_string(),
            user_id,
            name: "Test API Key".to_string(),
            key_prefix: "pk_live_test".to_string(),
            key_hash: "test_hash_12345".to_string(),
            description: Some("Test API key for unit tests".to_string()),
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window: 2_592_000, // 30 days
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Store API key
        db.create_api_key(&api_key).await.unwrap();

        // Retrieve API key by prefix and hash
        let retrieved = db
            .get_api_key_by_prefix("pk_live_test", "test_hash_12345")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, "test_key_id");
        assert_eq!(retrieved.name, "Test API Key");
        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.tier, ApiKeyTier::Starter);
        assert_eq!(retrieved.rate_limit_requests, 10_000);
        assert!(retrieved.is_active);
    }

    #[tokio::test]
    async fn test_get_user_api_keys() {
        let db = create_test_db().await;

        // Create test user
        let user = User::new(
            "multikeys@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        // Create multiple API keys for the user
        let api_key1 = ApiKey {
            id: "key1".to_string(),
            user_id,
            name: "Production Key".to_string(),
            key_prefix: "pk_live_prod".to_string(),
            key_hash: "hash1".to_string(),
            description: Some("Production environment".to_string()),
            tier: ApiKeyTier::Professional,
            rate_limit_requests: 100_000,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let api_key2 = ApiKey {
            id: "key2".to_string(),
            user_id,
            name: "Development Key".to_string(),
            key_prefix: "pk_live_dev".to_string(),
            key_hash: "hash2".to_string(),
            description: Some("Development environment".to_string()),
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_api_key(&api_key1).await.unwrap();
        db.create_api_key(&api_key2).await.unwrap();

        // Retrieve all keys for user
        let user_keys = db.get_user_api_keys(user_id).await.unwrap();

        assert_eq!(user_keys.len(), 2);
        assert!(user_keys.iter().any(|k| k.name == "Production Key"));
        assert!(user_keys.iter().any(|k| k.name == "Development Key"));
    }

    #[tokio::test]
    async fn test_api_key_last_used_update() {
        let db = create_test_db().await;

        // Create test user and API key
        let user = User::new(
            "usage@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let api_key = ApiKey {
            id: "usage_key".to_string(),
            user_id,
            name: "Usage Test Key".to_string(),
            key_prefix: "pk_live_usage".to_string(),
            key_hash: "usage_hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_api_key(&api_key).await.unwrap();

        // Initially last_used_at should be None
        let retrieved = db
            .get_api_key_by_prefix("pk_live_usage", "usage_hash")
            .await
            .unwrap()
            .unwrap();
        assert!(retrieved.last_used_at.is_none());

        // Update last used
        db.update_api_key_last_used("usage_key").await.unwrap();

        // Verify last_used_at is now set
        let updated = db
            .get_api_key_by_prefix("pk_live_usage", "usage_hash")
            .await
            .unwrap()
            .unwrap();
        assert!(updated.last_used_at.is_some());
        assert!(updated.last_used_at.unwrap() > retrieved.created_at);
    }

    #[tokio::test]
    async fn test_deactivate_api_key() {
        let db = create_test_db().await;

        // Create test user and API key
        let user = User::new(
            "deactivate@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let api_key = ApiKey {
            id: "deactivate_key".to_string(),
            user_id,
            name: "Deactivate Test Key".to_string(),
            key_prefix: "pk_live_deact".to_string(),
            key_hash: "deact_hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_api_key(&api_key).await.unwrap();

        // Initially should be active and retrievable
        let retrieved = db
            .get_api_key_by_prefix("pk_live_deact", "deact_hash")
            .await
            .unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_active);

        // Deactivate the key
        db.deactivate_api_key("deactivate_key", user_id)
            .await
            .unwrap();

        // Should no longer be retrievable (because query filters for is_active = true)
        let deactivated = db
            .get_api_key_by_prefix("pk_live_deact", "deact_hash")
            .await
            .unwrap();
        assert!(deactivated.is_none());

        // But should still appear in user's key list
        let user_keys = db.get_user_api_keys(user_id).await.unwrap();
        assert_eq!(user_keys.len(), 1);
        assert!(!user_keys[0].is_active);
    }

    #[tokio::test]
    async fn test_record_api_key_usage() {
        let db = create_test_db().await;

        // Create test user and API key
        let user = User::new(
            "tracking@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let api_key = ApiKey {
            id: "tracking_key".to_string(),
            user_id,
            name: "Usage Tracking Key".to_string(),
            key_prefix: "pk_live_track".to_string(),
            key_hash: "track_hash".to_string(),
            description: None,
            tier: ApiKeyTier::Professional,
            rate_limit_requests: 100_000,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_api_key(&api_key).await.unwrap();

        // Record some usage
        let usage1 = ApiKeyUsage {
            id: None,
            api_key_id: "tracking_key".to_string(),
            timestamp: Utc::now(),
            tool_name: "get_activities".to_string(),
            response_time_ms: Some(150),
            status_code: 200,
            error_message: None,
            request_size_bytes: Some(256),
            response_size_bytes: Some(1024),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("claude-mcp-client/1.0".to_string()),
        };

        let usage2 = ApiKeyUsage {
            id: None,
            api_key_id: "tracking_key".to_string(),
            timestamp: Utc::now(),
            tool_name: "analyze_activity".to_string(),
            response_time_ms: Some(75),
            status_code: 200,
            error_message: None,
            request_size_bytes: Some(128),
            response_size_bytes: Some(512),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("claude-mcp-client/1.0".to_string()),
        };

        db.record_api_key_usage(&usage1).await.unwrap();
        db.record_api_key_usage(&usage2).await.unwrap();

        // Get current usage count
        let current_usage = db.get_api_key_current_usage("tracking_key").await.unwrap();
        assert_eq!(current_usage, 2);
    }

    #[tokio::test]
    async fn test_api_key_usage_stats() {
        let db = create_test_db().await;

        // Create test user and API key
        let user = User::new(
            "stats@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let api_key = ApiKey {
            id: "stats_key".to_string(),
            user_id,
            name: "Stats Test Key".to_string(),
            key_prefix: "pk_live_stats".to_string(),
            key_hash: "stats_hash".to_string(),
            description: None,
            tier: ApiKeyTier::Enterprise,
            rate_limit_requests: u32::MAX,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_api_key(&api_key).await.unwrap();

        let now = Utc::now();
        let start_date = now - chrono::Duration::days(7);
        let end_date = now;

        // Record various usage patterns
        let usages = vec![
            ApiKeyUsage {
                id: None,
                api_key_id: "stats_key".to_string(),
                timestamp: now - chrono::Duration::hours(1),
                tool_name: "get_activities".to_string(),
                response_time_ms: Some(100),
                status_code: 200,
                error_message: None,
                request_size_bytes: Some(256),
                response_size_bytes: Some(1024),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("test-client".to_string()),
            },
            ApiKeyUsage {
                id: None,
                api_key_id: "stats_key".to_string(),
                timestamp: now - chrono::Duration::hours(2),
                tool_name: "get_activities".to_string(),
                response_time_ms: Some(200),
                status_code: 200,
                error_message: None,
                request_size_bytes: Some(256),
                response_size_bytes: Some(1024),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("test-client".to_string()),
            },
            ApiKeyUsage {
                id: None,
                api_key_id: "stats_key".to_string(),
                timestamp: now - chrono::Duration::hours(3),
                tool_name: "analyze_activity".to_string(),
                response_time_ms: Some(50),
                status_code: 400,
                error_message: Some("Invalid activity ID".to_string()),
                request_size_bytes: Some(128),
                response_size_bytes: Some(256),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("test-client".to_string()),
            },
        ];

        for usage in usages {
            db.record_api_key_usage(&usage).await.unwrap();
        }

        // Get usage statistics
        let stats = db
            .get_api_key_usage_stats("stats_key", start_date, end_date)
            .await
            .unwrap();

        assert_eq!(stats.api_key_id, "stats_key");
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.successful_requests, 2);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.total_response_time_ms, 350); // 100 + 200 + 50
    }

    #[tokio::test]
    async fn test_api_key_wrong_hash() {
        let db = create_test_db().await;

        // Create test user and API key
        let user = User::new(
            "wrong@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        let api_key = ApiKey {
            id: "wrong_key".to_string(),
            user_id,
            name: "Wrong Hash Test".to_string(),
            key_prefix: "pk_live_wrong".to_string(),
            key_hash: "correct_hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_api_key(&api_key).await.unwrap();

        // Try to retrieve with correct prefix but wrong hash
        let result = db
            .get_api_key_by_prefix("pk_live_wrong", "wrong_hash")
            .await
            .unwrap();
        assert!(result.is_none());

        // Try with correct hash
        let result = db
            .get_api_key_by_prefix("pk_live_wrong", "correct_hash")
            .await
            .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_api_key_expiration_handling() {
        let db = create_test_db().await;

        // Create test user
        let user = User::new(
            "expired@example.com".to_string(),
            "hashed_password".to_string(),
            None,
        );
        let user_id = db.create_user(&user).await.unwrap();

        // Create expired API key
        let expired_key = ApiKey {
            id: "expired_key".to_string(),
            user_id,
            name: "Expired Key".to_string(),
            key_prefix: "pk_live_exp".to_string(),
            key_hash: "expired_hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window: 2_592_000,
            is_active: true,
            last_used_at: None,
            expires_at: Some(Utc::now() - chrono::Duration::days(1)), // Expired yesterday
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now() - chrono::Duration::days(30),
        };

        db.create_api_key(&expired_key).await.unwrap();

        // Key should still be retrievable from database (expiration is handled at application level)
        let retrieved = db
            .get_api_key_by_prefix("pk_live_exp", "expired_hash")
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let key = retrieved.unwrap();
        assert!(key.expires_at.is_some());
        assert!(key.expires_at.unwrap() < Utc::now());
    }
}

// API Key Management Methods
impl Database {
    /// Create a new API key
    pub async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO api_keys (
                id, user_id, name, key_prefix, key_hash, description, tier,
                rate_limit_requests, rate_limit_window, is_active, expires_at,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
        )
        .bind(&api_key.id)
        .bind(api_key.user_id.to_string())
        .bind(&api_key.name)
        .bind(&api_key.key_prefix)
        .bind(&api_key.key_hash)
        .bind(&api_key.description)
        .bind(match api_key.tier {
            ApiKeyTier::Trial => "trial",
            ApiKeyTier::Starter => "starter",
            ApiKeyTier::Professional => "professional",
            ApiKeyTier::Enterprise => "enterprise",
        })
        .bind(api_key.rate_limit_requests as i64)
        .bind(api_key.rate_limit_window as i64)
        .bind(api_key.is_active)
        .bind(api_key.expires_at.map(|dt| dt.to_rfc3339()))
        .bind(api_key.created_at.to_rfc3339())
        .bind(api_key.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get API key by prefix and validate hash
    pub async fn get_api_key_by_prefix(
        &self,
        key_prefix: &str,
        key_hash: &str,
    ) -> Result<Option<ApiKey>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM api_keys 
            WHERE key_prefix = ?1 AND key_hash = ?2 AND is_active = true
            "#,
        )
        .bind(key_prefix)
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.parse_api_key_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get all API keys for a user
    pub async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM api_keys 
            WHERE user_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut keys = Vec::new();
        for row in rows {
            keys.push(self.parse_api_key_row(row)?);
        }

        Ok(keys)
    }

    /// Update API key last used timestamp
    pub async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE api_keys 
            SET last_used_at = ?1, updated_at = ?1
            WHERE id = ?2
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(api_key_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deactivate an API key
    pub async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE api_keys 
            SET is_active = false, updated_at = ?1
            WHERE id = ?2 AND user_id = ?3
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(api_key_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up expired API keys (deactivate them)
    pub async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE api_keys 
            SET is_active = false, updated_at = ?1
            WHERE is_active = true 
            AND expires_at IS NOT NULL 
            AND expires_at < ?2
            "#,
        )
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get all expired but still active API keys
    pub async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>> {
        let now = Utc::now().to_rfc3339();

        let rows = sqlx::query(
            r#"
            SELECT * FROM api_keys 
            WHERE is_active = true 
            AND expires_at IS NOT NULL 
            AND expires_at < ?1
            ORDER BY expires_at ASC
            "#,
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;

        let mut api_keys = Vec::new();
        for row in rows {
            if let Ok(api_key) = self.parse_api_key_row(row) {
                api_keys.push(api_key);
            }
        }

        Ok(api_keys)
    }

    /// Record API key usage
    pub async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO api_key_usage (
                api_key_id, timestamp, tool_name, response_time_ms, status_code,
                error_message, request_size_bytes, response_size_bytes,
                ip_address, user_agent
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&usage.api_key_id)
        .bind(usage.timestamp.to_rfc3339())
        .bind(&usage.tool_name)
        .bind(usage.response_time_ms.map(|ms| ms as i64))
        .bind(usage.status_code as i64)
        .bind(&usage.error_message)
        .bind(usage.request_size_bytes.map(|b| b as i64))
        .bind(usage.response_size_bytes.map(|b| b as i64))
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get current month usage for an API key
    pub async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        let start_of_month = Utc::now()
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();

        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM api_key_usage 
            WHERE api_key_id = ?1 AND timestamp >= ?2
            "#,
        )
        .bind(api_key_id)
        .bind(start_of_month.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u32)
    }

    /// Get usage statistics for an API key
    pub async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats> {
        // First get overall stats
        let stats_row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_requests,
                SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) as successful_requests,
                SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as failed_requests,
                COALESCE(SUM(response_time_ms), 0) as total_response_time_ms
            FROM api_key_usage
            WHERE api_key_id = ?1 AND timestamp >= ?2 AND timestamp < ?3
            "#,
        )
        .bind(api_key_id)
        .bind(start_date.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

        // Then get tool usage counts
        let tool_rows = sqlx::query(
            r#"
            SELECT tool_name, COUNT(*) as count
            FROM api_key_usage
            WHERE api_key_id = ?1 AND timestamp >= ?2 AND timestamp < ?3
            GROUP BY tool_name
            "#,
        )
        .bind(api_key_id)
        .bind(start_date.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        // Build tool usage JSON
        let mut tool_usage = serde_json::Map::new();
        for row in tool_rows {
            let tool_name: String = row.get("tool_name");
            let count: i64 = row.get("count");
            tool_usage.insert(tool_name, serde_json::Value::Number(count.into()));
        }

        Ok(ApiKeyUsageStats {
            api_key_id: api_key_id.to_string(),
            period_start: start_date,
            period_end: end_date,
            total_requests: stats_row.get::<i64, _>("total_requests") as u32,
            successful_requests: stats_row.get::<i64, _>("successful_requests") as u32,
            failed_requests: stats_row.get::<i64, _>("failed_requests") as u32,
            total_response_time_ms: stats_row.get::<i64, _>("total_response_time_ms") as u64,
            tool_usage: serde_json::Value::Object(tool_usage),
        })
    }

    /// Parse API key row from database
    fn parse_api_key_row(&self, row: sqlx::sqlite::SqliteRow) -> Result<ApiKey> {
        let tier_str: String = row.get("tier");
        let tier = match tier_str.as_str() {
            "trial" => ApiKeyTier::Trial,
            "starter" => ApiKeyTier::Starter,
            "professional" => ApiKeyTier::Professional,
            "enterprise" => ApiKeyTier::Enterprise,
            _ => ApiKeyTier::Starter,
        };

        Ok(ApiKey {
            id: row.get("id"),
            user_id: Uuid::parse_str(row.get("user_id"))?,
            name: row.get("name"),
            key_prefix: row.get("key_prefix"),
            key_hash: row.get("key_hash"),
            description: row.get("description"),
            tier,
            rate_limit_requests: row.get::<i64, _>("rate_limit_requests") as u32,
            rate_limit_window: row.get::<i64, _>("rate_limit_window") as u32,
            is_active: row.get("is_active"),
            last_used_at: row
                .get::<Option<String>, _>("last_used_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            expires_at: row
                .get::<Option<String>, _>("expires_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            created_at: DateTime::parse_from_rfc3339(row.get("created_at"))
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(row.get("updated_at"))
                .unwrap()
                .with_timezone(&Utc),
        })
    }
}
