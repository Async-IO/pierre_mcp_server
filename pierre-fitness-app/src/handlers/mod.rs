// ABOUTME: Fitness MCP tool handlers
// ABOUTME: Provider-agnostic handlers for fitness intelligence, goals, nutrition, and sleep

pub mod fitness_api;
pub mod goals;
pub mod intelligence;
pub mod nutrition;
pub mod provider_helpers;
pub mod sleep_recovery;

// Re-export commonly used types
pub use fitness_api::*;
pub use goals::*;
pub use intelligence::*;
pub use nutrition::*;
pub use provider_helpers::*;
pub use sleep_recovery::*;
