// ABOUTME: Sport type enumeration for fitness activities
// ABOUTME: Defines all supported sport types with parsing and display implementations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use serde::{Deserialize, Serialize};

use crate::config::FitnessConfig;

/// Enumeration of supported sport/activity types
///
/// This enum covers the most common fitness activities across all providers.
/// The `Other` variant handles provider-specific activity types that don't
/// map to the standard categories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SportType {
    /// Running activity
    Run,
    /// Cycling/biking activity
    Ride,
    /// Swimming activity
    Swim,
    /// Walking activity
    Walk,
    /// Hiking activity
    Hike,

    // Virtual/Indoor activities
    /// Indoor/trainer cycling activity
    VirtualRide,
    /// Treadmill running activity
    VirtualRun,
    /// Generic workout/exercise activity
    Workout,
    /// Yoga practice
    Yoga,

    // E-bike and specialty cycling
    /// Electric bike ride
    EbikeRide,
    /// Mountain biking activity
    MountainBike,
    /// Gravel cycling activity
    GravelRide,

    // Winter sports
    /// Cross-country skiing
    CrossCountrySkiing,
    /// Alpine/downhill skiing
    AlpineSkiing,
    /// Snowboarding activity
    Snowboarding,
    /// Snowshoeing activity
    Snowshoe,
    /// Ice skating activity
    IceSkating,
    /// Backcountry skiing
    BackcountrySkiing,

    // Water sports
    /// Kayaking activity
    Kayaking,
    /// Canoeing activity
    Canoeing,
    /// Rowing activity
    Rowing,
    /// Stand-up paddleboarding
    Paddleboarding,
    /// Surfing activity
    Surfing,
    /// Kitesurfing activity
    Kitesurfing,

    // Strength and fitness
    /// Weight/strength training
    StrengthTraining,
    /// `CrossFit` workout
    Crossfit,
    /// Pilates session
    Pilates,

    // Climbing and adventure
    /// Rock climbing activity
    RockClimbing,
    /// Trail running
    TrailRunning,

    // Team and racquet sports
    /// Soccer/football
    Soccer,
    /// Basketball
    Basketball,
    /// Tennis
    Tennis,
    /// Golf
    Golf,

    // Alternative transport
    /// Skateboarding
    Skateboarding,
    /// Inline skating
    InlineSkating,

    /// Other activity type not covered by standard categories
    Other(String),
}

impl SportType {
    /// Create `SportType` from provider string using configuration mapping
    #[must_use]
    pub fn from_provider_string(provider_sport: &str, fitness_config: &FitnessConfig) -> Self {
        // First check if we have a configured mapping
        if let Some(internal_name) = fitness_config.map_sport_type(provider_sport) {
            return Self::from_internal_string(internal_name);
        }

        // Direct string-to-enum mapping
        match provider_sport {
            "Run" => Self::Run,
            "Ride" => Self::Ride,
            "Swim" => Self::Swim,
            "Walk" => Self::Walk,
            "Hike" => Self::Hike,
            "VirtualRide" => Self::VirtualRide,
            "VirtualRun" => Self::VirtualRun,
            "Workout" => Self::Workout,
            "Yoga" => Self::Yoga,
            "EBikeRide" => Self::EbikeRide,
            "MountainBikeRide" => Self::MountainBike,
            "GravelRide" => Self::GravelRide,
            "CrossCountrySkiing" => Self::CrossCountrySkiing,
            "AlpineSkiing" => Self::AlpineSkiing,
            "Snowboarding" => Self::Snowboarding,
            "Snowshoe" => Self::Snowshoe,
            "IceSkate" => Self::IceSkating,
            "BackcountrySki" => Self::BackcountrySkiing,
            "Kayaking" => Self::Kayaking,
            "Canoeing" => Self::Canoeing,
            "Rowing" => Self::Rowing,
            "StandUpPaddling" => Self::Paddleboarding,
            "Surfing" => Self::Surfing,
            "Kitesurf" => Self::Kitesurfing,
            "WeightTraining" => Self::StrengthTraining,
            "Crossfit" => Self::Crossfit,
            "Pilates" => Self::Pilates,
            "RockClimbing" => Self::RockClimbing,
            "TrailRunning" => Self::TrailRunning,
            "Soccer" => Self::Soccer,
            "Basketball" => Self::Basketball,
            "Tennis" => Self::Tennis,
            "Golf" => Self::Golf,
            "Skateboard" => Self::Skateboarding,
            "InlineSkate" => Self::InlineSkating,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Create `SportType` from internal configuration string
    #[must_use]
    pub fn from_internal_string(internal_name: &str) -> Self {
        match internal_name {
            "run" => Self::Run,
            "bike_ride" => Self::Ride,
            "swim" => Self::Swim,
            "walk" => Self::Walk,
            "hike" => Self::Hike,
            "virtual_ride" => Self::VirtualRide,
            "virtual_run" => Self::VirtualRun,
            "workout" => Self::Workout,
            "yoga" => Self::Yoga,
            "ebike_ride" => Self::EbikeRide,
            "mountain_bike" => Self::MountainBike,
            "gravel_ride" => Self::GravelRide,
            "cross_country_skiing" => Self::CrossCountrySkiing,
            "alpine_skiing" => Self::AlpineSkiing,
            "snowboarding" => Self::Snowboarding,
            "snowshoe" => Self::Snowshoe,
            "ice_skating" => Self::IceSkating,
            "backcountry_skiing" => Self::BackcountrySkiing,
            "kayaking" => Self::Kayaking,
            "canoeing" => Self::Canoeing,
            "rowing" => Self::Rowing,
            "paddleboarding" => Self::Paddleboarding,
            "surfing" => Self::Surfing,
            "kitesurfing" => Self::Kitesurfing,
            "strength_training" => Self::StrengthTraining,
            "crossfit" => Self::Crossfit,
            "pilates" => Self::Pilates,
            "rock_climbing" => Self::RockClimbing,
            "trail_running" => Self::TrailRunning,
            "soccer" => Self::Soccer,
            "basketball" => Self::Basketball,
            "tennis" => Self::Tennis,
            "golf" => Self::Golf,
            "skateboarding" => Self::Skateboarding,
            "inline_skating" => Self::InlineSkating,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Get the human-readable name for this sport type
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Ride => "bike ride",
            Self::Swim => "swim",
            Self::Walk => "walk",
            Self::Hike => "hike",
            Self::VirtualRide => "indoor bike ride",
            Self::VirtualRun => "treadmill run",
            Self::Workout => "workout",
            Self::Yoga => "yoga session",
            Self::EbikeRide => "e-bike ride",
            Self::MountainBike => "mountain bike ride",
            Self::GravelRide => "gravel ride",
            Self::CrossCountrySkiing => "cross-country ski",
            Self::AlpineSkiing => "alpine ski",
            Self::Snowboarding => "snowboard session",
            Self::Snowshoe => "snowshoe hike",
            Self::IceSkating => "ice skating session",
            Self::BackcountrySkiing => "backcountry ski",
            Self::Kayaking => "kayak session",
            Self::Canoeing => "canoe trip",
            Self::Rowing => "rowing session",
            Self::Paddleboarding => "paddleboard session",
            Self::Surfing => "surf session",
            Self::Kitesurfing => "kitesurf session",
            Self::StrengthTraining => "strength training",
            Self::Crossfit => "CrossFit workout",
            Self::Pilates => "Pilates session",
            Self::RockClimbing => "climbing session",
            Self::TrailRunning => "trail run",
            Self::Soccer => "soccer game",
            Self::Basketball => "basketball game",
            Self::Tennis => "tennis match",
            Self::Golf => "golf round",
            Self::Skateboarding => "skate session",
            Self::InlineSkating => "inline skating",
            Self::Other(_name) => "activity", // Could use name but keeping generic
        }
    }
}
