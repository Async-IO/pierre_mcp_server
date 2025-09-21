// ABOUTME: Tool handlers with single responsibilities
// ABOUTME: Clean separation of concerns replacing monolithic handler functions

pub mod configuration;
pub mod connections;
pub mod goals;
pub mod intelligence;
pub mod strava_api;

// Re-export handler functions for registry
pub use configuration::*;
pub use connections::*;
pub use goals::*;
pub use intelligence::*;
pub use strava_api::*;
