// ABOUTME: Fitness MCP tool handlers
// ABOUTME: Provider-agnostic handlers for fitness intelligence, goals, nutrition, and sleep

pub mod fitness_api;
pub mod intelligence;
pub mod goals;
pub mod nutrition;
pub mod sleep_recovery;
pub mod provider_helpers;

// Re-export commonly used types
pub use fitness_api::*;
pub use intelligence::*;
pub use goals::*;
pub use nutrition::*;
pub use sleep_recovery::*;
pub use provider_helpers::*;
