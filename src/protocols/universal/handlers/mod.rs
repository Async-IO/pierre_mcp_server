// ABOUTME: Tool handlers with single responsibilities
// ABOUTME: Clean separation of concerns replacing monolithic handler functions
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Configuration management tool handlers
pub mod configuration;
/// OAuth provider connection tool handlers
pub mod connections;

// Configuration management handlers
pub use configuration::{
    handle_calculate_personalized_zones, handle_get_configuration_catalog,
    handle_get_configuration_profiles, handle_get_user_configuration,
    handle_update_user_configuration, handle_validate_configuration,
};

// OAuth provider connection handlers

/// Connect to OAuth provider
pub use connections::handle_connect_provider;
/// Disconnect from OAuth provider
pub use connections::handle_disconnect_provider;
/// Get OAuth connection status
pub use connections::handle_get_connection_status;
