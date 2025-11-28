// ABOUTME: Converters from Terra API models to Pierre unified data models
// ABOUTME: Maps Terra's standardized JSON schemas to Activity, SleepSession, HealthMetrics, etc.
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Terra to Pierre model converters
//!
//! This module provides conversion functions from Terra's standardized data models
//! to Pierre's unified fitness data models.

use crate::models::{
    Activity, Athlete, FoodItem, HealthMetrics, MealEntry, MealType, NutritionLog, RecoveryMetrics,
    SleepSession, SleepStage, SleepStageType, SportType,
};
use chrono::Utc;

use super::constants::{
    TERRA_ACTIVITY_ALPINE_SKI, TERRA_ACTIVITY_BASKETBALL, TERRA_ACTIVITY_CROSSFIT,
    TERRA_ACTIVITY_CROSS_COUNTRY_SKI, TERRA_ACTIVITY_EBIKE_RIDE, TERRA_ACTIVITY_GOLF,
    TERRA_ACTIVITY_GRAVEL_RIDE, TERRA_ACTIVITY_HIKE, TERRA_ACTIVITY_INDOOR_CYCLING,
    TERRA_ACTIVITY_INDOOR_RUN, TERRA_ACTIVITY_INLINE_SKATING, TERRA_ACTIVITY_KAYAKING,
    TERRA_ACTIVITY_MOUNTAIN_BIKE, TERRA_ACTIVITY_OPEN_WATER_SWIM, TERRA_ACTIVITY_PADDLEBOARD,
    TERRA_ACTIVITY_PILATES, TERRA_ACTIVITY_POOL_SWIM, TERRA_ACTIVITY_RIDE,
    TERRA_ACTIVITY_ROCK_CLIMBING, TERRA_ACTIVITY_ROWING, TERRA_ACTIVITY_RUN,
    TERRA_ACTIVITY_SKATEBOARDING, TERRA_ACTIVITY_SNOWBOARD, TERRA_ACTIVITY_SNOWSHOE,
    TERRA_ACTIVITY_SOCCER, TERRA_ACTIVITY_STRENGTH_TRAINING, TERRA_ACTIVITY_SURFING,
    TERRA_ACTIVITY_SWIM, TERRA_ACTIVITY_TENNIS, TERRA_ACTIVITY_TRAIL_RUN, TERRA_ACTIVITY_TREADMILL,
    TERRA_ACTIVITY_WALK, TERRA_ACTIVITY_YOGA, TERRA_SLEEP_STAGE_AWAKE, TERRA_SLEEP_STAGE_DEEP,
    TERRA_SLEEP_STAGE_REM,
};
use super::models::{
    TerraActivity, TerraAthlete, TerraBody, TerraDaily, TerraNutrition, TerraSleep,
    TerraSleepStage, TerraUser,
};

/// Converter utilities for Terra to Pierre models
pub struct TerraConverters;

/// Extracted activity metrics from Terra data
struct ActivityMetrics {
    distance_meters: Option<f64>,
    elevation_gain: Option<f64>,
    steps: Option<u32>,
    calories: Option<u32>,
    average_heart_rate: Option<u32>,
    max_heart_rate: Option<u32>,
    hrv_score: Option<f64>,
    average_speed: Option<f64>,
    max_speed: Option<f64>,
    average_cadence: Option<u32>,
    max_cadence: Option<u32>,
    average_power: Option<u32>,
    max_power: Option<u32>,
    normalized_power: Option<u32>,
    training_stress_score: Option<f32>,
    intensity_factor: Option<f32>,
    ftp: Option<u32>,
    spo2: Option<f32>,
    average_altitude: Option<f32>,
    suffer_score: Option<u32>,
    start_latitude: Option<f64>,
    start_longitude: Option<f64>,
}

impl TerraConverters {
    /// Extract activity metrics from Terra activity data
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn extract_activity_metrics(terra: &TerraActivity) -> ActivityMetrics {
        let distance_data = terra.distance_data.as_ref();
        let hr_data = terra.heart_rate_data.as_ref();
        let movement = terra.movement_data.as_ref();
        let power = terra.power_data.as_ref();
        let tss = terra.tss_data.as_ref();
        let oxygen = terra.oxygen_data.as_ref();
        let position = terra.position_data.as_ref();

        let (start_latitude, start_longitude) = position
            .and_then(|p| p.start_pos_lat_lng_deg)
            .map_or((None, None), |[lat, lng]| (Some(lat), Some(lng)));

        ActivityMetrics {
            distance_meters: distance_data.and_then(|d| d.distance_meters),
            elevation_gain: distance_data.and_then(|d| d.elevation_gain_metres),
            steps: distance_data.and_then(|d| d.steps).map(|s| s as u32),
            calories: terra
                .calories_data
                .as_ref()
                .and_then(|c| c.total_burned_calories)
                .map(|c| c as u32),
            average_heart_rate: hr_data.and_then(|h| h.avg_hr_bpm).map(|h| h as u32),
            max_heart_rate: hr_data.and_then(|h| h.max_hr_bpm).map(|h| h as u32),
            hrv_score: hr_data.and_then(|h| h.avg_hrv_rmssd),
            average_speed: movement.and_then(|m| m.avg_speed_metres_per_second),
            max_speed: movement.and_then(|m| m.max_speed_metres_per_second),
            average_cadence: movement.and_then(|m| m.avg_cadence).map(|c| c as u32),
            max_cadence: movement.and_then(|m| m.max_cadence).map(|c| c as u32),
            average_power: power.and_then(|p| p.avg_watts).map(|w| w as u32),
            max_power: power.and_then(|p| p.max_watts).map(|w| w as u32),
            normalized_power: power.and_then(|p| p.normalized_watts).map(|w| w as u32),
            training_stress_score: tss.and_then(|t| t.tss).map(|t| t as f32),
            intensity_factor: tss.and_then(|t| t.intensity_factor).map(|i| i as f32),
            ftp: tss.and_then(|t| t.ftp).map(|f| f as u32),
            spo2: oxygen
                .and_then(|o| o.avg_saturation_percentage)
                .map(|s| s as f32),
            average_altitude: distance_data
                .and_then(|d| d.avg_elevation_metres)
                .map(|e| e as f32),
            suffer_score: terra
                .strain_data
                .as_ref()
                .and_then(|s| s.strain_level)
                .map(|s| s as u32),
            start_latitude,
            start_longitude,
        }
    }

    /// Convert Terra activity to Pierre `Activity`
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn activity_from_terra(terra: &TerraActivity, terra_user: &TerraUser) -> Activity {
        let metadata = terra.metadata.as_ref();
        let provider_name = terra_user.provider.as_ref().map_or_else(
            || "terra".to_owned(),
            |p| format!("terra:{}", p.to_lowercase()),
        );

        let start_date = metadata.and_then(|m| m.start_time).unwrap_or_else(Utc::now);
        let end_time = metadata.and_then(|m| m.end_time);

        let duration_seconds = end_time
            .map(|end| (end - start_date).num_seconds().unsigned_abs())
            .or_else(|| {
                terra
                    .active_durations_data
                    .as_ref()
                    .and_then(|d| d.activity_seconds)
                    .map(|s| s as u64)
            })
            .unwrap_or(0);

        let metrics = Self::extract_activity_metrics(terra);
        let sport_type =
            Self::map_terra_activity_type(metadata.and_then(|m| m.activity_type).unwrap_or(0));

        Activity {
            id: metadata
                .and_then(|m| m.summary_id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: metadata
                .and_then(|m| m.name.clone())
                .unwrap_or_else(|| sport_type.display_name().to_owned()),
            sport_type,
            start_date,
            duration_seconds,
            distance_meters: metrics.distance_meters,
            elevation_gain: metrics.elevation_gain,
            average_heart_rate: metrics.average_heart_rate,
            max_heart_rate: metrics.max_heart_rate,
            average_speed: metrics.average_speed,
            max_speed: metrics.max_speed,
            calories: metrics.calories,
            steps: metrics.steps,
            heart_rate_zones: None,
            average_power: metrics.average_power,
            max_power: metrics.max_power,
            normalized_power: metrics.normalized_power,
            power_zones: None,
            ftp: metrics.ftp,
            average_cadence: metrics.average_cadence,
            max_cadence: metrics.max_cadence,
            hrv_score: metrics.hrv_score,
            recovery_heart_rate: None,
            temperature: None,
            humidity: None,
            average_altitude: metrics.average_altitude,
            wind_speed: None,
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            running_power: None,
            breathing_rate: None,
            spo2: metrics.spo2,
            training_stress_score: metrics.training_stress_score,
            intensity_factor: metrics.intensity_factor,
            suffer_score: metrics.suffer_score,
            time_series_data: None,
            start_latitude: metrics.start_latitude,
            start_longitude: metrics.start_longitude,
            city: metadata.and_then(|m| m.city.clone()),
            region: None,
            country: metadata.and_then(|m| m.country.clone()),
            trail_name: None,
            workout_type: None,
            sport_type_detail: metadata.and_then(|m| m.name.clone()),
            segment_efforts: None,
            provider: provider_name,
        }
    }

    /// Convert Terra sleep data to Pierre `SleepSession`
    ///
    /// Casts are validated by physiological constraints (sleep times in minutes, etc.)
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn sleep_from_terra(terra: &TerraSleep, terra_user: &TerraUser) -> SleepSession {
        let metadata = terra.metadata.as_ref();
        let provider_name = terra_user.provider.as_ref().map_or_else(
            || "terra".to_owned(),
            |p| format!("terra:{}", p.to_lowercase()),
        );

        let start_time = metadata.and_then(|m| m.start_time).unwrap_or_else(Utc::now);
        let end_time = metadata.and_then(|m| m.end_time).unwrap_or_else(Utc::now);

        let durations = terra.sleep_durations_data.as_ref();

        // Calculate times in minutes
        let in_bed_seconds = durations
            .and_then(|d| d.in_bed_seconds)
            .unwrap_or_else(|| (end_time - start_time).num_seconds() as f64);
        let time_in_bed = (in_bed_seconds / 60.0) as u32;

        let asleep_seconds = durations.and_then(|d| d.asleep_seconds).unwrap_or(0.0);
        let total_sleep_time = (asleep_seconds / 60.0) as u32;

        // Calculate efficiency
        let sleep_efficiency = if time_in_bed > 0 {
            (total_sleep_time as f32 / time_in_bed as f32) * 100.0
        } else {
            durations
                .and_then(|d| d.sleep_efficiency)
                .map_or(0.0, |e| e as f32)
        };

        // Convert sleep stages
        let stages = durations
            .and_then(|d| d.sleep_stages.as_ref())
            .map(|stages| {
                stages
                    .iter()
                    .filter_map(Self::convert_sleep_stage)
                    .collect()
            })
            .unwrap_or_default();

        // Extract HR/HRV during sleep
        let hr_data = terra.heart_rate_data.as_ref();
        let hrv_during_sleep = hr_data.and_then(|h| h.avg_hrv_rmssd);

        // Extract respiratory rate
        let respiratory_rate = terra
            .respiration_data
            .as_ref()
            .and_then(|r| r.avg_breaths_per_minute)
            .map(|r| r as f32);

        // Extract temperature variation
        let temperature_variation = terra
            .temperature_data
            .as_ref()
            .and_then(|t| t.delta_temperature_celsius)
            .map(|t| t as f32);

        // Extract readiness as sleep score
        let sleep_score = terra
            .readiness_data
            .as_ref()
            .and_then(|r| r.recovery_score.or(r.readiness_score))
            .map(|s| s as f32);

        SleepSession {
            id: metadata
                .and_then(|m| m.summary_id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            start_time,
            end_time,
            time_in_bed,
            total_sleep_time,
            sleep_efficiency,
            sleep_score,
            stages,
            hrv_during_sleep,
            respiratory_rate,
            temperature_variation,
            wake_count: durations.and_then(|d| d.num_awakenings).map(|n| n as u32),
            sleep_onset_latency: durations
                .and_then(|d| d.sleep_latency_seconds)
                .map(|s| (s / 60.0) as u32),
            provider: provider_name,
        }
    }

    /// Convert Terra sleep stage to Pierre `SleepStage`
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn convert_sleep_stage(terra_stage: &TerraSleepStage) -> Option<SleepStage> {
        let start_time = terra_stage.start_time?;
        let end_time = terra_stage.end_time?;
        let stage_type = Self::map_terra_sleep_stage(terra_stage.stage.unwrap_or(0));
        let duration_minutes = ((end_time - start_time).num_seconds() / 60) as u32;

        Some(SleepStage {
            stage_type,
            start_time,
            duration_minutes,
        })
    }

    /// Map Terra sleep stage code to Pierre `SleepStageType`
    ///
    /// Terra sleep stage codes:
    /// - 1: Awake
    /// - 2: Light sleep (default for unknown)
    /// - 3: Deep sleep
    /// - 4: REM sleep
    const fn map_terra_sleep_stage(stage: i32) -> SleepStageType {
        match stage {
            TERRA_SLEEP_STAGE_AWAKE => SleepStageType::Awake,
            TERRA_SLEEP_STAGE_DEEP => SleepStageType::Deep,
            TERRA_SLEEP_STAGE_REM => SleepStageType::Rem,
            // TERRA_SLEEP_STAGE_LIGHT (2) and unknown values default to Light
            _ => SleepStageType::Light,
        }
    }

    /// Convert Terra body data to Pierre `HealthMetrics`
    ///
    /// Casts are validated by physiological constraints
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn health_from_terra(terra: &TerraBody, terra_user: &TerraUser) -> HealthMetrics {
        let metadata = terra.metadata.as_ref();
        let measurements = terra.measurements_data.as_ref();
        let provider_name = terra_user.provider.as_ref().map_or_else(
            || "terra".to_owned(),
            |p| format!("terra:{}", p.to_lowercase()),
        );

        let date = metadata.and_then(|m| m.start_time).unwrap_or_else(Utc::now);

        HealthMetrics {
            date,
            weight: measurements.and_then(|m| m.weight_kg),
            body_fat_percentage: measurements
                .and_then(|m| m.body_fat_percentage)
                .map(|f| f as f32),
            muscle_mass: measurements.and_then(|m| m.muscle_mass_kg),
            bone_mass: measurements.and_then(|m| m.bone_mass_kg),
            body_water_percentage: measurements
                .and_then(|m| m.body_water_percentage)
                .map(|w| w as f32),
            bmr: measurements.and_then(|m| m.bmr).map(|b| b as u32),
            blood_pressure: measurements.and_then(|m| {
                match (m.blood_pressure_systolic, m.blood_pressure_diastolic) {
                    (Some(sys), Some(dia)) => Some((sys as u32, dia as u32)),
                    _ => None,
                }
            }),
            blood_glucose: measurements
                .and_then(|m| m.blood_glucose_mg_per_dl)
                .map(|g| g as f32),
            vo2_max: None,
            provider: provider_name,
        }
    }

    /// Convert Terra daily data to Pierre `RecoveryMetrics`
    ///
    /// Casts are validated by score ranges (0-100)
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn recovery_from_terra_daily(
        terra: &TerraDaily,
        terra_user: &TerraUser,
    ) -> RecoveryMetrics {
        let metadata = terra.metadata.as_ref();
        let provider_name = terra_user.provider.as_ref().map_or_else(
            || "terra".to_owned(),
            |p| format!("terra:{}", p.to_lowercase()),
        );

        let date = metadata.and_then(|m| m.start_time).unwrap_or_else(Utc::now);

        let scores = terra.scores.as_ref();
        let stress = terra.stress_data.as_ref();
        let hr_data = terra.heart_rate_data.as_ref();

        RecoveryMetrics {
            date,
            recovery_score: scores.and_then(|s| s.recovery_score).map(|r| r as f32),
            readiness_score: scores.and_then(|s| s.activity_score).map(|a| a as f32),
            hrv_status: None,
            sleep_score: scores.and_then(|s| s.sleep_score).map(|s| s as f32),
            stress_level: stress.and_then(|s| s.avg_stress_level).map(|l| l as f32),
            training_load: None,
            resting_heart_rate: hr_data.and_then(|h| h.resting_hr_bpm).map(|r| r as u32),
            body_temperature: None,
            resting_respiratory_rate: None,
            provider: provider_name,
        }
    }

    /// Convert Terra sleep readiness to Pierre `RecoveryMetrics`
    ///
    /// Casts are validated by score ranges and physiological constraints
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn recovery_from_terra_sleep(
        terra: &TerraSleep,
        terra_user: &TerraUser,
    ) -> RecoveryMetrics {
        let metadata = terra.metadata.as_ref();
        let provider_name = terra_user.provider.as_ref().map_or_else(
            || "terra".to_owned(),
            |p| format!("terra:{}", p.to_lowercase()),
        );

        let date = metadata.and_then(|m| m.end_time).unwrap_or_else(Utc::now);
        let readiness = terra.readiness_data.as_ref();
        let hr_data = terra.heart_rate_data.as_ref();
        let temp = terra.temperature_data.as_ref();

        RecoveryMetrics {
            date,
            recovery_score: readiness.and_then(|r| r.recovery_score).map(|s| s as f32),
            readiness_score: readiness.and_then(|r| r.readiness_score).map(|s| s as f32),
            hrv_status: readiness.and_then(|r| r.hrv_balance).map(|h| {
                if h > 0.0 {
                    "above_baseline".to_owned()
                } else if h < 0.0 {
                    "below_baseline".to_owned()
                } else {
                    "normal".to_owned()
                }
            }),
            sleep_score: readiness.and_then(|r| r.sleep_balance).map(|s| s as f32),
            stress_level: None,
            training_load: readiness.and_then(|r| r.activity_balance).map(|a| a as f32),
            resting_heart_rate: hr_data.and_then(|h| h.resting_hr_bpm).map(|r| r as u32),
            body_temperature: temp
                .and_then(|t| t.delta_temperature_celsius)
                .map(|d| d as f32),
            resting_respiratory_rate: terra
                .respiration_data
                .as_ref()
                .and_then(|r| r.avg_breaths_per_minute)
                .map(|r| r as f32),
            provider: provider_name,
        }
    }

    /// Convert Terra nutrition data to Pierre `NutritionLog`
    #[must_use]
    pub fn nutrition_from_terra(terra: &TerraNutrition, terra_user: &TerraUser) -> NutritionLog {
        let metadata = terra.metadata.as_ref();
        let summary = terra.summary.as_ref();
        let provider_name = terra_user.provider.as_ref().map_or_else(
            || "terra".to_owned(),
            |p| format!("terra:{}", p.to_lowercase()),
        );

        let date = metadata.and_then(|m| m.start_time).unwrap_or_else(Utc::now);

        // Convert meals
        let meals = terra
            .meals
            .as_ref()
            .map(|meals| {
                meals
                    .iter()
                    .map(|m| {
                        let macros = m.macros.as_ref();
                        MealEntry {
                            meal_type: m
                                .name
                                .as_ref()
                                .map_or(MealType::Other, |n| MealType::from_str_lossy(n)),
                            timestamp: m.timestamp,
                            name: m.name.clone(),
                            calories: macros.and_then(|ma| ma.calories),
                            protein_g: macros.and_then(|ma| ma.protein_g),
                            carbohydrates_g: macros.and_then(|ma| ma.carbohydrates_g),
                            fat_g: macros.and_then(|ma| ma.fat_g),
                            food_items: m
                                .food_items
                                .as_ref()
                                .map(|items| {
                                    items
                                        .iter()
                                        .filter_map(|item| {
                                            item.name.as_ref().map(|name| {
                                                let item_macros = item.macros.as_ref();
                                                FoodItem {
                                                    name: name.clone(),
                                                    brand: item.brand.clone(),
                                                    serving_size: item.serving_size,
                                                    serving_unit: item.serving_unit.clone(),
                                                    servings: item.servings,
                                                    calories: item.calories,
                                                    protein_g: item_macros
                                                        .and_then(|m| m.protein_g),
                                                    carbohydrates_g: item_macros
                                                        .and_then(|m| m.carbohydrates_g),
                                                    fat_g: item_macros.and_then(|m| m.fat_g),
                                                }
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        NutritionLog {
            id: metadata
                .and_then(|m| m.summary_id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            date,
            total_calories: summary.and_then(|s| s.calories),
            protein_g: summary.and_then(|s| s.protein_g),
            carbohydrates_g: summary.and_then(|s| s.carbohydrates_g),
            fat_g: summary.and_then(|s| s.fat_g),
            fiber_g: summary.and_then(|s| s.fiber_g),
            sugar_g: summary.and_then(|s| s.sugar_g),
            sodium_mg: summary.and_then(|s| s.sodium_mg),
            water_ml: summary.and_then(|s| s.water_ml),
            meals,
            provider: provider_name,
        }
    }

    /// Convert Terra user to Pierre `Athlete`
    #[must_use]
    pub fn athlete_from_terra(terra: &TerraAthlete) -> Athlete {
        Athlete {
            id: terra.user_id.clone(),
            username: terra
                .reference_id
                .clone()
                .unwrap_or_else(|| terra.user_id.clone()),
            firstname: terra.first_name.clone(),
            lastname: terra.last_name.clone(),
            profile_picture: None,
            provider: format!("terra:{}", terra.provider.to_lowercase()),
        }
    }

    /// Map Terra activity type code to Pierre `SportType`
    ///
    /// Terra activity types: <https://docs.tryterra.co/reference/activity-types>
    #[must_use]
    pub const fn map_terra_activity_type(activity_type: i32) -> SportType {
        match activity_type {
            // Running variants (1-4)
            // Both outdoor and indoor running map to Run
            TERRA_ACTIVITY_RUN | TERRA_ACTIVITY_INDOOR_RUN => SportType::Run,
            TERRA_ACTIVITY_TRAIL_RUN => SportType::TrailRunning,
            TERRA_ACTIVITY_TREADMILL => SportType::VirtualRun,

            // Cycling variants (5-9)
            TERRA_ACTIVITY_RIDE => SportType::Ride,
            TERRA_ACTIVITY_INDOOR_CYCLING => SportType::VirtualRide,
            TERRA_ACTIVITY_MOUNTAIN_BIKE => SportType::MountainBike,
            TERRA_ACTIVITY_GRAVEL_RIDE => SportType::GravelRide,
            TERRA_ACTIVITY_EBIKE_RIDE => SportType::EbikeRide,

            // Swimming (10-12) - all variants map to Swim
            TERRA_ACTIVITY_SWIM | TERRA_ACTIVITY_POOL_SWIM | TERRA_ACTIVITY_OPEN_WATER_SWIM => {
                SportType::Swim
            }

            // Walking/Hiking (13-14)
            TERRA_ACTIVITY_WALK => SportType::Walk,
            TERRA_ACTIVITY_HIKE => SportType::Hike,

            // Winter sports (15-18)
            TERRA_ACTIVITY_CROSS_COUNTRY_SKI => SportType::CrossCountrySkiing,
            TERRA_ACTIVITY_ALPINE_SKI => SportType::AlpineSkiing,
            TERRA_ACTIVITY_SNOWBOARD => SportType::Snowboarding,
            TERRA_ACTIVITY_SNOWSHOE => SportType::Snowshoe,

            // Water sports (19-22)
            TERRA_ACTIVITY_ROWING => SportType::Rowing,
            TERRA_ACTIVITY_KAYAKING => SportType::Kayaking,
            TERRA_ACTIVITY_PADDLEBOARD => SportType::Paddleboarding,
            TERRA_ACTIVITY_SURFING => SportType::Surfing,

            // Gym/fitness (30-33)
            TERRA_ACTIVITY_STRENGTH_TRAINING => SportType::StrengthTraining,
            TERRA_ACTIVITY_CROSSFIT => SportType::Crossfit,
            TERRA_ACTIVITY_YOGA => SportType::Yoga,
            TERRA_ACTIVITY_PILATES => SportType::Pilates,

            // Team sports (40-43)
            TERRA_ACTIVITY_SOCCER => SportType::Soccer,
            TERRA_ACTIVITY_BASKETBALL => SportType::Basketball,
            TERRA_ACTIVITY_TENNIS => SportType::Tennis,
            TERRA_ACTIVITY_GOLF => SportType::Golf,

            // Other activities (50-52)
            TERRA_ACTIVITY_ROCK_CLIMBING => SportType::RockClimbing,
            TERRA_ACTIVITY_SKATEBOARDING => SportType::Skateboarding,
            TERRA_ACTIVITY_INLINE_SKATING => SportType::InlineSkating,

            // Unknown activity types default to generic Workout
            _ => SportType::Workout,
        }
    }
}
