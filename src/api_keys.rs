// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # API Key Management
//!
//! Provides B2B API key generation, validation, and usage tracking
//! for the Pierre MCP Fitness API platform.

use crate::constants::system_config::*;
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// API Key tiers with rate limits
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyTier {
    Trial,        // 1,000 requests/month, auto-expires in 14 days
    Starter,      // 10,000 requests/month
    Professional, // 100,000 requests/month
    Enterprise,   // Unlimited
}

impl ApiKeyTier {
    pub fn monthly_limit(&self) -> Option<u32> {
        match self {
            ApiKeyTier::Trial => Some(TRIAL_MONTHLY_LIMIT),
            ApiKeyTier::Starter => Some(STARTER_MONTHLY_LIMIT),
            ApiKeyTier::Professional => Some(PROFESSIONAL_MONTHLY_LIMIT),
            ApiKeyTier::Enterprise => None, // Unlimited
        }
    }

    pub fn rate_limit_window(&self) -> u32 {
        RATE_LIMIT_WINDOW_SECONDS // 30 days in seconds
    }

    /// Default expiration in days for trial keys
    pub fn default_trial_days(&self) -> Option<i64> {
        match self {
            ApiKeyTier::Trial => Some(TRIAL_PERIOD_DAYS as i64), // Trial period
            _ => None,
        }
    }

    /// Check if this is a trial tier
    pub fn is_trial(&self) -> bool {
        matches!(self, ApiKeyTier::Trial)
    }

    /// Get string representation for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyTier::Trial => "trial",
            ApiKeyTier::Starter => "starter",
            ApiKeyTier::Professional => "professional",
            ApiKeyTier::Enterprise => "enterprise",
        }
    }
}

impl std::str::FromStr for ApiKeyTier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trial" => Ok(ApiKeyTier::Trial),
            "starter" => Ok(ApiKeyTier::Starter),
            "professional" => Ok(ApiKeyTier::Professional),
            "enterprise" => Ok(ApiKeyTier::Enterprise),
            _ => Err(anyhow::anyhow!("Invalid API key tier: {}", s)),
        }
    }
}

/// API Key model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub user_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u32,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// API Key creation request with rate limit
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
    pub rate_limit_requests: Option<u32>, // 0 = unlimited
    pub expires_in_days: Option<i64>,
}

/// New simplified API Key creation request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequestSimple {
    pub name: String,
    pub description: Option<String>,
    pub rate_limit_requests: u32, // 0 = unlimited
    pub expires_in_days: Option<i64>,
}

/// API Key response (includes the actual key only on creation)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>, // Only included on creation
}

/// Usage record for tracking
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyUsage {
    pub id: Option<i64>,
    pub api_key_id: String,
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub response_time_ms: Option<u32>,
    pub status_code: u16,
    pub error_message: Option<String>,
    pub request_size_bytes: Option<u32>,
    pub response_size_bytes: Option<u32>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Aggregated usage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyUsageStats {
    pub api_key_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub total_response_time_ms: u64,
    pub tool_usage: serde_json::Value, // JSON object with tool counts
}

/// Rate limit status
#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    pub is_rate_limited: bool,
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub reset_at: Option<DateTime<Utc>>,
}

/// Generated API key data
#[derive(Debug)]
pub struct ApiKeyData {
    pub full_key: String,
    pub key_prefix: String,
    pub key_hash: String,
}

/// API Key Manager
#[derive(Clone)]
pub struct ApiKeyManager {
    key_prefix: &'static str,
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new() -> Self {
        Self {
            key_prefix: "pk_live_", // Production keys
        }
    }

    /// Generate a new API key with optional trial prefix
    pub fn generate_api_key(&self, is_trial: bool) -> ApiKeyData {
        // Generate 32 random bytes for the key
        let random_bytes: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        // Full key format: pk_live_<32 random chars> or pk_trial_<32 random chars>
        let prefix = if is_trial {
            "pk_trial_"
        } else {
            self.key_prefix
        };
        let full_key = format!("{}{}", prefix, random_bytes);

        // Create key prefix for identification (first 12 chars)
        // More efficient: use string slicing instead of collecting chars
        let key_prefix = if full_key.len() >= 12 {
            full_key[..12].to_string()
        } else {
            full_key.clone()
        };

        // Hash the full key for storage
        let mut hasher = Sha256::new();
        hasher.update(full_key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());

        ApiKeyData {
            full_key,
            key_prefix,
            key_hash,
        }
    }

    /// Validate an API key format
    pub fn validate_key_format(&self, api_key: &str) -> Result<()> {
        if !api_key.starts_with(self.key_prefix) && !api_key.starts_with("pk_trial_") {
            anyhow::bail!("Invalid API key format");
        }

        let expected_len = if api_key.starts_with("pk_trial_") {
            41 // pk_trial_ (9) + 32 chars
        } else {
            40 // pk_live_ (8) + 32 chars
        };

        if api_key.len() != expected_len {
            anyhow::bail!("Invalid API key length");
        }

        Ok(())
    }

    /// Extract key prefix from full key
    pub fn extract_key_prefix(&self, api_key: &str) -> String {
        api_key.chars().take(12).collect()
    }

    /// Hash an API key for comparison
    pub fn hash_key(&self, api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check if an API key string is a trial key
    pub fn is_trial_key(&self, api_key: &str) -> bool {
        api_key.starts_with("pk_trial_")
    }

    /// Create a new API key with simplified request
    pub async fn create_api_key_simple(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequestSimple,
    ) -> Result<(ApiKey, String)> {
        // Determine tier based on rate limit (keep trial functionality but don't expose in UI)
        let tier = if request.rate_limit_requests <= 1_000 {
            ApiKeyTier::Trial
        } else if request.rate_limit_requests <= 10_000 {
            ApiKeyTier::Starter
        } else if request.rate_limit_requests <= 100_000 {
            ApiKeyTier::Professional
        } else {
            ApiKeyTier::Enterprise
        };

        let is_trial = tier.is_trial();

        // Generate the key components
        let api_key_data = self.generate_api_key(is_trial);
        let full_key = api_key_data.full_key;
        let key_prefix = api_key_data.key_prefix;
        let key_hash = api_key_data.key_hash;

        // Calculate expiration
        let expires_at = if is_trial {
            let days = request
                .expires_in_days
                .or_else(|| tier.default_trial_days())
                .unwrap_or(14);
            Some(Utc::now() + Duration::days(days))
        } else {
            request
                .expires_in_days
                .map(|days| Utc::now() + Duration::days(days))
        };

        // Use custom rate limits
        let rate_limit_requests = if request.rate_limit_requests == 0 {
            u32::MAX // Unlimited
        } else {
            request.rate_limit_requests
        };
        let rate_limit_window = tier.rate_limit_window();

        // Create the API key record
        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            user_id,
            name: request.name,
            key_prefix,
            key_hash,
            description: request.description,
            tier,
            rate_limit_requests,
            rate_limit_window_seconds: rate_limit_window,
            is_active: true,
            last_used_at: None,
            expires_at,
            created_at: Utc::now(),
        };

        Ok((api_key, full_key))
    }

    /// Create a new API key (legacy method with tier)
    pub async fn create_api_key(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequest,
    ) -> Result<(ApiKey, String)> {
        // Check if this is a trial key
        let is_trial = request.tier.is_trial();

        // Generate the key components
        let api_key_data = self.generate_api_key(is_trial);
        let full_key = api_key_data.full_key;
        let key_prefix = api_key_data.key_prefix;
        let key_hash = api_key_data.key_hash;

        // Calculate expiration
        // For trial keys, use default trial days if not specified
        let expires_at = if is_trial {
            let days = request
                .expires_in_days
                .or_else(|| request.tier.default_trial_days())
                .unwrap_or(14);
            Some(Utc::now() + Duration::days(days))
        } else {
            request
                .expires_in_days
                .map(|days| Utc::now() + Duration::days(days))
        };

        // Get rate limits - use custom if provided, otherwise use tier defaults
        let rate_limit_requests = request
            .rate_limit_requests
            .unwrap_or_else(|| request.tier.monthly_limit().unwrap_or(u32::MAX));
        let rate_limit_window = request.tier.rate_limit_window();

        // Create the API key record
        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            user_id,
            name: request.name,
            key_prefix,
            key_hash,
            description: request.description,
            tier: request.tier,
            rate_limit_requests,
            rate_limit_window_seconds: rate_limit_window,
            is_active: true,
            last_used_at: None,
            expires_at,
            created_at: Utc::now(),
        };

        Ok((api_key, full_key))
    }

    /// Create a trial API key with default settings
    pub async fn create_trial_key(
        &self,
        user_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<(ApiKey, String)> {
        let request = CreateApiKeyRequest {
            name,
            description,
            tier: ApiKeyTier::Trial,
            rate_limit_requests: None, // Use tier default
            expires_in_days: None,     // Will use default 14 days
        };

        self.create_api_key(user_id, request).await
    }

    /// Check if a key is valid and active
    pub fn is_key_valid(&self, api_key: &ApiKey) -> Result<()> {
        if !api_key.is_active {
            anyhow::bail!("API key is inactive");
        }

        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                anyhow::bail!("API key has expired");
            }
        }

        Ok(())
    }

    /// Get rate limit status for an API key
    pub fn rate_limit_status(&self, api_key: &ApiKey, current_usage: u32) -> RateLimitStatus {
        match api_key.tier {
            ApiKeyTier::Enterprise => RateLimitStatus {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
            },
            _ => {
                let limit = api_key.rate_limit_requests;
                let remaining = limit.saturating_sub(current_usage);
                let is_rate_limited = current_usage >= limit;

                // Calculate reset time (beginning of next month)
                let now = Utc::now();
                let next_month = if now.month() == 12 {
                    now.with_year(now.year() + 1)
                        .expect("Failed to set year for next month calculation")
                        .with_month(1)
                        .expect("Failed to set month to January")
                } else {
                    now.with_month(now.month() + 1)
                        .expect("Failed to increment month")
                };

                let reset_at = next_month
                    .with_day(1)
                    .expect("Failed to set day to 1st of month")
                    .with_hour(0)
                    .expect("Failed to set hour to 0")
                    .with_minute(0)
                    .expect("Failed to set minute to 0")
                    .with_second(0)
                    .expect("Failed to set second to 0");

                RateLimitStatus {
                    is_rate_limited,
                    limit: Some(limit),
                    remaining: Some(remaining),
                    reset_at: Some(reset_at),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_generation() {
        let manager = ApiKeyManager::new();

        // Test regular key generation
        let api_key_data = manager.generate_api_key(false);
        let (full_key, prefix, hash) = (
            api_key_data.full_key,
            api_key_data.key_prefix,
            api_key_data.key_hash,
        );
        assert!(full_key.starts_with("pk_live_"));
        assert_eq!(full_key.len(), 40);
        assert_eq!(prefix.len(), 12);
        assert_eq!(hash.len(), 64); // SHA-256 hex

        // Test trial key generation
        let trial_data = manager.generate_api_key(true);
        let (trial_key, trial_prefix, trial_hash) = (
            trial_data.full_key,
            trial_data.key_prefix,
            trial_data.key_hash,
        );
        assert!(trial_key.starts_with("pk_trial_"));
        assert_eq!(trial_key.len(), 41);
        assert_eq!(trial_prefix.len(), 12);
        assert_eq!(trial_hash.len(), 64);
    }

    #[test]
    fn test_key_validation() {
        let manager = ApiKeyManager::new();

        // Test regular key validation
        assert!(manager
            .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz123456")
            .is_ok());

        // Test trial key validation
        assert!(manager
            .validate_key_format("pk_trial_abcdefghijklmnopqrstuvwxyz123456")
            .is_ok());

        // Test invalid keys
        assert!(manager.validate_key_format("invalid_key").is_err());
        assert!(manager.validate_key_format("pk_live_short").is_err());
        assert!(manager.validate_key_format("pk_trial_short").is_err());
    }

    #[test]
    fn test_tier_limits() {
        assert_eq!(ApiKeyTier::Trial.monthly_limit(), Some(1_000));
        assert_eq!(ApiKeyTier::Starter.monthly_limit(), Some(10_000));
        assert_eq!(ApiKeyTier::Professional.monthly_limit(), Some(100_000));
        assert_eq!(ApiKeyTier::Enterprise.monthly_limit(), None);

        // Test trial defaults
        assert_eq!(ApiKeyTier::Trial.default_trial_days(), Some(14));
        assert_eq!(ApiKeyTier::Starter.default_trial_days(), None);
        assert!(ApiKeyTier::Trial.is_trial());
        assert!(!ApiKeyTier::Starter.is_trial());
    }

    #[test]
    fn test_rate_limit_calculation() {
        let manager = ApiKeyManager::new();

        let api_key = ApiKey {
            id: "test".to_string(),
            user_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            key_prefix: "pk_live_test".to_string(),
            key_hash: "hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        let status = manager.rate_limit_status(&api_key, 5000);
        assert!(!status.is_rate_limited);
        assert_eq!(status.remaining, Some(5000));

        let status = manager.rate_limit_status(&api_key, 10_000);
        assert!(status.is_rate_limited);
        assert_eq!(status.remaining, Some(0));
    }

    #[test]
    fn test_rate_limit_enterprise_unlimited() {
        let manager = ApiKeyManager::new();

        let enterprise_key = ApiKey {
            id: "enterprise".to_string(),
            user_id: Uuid::new_v4(),
            name: "Enterprise Key".to_string(),
            key_prefix: "pk_live_ent".to_string(),
            key_hash: "hash".to_string(),
            description: None,
            tier: ApiKeyTier::Enterprise,
            rate_limit_requests: u32::MAX,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        // Enterprise tier should never be rate limited
        let status = manager.rate_limit_status(&enterprise_key, 0);
        assert!(!status.is_rate_limited);
        assert_eq!(status.limit, None);
        assert_eq!(status.remaining, None);
        assert_eq!(status.reset_at, None);

        let status = manager.rate_limit_status(&enterprise_key, 1_000_000);
        assert!(!status.is_rate_limited);
        assert_eq!(status.limit, None);
        assert_eq!(status.remaining, None);
        assert_eq!(status.reset_at, None);
    }

    #[test]
    fn test_rate_limit_professional_tier() {
        let manager = ApiKeyManager::new();

        let professional_key = ApiKey {
            id: "professional".to_string(),
            user_id: Uuid::new_v4(),
            name: "Professional Key".to_string(),
            key_prefix: "pk_live_pro".to_string(),
            key_hash: "hash".to_string(),
            description: None,
            tier: ApiKeyTier::Professional,
            rate_limit_requests: 100_000,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        // Under limit
        let status = manager.rate_limit_status(&professional_key, 50_000);
        assert!(!status.is_rate_limited);
        assert_eq!(status.limit, Some(100_000));
        assert_eq!(status.remaining, Some(50_000));
        assert!(status.reset_at.is_some());

        // At limit
        let status = manager.rate_limit_status(&professional_key, 100_000);
        assert!(status.is_rate_limited);
        assert_eq!(status.limit, Some(100_000));
        assert_eq!(status.remaining, Some(0));

        // Over limit
        let status = manager.rate_limit_status(&professional_key, 150_000);
        assert!(status.is_rate_limited);
        assert_eq!(status.limit, Some(100_000));
        assert_eq!(status.remaining, Some(0)); // Should be 0, not negative
    }

    #[test]
    fn test_rate_limit_reset_time_calculation() {
        let manager = ApiKeyManager::new();

        let api_key = ApiKey {
            id: "reset_test".to_string(),
            user_id: Uuid::new_v4(),
            name: "Reset Test Key".to_string(),
            key_prefix: "pk_live_reset".to_string(),
            key_hash: "hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        let status = manager.rate_limit_status(&api_key, 5000);

        // Reset time should be beginning of next month
        if let Some(reset_at) = status.reset_at {
            let now = Utc::now();

            // Reset should be in the future
            assert!(reset_at > now);

            // Reset should be at beginning of day (hour 0, minute 0, second 0)
            assert_eq!(reset_at.hour(), 0);
            assert_eq!(reset_at.minute(), 0);
            assert_eq!(reset_at.second(), 0);

            // Reset should be on the 1st day of some month
            assert_eq!(reset_at.day(), 1);
        } else {
            panic!("Reset time should be set for non-enterprise tiers");
        }
    }

    #[test]
    fn test_api_key_validation_edge_cases() {
        let manager = ApiKeyManager::new();

        // Test various invalid key formats
        assert!(manager.validate_key_format("").is_err());
        assert!(manager.validate_key_format("pk_live_").is_err()); // Too short
        assert!(manager
            .validate_key_format("sk_live_abcdefghijklmnopqrstuvwxyz123456")
            .is_err()); // Wrong prefix
        assert!(manager
            .validate_key_format("pk_test_abcdefghijklmnopqrstuvwxyz123456")
            .is_err()); // Wrong prefix
        assert!(manager
            .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz12345")
            .is_err()); // Too short
        assert!(manager
            .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz1234567")
            .is_err()); // Too long

        // Test valid format
        assert!(manager
            .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz123456")
            .is_ok());
    }

    #[test]
    fn test_key_prefix_extraction() {
        let manager = ApiKeyManager::new();

        let full_key = "pk_live_abcdefghijklmnopqrstuvwxyz123456";
        let prefix = manager.extract_key_prefix(full_key);

        assert_eq!(prefix, "pk_live_abcd");
        assert_eq!(prefix.len(), 12);
    }

    #[test]
    fn test_key_hashing_consistency() {
        let manager = ApiKeyManager::new();

        let key = "pk_live_test_key_for_hashing_12345678";
        let hash1 = manager.hash_key(key);
        let hash2 = manager.hash_key(key);

        // Same key should always produce same hash
        assert_eq!(hash1, hash2);

        // Hash should be SHA-256 hex (64 characters)
        assert_eq!(hash1.len(), 64);

        // Different keys should produce different hashes
        let different_key = "pk_live_different_key_12345678901234";
        let hash3 = manager.hash_key(different_key);
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_create_api_key_with_expiration() {
        let manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Expiring Key".to_string(),
            description: Some("Test key with expiration".to_string()),
            tier: ApiKeyTier::Professional,
            rate_limit_requests: None,
            expires_in_days: Some(30),
        };

        let (api_key, full_key) = manager.create_api_key(user_id, request).await.unwrap();

        // Check expiration is set correctly
        assert!(api_key.expires_at.is_some());
        let expires_at = api_key.expires_at.unwrap();
        let expected_expiry = Utc::now() + Duration::days(30);

        // Should be within 1 minute of expected (to account for test execution time)
        let diff = (expires_at - expected_expiry).num_seconds().abs();
        assert!(
            diff < 60,
            "Expiration time should be within 1 minute of expected"
        );

        // Check other properties
        assert_eq!(api_key.user_id, user_id);
        assert_eq!(api_key.name, "Expiring Key");
        assert_eq!(api_key.tier, ApiKeyTier::Professional);
        assert_eq!(api_key.rate_limit_requests, 100_000);
        assert!(api_key.is_active);

        // Full key should be valid format
        assert!(manager.validate_key_format(&full_key).is_ok());
        assert!(full_key.starts_with("pk_live_"));
        assert_eq!(full_key.len(), 40);
    }

    #[tokio::test]
    async fn test_create_api_key_without_expiration() {
        let manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Permanent Key".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: None,
            expires_in_days: None,
        };

        let (api_key, _full_key) = manager.create_api_key(user_id, request).await.unwrap();

        // Should not have expiration
        assert!(api_key.expires_at.is_none());
        assert_eq!(api_key.tier, ApiKeyTier::Starter);
        assert_eq!(api_key.rate_limit_requests, 10_000);
    }

    #[test]
    fn test_api_key_validation_scenarios() {
        let manager = ApiKeyManager::new();

        // Test active key
        let active_key = ApiKey {
            id: "active".to_string(),
            user_id: Uuid::new_v4(),
            name: "Active Key".to_string(),
            key_prefix: "pk_live_active".to_string(),
            key_hash: "hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        assert!(manager.is_key_valid(&active_key).is_ok());

        // Test inactive key
        let mut inactive_key = active_key.clone();
        inactive_key.is_active = false;

        assert!(manager.is_key_valid(&inactive_key).is_err());

        // Test expired key
        let mut expired_key = active_key.clone();
        expired_key.expires_at = Some(Utc::now() - Duration::days(1));

        assert!(manager.is_key_valid(&expired_key).is_err());

        // Test key expiring in future (should be valid)
        let mut future_expiry_key = active_key.clone();
        future_expiry_key.expires_at = Some(Utc::now() + Duration::days(1));

        assert!(manager.is_key_valid(&future_expiry_key).is_ok());
    }

    #[test]
    fn test_tier_specific_properties() {
        // Test that each tier has correct limits
        assert_eq!(ApiKeyTier::Starter.monthly_limit(), Some(10_000));
        assert_eq!(ApiKeyTier::Professional.monthly_limit(), Some(100_000));
        assert_eq!(ApiKeyTier::Enterprise.monthly_limit(), None);

        // Test rate limit windows are consistent
        assert_eq!(ApiKeyTier::Starter.rate_limit_window(), 30 * 24 * 60 * 60);
        assert_eq!(
            ApiKeyTier::Professional.rate_limit_window(),
            30 * 24 * 60 * 60
        );
        assert_eq!(
            ApiKeyTier::Enterprise.rate_limit_window(),
            30 * 24 * 60 * 60
        );
    }

    #[test]
    fn test_generate_multiple_unique_keys() {
        let manager = ApiKeyManager::new();

        // Generate multiple keys and ensure they're all different
        let mut keys = Vec::new();
        for i in 0..10 {
            let is_trial = i % 2 == 0; // Alternate between trial and regular keys
            let api_key_data = manager.generate_api_key(is_trial);
            let (full_key, prefix, hash) = (
                api_key_data.full_key,
                api_key_data.key_prefix,
                api_key_data.key_hash,
            );
            keys.push((full_key, prefix, hash));
        }

        // Check all keys are unique
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i].0, keys[j].0, "Full keys should be unique");
                assert_ne!(keys[i].1, keys[j].1, "Prefixes should be unique");
                assert_ne!(keys[i].2, keys[j].2, "Hashes should be unique");
            }
        }

        // Check all keys have correct format
        for (i, (full_key, prefix, hash)) in keys.iter().enumerate() {
            assert!(manager.validate_key_format(full_key).is_ok());
            assert_eq!(prefix.len(), 12);
            assert_eq!(hash.len(), 64);
            if i % 2 == 0 {
                assert!(full_key.starts_with("pk_trial_"));
                assert!(manager.is_trial_key(full_key));
            } else {
                assert!(full_key.starts_with("pk_live_"));
                assert!(!manager.is_trial_key(full_key));
            }
        }
    }

    #[tokio::test]
    async fn test_create_trial_key() {
        let manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();

        // Create a trial key using the convenience method
        let (api_key, full_key) = manager
            .create_trial_key(
                user_id,
                "Test Trial Key".to_string(),
                Some("Testing trial functionality".to_string()),
            )
            .await
            .unwrap();

        // Verify trial key properties
        assert_eq!(api_key.tier, ApiKeyTier::Trial);
        assert_eq!(api_key.rate_limit_requests, 1_000);
        assert!(api_key.expires_at.is_some());

        // Verify key format
        assert!(full_key.starts_with("pk_trial_"));
        assert!(manager.is_trial_key(&full_key));
        assert!(manager.validate_key_format(&full_key).is_ok());

        // Verify expiration is set to 14 days
        let expires_at = api_key.expires_at.unwrap();
        let expected_expiry = Utc::now() + Duration::days(14);
        let diff = (expires_at - expected_expiry).num_seconds().abs();
        assert!(diff < 60, "Trial key should expire in 14 days");
    }

    #[tokio::test]
    async fn test_create_trial_key_with_custom_expiration() {
        let manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Custom Trial".to_string(),
            description: None,
            tier: ApiKeyTier::Trial,
            rate_limit_requests: None,
            expires_in_days: Some(7), // Custom 7 day trial
        };

        let (api_key, full_key) = manager.create_api_key(user_id, request).await.unwrap();

        // Verify custom expiration is respected
        assert!(api_key.expires_at.is_some());
        let expires_at = api_key.expires_at.unwrap();
        let expected_expiry = Utc::now() + Duration::days(7);
        let diff = (expires_at - expected_expiry).num_seconds().abs();
        assert!(diff < 60, "Trial key should expire in 7 days");

        assert!(full_key.starts_with("pk_trial_"));
    }
}
