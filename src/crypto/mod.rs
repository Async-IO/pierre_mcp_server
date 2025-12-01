// ABOUTME: Cryptography module providing secure encryption and key management
// ABOUTME: Centralizes all cryptographic operations for the pierre_mcp_server
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Cryptographic utilities for Pierre MCP Server

/// Key management for A2A protocol
pub mod keys;

/// Re-export key management types
pub use keys::{A2AKeyManager, A2AKeypair, A2APublicKey};
