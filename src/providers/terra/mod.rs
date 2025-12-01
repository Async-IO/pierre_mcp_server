// ABOUTME: Terra unified API provider for accessing 150+ wearables through a single integration
// ABOUTME: Implements webhook-based data ingestion with local caching for FitnessProvider trait compliance
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Terra Provider Module
//!
//! Terra is a unified API platform that provides access to 150+ wearables and health data sources
//! through a single integration. Unlike direct provider integrations (Strava, Garmin), Terra uses
//! a push-based webhook model where data is automatically sent to your endpoint when users sync
//! their devices.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                       Pierre MCP Server                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                    TerraProvider                        ││
//! │  │  (implements FitnessProvider, reads from cache)         ││
//! │  └───────────────────────────┬─────────────────────────────┘│
//! │                              │                               │
//! │  ┌───────────────────────────▼─────────────────────────────┐│
//! │  │                    TerraDataCache                       ││
//! │  │       (In-memory storage for webhook data)              ││
//! │  └───────────────────────────┬─────────────────────────────┘│
//! │                              │                               │
//! │  ┌───────────────────────────▼─────────────────────────────┐│
//! │  │                 TerraWebhookHandler                     ││
//! │  │    (receives POST from Terra, validates signature)      ││
//! │  └───────────────────────────┬─────────────────────────────┘│
//! └──────────────────────────────┼───────────────────────────────┘
//!                                │ Webhook POST
//!                                ▼
//!                        ┌───────────────┐
//!                        │   Terra API   │
//!                        │ (150+ sources)│
//!                        └───────────────┘
//! ```
//!
//! ## Supported Data Types
//!
//! - **Activities**: Workouts, runs, rides, swims, etc.
//! - **Sleep**: Sleep sessions with stages (deep, light, REM, awake)
//! - **Body**: Weight, body fat percentage, BMI
//! - **Daily**: Daily activity summaries, steps, calories
//! - **Nutrition**: Food logs from integrations like MyFitnessPal
//!
//! ## Webhook Events
//!
//! Terra pushes the following event types:
//! - `activity` - Completed workout data
//! - `sleep` - Sleep session data
//! - `body` - Body measurements
//! - `daily` - Daily activity metrics
//! - `nutrition` - Nutrition/food log data
//! - `auth` - Authentication status changes
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::providers::terra::{TerraProvider, TerraDataCache};
//!
//! // Create cache and provider
//! let cache = TerraDataCache::new_in_memory();
//! let provider = TerraProvider::new(cache);
//!
//! // Provider reads from cache populated by webhook handler
//! let activities = provider.get_activities(Some(10), None).await?;
//! ```

mod api_client;
mod cache;
mod converters;
mod provider;

// Public modules for external access
pub mod constants;
pub mod models;
pub mod webhook;

pub use api_client::{TerraApiClient, TerraApiConfig};
pub use cache::TerraDataCache;
pub use converters::TerraConverters;
pub use models::{
    TerraActivity, TerraAthlete, TerraBody, TerraDaily, TerraNutrition, TerraSleep,
    TerraWebhookPayload,
};
pub use provider::{TerraDescriptor, TerraProvider, TerraProviderFactory};
pub use webhook::{
    SignatureValidation, TerraWebhookHandler, WebhookResult, WebhookSignatureValidator,
};
