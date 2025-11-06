// ABOUTME: Cryptography module providing secure encryption and key management
// ABOUTME: Centralizes all cryptographic operations for the pierre_mcp_server
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Cryptographic utilities for Pierre MCP Server

/// Key management for A2A protocol
pub mod keys;

/// Re-export key management types
pub use keys::{A2AKeyManager, A2AKeypair, A2APublicKey};
