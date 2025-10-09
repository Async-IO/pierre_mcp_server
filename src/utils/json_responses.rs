// ABOUTME: JSON response utilities to eliminate duplication across error and success responses
// ABOUTME: Provides standardized response builders for consistent API responses and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use serde_json::{json, Value};

/// Create a simple error response with just an error message
#[must_use]
pub fn simple_error(message: &str) -> Value {
    json!({ "error": message })
}

/// Create a simple success response with a message
#[must_use]
pub fn simple_success(message: &str) -> Value {
    json!({ "success": true, "message": message })
}

/// Create a detailed error response with error code and description
#[must_use]
pub fn detailed_error(error_code: &str, description: &str) -> Value {
    json!({
        "success": false,
        "error": error_code,
        "error_description": description
    })
}

/// Create a detailed success response with data
#[must_use]
pub fn detailed_success(message: &str, data: &Value) -> Value {
    json!({
        "success": true,
        "message": message,
        "data": data.clone() // Safe: JSON value ownership for response
    })
}

/// Create an OAuth/authentication error response
#[must_use]
pub fn oauth_error(error_type: &str, details: &str, provider: Option<&str>) -> Value {
    let mut response = json!({
        "error": error_type,
        "details": details
    });

    if let Some(p) = provider {
        response["provider"] = json!(p);
    }

    response
}

/// Create a validation error response with details
#[must_use]
pub fn validation_error(message: &str, details: &Value) -> Value {
    json!({
        "error": "validation_failed",
        "message": message,
        "details": details.clone() // Safe: JSON value ownership for error response
    })
}

/// Create a not found error response
#[must_use]
pub fn not_found_error(resource: &str) -> Value {
    json!({
        "error": "not_found",
        "message": format!("{resource} not found")
    })
}

/// Create an unauthorized error response
#[must_use]
pub fn unauthorized_error(message: &str) -> Value {
    json!({
        "error": "unauthorized",
        "message": message
    })
}

/// Create a service unavailable error response
#[must_use]
pub fn service_unavailable_error(service: &str, message: &str) -> Value {
    json!({
        "error": "service_unavailable",
        "service": service,
        "message": message,
        "is_real_data": false
    })
}

/// Create a connection/token error for fitness providers
#[must_use]
pub fn provider_connection_error(provider: &str, _message: &str) -> Value {
    json!({
        "error": format!("No {provider} token found for user - please connect your {provider} account first"),
        "is_real_data": false,
        "note": format!("Connect your {provider} account via the OAuth flow to get real data")
    })
}

/// Create a data serialization error response
#[must_use]
pub fn serialization_error(data_type: &str) -> Value {
    json!({
        "error": format!("Failed to serialize {data_type}"),
        "is_real_data": false
    })
}

/// Create a rate limit error response
#[must_use]
pub fn rate_limit_error(limit: u32, reset_time: Option<&str>) -> Value {
    let mut response = json!({
        "error": "rate_limit_exceeded",
        "message": "API rate limit exceeded",
        "limit": limit
    });

    if let Some(reset) = reset_time {
        response["reset_time"] = json!(reset);
    }

    response
}

/// Create an invalid format error response
#[must_use]
pub fn invalid_format_error(field: &str, expected_format: &str) -> Value {
    json!({
        "error": format!("Invalid {field} format. Use {expected_format}.")
    })
}

/// Create a registration failed error with detailed information
#[must_use]
pub fn registration_failed_error(error_description: &str, details: &str) -> Value {
    json!({
        "success": false,
        "error": "registration_failed",
        "error_description": error_description,
        "details": details
    })
}

/// Create a generic API error response
#[must_use]
pub fn api_error(message: &str) -> Value {
    json!({ "error": message.to_string() })
}

/// Create a success response for A2A client registration
#[must_use]
pub fn a2a_registration_success(
    client_id: &str,
    client_secret: &str,
    api_key: &str,
    public_key: &str,
    private_key: &str,
    key_type: &str,
) -> Value {
    json!({
        "success": true,
        "message": "A2A client registered successfully",
        "data": {
            "client_id": client_id,
            "client_secret": client_secret,
            "api_key": api_key,
            "public_key": public_key,
            "private_key": private_key,
            "key_type": key_type,
            "next_steps": {
                "documentation": "https://docs.pierre.ai/a2a",
                "authentication": "Use the provided credentials for A2A protocol authentication",
                "endpoints": {
                    "a2a_protocol": "/a2a/protocol",
                    "agent_card": "/.well-known/agent.json"
                }
            }
        }
    })
}

/// Create an activity not found error
#[must_use]
pub fn activity_not_found_error(activity_id: &str, provider: Option<&str>) -> Value {
    let message = provider.map_or_else(
        || "Activity not found".to_string(),
        |p| format!("Activity not found or user not connected to {p}"),
    );

    json!({
        "error": message,
        "activity_id": activity_id,
        "is_real_data": false
    })
}
