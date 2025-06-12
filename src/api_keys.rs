// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # API Key Management
//! 
//! Provides B2B API key generation, validation, and usage tracking
//! for the Pierre MCP Fitness API platform.

use anyhow::Result;
use chrono::{DateTime, Utc, Duration, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use uuid::Uuid;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

/// API Key tiers with rate limits
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyTier {
    Starter,      // 10,000 requests/month
    Professional, // 100,000 requests/month
    Enterprise,   // Unlimited
}

impl ApiKeyTier {
    pub fn monthly_limit(&self) -> Option<u32> {
        match self {
            ApiKeyTier::Starter => Some(10_000),
            ApiKeyTier::Professional => Some(100_000),
            ApiKeyTier::Enterprise => None, // Unlimited
        }
    }
    
    pub fn rate_limit_window(&self) -> u32 {
        30 * 24 * 60 * 60 // 30 days in seconds
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
    pub rate_limit_window: u32,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API Key creation request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
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

/// API Key Manager
#[derive(Clone)]
pub struct ApiKeyManager {
    key_prefix: String,
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new() -> Self {
        Self {
            key_prefix: "pk_live_".to_string(), // Production keys
        }
    }
    
    /// Generate a new API key
    pub fn generate_api_key(&self) -> (String, String, String) {
        // Generate 32 random bytes for the key
        let random_bytes: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        // Full key format: pk_live_<32 random chars>
        let full_key = format!("{}{}", self.key_prefix, random_bytes);
        
        // Create key prefix for identification (first 12 chars)
        let key_prefix = full_key.chars().take(12).collect::<String>();
        
        // Hash the full key for storage
        let mut hasher = Sha256::new();
        hasher.update(full_key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());
        
        (full_key, key_prefix, key_hash)
    }
    
    /// Validate an API key format
    pub fn validate_key_format(&self, api_key: &str) -> Result<()> {
        if !api_key.starts_with(&self.key_prefix) {
            anyhow::bail!("Invalid API key format");
        }
        
        if api_key.len() != 40 { // pk_live_ (8) + 32 chars
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
    
    /// Create a new API key
    pub async fn create_api_key(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequest,
    ) -> Result<(ApiKey, String)> {
        // Generate the key components
        let (full_key, key_prefix, key_hash) = self.generate_api_key();
        
        // Calculate expiration
        let expires_at = request.expires_in_days.map(|days| {
            Utc::now() + Duration::days(days)
        });
        
        // Get rate limits for tier
        let rate_limit_requests = request.tier.monthly_limit().unwrap_or(u32::MAX);
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
            rate_limit_window,
            is_active: true,
            last_used_at: None,
            expires_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        Ok((api_key, full_key))
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
    
    /// Calculate rate limit status
    pub fn calculate_rate_limit_status(
        &self,
        api_key: &ApiKey,
        current_usage: u32,
    ) -> RateLimitStatus {
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
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                } else {
                    now.with_month(now.month() + 1).unwrap()
                };
                
                let reset_at = next_month
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap();
                
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

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_key_generation() {
        let manager = ApiKeyManager::new();
        let (full_key, prefix, hash) = manager.generate_api_key();
        
        assert!(full_key.starts_with("pk_live_"));
        assert_eq!(full_key.len(), 40);
        assert_eq!(prefix.len(), 12);
        assert_eq!(hash.len(), 64); // SHA-256 hex
    }
    
    #[test]
    fn test_key_validation() {
        let manager = ApiKeyManager::new();
        
        assert!(manager.validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz123456").is_ok());
        assert!(manager.validate_key_format("invalid_key").is_err());
        assert!(manager.validate_key_format("pk_live_short").is_err());
    }
    
    #[test]
    fn test_tier_limits() {
        assert_eq!(ApiKeyTier::Starter.monthly_limit(), Some(10_000));
        assert_eq!(ApiKeyTier::Professional.monthly_limit(), Some(100_000));
        assert_eq!(ApiKeyTier::Enterprise.monthly_limit(), None);
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
            rate_limit_window: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let status = manager.calculate_rate_limit_status(&api_key, 5000);
        assert!(!status.is_rate_limited);
        assert_eq!(status.remaining, Some(5000));
        
        let status = manager.calculate_rate_limit_status(&api_key, 10_000);
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
            rate_limit_window: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Enterprise tier should never be rate limited
        let status = manager.calculate_rate_limit_status(&enterprise_key, 0);
        assert!(!status.is_rate_limited);
        assert_eq!(status.limit, None);
        assert_eq!(status.remaining, None);
        assert_eq!(status.reset_at, None);
        
        let status = manager.calculate_rate_limit_status(&enterprise_key, 1_000_000);
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
            rate_limit_window: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Under limit
        let status = manager.calculate_rate_limit_status(&professional_key, 50_000);
        assert!(!status.is_rate_limited);
        assert_eq!(status.limit, Some(100_000));
        assert_eq!(status.remaining, Some(50_000));
        assert!(status.reset_at.is_some());
        
        // At limit
        let status = manager.calculate_rate_limit_status(&professional_key, 100_000);
        assert!(status.is_rate_limited);
        assert_eq!(status.limit, Some(100_000));
        assert_eq!(status.remaining, Some(0));
        
        // Over limit
        let status = manager.calculate_rate_limit_status(&professional_key, 150_000);
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
            rate_limit_window: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let status = manager.calculate_rate_limit_status(&api_key, 5000);
        
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
        assert!(manager.validate_key_format("sk_live_abcdefghijklmnopqrstuvwxyz123456").is_err()); // Wrong prefix
        assert!(manager.validate_key_format("pk_test_abcdefghijklmnopqrstuvwxyz123456").is_err()); // Wrong prefix
        assert!(manager.validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz12345").is_err()); // Too short
        assert!(manager.validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz1234567").is_err()); // Too long
        
        // Test valid format
        assert!(manager.validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz123456").is_ok());
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
            expires_in_days: Some(30),
        };
        
        let (api_key, full_key) = manager.create_api_key(user_id, request).await.unwrap();
        
        // Check expiration is set correctly
        assert!(api_key.expires_at.is_some());
        let expires_at = api_key.expires_at.unwrap();
        let expected_expiry = Utc::now() + Duration::days(30);
        
        // Should be within 1 minute of expected (to account for test execution time)
        let diff = (expires_at - expected_expiry).num_seconds().abs();
        assert!(diff < 60, "Expiration time should be within 1 minute of expected");
        
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
            rate_limit_window: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
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
        assert_eq!(ApiKeyTier::Professional.rate_limit_window(), 30 * 24 * 60 * 60);
        assert_eq!(ApiKeyTier::Enterprise.rate_limit_window(), 30 * 24 * 60 * 60);
    }

    #[test]
    fn test_generate_multiple_unique_keys() {
        let manager = ApiKeyManager::new();
        
        // Generate multiple keys and ensure they're all different
        let mut keys = Vec::new();
        for _ in 0..10 {
            let (full_key, prefix, hash) = manager.generate_api_key();
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
        for (full_key, prefix, hash) in keys {
            assert!(manager.validate_key_format(&full_key).is_ok());
            assert_eq!(prefix.len(), 12);
            assert_eq!(hash.len(), 64);
            assert!(full_key.starts_with("pk_live_"));
        }
    }
}