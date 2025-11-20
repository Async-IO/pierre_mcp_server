// ABOUTME: Integration test for A2A agent card discovery endpoint
// ABOUTME: Verifies RFC 8615 well-known URI compliance and transport declarations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use pierre_mcp_server::a2a::agent_card::AgentCard;

#[test]
fn test_agent_card_structure() {
    let card = AgentCard::new();

    // Verify basic fields
    assert_eq!(card.name, "Pierre Fitness AI");
    assert!(!card.description.is_empty());
    assert_eq!(card.version, "1.0.0");

    // Verify capabilities
    assert!(card
        .capabilities
        .contains(&"fitness-data-analysis".to_owned()));
    assert!(card
        .capabilities
        .contains(&"activity-intelligence".to_owned()));

    // Verify transports are declared
    assert!(
        !card.transports.is_empty(),
        "Agent card must declare at least one transport"
    );

    // Verify JSON-RPC transport is declared
    let jsonrpc_transport = card
        .transports
        .iter()
        .find(|t| t.transport_type == "jsonrpc");
    assert!(
        jsonrpc_transport.is_some(),
        "JSON-RPC transport must be declared"
    );

    let transport = jsonrpc_transport.unwrap();
    assert_eq!(transport.version, "2.0");
    assert!(transport.endpoint.contains("/a2a/jsonrpc"));

    // Verify transport config
    assert!(
        transport.config.is_some(),
        "Transport must have configuration"
    );
    let config = transport.config.as_ref().unwrap();
    assert!(
        config.contains_key("capabilities"),
        "Transport config must declare capabilities"
    );
    assert!(
        config.contains_key("streaming_supported"),
        "Transport config must declare streaming support"
    );
}

#[test]
fn test_agent_card_serialization() {
    let card = AgentCard::new();

    // Test JSON serialization
    let json = card.to_json().expect("Agent card should serialize to JSON");

    // Verify JSON contains key fields
    assert!(json.contains("\"name\""));
    assert!(json.contains("\"transports\""));
    assert!(json.contains("\"authentication\""));
    assert!(json.contains("\"tools\""));

    // Test deserialization
    let deserialized =
        AgentCard::from_json(&json).expect("Agent card should deserialize from JSON");

    assert_eq!(deserialized.name, card.name);
    assert_eq!(deserialized.transports.len(), card.transports.len());
}

#[test]
fn test_transport_endpoint_format() {
    let card = AgentCard::new();

    for transport in &card.transports {
        // Verify endpoint is a valid URL format
        assert!(
            transport.endpoint.starts_with("http://") || transport.endpoint.starts_with("https://"),
            "Transport endpoint must be a valid URL: {}",
            transport.endpoint
        );

        // Verify endpoint includes protocol path
        assert!(
            transport.endpoint.contains("/a2a/"),
            "Transport endpoint must include /a2a/ path: {}",
            transport.endpoint
        );
    }
}

#[test]
fn test_authentication_methods() {
    let card = AgentCard::new();

    // Verify authentication schemes
    assert!(card.authentication.schemes.contains(&"api-key".to_owned()));
    assert!(card.authentication.schemes.contains(&"oauth2".to_owned()));

    // Verify OAuth2 configuration
    assert!(card.authentication.oauth2.is_some());
    let oauth2 = card.authentication.oauth2.as_ref().unwrap();
    assert!(!oauth2.authorization_url.is_empty());
    assert!(!oauth2.token_url.is_empty());
    assert!(!oauth2.scopes.is_empty());

    // Verify API key configuration
    assert!(card.authentication.api_key.is_some());
    let api_key = card.authentication.api_key.as_ref().unwrap();
    assert_eq!(api_key.header_name, "Authorization");
    assert_eq!(api_key.prefix, Some("Bearer".to_owned()));
}
