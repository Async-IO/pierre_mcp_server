// ABOUTME: Synthetic fitness data generator for automated intelligence testing
// ABOUTME: Creates realistic running, cycling, and swimming activities with configurable patterns
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Duration, Utc};
use pierre_mcp_server::models::{Activity, ActivityBuilder as ModelActivityBuilder, SportType};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

/// Builder for creating synthetic fitness activity data
///
/// Provides deterministic, reproducible generation of realistic fitness activities
/// for testing intelligence algorithms without requiring real OAuth connections.
///
/// # Examples
///
/// ```
/// use tests::synthetic_data::SyntheticDataBuilder;
/// use chrono::Utc;
///
/// let builder = SyntheticDataBuilder::new(42); // Deterministic seed
/// let activity = builder.generate_run()
///     .duration_minutes(30)
///     .distance_km(5.0)
///     .start_date(Utc::now())
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SyntheticDataBuilder {
    // Reserved for future algorithmic tests requiring seed reproducibility verification
    #[allow(dead_code)]
    seed: u64,
    rng: ChaCha8Rng,
}

impl SyntheticDataBuilder {
    /// Create new builder with deterministic seed for reproducibility
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Generate a synthetic running activity
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: uses &mut self.rng
    pub fn generate_run(&mut self) -> ActivityBuilder<'_> {
        ActivityBuilder::new(SportType::Run, &mut self.rng)
    }

    /// Generate a synthetic cycling activity
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: uses &mut self.rng
    pub fn generate_ride(&mut self) -> ActivityBuilder<'_> {
        ActivityBuilder::new(SportType::Ride, &mut self.rng)
    }

    /// Generate a synthetic swimming activity
    /// Reserved for future swimming-specific training pattern tests
    #[must_use]
    #[allow(dead_code, clippy::missing_const_for_fn)] // Cannot be const: uses &mut self.rng
    pub fn generate_swim(&mut self) -> ActivityBuilder<'_> {
        ActivityBuilder::new(SportType::Swim, &mut self.rng)
    }

    /// Generate a series of activities following a specific pattern
    #[must_use]
    pub fn generate_pattern(&mut self, pattern: TrainingPattern) -> Vec<Activity> {
        match pattern {
            TrainingPattern::BeginnerRunnerImproving => self.beginner_runner_improving(),
            TrainingPattern::ExperiencedCyclistConsistent => self.experienced_cyclist_consistent(),
            TrainingPattern::Overtraining => self.overtraining_scenario(),
            TrainingPattern::InjuryRecovery => self.injury_recovery(),
        }
    }

    /// Beginner runner improving 35% over 6 weeks
    /// Realistic progression for new runner building fitness
    fn beginner_runner_improving(&mut self) -> Vec<Activity> {
        let mut activities = Vec::new();
        let base_date = Utc::now() - Duration::days(42); // 6 weeks ago

        // Week 1-2: 3 runs/week, 20 min @ 6:30/km pace
        for week in 0..2 {
            for run in 0..3 {
                let date = base_date + Duration::days(week * 7 + run * 2);
                let activity = self
                    .generate_run()
                    .duration_minutes(20)
                    .pace_min_per_km(6.5)
                    .start_date(date)
                    .heart_rate(150, 165)
                    .build();
                activities.push(activity);
            }
        }

        // Week 3-4: 4 runs/week, 25 min @ 6:00/km pace (improving)
        for week in 2..4 {
            for run in 0..4 {
                let date = base_date + Duration::days(week * 7 + (run * 2));
                let activity = self
                    .generate_run()
                    .duration_minutes(25)
                    .pace_min_per_km(6.0)
                    .start_date(date)
                    .heart_rate(145, 160)
                    .build();
                activities.push(activity);
            }
        }

        // Week 5-6: 4 runs/week, 30 min @ 5:30/km pace (improved 35%)
        for week in 4..6 {
            for run in 0..4 {
                let date = base_date + Duration::days(week * 7 + (run * 2));
                let activity = self
                    .generate_run()
                    .duration_minutes(30)
                    .pace_min_per_km(5.5)
                    .start_date(date)
                    .heart_rate(140, 155)
                    .build();
                activities.push(activity);
            }
        }

        activities
    }

    /// Experienced cyclist with consistent performance
    /// No significant variations, stable FTP and power output
    fn experienced_cyclist_consistent(&mut self) -> Vec<Activity> {
        let mut activities = Vec::new();
        let base_date = Utc::now() - Duration::days(28); // 4 weeks ago

        // 5 rides per week, consistent metrics
        for week in 0..4 {
            for ride in 0..5 {
                let date = base_date + Duration::days(week * 7 + ride);

                // Mix of easy, tempo, and interval rides
                let (duration, power, hr) = match ride {
                    0 | 1 => (90, 180, 135), // Easy ride
                    2 => (60, 220, 155),     // Tempo ride
                    3 => (45, 250, 165),     // Interval ride
                    4 => (120, 170, 130),    // Long easy ride
                    _ => unreachable!(),
                };

                let activity = self
                    .generate_ride()
                    .duration_minutes(duration)
                    .average_power(power)
                    .ftp(250) // Consistent FTP
                    .start_date(date)
                    .heart_rate(hr, hr + 15)
                    .build();
                activities.push(activity);
            }
        }

        activities
    }

    /// Overtraining scenario - declining performance with high volume
    /// TSB drops below -30, performance degrades
    fn overtraining_scenario(&mut self) -> Vec<Activity> {
        let mut activities = Vec::new();
        let base_date = Utc::now() - Duration::days(21); // 3 weeks ago

        // Week 1: Normal volume, good performance
        for day in 0..6 {
            let date = base_date + Duration::days(day);
            let activity = self
                .generate_run()
                .duration_minutes(45)
                .pace_min_per_km(5.0)
                .start_date(date)
                .heart_rate(145, 160)
                .build();
            activities.push(activity);
        }

        // Week 2: Increased volume, slight performance drop
        for day in 7..14 {
            let date = base_date + Duration::days(day);
            let activity = self
                .generate_run()
                .duration_minutes(60)
                .pace_min_per_km(5.2) // Slightly slower
                .start_date(date)
                .heart_rate(150, 165) // Higher HR for same pace
                .build();
            activities.push(activity);
        }

        // Week 3: High volume, significant performance decline
        for day in 14..21 {
            let date = base_date + Duration::days(day);
            let activity = self
                .generate_run()
                .duration_minutes(75)
                .pace_min_per_km(5.8) // Much slower
                .start_date(date)
                .heart_rate(155, 170) // High HR, slow pace = overtraining
                .build();
            activities.push(activity);
        }

        activities
    }

    /// Injury recovery pattern - gap followed by gradual return
    /// Shows realistic return to training after injury
    fn injury_recovery(&mut self) -> Vec<Activity> {
        let mut activities = Vec::new();
        let base_date = Utc::now() - Duration::days(56); // 8 weeks ago

        // Week 1-2: Normal training before injury
        for day in 0..14 {
            if day % 2 == 0 {
                let date = base_date + Duration::days(day);
                let activity = self
                    .generate_run()
                    .duration_minutes(40)
                    .pace_min_per_km(5.0)
                    .start_date(date)
                    .heart_rate(145, 160)
                    .build();
                activities.push(activity);
            }
        }

        // Week 3-4: 2-week gap (injury)
        // No activities

        // Week 5-6: Gradual return, easy pace
        for day in 28..42 {
            if day % 3 == 0 {
                let date = base_date + Duration::days(day);
                let activity = self
                    .generate_run()
                    .duration_minutes(20)
                    .pace_min_per_km(6.5)
                    .start_date(date)
                    .heart_rate(135, 150)
                    .build();
                activities.push(activity);
            }
        }

        // Week 7-8: Building back up
        for day in 42..56 {
            if day % 2 == 0 {
                let date = base_date + Duration::days(day);
                let activity = self
                    .generate_run()
                    .duration_minutes(30)
                    .pace_min_per_km(5.5)
                    .start_date(date)
                    .heart_rate(140, 155)
                    .build();
                activities.push(activity);
            }
        }

        activities
    }
}

/// Training patterns for different scenarios
#[derive(Debug, Clone, Copy)]
pub enum TrainingPattern {
    /// Beginner runner showing 35% improvement over 6 weeks
    BeginnerRunnerImproving,
    /// Experienced cyclist with stable, consistent performance
    ExperiencedCyclistConsistent,
    /// Athlete showing signs of overtraining
    Overtraining,
    /// Return from injury with gradual progression
    InjuryRecovery,
}

/// Builder for individual activity construction
pub struct ActivityBuilder<'a> {
    sport_type: SportType,
    rng: &'a mut ChaCha8Rng,
    duration_seconds: Option<u64>,
    distance_meters: Option<f64>,
    start_date: Option<DateTime<Utc>>,
    average_heart_rate: Option<u32>,
    max_heart_rate: Option<u32>,
    average_power: Option<u32>,
    ftp: Option<u32>,
    pace_min_per_km: Option<f64>,
}

impl<'a> ActivityBuilder<'a> {
    const fn new(sport_type: SportType, rng: &'a mut ChaCha8Rng) -> Self {
        Self {
            sport_type,
            rng,
            duration_seconds: None,
            distance_meters: None,
            start_date: None,
            average_heart_rate: None,
            max_heart_rate: None,
            average_power: None,
            ftp: None,
            pace_min_per_km: None,
        }
    }

    /// Set activity duration in minutes
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Builder pattern methods
    pub fn duration_minutes(mut self, minutes: u64) -> Self {
        self.duration_seconds = Some(minutes * 60);
        self
    }

    /// Set distance in kilometers
    /// Reserved for future distance-specific activity generation tests
    #[must_use]
    #[allow(dead_code, clippy::missing_const_for_fn)] // Builder pattern methods
    pub fn distance_km(mut self, km: f64) -> Self {
        self.distance_meters = Some(km * 1000.0);
        self
    }

    /// Set pace in minutes per kilometer (for running)
    /// This will calculate appropriate distance based on duration
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Builder pattern methods
    pub fn pace_min_per_km(mut self, pace: f64) -> Self {
        self.pace_min_per_km = Some(pace);
        self
    }

    /// Set start date/time
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Builder pattern methods
    pub fn start_date(mut self, date: DateTime<Utc>) -> Self {
        self.start_date = Some(date);
        self
    }

    /// Set heart rate range (average, max)
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Builder pattern methods
    pub fn heart_rate(mut self, avg: u32, max: u32) -> Self {
        self.average_heart_rate = Some(avg);
        self.max_heart_rate = Some(max);
        self
    }

    /// Set average power (for cycling)
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Builder pattern methods
    pub fn average_power(mut self, watts: u32) -> Self {
        self.average_power = Some(watts);
        self
    }

    /// Set FTP (Functional Threshold Power for cycling)
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Builder pattern methods
    pub fn ftp(mut self, watts: u32) -> Self {
        self.ftp = Some(watts);
        self
    }

    /// Build the final Activity
    ///
    /// Complex builder method that constructs Activity with many fields requiring
    /// calculations and default values. The length is necessary for proper field initialization.
    #[must_use]
    #[allow(clippy::too_many_lines)] // Builder pattern requires comprehensive field initialization
    pub fn build(self) -> Activity {
        let id = format!("synthetic_{}", self.rng.gen::<u64>());
        let sport_type = self.sport_type; // Save before move
        let duration = self.duration_seconds.unwrap_or(1800); // Default 30 min

        // Calculate distance from pace if specified
        let distance = if let Some(pace) = self.pace_min_per_km {
            // Duration is in seconds (u64), precision loss acceptable for distance calculation
            #[allow(clippy::cast_precision_loss)]
            let duration_hours = duration as f64 / 3600.0;
            let speed_kmh = 60.0 / pace;
            Some(speed_kmh * duration_hours * 1000.0) // meters
        } else {
            self.distance_meters.or(
                // Default distances based on sport type
                match sport_type {
                    SportType::Ride => Some(25000.0),
                    SportType::Swim => Some(1500.0),
                    _ => Some(5000.0), // Run and other sport types default to 5km
                },
            )
        };

        // Calculate average speed if distance is known
        // Duration is seconds (u64), precision loss acceptable for speed calculation
        #[allow(clippy::cast_precision_loss)]
        let average_speed = distance.map(|d| d / duration as f64);

        // Generate realistic elevation for outdoor activities
        let elevation_gain = match sport_type {
            SportType::Run | SportType::Ride => {
                Some(distance.unwrap_or(5000.0) / 100.0 * self.rng.gen_range(0.5..2.0))
            }
            _ => None,
        };

        let mut builder = ModelActivityBuilder::new(
            id,
            format!("{sport_type:?} Activity"),
            sport_type.clone(),
            self.start_date.unwrap_or_else(Utc::now),
            duration,
            "synthetic",
        );

        if let Some(dist) = distance {
            builder = builder.distance_meters(dist);
        }
        if let Some(elev) = elevation_gain {
            builder = builder.elevation_gain(elev);
        }
        if let Some(hr) = self.average_heart_rate {
            builder = builder.average_heart_rate(hr);
        }
        if let Some(hr) = self.max_heart_rate {
            builder = builder.max_heart_rate(hr);
        }
        if let Some(speed) = average_speed {
            builder = builder.average_speed(speed);
        }
        if let Some(speed) = average_speed.map(|s| s * 1.15) {
            builder = builder.max_speed(speed);
        }

        // Duration in seconds, division by 60 for minutes, safe truncation
        #[allow(clippy::cast_possible_truncation)]
        let calories = (duration / 60) as u32 * 10; // Rough estimate
        builder = builder.calories(calories);

        // Duration in seconds, division by 60 for minutes, safe truncation
        #[allow(clippy::cast_possible_truncation)]
        let steps_value = (duration / 60) as u32 * 170; // ~170 steps/min

        if sport_type == SportType::Run {
            builder = builder.steps(steps_value);
        }

        if let Some(power) = self.average_power {
            builder = builder.average_power(power);
        }
        // Max power calculation: 30% increase from average, safe precision, truncation, and sign loss
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        if let Some(max_power) = self.average_power.map(|p| (f64::from(p) * 1.3) as u32) {
            builder = builder.max_power(max_power);
        }
        if let Some(norm_power) = self.average_power {
            builder = builder.normalized_power(norm_power);
        }
        if let Some(ftp_val) = self.ftp {
            builder = builder.ftp(ftp_val);
        }

        let avg_cadence = match sport_type {
            SportType::Run => Some(self.rng.gen_range(170..180)),
            SportType::Ride => Some(self.rng.gen_range(85..95)),
            _ => None,
        };
        if let Some(cadence) = avg_cadence {
            builder = builder.average_cadence(cadence);
        }

        builder = builder
            .temperature(self.rng.gen_range(10.0..25.0))
            .humidity(self.rng.gen_range(40.0..70.0))
            .average_altitude(self.rng.gen_range(100.0..500.0))
            .start_latitude(45.5017) // Montreal
            .start_longitude(-73.5673)
            .city("Montreal".to_owned())
            .region("Quebec".to_owned())
            .country("Canada".to_owned())
            .trail_name("Synthetic Training Route".to_owned())
            .sport_type_detail(format!("{sport_type:?}"));

        if sport_type == SportType::Run {
            builder = builder
                .ground_contact_time(self.rng.gen_range(200..250))
                .vertical_oscillation(self.rng.gen_range(7.0..10.0))
                .stride_length(self.rng.gen_range(1.1..1.4))
                .running_power(self.rng.gen_range(200..280))
                .workout_type(10); // Trail run
        }

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        let mut builder1 = SyntheticDataBuilder::new(42);
        let mut builder2 = SyntheticDataBuilder::new(42);

        let activity1 = builder1.generate_run().duration_minutes(30).build();
        let activity2 = builder2.generate_run().duration_minutes(30).build();

        // Same seed should produce identical activities
        assert_eq!(activity1.id(), activity2.id());
        assert_eq!(activity1.duration_seconds(), activity2.duration_seconds());
    }

    #[test]
    fn test_beginner_runner_pattern() {
        let mut builder = SyntheticDataBuilder::new(42);
        let activities = builder.generate_pattern(TrainingPattern::BeginnerRunnerImproving);

        // Should have activities across 6 weeks (3+3+4+4+4+4 = 22 runs)
        assert!(activities.len() >= 20 && activities.len() <= 24);

        // First activity should be slower than last (improvement)
        let first = &activities[0];
        let last = &activities[activities.len() - 1];

        if let (Some(first_speed), Some(last_speed)) = (first.average_speed(), last.average_speed())
        {
            assert!(last_speed > first_speed, "Last activity should be faster");
        }
    }

    #[test]
    fn test_consistent_cyclist_pattern() {
        let mut builder = SyntheticDataBuilder::new(42);
        let activities = builder.generate_pattern(TrainingPattern::ExperiencedCyclistConsistent);

        // Should have 5 rides/week * 4 weeks = 20 rides
        assert_eq!(activities.len(), 20);

        // All activities should be cycling
        assert!(activities
            .iter()
            .all(|a| *a.sport_type() == SportType::Ride));

        // Should have consistent FTP
        let ftp_values: Vec<_> = activities.iter().filter_map(Activity::ftp).collect();
        assert!(ftp_values.iter().all(|&ftp| ftp == 250));
    }

    #[test]
    fn test_overtraining_pattern() {
        let mut builder = SyntheticDataBuilder::new(42);
        let activities = builder.generate_pattern(TrainingPattern::Overtraining);

        // Should show declining performance (slower pace with higher HR)
        let week1_avg = f64::from(
            activities[0..6]
                .iter()
                .filter_map(Activity::average_heart_rate)
                .sum::<u32>(),
        ) / 6.0;

        let week3_avg = f64::from(
            activities[activities.len() - 7..]
                .iter()
                .filter_map(Activity::average_heart_rate)
                .sum::<u32>(),
        ) / 7.0;

        assert!(
            week3_avg > week1_avg,
            "Week 3 HR should be higher than Week 1"
        );
    }

    #[test]
    fn test_injury_recovery_has_gap() {
        let mut builder = SyntheticDataBuilder::new(42);
        let activities = builder.generate_pattern(TrainingPattern::InjuryRecovery);

        // Should have activities before and after injury
        assert!(activities.len() >= 15);

        // Check that there's a time gap (injury period)
        let dates: Vec<_> = activities.iter().map(Activity::start_date).collect();
        let mut max_gap = Duration::days(0);

        for i in 1..dates.len() {
            let gap = dates[i] - dates[i - 1];
            if gap > max_gap {
                max_gap = gap;
            }
        }

        // Should have at least a 10-day gap (injury period)
        assert!(max_gap.num_days() >= 10, "Should have injury gap");
    }
}
