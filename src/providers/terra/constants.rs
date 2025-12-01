// ABOUTME: Constants for Terra API type mappings
// ABOUTME: Defines named constants for activity types, sleep stages, and API URLs
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Terra API constants
//!
//! This module provides named constants for Terra's numeric type codes,
//! avoiding magic numbers in the converter code. Reference:
//! <https://docs.tryterra.co/reference/activity-types>

// =============================================================================
// API URLs
// =============================================================================

/// Base URL for Terra API v2
pub const TERRA_API_BASE_URL: &str = "https://api.tryterra.co/v2";

/// URL for generating widget sessions (user authentication)
pub const TERRA_WIDGET_SESSION_URL: &str = "https://api.tryterra.co/v2/auth/generateWidgetSession";

/// URL for token operations
pub const TERRA_TOKEN_URL: &str = "https://api.tryterra.co/v2/auth/token";

/// URL for deauthenticating users
pub const TERRA_DEAUTH_URL: &str = "https://api.tryterra.co/v2/auth/deauthenticateUser";

// =============================================================================
// Sleep Stage Types
// Reference: Terra Sleep Data Schema
// =============================================================================

/// Sleep stage: Awake
pub const TERRA_SLEEP_STAGE_AWAKE: i32 = 1;

/// Sleep stage: Light sleep
pub const TERRA_SLEEP_STAGE_LIGHT: i32 = 2;

/// Sleep stage: Deep sleep
pub const TERRA_SLEEP_STAGE_DEEP: i32 = 3;

/// Sleep stage: REM sleep
pub const TERRA_SLEEP_STAGE_REM: i32 = 4;

// =============================================================================
// Activity Types
// Reference: https://docs.tryterra.co/reference/activity-types
// =============================================================================

// Running variants (1-4)
/// Running activity
pub const TERRA_ACTIVITY_RUN: i32 = 1;

/// Indoor/treadmill running
pub const TERRA_ACTIVITY_INDOOR_RUN: i32 = 2;

/// Trail running
pub const TERRA_ACTIVITY_TRAIL_RUN: i32 = 3;

/// Treadmill (virtual run)
pub const TERRA_ACTIVITY_TREADMILL: i32 = 4;

// Cycling variants (5-9)
/// Road cycling
pub const TERRA_ACTIVITY_RIDE: i32 = 5;

/// Indoor cycling (virtual ride)
pub const TERRA_ACTIVITY_INDOOR_CYCLING: i32 = 6;

/// Mountain biking
pub const TERRA_ACTIVITY_MOUNTAIN_BIKE: i32 = 7;

/// Gravel riding
pub const TERRA_ACTIVITY_GRAVEL_RIDE: i32 = 8;

/// Electric bike ride
pub const TERRA_ACTIVITY_EBIKE_RIDE: i32 = 9;

// Swimming variants (10-12)
/// General swimming
pub const TERRA_ACTIVITY_SWIM: i32 = 10;

/// Pool swimming
pub const TERRA_ACTIVITY_POOL_SWIM: i32 = 11;

/// Open water swimming
pub const TERRA_ACTIVITY_OPEN_WATER_SWIM: i32 = 12;

// Walking/Hiking (13-14)
/// Walking
pub const TERRA_ACTIVITY_WALK: i32 = 13;

/// Hiking
pub const TERRA_ACTIVITY_HIKE: i32 = 14;

// Winter sports (15-18)
/// Cross-country skiing
pub const TERRA_ACTIVITY_CROSS_COUNTRY_SKI: i32 = 15;

/// Alpine/downhill skiing
pub const TERRA_ACTIVITY_ALPINE_SKI: i32 = 16;

/// Snowboarding
pub const TERRA_ACTIVITY_SNOWBOARD: i32 = 17;

/// Snowshoeing
pub const TERRA_ACTIVITY_SNOWSHOE: i32 = 18;

// Water sports (19-22)
/// Rowing
pub const TERRA_ACTIVITY_ROWING: i32 = 19;

/// Kayaking
pub const TERRA_ACTIVITY_KAYAKING: i32 = 20;

/// Stand-up paddleboarding
pub const TERRA_ACTIVITY_PADDLEBOARD: i32 = 21;

/// Surfing
pub const TERRA_ACTIVITY_SURFING: i32 = 22;

// Gym/Fitness (30-33)
/// Strength training / weight lifting
pub const TERRA_ACTIVITY_STRENGTH_TRAINING: i32 = 30;

/// `CrossFit` workout
pub const TERRA_ACTIVITY_CROSSFIT: i32 = 31;

/// Yoga
pub const TERRA_ACTIVITY_YOGA: i32 = 32;

/// Pilates
pub const TERRA_ACTIVITY_PILATES: i32 = 33;

// Team sports (40-43)
/// Soccer/Football
pub const TERRA_ACTIVITY_SOCCER: i32 = 40;

/// Basketball
pub const TERRA_ACTIVITY_BASKETBALL: i32 = 41;

/// Tennis
pub const TERRA_ACTIVITY_TENNIS: i32 = 42;

/// Golf
pub const TERRA_ACTIVITY_GOLF: i32 = 43;

// Other activities (50-52)
/// Rock climbing
pub const TERRA_ACTIVITY_ROCK_CLIMBING: i32 = 50;

/// Skateboarding
pub const TERRA_ACTIVITY_SKATEBOARDING: i32 = 51;

/// Inline skating / rollerblading
pub const TERRA_ACTIVITY_INLINE_SKATING: i32 = 52;

// Generic
/// Unknown/generic activity type
pub const TERRA_ACTIVITY_UNKNOWN: i32 = 0;
