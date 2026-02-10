// ABOUTME: Terra webhook handler for receiving push data from 150+ wearables
// ABOUTME: Validates signatures, parses payloads, and stores data in cache for provider access
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Terra webhook handler
//!
//! This module handles incoming webhook events from Terra. When users sync their
//! wearables, Terra pushes data to your configured webhook endpoint.
//!
//! ## Security
//!
//! All webhook requests include a `terra-signature` header containing an HMAC-SHA256
//! signature. This handler validates signatures before processing any data.
//!
//! ## Event Types
//!
//! - `activity` - Workout/activity data
//! - `sleep` - Sleep session data
//! - `body` - Body metrics (weight, body fat, etc.)
//! - `daily` - Daily activity summaries
//! - `nutrition` - Nutrition/food log data
//! - `auth` - Authentication events (user connected/disconnected)

use ring::hmac;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use super::cache::TerraDataCache;
use super::converters::TerraConverters;
use super::models::{TerraDataWrapper, TerraUser, TerraWebhookPayload};

/// Webhook signature validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureValidation {
    /// Signature is valid
    Valid,
    /// Signature is invalid
    Invalid,
    /// Signature header is missing
    Missing,
    /// No signing secret configured — validation cannot be performed
    NotConfigured,
}

/// Validates Terra webhook signatures
pub struct WebhookSignatureValidator {
    /// Terra webhook signing secret
    signing_secret: String,
}

impl WebhookSignatureValidator {
    /// Create a new signature validator
    #[must_use]
    pub const fn new(signing_secret: String) -> Self {
        Self { signing_secret }
    }

    /// Validate a webhook request signature
    ///
    /// # Arguments
    /// * `signature_header` - Value of the `terra-signature` header
    /// * `body` - Raw request body bytes
    ///
    /// # Returns
    /// `SignatureValidation` indicating whether the signature is valid
    #[must_use]
    pub fn validate(&self, signature_header: Option<&str>, body: &[u8]) -> SignatureValidation {
        let Some(signature) = signature_header else {
            return SignatureValidation::Missing;
        };

        // Parse the signature format: "t=timestamp,v1=signature"
        let parts: Vec<&str> = signature.split(',').collect();
        let sig_part = parts.iter().find(|p| p.starts_with("v1="));

        let Some(sig_value) = sig_part.and_then(|p| p.strip_prefix("v1=")) else {
            return SignatureValidation::Invalid;
        };

        // Compute expected signature using ring::hmac
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.signing_secret.as_bytes());
        let tag = hmac::sign(&key, body);
        let expected = hex::encode(tag.as_ref());

        // Constant-time comparison to prevent timing attacks
        if subtle::ConstantTimeEq::ct_eq(sig_value.as_bytes(), expected.as_bytes()).into() {
            SignatureValidation::Valid
        } else {
            SignatureValidation::Invalid
        }
    }
}

/// Result of processing a webhook event
#[derive(Debug, Clone)]
pub enum WebhookResult {
    /// Successfully processed data
    Success {
        /// Event type that was processed
        event_type: String,
        /// Number of items processed
        items_processed: usize,
        /// Terra user ID
        user_id: String,
    },
    /// Authentication event (user connected/disconnected)
    AuthEvent {
        /// Event type (auth, deauth, `user_reauth`)
        event_type: String,
        /// Status message
        status: String,
        /// Terra user ID (if available)
        user_id: Option<String>,
        /// Reference ID (if available)
        reference_id: Option<String>,
    },
    /// Unknown or unhandled event type
    Unhandled {
        /// Event type
        event_type: String,
    },
    /// Error processing webhook
    Error {
        /// Error message
        message: String,
    },
}

/// Terra webhook handler
///
/// Processes incoming webhook events from Terra and stores data in the cache.
pub struct TerraWebhookHandler {
    cache: Arc<TerraDataCache>,
    validator: Option<WebhookSignatureValidator>,
}

impl TerraWebhookHandler {
    /// Create a new webhook handler
    #[must_use]
    pub const fn new(cache: Arc<TerraDataCache>) -> Self {
        Self {
            cache,
            validator: None,
        }
    }

    /// Create a webhook handler with signature validation
    #[must_use]
    pub const fn with_validation(cache: Arc<TerraDataCache>, signing_secret: String) -> Self {
        Self {
            cache,
            validator: Some(WebhookSignatureValidator::new(signing_secret)),
        }
    }

    /// Validate a webhook request signature
    #[must_use]
    pub fn validate_signature(
        &self,
        signature_header: Option<&str>,
        body: &[u8],
    ) -> SignatureValidation {
        self.validator
            .as_ref()
            .map_or(SignatureValidation::NotConfigured, |v| {
                v.validate(signature_header, body)
            })
    }

    /// Parse webhook payload from raw bytes
    fn parse_payload(body: &[u8]) -> Result<TerraWebhookPayload, WebhookResult> {
        serde_json::from_slice(body).map_err(|e| {
            error!("Failed to parse Terra webhook payload: {}", e);
            WebhookResult::Error {
                message: format!("JSON parse error: {e}"),
            }
        })
    }

    /// Check if event type is an authentication event
    fn is_auth_event(event_type: &str) -> bool {
        matches!(event_type, "auth" | "deauth" | "user_reauth")
    }

    /// Extract and validate user from payload
    fn extract_user(payload: &TerraWebhookPayload) -> Result<&TerraUser, WebhookResult> {
        payload.user.as_ref().ok_or_else(|| {
            warn!(
                "Terra webhook missing user field for event type: {}",
                payload.event_type
            );
            WebhookResult::Error {
                message: "Missing user field in payload".to_owned(),
            }
        })
    }

    /// Dispatch to appropriate event processor
    async fn dispatch_event(
        &self,
        payload: &TerraWebhookPayload,
        user: &TerraUser,
        event_type: &str,
    ) -> Result<usize, WebhookResult> {
        match event_type {
            "activity" => Ok(self.process_activities(payload, user).await),
            "sleep" => Ok(self.process_sleep(payload, user).await),
            "body" => Ok(self.process_body(payload, user).await),
            "daily" => Ok(self.process_daily(payload, user).await),
            "nutrition" => Ok(self.process_nutrition(payload, user).await),
            _ => {
                info!("Unhandled Terra webhook event type: {}", event_type);
                Err(WebhookResult::Unhandled {
                    event_type: event_type.to_owned(),
                })
            }
        }
    }

    /// Process a webhook payload
    ///
    /// # Arguments
    /// * `body` - Raw JSON body of the webhook request
    ///
    /// # Returns
    /// `WebhookResult` indicating the outcome of processing
    pub async fn process(&self, body: &[u8]) -> WebhookResult {
        let payload = match Self::parse_payload(body) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let event_type = payload.event_type.clone();
        debug!("Processing Terra webhook event: {}", event_type);

        if Self::is_auth_event(&event_type) {
            return Self::handle_auth_event(&payload);
        }

        let user = match Self::extract_user(&payload) {
            Ok(u) => u,
            Err(e) => return e,
        };

        let user_id = user.user_id.clone();

        if let Some(ref ref_id) = user.reference_id {
            self.cache.register_user_mapping(ref_id, &user_id).await;
        }

        let items_processed = match self.dispatch_event(&payload, user, &event_type).await {
            Ok(count) => count,
            Err(result) => return result,
        };

        info!(
            "Processed {} {} items for Terra user {}",
            items_processed, event_type, user_id
        );

        WebhookResult::Success {
            event_type,
            items_processed,
            user_id,
        }
    }

    /// Handle authentication events
    fn handle_auth_event(payload: &TerraWebhookPayload) -> WebhookResult {
        let event_type = payload.event_type.clone();
        let status = payload
            .status
            .clone()
            .unwrap_or_else(|| "unknown".to_owned());

        let user_id = payload.user.as_ref().map(|u| u.user_id.clone());
        let reference_id = payload.user.as_ref().and_then(|u| u.reference_id.clone());

        info!(
            "Terra auth event: {} - status: {} - user: {:?}",
            event_type, status, user_id
        );

        WebhookResult::AuthEvent {
            event_type,
            status,
            user_id,
            reference_id,
        }
    }

    /// Process activity data
    async fn process_activities(&self, payload: &TerraWebhookPayload, user: &TerraUser) -> usize {
        let Some(data) = payload.data.as_ref() else {
            return 0;
        };

        let mut count = 0;
        for item in data {
            if let TerraDataWrapper::Activity(terra_activity) = item {
                let activity = TerraConverters::activity_from_terra(terra_activity, user);
                self.cache.store_activity(&user.user_id, activity).await;
                count += 1;
            }
        }
        count
    }

    /// Process sleep data
    async fn process_sleep(&self, payload: &TerraWebhookPayload, user: &TerraUser) -> usize {
        let Some(data) = payload.data.as_ref() else {
            return 0;
        };

        let mut count = 0;
        for item in data {
            if let TerraDataWrapper::Sleep(terra_sleep) = item {
                // Store sleep session
                let sleep = TerraConverters::sleep_from_terra(terra_sleep, user);
                self.cache.store_sleep_session(&user.user_id, sleep).await;

                // Also extract recovery metrics from readiness data
                let recovery = TerraConverters::recovery_from_terra_sleep(terra_sleep, user);
                self.cache
                    .store_recovery_metrics(&user.user_id, recovery)
                    .await;

                count += 1;
            }
        }
        count
    }

    /// Process body metrics data
    async fn process_body(&self, payload: &TerraWebhookPayload, user: &TerraUser) -> usize {
        let Some(data) = payload.data.as_ref() else {
            return 0;
        };

        let mut count = 0;
        for item in data {
            if let TerraDataWrapper::Body(terra_body) = item {
                let health = TerraConverters::health_from_terra(terra_body, user);
                self.cache.store_health_metrics(&user.user_id, health).await;
                count += 1;
            }
        }
        count
    }

    /// Process daily summary data
    async fn process_daily(&self, payload: &TerraWebhookPayload, user: &TerraUser) -> usize {
        let Some(data) = payload.data.as_ref() else {
            return 0;
        };

        let mut count = 0;
        for item in data {
            if let TerraDataWrapper::Daily(terra_daily) = item {
                let recovery = TerraConverters::recovery_from_terra_daily(terra_daily, user);
                self.cache
                    .store_recovery_metrics(&user.user_id, recovery)
                    .await;
                count += 1;
            }
        }
        count
    }

    /// Process nutrition data
    async fn process_nutrition(&self, payload: &TerraWebhookPayload, user: &TerraUser) -> usize {
        let Some(data) = payload.data.as_ref() else {
            return 0;
        };

        let mut count = 0;
        for item in data {
            if let TerraDataWrapper::Nutrition(terra_nutrition) = item {
                let nutrition = TerraConverters::nutrition_from_terra(terra_nutrition, user);
                self.cache
                    .store_nutrition_log(&user.user_id, nutrition)
                    .await;
                count += 1;
            }
        }
        count
    }
}
