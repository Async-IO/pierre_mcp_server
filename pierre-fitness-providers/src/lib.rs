// ABOUTME: Pierre Fitness Providers - Private provider implementations
// ABOUTME: Strava, Garmin, and Synthetic providers for fitness data access

#[cfg(feature = "strava")]
pub mod strava;

#[cfg(feature = "garmin")]
pub mod garmin;

#[cfg(feature = "synthetic")]
pub mod synthetic;

#[cfg(feature = "strava")]
pub use strava::*;

#[cfg(feature = "garmin")]
pub use garmin::*;

#[cfg(feature = "synthetic")]
pub use synthetic::*;
